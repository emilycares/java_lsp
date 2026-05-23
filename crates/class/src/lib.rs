#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
//! A minimal class file parser.
//! Skips parsing data not used by `java_lsp`

use std::str::from_utf8;

use dto::{
    Access, Class, ClassParserError, ClassSignature, ImportUnit, JType, Parameter,
    SourceDestination, SuperClass,
};
use my_string::{
    MyString,
    smol_str::{SmolStr, SmolStrBuilder, StrExt, ToSmolStr},
};

const U8_LEN: usize = 1;
const U16_LEN: usize = 2;
const U32_LEN: usize = 4;
const U64_LEN: usize = 8;

pub fn load_class(
    data: &[u8],
    class_path: MyString,
    source: SourceDestination,
    filter: bool,
) -> Result<Class, ClassParserError> {
    let (c, _) = parser_base(data, 0)?;

    if filter && !c.access_flags.intersects(Access::Public) {
        return Err(ClassParserError::Ignoring);
    }

    let name = lookup_class_name(&c, c.this_class.into())?;

    let mut used_classes = Vec::new();
    let mut methods = Vec::new();
    let mut fields = Vec::new();
    let mut deprecated = false;
    let mut class_signature = None;

    for a in &c.attributes {
        if a.name == 0 {
            continue;
        }
        let attribute_name = lookup_string(&c, a.name)?;

        if attribute_name == "Signature" {
            let info = a.lookup(data)?;
            let (sig, _) = get_u16(info, 0)?;
            let sig = lookup_string(&c, sig)?;
            let (sig, _) = parse_class_signature_info(sig)?;
            class_signature = Some(sig);
        } else if attribute_name == "Code" {
            let info = a.lookup(data)?;
            let (out, _) = parse_code_attribute(info, 0, a.start, a.end)?;
            parse_used_classes(&c, data, &out, &mut used_classes)?;
        } else if attribute_name == "Deprecated" {
            if filter {
                return Err(ClassParserError::Ignoring);
            }
            deprecated = true;
        }
    }

    for m in &c.methods {
        let method = parse_method(&c, data, m, filter);
        if matches!(method, Err(ClassParserError::Ignoring)) {
            continue;
        }
        let (method, code_attribute) = method?;
        if let Some(code_attribute) = code_attribute {
            parse_used_classes(&c, data, &code_attribute, &mut used_classes)?;
            for p in &method.parameters {
                if matches!(
                    p.jtype,
                    JType::Class(_) | JType::Array(_) | JType::Generic(_, _)
                ) {
                    jtype_class_names(p.jtype.clone(), &mut used_classes);
                }
            }
        }
        jtype_class_names(method.ret.clone(), &mut used_classes);
        methods.push(method);
    }
    for f in &c.fields {
        let field = parse_field(&c, f, filter);
        if matches!(field, Err(ClassParserError::Ignoring)) {
            continue;
        }
        let field = field?;
        jtype_class_names(field.jtype.clone(), &mut used_classes);
        fields.push(field);
    }

    let package = class_path
        .trim_end_matches(name.as_str())
        .trim_end_matches('.');
    let mut imports = vec![ImportUnit::Package(package.into())];
    imports.extend(
        used_classes
            .into_iter()
            .filter(|i| *i != class_path)
            .map(ImportUnit::Class),
    );

    let super_interfaces: Vec<_> = c
        .interfaces
        .iter()
        .filter(|i| i != &&0)
        .map(|index| {
            lookup_string(&c, *index).map_or(SuperClass::None, |i| {
                SuperClass::ClassPath(i.replace_smolstr("/", "."))
            })
        })
        .collect();

    let mut super_class = SuperClass::None;
    if c.super_class != 0 {
        let a = lookup_string(&c, c.super_class)?;
        if a != "java/lang/Object" {
            super_class = SuperClass::ClassPath(a.replace_smolstr("/", "."));
        }
    }

    imports.dedup();

    Ok(Class {
        source,
        class_path,
        super_interfaces,
        super_class,
        imports,
        access: parse_class_access(c.access_flags, deprecated),
        signature: class_signature,
        name,
        methods,
        fields,
    })
}

fn lookup_class_name(c: &Base, index: usize) -> Result<MyString, ClassParserError> {
    match c.const_pool.pool.get(index.saturating_sub(1)) {
        Some(ConstEntry::Class { name }) => Ok(lookup_string(c, *name)?
            .split('/')
            .next_back()
            .map(Into::into)
            .ok_or(ClassParserError::InvalidName)?),
        _ => Err(ClassParserError::ExpectedString),
    }
}

fn parse_field(c: &Base, field: &Field, filter: bool) -> Result<dto::Field, ClassParserError> {
    if filter
        && field
            .access_flags
            .intersects(Access::Private | Access::Protected)
    {
        return Err(ClassParserError::Ignoring);
    }
    Ok(dto::Field {
        access: field.access_flags.clone(),
        name: lookup_string(c, field.name)?.to_smolstr(),
        jtype: parse_field_type(lookup_string(c, field.descriptor)?.as_bytes(), 0)?.0,
        source: None,
    })
}

fn parse_method(
    c: &Base,
    data: &[u8],
    method: &Method,
    filter: bool,
) -> Result<(dto::Method, Option<CodeAttribute>), ClassParserError> {
    if filter
        && method
            .access_flags
            .intersects(Access::Private | Access::Protected)
    {
        return Err(ClassParserError::Ignoring);
    }
    let lname = lookup_string(c, method.name)?;

    if lname.starts_with("lambda$") {
        return Err(ClassParserError::Ignoring);
    }

    let mut code_attribute = None;

    let mut parameters = Vec::new();
    let mut parameter_names = Vec::new();
    let mut throws = Vec::new();
    let mut deprecated = false;
    let mut signature_index = None;
    let mut method_parameter_index = None;
    let mut exception_index = None;
    let mut ret = JType::Void;

    for (index, attribute) in method.attributes.iter().enumerate() {
        let name = lookup_string(c, attribute.name)?;
        if name == "Signature" {
            signature_index = Some(index);
        } else if name == "MethodParameters" {
            method_parameter_index = Some(index);
        } else if name == "Exceptions" {
            exception_index = Some(index);
        } else if name == "Deprecated" {
            if filter {
                return Err(ClassParserError::Ignoring);
            }
            deprecated = true;
        } else if name == "Code" {
            let info = attribute.lookup(data)?;
            let (ca, _) = parse_code_attribute(info, 0, attribute.start, attribute.end)?;
            code_attribute = Some(ca);
        }
    }
    let no_parameter_names_and_signature =
        method_parameter_index.is_none() && signature_index.is_none();
    if no_parameter_names_and_signature {
        let smol_str = lookup_string(c, method.descriptor)?;
        let (_, md) = parse_method_descriptor(smol_str)?;
        ret = md.return_type;
        for p in md.param_types {
            parameters.push(Parameter {
                name: None,
                jtype: p,
            });
        }
    }

    if let Some(index) = method_parameter_index {
        let attribute = method
            .attributes
            .get(index)
            .ok_or(ClassParserError::InvalidAttributeIndex)?;

        let info = attribute.lookup(data)?;
        let (info, _) = parse_method_parameters_attribute(info, 0)?;
        if signature_index.is_some() {
            for p in info {
                let name = lookup_string(c, p.name_index)
                    .ok()
                    .filter(|i| !i.is_empty());
                parameter_names.push(name);
            }
        } else {
            let (_, md) = parse_method_descriptor(lookup_string(c, method.descriptor)?)?;
            ret = md.return_type;
            let mut params = md.param_types.into_iter();
            for p in info {
                let jtype = params.next().ok_or(ClassParserError::NotEnogthParams)?;
                if p.name_index == 0 {
                    parameters.push(Parameter { name: None, jtype });
                } else {
                    let name = lookup_string(c, p.name_index)
                        .ok()
                        .filter(|i| !i.is_empty())
                        .map(SmolStr::new);
                    parameters.push(Parameter { name, jtype });
                }
            }
        }
    }

    if let Some(index) = signature_index {
        let attribute = method
            .attributes
            .get(index)
            .ok_or(ClassParserError::InvalidAttributeIndex)?;

        let info = attribute.lookup(data)?;
        let (sig, _) = get_u16(info, 0)?;
        let sig = lookup_string(c, sig)?;
        let (sig, _) = parse_method_signature_info(sig)?;
        let mut name_iter = parameter_names.into_iter();
        parameters.extend(sig.params.into_iter().map(|jtype| Parameter {
            name: name_iter.next().flatten().map(SmolStr::new),
            jtype,
        }));

        ret = sig.ret;
    }

    if let Some(index) = exception_index {
        let attribute = method
            .attributes
            .get(index)
            .ok_or(ClassParserError::InvalidAttributeIndex)?;
        let info = attribute.lookup(data)?;
        let (info, _) = parse_exceptions_attribute(info, 0)?;

        for exception in info {
            let class_name = lookup_string(c, exception)?;
            throws.push(JType::Class(class_name.replace_smolstr("/", ".")));
        }
    }

    let name = if lname == "<init>" {
        None
    } else {
        Some(SmolStr::new(lname))
    };
    Ok((
        dto::Method {
            access: parse_method_access(method, deprecated),
            name,
            parameters,
            ret,
            throws,
            source: None,
        },
        code_attribute,
    ))
}

fn parse_exceptions_attribute(
    data: &[u8],
    pos: usize,
) -> Result<(Vec<u16>, usize), ClassParserError> {
    let (count, pos) = get_u16(data, pos)?;
    let mut out = Vec::with_capacity(count as usize);
    let mut pos = pos;
    for _ in 0..count {
        let (o, npos) = get_u16(data, pos)?;
        out.push(o);
        pos = npos;
    }
    Ok((out, pos))
}

struct MethodParametersAttribute {
    name_index: u16,
}

fn parse_method_parameters_attribute(
    data: &[u8],
    pos: usize,
) -> Result<(Vec<MethodParametersAttribute>, usize), ClassParserError> {
    let (count, pos) = get_u8(data, pos)?;
    let mut pos = pos;
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let (o, npos) = parse_method_parameters_attribute_inner(data, pos)?;
        out.push(o);
        pos = npos;
    }
    Ok((out, pos))
}
fn parse_method_parameters_attribute_inner(
    data: &[u8],
    pos: usize,
) -> Result<(MethodParametersAttribute, usize), ClassParserError> {
    let (name_index, pos) = get_u16(data, pos)?;
    let pos = pos + 2;
    Ok((MethodParametersAttribute { name_index }, pos))
}

#[derive(Debug)]
#[allow(dead_code)]
struct MethodSignature {
    pub args: Vec<MyString>,
    pub params: Vec<JType>,
    pub ret: JType,
}
fn parse_method_signature_info(sig: &str) -> Result<(MethodSignature, usize), ClassParserError> {
    let content = sig.as_bytes();
    let mut pos = 0;
    let mut args = Vec::new();
    let mut params = Vec::new();
    if let Ok(npos) = assert_char(content, pos, b'<') {
        pos = npos;
        loop {
            if let Ok(npos) = assert_char(content, pos, b';') {
                pos = npos;
            }
            if let Ok(npos) = assert_char(content, pos, b'>') {
                pos = npos;
                break;
            }
            let mut arg = SmolStrBuilder::new();
            loop {
                let v = content.get(pos).ok_or(ClassParserError::EOF)?;
                if *v == b':' {
                    break;
                }
                let i = u32::from(*v);
                let c = char::from_u32(i).ok_or(ClassParserError::GenericParameterName)?;
                arg.push(c);
                pos += 1;
            }
            args.push(arg.finish());
            let npos = assert_char(content, pos, b':')?;
            pos = npos;
            if let Ok(npos) = assert_char(content, pos, b':') {
                pos = npos;
            }
            let (_, npos) = parse_field_type(content, pos)?;
            pos = npos;
        }
    }
    let mut pos = assert_char(content, pos, b'(')?;
    if let Ok(npos) = assert_char(content, pos, b')') {
        pos = npos;
    } else {
        loop {
            if let Ok(npos) = assert_char(content, pos, b';') {
                pos = npos;
            }
            if let Ok(npos) = assert_char(content, pos, b')') {
                pos = npos;
                break;
            }
            let (ty, npos) = parse_field_type(content, pos)?;
            params.push(ty);
            pos = npos;
        }
    }
    let (ret, pos) = parse_field_type(content, pos)?;
    Ok((MethodSignature { args, params, ret }, pos))
}
fn parse_class_signature_info(sig: &str) -> Result<(ClassSignature, usize), ClassParserError> {
    let content = sig.as_bytes();
    let mut pos = 0;
    let mut args = Vec::new();
    if let Ok(npos) = assert_char(content, pos, b'<') {
        pos = npos;
        loop {
            if let Ok(npos) = assert_char(content, pos, b';') {
                pos = npos;
            }
            if let Ok(npos) = assert_char(content, pos, b'>') {
                pos = npos;
                break;
            }
            let mut arg = SmolStrBuilder::new();
            loop {
                let v = content.get(pos).ok_or(ClassParserError::EOF)?;
                if *v == b':' {
                    break;
                }
                let c =
                    char::from_u32(u32::from(*v)).ok_or(ClassParserError::GenericParameterName)?;
                arg.push(c);
                pos += 1;
            }
            args.push(arg.finish());
            let npos = assert_char(content, pos, b':')?;
            pos = npos;
            if let Ok(npos) = assert_char(content, pos, b':') {
                pos = npos;
            }
            let (_, npos) = parse_field_type(content, pos)?;
            pos = npos;
        }
    }
    let (ret, pos) = parse_field_type(content, pos)?;
    Ok((ClassSignature { args, ret }, pos))
}

fn assert_char(content: &[u8], pos: usize, p: u8) -> Result<usize, ClassParserError> {
    let Some(c) = content.get(pos) else {
        return Err(ClassParserError::EOF);
    };

    if *c != p {
        // let expected = char::from_u32(p as u32);
        // let got = char::from_u32(*c as u32);
        // eprintln!("expected: {expected:?}, got: {got:?}");
        return Err(ClassParserError::ExpectedOther);
    }

    Ok(pos + 1)
}

struct CodeAttribute {
    start: usize,
    end: usize,
    pub attributes: Vec<Attribute>,
}
impl CodeAttribute {
    pub fn lookup<'a>(&'a self, data: &'a [u8]) -> Result<&'a [u8], ClassParserError> {
        data.get(self.start..self.end).ok_or(ClassParserError::EOF)
    }
}

fn parse_code_attribute(
    data: &[u8],
    pos: usize,
    start: usize,
    end: usize,
) -> Result<(CodeAttribute, usize), ClassParserError> {
    let mut pos = pos;
    pos = pos.saturating_add(U16_LEN + U16_LEN);

    let (code_length, pos) = get_u32(data, pos)?;
    let pos = pos.saturating_add(code_length as usize);

    let (exception_table_length, pos) = get_u16(data, pos)?;
    let exception_table = (exception_table_length as usize).saturating_mul(8);
    let pos = pos.saturating_add(exception_table);

    let (attributes, pos) = parse_attributes(data, pos)?;

    Ok((
        CodeAttribute {
            start,
            end,
            attributes,
        },
        pos,
    ))
}

/// Returns list of descriptors
fn parse_local_variable_table_attribute(
    data: &[u8],
    pos: usize,
) -> Result<(Vec<u16>, usize), ClassParserError> {
    let (table_length, pos) = get_u16(data, pos)?;
    let mut out = Vec::with_capacity(table_length as usize);
    let mut pos = pos;

    for _ in 0..table_length {
        pos += 3 * U16_LEN;
        let (descriptor_index, npos) = get_u16(data, pos)?;
        pos = npos;
        out.push(descriptor_index);
        // skip u16
        pos += U16_LEN;
    }

    Ok((out, pos))
}

fn parse_used_classes(
    c: &Base,
    data: &[u8],
    code_attribute: &CodeAttribute,
    used_classes: &mut Vec<SmolStr>,
) -> Result<(), ClassParserError> {
    let info = code_attribute.lookup(data)?;

    for attribute in &code_attribute.attributes {
        let attribute_name = lookup_string(c, attribute.name)?;
        if attribute_name == "LocalVariableTable" {
            let info = attribute.lookup(info)?;
            let (descriptors, _) = parse_local_variable_table_attribute(info, 0)?;
            for f in descriptors {
                let field_desc = lookup_string(c, f)?;
                let (field_desc, _) = parse_field_type(field_desc.as_bytes(), 0)?;
                jtype_class_names(field_desc, used_classes);
            }
        }
    }
    Ok(())
}

fn jtype_class_names(i: JType, used_classes: &mut Vec<SmolStr>) {
    match i {
        JType::Class(class) => {
            used_classes.push(class);
        }
        JType::Array(jtype) => jtype_class_names(*jtype, used_classes),
        JType::Generic(class, jtypes) => {
            for j in jtypes {
                jtype_class_names(j, used_classes);
            }
            used_classes.push(class);
        }
        _ => (),
    }
}

#[derive(Debug)]
struct MethodDescriptor {
    param_types: Vec<JType>,
    return_type: JType,
}

fn parse_method_descriptor(
    descriptor: &str,
) -> Result<(usize, MethodDescriptor), ClassParserError> {
    let content = descriptor.as_bytes();
    let pos = 0;
    if let Ok((pos, param_types)) = parse_param_types(content, pos) {
        let (return_type, pos) = parse_field_type(content, pos)?;
        return Ok((
            pos,
            MethodDescriptor {
                param_types,
                return_type,
            },
        ));
    }
    let (return_type, pos) = parse_field_type(content, pos)?;
    Ok((
        pos,
        MethodDescriptor {
            param_types: Vec::new(),
            return_type,
        },
    ))
}

fn parse_param_types(content: &[u8], pos: usize) -> Result<(usize, Vec<JType>), ClassParserError> {
    let pos = assert_char(content, pos, b'(')?;
    let mut pos = pos;
    let mut out = Vec::new();
    loop {
        if let Ok(npos) = assert_char(content, pos, b')') {
            pos = npos;
            break;
        }
        let (filed_type, npos) = parse_field_type(content, pos)?;
        out.push(filed_type);
        pos = npos;
    }
    Ok((pos, out))
}

fn parse_field_type(content: &[u8], pos: usize) -> Result<(JType, usize), ClassParserError> {
    let c = content.get(pos).ok_or(ClassParserError::EOF)?;
    match c {
        b'B' => Ok((JType::Byte, pos + 1)),
        b'C' => Ok((JType::Char, pos + 1)),
        b'D' => Ok((JType::Double, pos + 1)),
        b'F' => Ok((JType::Float, pos + 1)),
        b'I' => Ok((JType::Int, pos + 1)),
        b'J' => Ok((JType::Long, pos + 1)),
        b'S' => Ok((JType::Short, pos + 1)),
        b'Z' => Ok((JType::Boolean, pos + 1)),
        b'V' => Ok((JType::Void, pos + 1)),
        b'T' => {
            let mut pos = pos + 1;
            let mut param = SmolStrBuilder::new();
            loop {
                let v = content.get(pos).ok_or(ClassParserError::EOF)?;
                if *v == b';' {
                    break;
                }
                let c =
                    char::from_u32(u32::from(*v)).ok_or(ClassParserError::GenericParameterName)?;
                param.push(c);
                pos += 1;
            }
            Ok((JType::Parameter(param.finish()), pos))
        }
        b'L' => {
            let pos = pos + 1;
            let (mut pos, mut out) = parse_jtype_class_name(content, pos)?;
            while let Some(next) = content.get(pos)
                && next == &b'.'
            {
                let (npos, inner) = parse_jtype_class_name(content, pos + 1)?;
                pos = npos;
                out = JType::Access {
                    base: Box::new(out),
                    inner: Box::new(inner),
                }
            }
            Ok((out, pos))
        }
        b'[' => {
            let (inner, npos) = parse_field_type(content, pos + 1)?;
            Ok((JType::Array(Box::new(inner)), npos))
        }
        _ => {
            // let got = char::from_u32(u32::from(*c));
            Err(ClassParserError::UnknownType)
        }
    }
}

fn parse_jtype_class_name(
    content: &[u8],
    mut pos: usize,
) -> Result<(usize, JType), ClassParserError> {
    let mut class_name = SmolStrBuilder::new();
    let mut args = Vec::new();
    while let Some(c) = content.get(pos) {
        if c == &b'<' {
            pos += 1;
            if let Ok(npos) = assert_char(content, pos, b'+') {
                pos = npos;
            }
            loop {
                let mut star = false;
                if let Ok(npos) = assert_char(content, pos, b'>') {
                    pos = npos;
                    break;
                }
                if let Ok(npos) = assert_char(content, pos, b'*') {
                    // any
                    pos = npos;
                    star = true;
                }
                if let Ok(npos) = assert_char(content, pos, b'-') {
                    // supper
                    pos = npos;
                }
                if let Ok(npos) = assert_char(content, pos, b'+') {
                    // extends
                    pos = npos;
                }
                if star {
                    if let Ok(npos) = assert_char(content, pos, b'*') {
                        pos = npos;
                        continue;
                    }
                    if let Ok(npos) = assert_char(content, pos, b'>') {
                        pos = npos;
                        break;
                    }
                    if let Ok((arg, npos)) = parse_field_type(content, pos) {
                        args.push(arg);
                        pos = npos;
                        if let Ok(npos) = assert_char(content, pos, b';') {
                            pos = npos;
                        }
                    }
                    continue;
                }
                let (arg, npos) = parse_field_type(content, pos)?;
                args.push(arg);
                pos = npos;
                if let Ok(npos) = assert_char(content, pos, b';') {
                    pos = npos;
                }
            }

            break;
        }
        if c == &b';' {
            pos += 1;
            break;
        }
        class_name.push(*c as char);
        pos += 1;
    }
    let class_name = class_name.finish().replace_smolstr("/", ".");
    if !args.is_empty() {
        return Ok((pos, JType::Generic(class_name, args)));
    }
    Ok((pos, JType::Class(class_name)))
}

#[derive(Debug)]
pub struct ModuleInfo {
    pub exports: Vec<MyString>,
}

pub fn load_module(data: &[u8]) -> Result<ModuleInfo, ClassParserError> {
    let (c, _) = parser_base(data, 0)?;
    for a in &c.attributes {
        let name = lookup_string(&c, a.name)?;
        if name == "Module" {
            let info = a.lookup(data)?;
            let (module, _) = parse_module_attribute(info, 0)?;
            let module_name = lookup_string(&c, module.name_index)?;
            let mut exports = vec![module_name.replace('.', "/").to_smolstr()];
            for e in module.exports {
                if !e.exports_to_index.is_empty() {
                    continue;
                }
                let name = lookup_string(&c, e.exports_index)?;
                exports.push(name.replace('.', "/").to_smolstr());
            }
            return Ok(ModuleInfo { exports });
        }
    }
    Err(ClassParserError::NoModuleAttribute)
}

struct ModuleAttribute {
    name_index: u16,
    exports: Vec<ModuleExportsAttribute>,
}

fn parse_module_attribute(
    data: &[u8],
    pos: usize,
) -> Result<(ModuleAttribute, usize), ClassParserError> {
    let (name_index, pos) = get_u16(data, pos)?;
    let pos = pos + (U16_LEN + U16_LEN);

    let (requires_count, pos) = get_u16(data, pos)?;
    let pos = pos.saturating_add((requires_count as usize).saturating_mul(6));

    let (exports_count, pos) = get_u16(data, pos)?;
    let mut exports = Vec::with_capacity(exports_count as usize);
    let mut pos = pos;
    for _ in 0..exports_count {
        let (o, npos) = parse_module_exports(data, pos)?;
        exports.push(o);
        pos = npos;
    }

    Ok((
        ModuleAttribute {
            name_index,
            exports,
        },
        pos,
    ))
}
struct ModuleExportsAttribute {
    exports_index: u16,
    exports_to_index: Vec<u16>,
}

fn parse_module_exports(
    data: &[u8],
    pos: usize,
) -> Result<(ModuleExportsAttribute, usize), ClassParserError> {
    let (exports_index, pos) = get_u16(data, pos)?;
    let pos = pos + U16_LEN;
    let (count, pos) = get_u16(data, pos)?;
    let mut exports_to_index = Vec::with_capacity(count as usize);
    let mut pos = pos;

    for _ in 0..count {
        let (o, npos) = get_u16(data, pos)?;
        exports_to_index.push(o);
        pos = npos;
    }
    Ok((
        ModuleExportsAttribute {
            exports_index,
            exports_to_index,
        },
        pos,
    ))
}

fn parse_class_access(flags: Access, deprecated: bool) -> Access {
    let mut access = flags;
    if deprecated {
        access.insert(Access::Deprecated);
    }
    access
}

fn parse_method_access(method: &Method, deprecated: bool) -> Access {
    let mut access = method.access_flags.clone();
    if deprecated {
        access.insert(Access::Deprecated);
    }
    access
}

fn lookup_string(c: &Base, index: u16) -> Result<&str, ClassParserError> {
    lookup_string_inner(c, index, 0)
}

fn lookup_string_inner(c: &Base, index: u16, depth: u8) -> Result<&str, ClassParserError> {
    if depth == 5 {
        return Err(ClassParserError::NameRecursion);
    }
    if index == 0 {
        return Err(ClassParserError::StringIndexZero);
    }
    let con = &c.const_pool.pool.get((index - 1) as usize);
    match con {
        Some(ConstEntry::Utf8(utf8)) => Ok(utf8),
        Some(
            ConstEntry::Module { name } | ConstEntry::Package { name } | ConstEntry::Class { name },
        ) => lookup_string_inner(c, *name, depth + 1),
        _ => Err(ClassParserError::ExpectedString),
    }
}

struct Base {
    pub const_pool: ConstPool,
    pub access_flags: Access,
    pub this_class: u16,
    pub super_class: u16,

    pub interfaces: Vec<u16>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub attributes: Vec<Attribute>,
}

struct Field {
    pub access_flags: Access,
    pub name: u16,
    pub descriptor: u16,
}
struct Method {
    pub access_flags: Access,
    pub name: u16,
    pub descriptor: u16,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug)]
struct Attribute {
    pub name: u16,
    pub start: usize,
    pub end: usize,
}

impl Attribute {
    pub fn lookup<'a>(&'a self, data: &'a [u8]) -> Result<&'a [u8], ClassParserError> {
        data.get(self.start..self.end).ok_or(ClassParserError::EOF)
    }
}

fn parser_base(data: &[u8], pos: usize) -> Result<(Base, usize), ClassParserError> {
    let pos = expect_data(data, pos, &[0xCA, 0xFE, 0xBA, 0xBE])
        .map_err(|_| ClassParserError::NotAClass)?;

    let pos = pos + (U16_LEN + U16_LEN);

    let (const_pool, pos) = parse_const_pool(data, pos)?;

    let (access_flags, pos) = parse_class_access_flags(data, pos)?;
    let (this_class, pos) = get_u16(data, pos)?;
    let (super_class, pos) = get_u16(data, pos)?;

    let (interfaces, pos) = parse_interfaces(data, pos)?;
    let (fields, pos) = parse_fields(data, pos)?;
    let (methods, pos) = parse_methods(data, pos)?;
    let (attributes, pos) = parse_attributes(data, pos)?;

    Ok((
        Base {
            const_pool,

            access_flags,
            this_class,
            super_class,

            interfaces,
            fields,
            methods,
            attributes,
        },
        pos,
    ))
}

fn parse_class_access_flags(data: &[u8], pos: usize) -> Result<(Access, usize), ClassParserError> {
    let (flags, pos) = get_u16(data, pos)?;
    let mut out = Access::empty();

    if (flags & 0x0001) != 0 {
        out |= Access::Public;
    }
    if (flags & 0x0010) != 0 {
        out |= Access::Final;
    }
    if (flags & 0x0020) != 0 {
        out |= Access::Super;
    }
    if (flags & 0x0200) != 0 {
        out |= Access::Interface;
    }
    if (flags & 0x0400) != 0 {
        out |= Access::Abstract;
    }
    if (flags & 0x4000) != 0 {
        out |= Access::Enum;
    }

    Ok((out, pos))
}

fn parse_fields(data: &[u8], pos: usize) -> Result<(Vec<Field>, usize), ClassParserError> {
    let (size, pos) = get_u16(data, pos)?;
    let mut pos = pos;
    let mut out = Vec::with_capacity(size as usize);

    for _ in 0..size {
        let (field, npos) = parse_class_field(data, pos)?;
        pos = npos;
        out.push(field);
    }

    Ok((out, pos))
}

fn parse_class_field(data: &[u8], pos: usize) -> Result<(Field, usize), ClassParserError> {
    let (access_flags, pos) = parse_field_access_flags(data, pos)?;
    let (name, pos) = get_u16(data, pos)?;
    let (descriptor, pos) = get_u16(data, pos)?;
    let (_, pos) = parse_attributes(data, pos)?;
    Ok((
        Field {
            access_flags,
            name,
            descriptor,
        },
        pos,
    ))
}

fn parse_field_access_flags(data: &[u8], pos: usize) -> Result<(Access, usize), ClassParserError> {
    let (flags, pos) = get_u16(data, pos)?;
    let mut out = Access::empty();
    if (flags & 0x0001) != 0 {
        out |= Access::Public;
    }
    if (flags & 0x0002) != 0 {
        out |= Access::Private;
    }
    if (flags & 0x0004) != 0 {
        out |= Access::Protected;
    }
    if (flags & 0x0008) != 0 {
        out |= Access::Static;
    }
    if (flags & 0x0010) != 0 {
        out |= Access::Final;
    }
    if (flags & 0x0040) != 0 {
        out |= Access::Volatile;
    }
    if (flags & 0x0080) != 0 {
        out |= Access::Transient;
    }
    if (flags & 0x1000) != 0 {
        out |= Access::Synthetic;
    }
    if (flags & 0x4000) != 0 {
        out |= Access::Enum;
    }

    Ok((out, pos))
}
fn parse_methods(data: &[u8], pos: usize) -> Result<(Vec<Method>, usize), ClassParserError> {
    let (size, pos) = get_u16(data, pos)?;
    let mut pos = pos;
    let mut out = Vec::with_capacity(size as usize);

    for _ in 0..size {
        let (field, npos) = parse_class_method(data, pos)?;
        pos = npos;
        out.push(field);
    }

    Ok((out, pos))
}
fn parse_class_method(data: &[u8], pos: usize) -> Result<(Method, usize), ClassParserError> {
    let (access_flags, pos) = parse_method_access_flags(data, pos)?;
    let (name, pos) = get_u16(data, pos)?;
    let (descriptor, pos) = get_u16(data, pos)?;
    let (attributes, pos) = parse_attributes(data, pos)?;
    Ok((
        Method {
            access_flags,
            name,
            descriptor,
            attributes,
        },
        pos,
    ))
}

fn parse_method_access_flags(data: &[u8], pos: usize) -> Result<(Access, usize), ClassParserError> {
    let (flags, pos) = get_u16(data, pos)?;
    let mut out = Access::empty();
    if (flags & 0x0001) != 0 {
        out |= Access::Public;
    }
    if (flags & 0x0002) != 0 {
        out |= Access::Private;
    }
    if (flags & 0x0004) != 0 {
        out |= Access::Protected;
    }
    if (flags & 0x0008) != 0 {
        out |= Access::Static;
    }
    if (flags & 0x0010) != 0 {
        out |= Access::Final;
    }
    if (flags & 0x0400) != 0 {
        out |= Access::Abstract;
    }

    Ok((out, pos))
}

fn parse_attributes(data: &[u8], pos: usize) -> Result<(Vec<Attribute>, usize), ClassParserError> {
    let (size, pos) = get_u16(data, pos)?;
    let mut pos = pos;
    let mut out = Vec::with_capacity(size as usize);

    for _ in 0..size {
        let (attribute, npos) = parse_attribute(data, pos)?;
        pos = npos;
        out.push(attribute);
    }

    Ok((out, pos))
}
fn parse_attribute(data: &[u8], pos: usize) -> Result<(Attribute, usize), ClassParserError> {
    let (name, pos) = get_u16(data, pos)?;
    let (length, pos) = get_u32(data, pos)?;
    let start = pos;
    let end = pos.saturating_add(length as usize);
    Ok((Attribute { name, start, end }, end))
}

fn parse_interfaces(data: &[u8], pos: usize) -> Result<(Vec<u16>, usize), ClassParserError> {
    let (size, pos) = get_u16(data, pos)?;
    let mut pos = pos;
    let mut out = Vec::with_capacity(size as usize);

    for _ in 0..size {
        let (interface, npos) = get_u16(data, pos)?;
        pos = npos;
        out.push(interface);
    }

    Ok((out, pos))
}

pub struct ConstPool {
    pub pool: Vec<ConstEntry>,
}

fn parse_const_pool(data: &[u8], pos: usize) -> Result<(ConstPool, usize), ClassParserError> {
    let (size, pos) = get_u16(data, pos)?;
    let size = size.saturating_sub(1) as usize;
    let mut pool = Vec::with_capacity(size);
    let mut pos = pos;

    let mut idx = 0;

    while idx < size {
        let (n, npos, len) = parse_constant(data, pos)?;
        pos = npos;
        pool.push(n);

        if len == 2 {
            pool.push(ConstEntry::Empty);
        }

        idx += len;
    }
    Ok((ConstPool { pool }, pos))
}

#[derive(Debug)]
pub enum ConstEntry {
    Empty,
    Utf8(SmolStr),
    String { name: u16 },
    Module { name: u16 },
    Package { name: u16 },
    Class { name: u16 },
    MethodRef,
    NameAndType,
    InterfaceMethodRef,
    FieldRef,
    Dynamic,
    InvokeDynamic,
    Float,
    Double,
    Integer,
    Long,
    MehthodHandle,
    MethodType,
    RuntimeString,
}

/// Returns the constant, next pos, how meany slots the constant takes
fn parse_constant(data: &[u8], pos: usize) -> Result<(ConstEntry, usize, usize), ClassParserError> {
    let (kind, pos) = get_u8(data, pos)?;

    match kind {
        1 => parse_utf8_const(data, pos),
        3 => Ok(parse_integer_const(pos)),
        4 => Ok(parse_float_const(pos)),
        5 => Ok(parse_long_const(pos)),
        6 => Ok(parse_double_const(pos)),
        7 => parse_class_const(data, pos),
        8 => parse_string_const(data, pos),
        9 => Ok(parse_field_ref_const(pos)),
        10 => Ok(parse_method_ref_const(pos)),
        11 => Ok(parse_interface_method_ref_const(pos)),
        12 => Ok(parse_name_and_type_const(pos)),
        15 => Ok(parse_method_handle_const(pos)),
        16 => Ok(parse_method_type_const(pos)),
        17 => Ok(parse_dynamic_const(pos)),
        18 => Ok(parse_invoke_dynamic_const(pos)),
        19 => parse_module_const(data, pos),
        20 => parse_package_const(data, pos),
        _ => Err(ClassParserError::UnknownConstant),
    }
}

fn parse_utf8_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), ClassParserError> {
    let (len, pos) = get_u16(data, pos)?;
    let len = len as usize;
    let end = pos + len;
    let inner = data.get(pos..end).ok_or(ClassParserError::EOF)?;
    let mu = mutf8::mutf8_to_utf8(inner).map_err(|_| ClassParserError::Mutf8)?;
    if let Ok(mu) = from_utf8(&mu) {
        return Ok((ConstEntry::Utf8(SmolStr::new(mu)), end, 1));
    }
    Ok((ConstEntry::RuntimeString, end, 1))
}

fn parse_class_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), ClassParserError> {
    let (name, pos) = get_u16(data, pos)?;
    Ok((ConstEntry::Class { name }, pos, 1))
}
fn parse_string_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), ClassParserError> {
    let (name, pos) = get_u16(data, pos)?;
    Ok((ConstEntry::String { name }, pos, 1))
}
fn parse_module_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), ClassParserError> {
    let (name, pos) = get_u16(data, pos)?;
    Ok((ConstEntry::Module { name }, pos, 1))
}
fn parse_package_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), ClassParserError> {
    let (name, pos) = get_u16(data, pos)?;
    Ok((ConstEntry::Package { name }, pos, 1))
}
const fn parse_field_ref_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (ConstEntry::FieldRef, pos + (U16_LEN + U16_LEN), 1)
}

const fn parse_method_ref_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (ConstEntry::MethodRef, pos + (U16_LEN + U16_LEN), 1)
}
const fn parse_integer_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content u32
    (ConstEntry::Integer, pos + U32_LEN, 1)
}
const fn parse_float_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content u32
    (ConstEntry::Float, pos + U32_LEN, 1)
}
const fn parse_long_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content u64
    (ConstEntry::Long, pos + U64_LEN, 2)
}
const fn parse_double_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content u64
    (ConstEntry::Double, pos + U64_LEN, 2)
}
const fn parse_interface_method_ref_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (ConstEntry::InterfaceMethodRef, pos + (U16_LEN + U16_LEN), 1)
}
const fn parse_name_and_type_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (ConstEntry::NameAndType, pos + (U16_LEN + U16_LEN), 1)
}
const fn parse_dynamic_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (ConstEntry::Dynamic, pos + (U16_LEN + U16_LEN), 1)
}
const fn parse_method_handle_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (ConstEntry::MehthodHandle, pos + (U8_LEN + U16_LEN), 1)
}
const fn parse_method_type_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 1 * u16
    (ConstEntry::MethodType, pos + U16_LEN, 1)
}
const fn parse_invoke_dynamic_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (ConstEntry::InvokeDynamic, pos + (U16_LEN + U16_LEN), 1)
}

fn get_u8(data: &[u8], pos: usize) -> Result<(u8, usize), ClassParserError> {
    let Some(get) = data.get(pos) else {
        return Err(ClassParserError::EOF);
    };

    Ok((*get, pos + 1))
}
fn get_u16(data: &[u8], pos: usize) -> Result<(u16, usize), ClassParserError> {
    let next = pos + 2;
    let items = data.get(pos..next).ok_or(ClassParserError::EOF)?;
    let get = <[u8; 2]>::try_from(items).map_err(|_| ClassParserError::Number)?;
    let out = u16::from_be_bytes(get);

    Ok((out, next))
}
fn get_u32(data: &[u8], pos: usize) -> Result<(u32, usize), ClassParserError> {
    let next = pos + 4;
    let items = data.get(pos..next).ok_or(ClassParserError::EOF)?;
    let get = <[u8; 4]>::try_from(items).map_err(|_| ClassParserError::Number)?;
    let out = u32::from_be_bytes(get);

    Ok((out, next))
}

#[track_caller]
#[inline]
fn expect_data(data: &[u8], pos: usize, expected: &[u8]) -> Result<usize, ClassParserError> {
    let len = expected.len();
    let Some(get) = data.get(pos..pos + len) else {
        return Err(ClassParserError::EOF);
    };

    let cond = get != expected;
    if cond {
        return Err(ClassParserError::NotAsExpected);
    }
    Ok(pos + len)
}

#[cfg(test)]
mod tests {
    use crate::{load_class, load_module, parse_field_type};
    use dto::SourceDestination;
    use expect_test::expect;
    use my_string::smol_str::SmolStr;
    // #[test]
    // fn a() {
    //     use expect_test::expect;
    //     use my_string::smol_str::SmolStr;
    //
    //     let result = load_class(
    //         include_bytes!(
    //             ""
    //         ),
    //         SmolStr::new("ch.emilycares.Everything"),
    //         SourceDestination::None,
    //         false,
    //     );
    //     let expected = expect![[""]];
    //     expected.assert_debug_eq(&result.unwrap());
    // }

    #[cfg(not(windows))]
    #[test]
    fn relative_source() {
        use expect_test::expect;
        use my_string::smol_str::SmolStr;

        let result = load_class(
            include_bytes!("../../parser/test/Everything.class"),
            SmolStr::new("ch.emilycares.Everything"),
            SourceDestination::None,
            false,
        );
        let expected = expect![[r#"
            Class {
                class_path: "ch.emilycares.Everything",
                source: None,
                access: Access(
                    Public | Super,
                ),
                imports: [
                    Package(
                        "ch.emilycares",
                    ),
                ],
                signature: None,
                name: "Everything",
                methods: [
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: None,
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            0x0,
                        ),
                        name: Some(
                            "method",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: Some(
                            "public_method",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            Private,
                        ),
                        name: Some(
                            "private_method",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            0x0,
                        ),
                        name: Some(
                            "out",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Int,
                        source: None,
                    },
                    Method {
                        access: Access(
                            0x0,
                        ),
                        name: Some(
                            "add",
                        ),
                        parameters: [
                            Parameter {
                                name: Some(
                                    "a",
                                ),
                                jtype: Int,
                            },
                            Parameter {
                                name: Some(
                                    "b",
                                ),
                                jtype: Int,
                            },
                        ],
                        throws: [],
                        ret: Int,
                        source: None,
                    },
                    Method {
                        access: Access(
                            Static,
                        ),
                        name: Some(
                            "sadd",
                        ),
                        parameters: [
                            Parameter {
                                name: Some(
                                    "a",
                                ),
                                jtype: Int,
                            },
                            Parameter {
                                name: Some(
                                    "b",
                                ),
                                jtype: Int,
                            },
                        ],
                        throws: [],
                        ret: Int,
                        source: None,
                    },
                ],
                fields: [
                    Field {
                        access: Access(
                            0x0,
                        ),
                        name: "noprop",
                        jtype: Int,
                        source: None,
                    },
                    Field {
                        access: Access(
                            Public,
                        ),
                        name: "publicproperty",
                        jtype: Int,
                        source: None,
                    },
                    Field {
                        access: Access(
                            Private,
                        ),
                        name: "privateproperty",
                        jtype: Int,
                        source: None,
                    },
                ],
                super_class: None,
                super_interfaces: [],
            }
        "#]];
        expected.assert_debug_eq(&result.unwrap());
    }

    #[test]
    fn everything() {
        let result = load_class(
            include_bytes!("../../parser/test/Everything.class"),
            SmolStr::new("ch.emilycares.Everything"),
            SourceDestination::None,
            false,
        );

        let expected = expect![[r#"
            Class {
                class_path: "ch.emilycares.Everything",
                source: None,
                access: Access(
                    Public | Super,
                ),
                imports: [
                    Package(
                        "ch.emilycares",
                    ),
                ],
                signature: None,
                name: "Everything",
                methods: [
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: None,
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            0x0,
                        ),
                        name: Some(
                            "method",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: Some(
                            "public_method",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            Private,
                        ),
                        name: Some(
                            "private_method",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            0x0,
                        ),
                        name: Some(
                            "out",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Int,
                        source: None,
                    },
                    Method {
                        access: Access(
                            0x0,
                        ),
                        name: Some(
                            "add",
                        ),
                        parameters: [
                            Parameter {
                                name: Some(
                                    "a",
                                ),
                                jtype: Int,
                            },
                            Parameter {
                                name: Some(
                                    "b",
                                ),
                                jtype: Int,
                            },
                        ],
                        throws: [],
                        ret: Int,
                        source: None,
                    },
                    Method {
                        access: Access(
                            Static,
                        ),
                        name: Some(
                            "sadd",
                        ),
                        parameters: [
                            Parameter {
                                name: Some(
                                    "a",
                                ),
                                jtype: Int,
                            },
                            Parameter {
                                name: Some(
                                    "b",
                                ),
                                jtype: Int,
                            },
                        ],
                        throws: [],
                        ret: Int,
                        source: None,
                    },
                ],
                fields: [
                    Field {
                        access: Access(
                            0x0,
                        ),
                        name: "noprop",
                        jtype: Int,
                        source: None,
                    },
                    Field {
                        access: Access(
                            Public,
                        ),
                        name: "publicproperty",
                        jtype: Int,
                        source: None,
                    },
                    Field {
                        access: Access(
                            Private,
                        ),
                        name: "privateproperty",
                        jtype: Int,
                        source: None,
                    },
                ],
                super_class: None,
                super_interfaces: [],
            }
        "#]];
        expected.assert_debug_eq(&result.unwrap());
    }
    #[test]
    fn super_base() {
        let result = load_class(
            include_bytes!("../../parser/test/Super.class"),
            SmolStr::new_inline("ch.emilycares.Super"),
            SourceDestination::None,
            false,
        );

        let expected = expect![[r#"
            Class {
                class_path: "ch.emilycares.Super",
                source: None,
                access: Access(
                    Public | Super,
                ),
                imports: [
                    Package(
                        "ch.emilycares",
                    ),
                ],
                signature: None,
                name: "Super",
                methods: [
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: None,
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                ],
                fields: [],
                super_class: ClassPath(
                    "java.io.IOException",
                ),
                super_interfaces: [],
            }
        "#]];
        expected.assert_debug_eq(&result.unwrap());
    }
    #[test]
    fn thrower() {
        let result = load_class(
            include_bytes!("../../parser/test/Thrower.class"),
            SmolStr::new_inline("ch.emilycares.Thrower"),
            SourceDestination::None,
            false,
        );
        let expected = expect![[r#"
            Class {
                class_path: "ch.emilycares.Thrower",
                source: None,
                access: Access(
                    Public | Super,
                ),
                imports: [
                    Package(
                        "ch.emilycares",
                    ),
                ],
                signature: None,
                name: "Thrower",
                methods: [
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: None,
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: Some(
                            "ioThrower",
                        ),
                        parameters: [],
                        throws: [
                            Class(
                                "java.io.IOException",
                            ),
                        ],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: Some(
                            "ioThrower",
                        ),
                        parameters: [
                            Parameter {
                                name: Some(
                                    "a",
                                ),
                                jtype: Int,
                            },
                        ],
                        throws: [
                            Class(
                                "java.io.IOException",
                            ),
                            Class(
                                "java.io.IOException",
                            ),
                        ],
                        ret: Void,
                        source: None,
                    },
                ],
                fields: [],
                super_class: None,
                super_interfaces: [],
            }
        "#]];
        expected.assert_debug_eq(&result.unwrap());
    }
    #[test]
    fn super_interfaces() {
        let result = load_class(
            include_bytes!("../../parser/test/SuperInterface.class"),
            SmolStr::new("ch.emilycares.SuperInterface"),
            SourceDestination::None,
            false,
        );

        let expected = expect![[r#"
            Class {
                class_path: "ch.emilycares.SuperInterface",
                source: None,
                access: Access(
                    Public | Interface | Abstract,
                ),
                imports: [
                    Package(
                        "ch.emilycares",
                    ),
                    Class(
                        "java.util.stream.Stream",
                    ),
                ],
                signature: Some(
                    ClassSignature {
                        args: [
                            "E",
                        ],
                        ret: Class(
                            "java.lang.Object",
                        ),
                    },
                ),
                name: "SuperInterface",
                methods: [
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: Some(
                            "stream",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Generic(
                            "java.util.stream.Stream",
                            [
                                Parameter(
                                    "E",
                                ),
                            ],
                        ),
                        source: None,
                    },
                ],
                fields: [],
                super_class: None,
                super_interfaces: [
                    ClassPath(
                        "java.util.Collection",
                    ),
                    ClassPath(
                        "java.util.List",
                    ),
                ],
            }
        "#]];
        expected.assert_debug_eq(&result.unwrap());
    }
    #[test]
    fn variables() {
        let result = load_class(
            include_bytes!("../../parser/test/LocalVariableTable.class"),
            SmolStr::new("ch.emilycares.LocalVariableTable"),
            SourceDestination::None,
            false,
        );

        let expected = expect![[r#"
            Class {
                class_path: "ch.emilycares.LocalVariableTable",
                source: None,
                access: Access(
                    Public | Super,
                ),
                imports: [
                    Package(
                        "ch.emilycares",
                    ),
                    Class(
                        "java.util.HashMap",
                    ),
                    Class(
                        "java.util.HashSet",
                    ),
                ],
                signature: None,
                name: "LocalVariableTable",
                methods: [
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: None,
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: Some(
                            "hereIsCode",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: Some(
                            "hereIsCode",
                        ),
                        parameters: [
                            Parameter {
                                name: Some(
                                    "a",
                                ),
                                jtype: Int,
                            },
                            Parameter {
                                name: Some(
                                    "b",
                                ),
                                jtype: Int,
                            },
                        ],
                        throws: [],
                        ret: Int,
                        source: None,
                    },
                ],
                fields: [
                    Field {
                        access: Access(
                            Private,
                        ),
                        name: "a",
                        jtype: Class(
                            "java.util.HashSet",
                        ),
                        source: None,
                    },
                ],
                super_class: None,
                super_interfaces: [],
            }
        "#]];
        expected.assert_debug_eq(&result.unwrap());
    }
    #[test]
    fn variants() {
        let result = load_class(
            include_bytes!("../../parser/test/Variants.class"),
            SmolStr::new("ch.emilycares.Variants"),
            SourceDestination::None,
            false,
        );

        let expected = expect![[r#"
            Class {
                class_path: "ch.emilycares.Variants",
                source: None,
                access: Access(
                    Public | Final | Super | Enum,
                ),
                imports: [
                    Package(
                        "ch.emilycares",
                    ),
                    Class(
                        "java.lang.String",
                    ),
                ],
                signature: Some(
                    ClassSignature {
                        args: [],
                        ret: Generic(
                            "java.lang.Enum",
                            [
                                Class(
                                    "ch.emilycares.Variants",
                                ),
                            ],
                        ),
                    },
                ),
                name: "Variants",
                methods: [
                    Method {
                        access: Access(
                            Public | Static,
                        ),
                        name: Some(
                            "values",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Array(
                            Class(
                                "ch.emilycares.Variants",
                            ),
                        ),
                        source: None,
                    },
                    Method {
                        access: Access(
                            Public | Static,
                        ),
                        name: Some(
                            "valueOf",
                        ),
                        parameters: [
                            Parameter {
                                name: Some(
                                    "name",
                                ),
                                jtype: Class(
                                    "java.lang.String",
                                ),
                            },
                        ],
                        throws: [],
                        ret: Class(
                            "ch.emilycares.Variants",
                        ),
                        source: None,
                    },
                    Method {
                        access: Access(
                            Private,
                        ),
                        name: None,
                        parameters: [
                            Parameter {
                                name: Some(
                                    "$enum$name",
                                ),
                                jtype: Class(
                                    "java.lang.String",
                                ),
                            },
                        ],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                    Method {
                        access: Access(
                            Public,
                        ),
                        name: Some(
                            "getTag",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Class(
                            "java.lang.String",
                        ),
                        source: None,
                    },
                    Method {
                        access: Access(
                            Private | Static,
                        ),
                        name: Some(
                            "$values",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Array(
                            Class(
                                "ch.emilycares.Variants",
                            ),
                        ),
                        source: None,
                    },
                    Method {
                        access: Access(
                            Static,
                        ),
                        name: Some(
                            "<clinit>",
                        ),
                        parameters: [],
                        throws: [],
                        ret: Void,
                        source: None,
                    },
                ],
                fields: [
                    Field {
                        access: Access(
                            Public | Static | Final | Enum,
                        ),
                        name: "A",
                        jtype: Class(
                            "ch.emilycares.Variants",
                        ),
                        source: None,
                    },
                    Field {
                        access: Access(
                            Public | Static | Final | Enum,
                        ),
                        name: "B",
                        jtype: Class(
                            "ch.emilycares.Variants",
                        ),
                        source: None,
                    },
                    Field {
                        access: Access(
                            Public | Static | Final | Enum,
                        ),
                        name: "C",
                        jtype: Class(
                            "ch.emilycares.Variants",
                        ),
                        source: None,
                    },
                    Field {
                        access: Access(
                            Private | Final,
                        ),
                        name: "tag",
                        jtype: Class(
                            "java.lang.String",
                        ),
                        source: None,
                    },
                    Field {
                        access: Access(
                            Private | Static | Final | Synthetic,
                        ),
                        name: "$VALUES",
                        jtype: Array(
                            Class(
                                "ch.emilycares.Variants",
                            ),
                        ),
                        source: None,
                    },
                ],
                super_class: ClassPath(
                    "java.lang.Enum",
                ),
                super_interfaces: [],
            }
        "#]];
        expected.assert_debug_eq(&result.unwrap());
    }

    #[test]
    fn module_java_desktop() {
        let result = load_module(include_bytes!(
            "../../parser/test/module-info-java-desktop.class"
        ));
        let expected = expect![[r#"
            ModuleInfo {
                exports: [
                    "java/desktop",
                    "java/applet",
                    "java/awt",
                    "java/awt/color",
                    "java/awt/desktop",
                    "java/awt/dnd",
                    "java/awt/event",
                    "java/awt/font",
                    "java/awt/geom",
                    "java/awt/im",
                    "java/awt/im/spi",
                    "java/awt/image",
                    "java/awt/image/renderable",
                    "java/awt/print",
                    "java/beans",
                    "java/beans/beancontext",
                    "javax/accessibility",
                    "javax/imageio",
                    "javax/imageio/event",
                    "javax/imageio/metadata",
                    "javax/imageio/plugins/bmp",
                    "javax/imageio/plugins/jpeg",
                    "javax/imageio/plugins/tiff",
                    "javax/imageio/spi",
                    "javax/imageio/stream",
                    "javax/print",
                    "javax/print/attribute",
                    "javax/print/attribute/standard",
                    "javax/print/event",
                    "javax/sound",
                    "javax/sound/midi",
                    "javax/sound/midi/spi",
                    "javax/sound/sampled",
                    "javax/sound/sampled/spi",
                    "javax/swing",
                    "javax/swing/border",
                    "javax/swing/colorchooser",
                    "javax/swing/event",
                    "javax/swing/filechooser",
                    "javax/swing/plaf",
                    "javax/swing/plaf/basic",
                    "javax/swing/plaf/metal",
                    "javax/swing/plaf/multi",
                    "javax/swing/plaf/nimbus",
                    "javax/swing/plaf/synth",
                    "javax/swing/table",
                    "javax/swing/text",
                    "javax/swing/text/html",
                    "javax/swing/text/html/parser",
                    "javax/swing/text/rtf",
                    "javax/swing/tree",
                    "javax/swing/undo",
                ],
            }
        "#]];
        expected.assert_debug_eq(&result.unwrap());
    }

    #[test]
    fn module_jakarta() {
        let result = load_module(include_bytes!(
            "../../parser/test/module-info-jakarta.class"
        ));
        let expected = expect![[r#"
            ModuleInfo {
                exports: [
                    "jakarta/inject",
                    "jakarta/inject",
                ],
            }
        "#]];
        expected.assert_debug_eq(&result.unwrap());
    }

    #[test]
    fn jtype_access() {
        let content = b"Ljava/util/HashMap<LA;LB;>.Factory";
        let result = parse_field_type(content, 0).unwrap();
        assert_eq!(content.len(), result.1);
        let expected = expect![[r#"
            Access {
                base: Generic(
                    "java.util.HashMap",
                    [
                        Class(
                            "A",
                        ),
                        Class(
                            "B",
                        ),
                    ],
                ),
                inner: Class(
                    "Factory",
                ),
            }
        "#]];
        expected.assert_debug_eq(&result.0);
    }
}
