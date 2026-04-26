use classfile_parser::attribute_info::{AttributeInfo, CodeAttribute};
use classfile_parser::constant_info::ConstantInfo;
use classfile_parser::field_info::{FieldAccessFlags, FieldInfo};
use classfile_parser::method_info::MethodAccessFlags;
use classfile_parser::{ClassAccessFlags, ClassFile, class_parser};
use dto::{Access, Class, Field, ImportUnit, JType, Method, Parameter, SuperClass};
use dto::{ClassParserError, SourceDestination};
use my_string::MyString;
use my_string::smol_str::{SmolStr, SmolStrBuilder, StrExt, ToSmolStr};

pub fn load_class(
    bytes: &[u8],
    class_path: MyString,
    source: SourceDestination,
    filter: bool,
) -> Result<Class, ClassParserError> {
    let _ = expect_data(bytes, 0, &[0xCA, 0xFE, 0xBA, 0xBE])
        .map_err(|_| ClassParserError::NotAClass)?;
    let (_, c) = class_parser(bytes).map_err(|_| ClassParserError::BaseParser)?;
    if filter && !c.access_flags.intersects(ClassAccessFlags::PUBLIC) {
        return Err(ClassParserError::Private);
    }

    let name = lookup_class_name(&c, c.this_class.into())?;

    let mut used_classes = Vec::new();
    let mut methods = Vec::new();
    let mut fields = Vec::new();
    let mut deprecated = false;

    for a in &c.attributes {
        if a.attribute_name_index == 0 {
            continue;
        }
        let attribute_name = lookup_string(&c, a.attribute_name_index)?;

        // if attribute_name == "Signature" {
        //     let (_, sig) = classfile_parser::attribute_info::signature_attribute_parser(&a.info)
        //         .map_err(|_| ClassParserError::SignatureAttribute)?;
        //     let sig = lookup_string(&c, sig.signature_index)?;
        //     let (_, sig) = parse_class_signature_info(&sig)?;
        //     dbg!(sig);
        //     continue;
        // }
        if attribute_name == "Code" {
            let (_, out) = classfile_parser::attribute_info::code_attribute_parser(&a.info)
                .map_err(|_| ClassParserError::CodeAttribute)?;
            used_classes.extend(parse_used_classes(&c, out)?);
        } else if attribute_name == "Deprecated" {
            deprecated = true;
        }
    }

    for m in &c.methods {
        if filter
            && m.access_flags
                .intersects(MethodAccessFlags::PRIVATE | MethodAccessFlags::PROTECTED)
        {
            continue;
        }
        let method = parse_method(&c, m);
        if matches!(method, Err(ClassParserError::IgnoringLambda)) {
            continue;
        }
        let method = method?;
        let code_attribute = parse_code_attribute(&c, &m.attributes)?;
        if let Some(code_attribute) = code_attribute {
            let u = parse_used_classes(&c, code_attribute)?;
            used_classes.extend(u);
            used_classes.extend(
                method
                    .parameters
                    .iter()
                    .filter(|i| {
                        matches!(
                            i.jtype,
                            JType::Class(_) | JType::Array(_) | JType::Generic(_, _)
                        )
                    })
                    .flat_map(|i| jtype_class_names(i.jtype.clone())),
            );
        }
        used_classes.extend(jtype_class_names(method.ret.clone()));
        methods.push(method);
    }
    for f in &c.fields {
        if filter
            && f.access_flags
                .intersects(FieldAccessFlags::PRIVATE | FieldAccessFlags::PROTECTED)
        {
            continue;
        }
        let field = parse_field(&c, f)?;
        used_classes.extend(jtype_class_names(field.jtype.clone()));
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

    Ok(Class {
        source,
        class_path,
        super_interfaces,
        super_class,
        imports,
        access: parse_class_access(c.access_flags, deprecated),
        name,
        methods,
        fields,
    })
}

fn lookup_class_name(c: &ClassFile, index: usize) -> Result<MyString, ClassParserError> {
    match c.const_pool.get(index.saturating_sub(1)) {
        Some(ConstantInfo::Class(class)) => Ok(lookup_string(c, class.name_index)?
            .split('/')
            .next_back()
            .map(Into::into)
            .ok_or(ClassParserError::InvalidName)?),
        _ => Err(ClassParserError::ExpectedString),
    }
}

fn parse_field(c: &ClassFile, field: &FieldInfo) -> Result<Field, ClassParserError> {
    Ok(Field {
        access: parse_field_access(field),
        name: lookup_string(c, field.name_index)?,
        jtype: parse_field_type(lookup_string(c, field.descriptor_index)?.as_bytes(), 0)?.1,
        source: None,
    })
}

fn parse_method(
    c: &ClassFile,
    method: &classfile_parser::method_info::MethodInfo,
) -> Result<Method, ClassParserError> {
    let lname = lookup_string(c, method.name_index)?;

    if lname.starts_with("lambda$") {
        return Err(ClassParserError::IgnoringLambda);
    }

    let mut parameters = Vec::new();
    let mut parameter_names = Vec::new();
    let mut throws = Vec::new();
    let mut deprecated = false;
    let mut signature_index = None;
    let mut method_parameter_index = None;
    let mut exception_index = None;
    let mut ret = JType::Void;

    for (index, attribute) in method.attributes.iter().enumerate() {
        let name = lookup_string(c, attribute.attribute_name_index)?;
        if name == "Signature" {
            signature_index = Some(index);
        } else if name == "MethodParameters" {
            method_parameter_index = Some(index);
        } else if name == "Exceptions" {
            exception_index = Some(index);
        } else if name == "Deprecated" {
            deprecated = true;
        }
    }
    let no_parameter_names_and_signature =
        method_parameter_index.is_none() && signature_index.is_none();
    if no_parameter_names_and_signature {
        let (_, md) = parse_method_descriptor(&lookup_string(c, method.descriptor_index)?)?;
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

        let (_, info) =
            classfile_parser::attribute_info::method_parameters_attribute_parser(&attribute.info)
                .map_err(|_| ClassParserError::MethodParameters)?;
        if signature_index.is_some() {
            for p in info.parameters {
                let name = lookup_string(c, p.name_index)
                    .ok()
                    .filter(|i| !SmolStr::is_empty(i));
                parameter_names.push(name);
            }
        } else {
            let (_, md) = parse_method_descriptor(&lookup_string(c, method.descriptor_index)?)?;
            ret = md.return_type;
            let mut params = md.param_types.into_iter();
            for p in info.parameters {
                let jtype = params.next().ok_or(ClassParserError::NotEnogthParams)?;
                if p.name_index == 0 {
                    parameters.push(Parameter { name: None, jtype });
                } else {
                    let name = lookup_string(c, p.name_index)
                        .ok()
                        .filter(|i| !SmolStr::is_empty(i));
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

        let (_, sig) =
            classfile_parser::attribute_info::signature_attribute_parser(&attribute.info)
                .map_err(|_| ClassParserError::SignatureAttribute)?;
        let sig = lookup_string(c, sig.signature_index)?;
        let (_, sig) = parse_method_signature_info(&sig)?;
        let mut name_iter = parameter_names.into_iter();
        parameters.extend(sig.params.into_iter().map(|jtype| Parameter {
            name: name_iter.next().flatten(),
            jtype,
        }));

        ret = sig.ret;
    }

    if let Some(index) = exception_index {
        let attribute = method
            .attributes
            .get(index)
            .ok_or(ClassParserError::InvalidAttributeIndex)?;
        let (_, info) =
            classfile_parser::attribute_info::exceptions_attribute_parser(&attribute.info)
                .map_err(|_| ClassParserError::ExceptionsAttribute)?;

        for exception in info.exception_table {
            let class_name = lookup_string(c, exception)?;
            if let Some((_, name)) = class_name.rsplit_once('/') {
                throws.push(JType::Class(name.to_smolstr()));
            }
        }
    }

    let name = if lname == "<init>" { None } else { Some(lname) };
    Ok(Method {
        access: parse_method_access(method, deprecated),
        name,
        parameters,
        ret,
        throws,
        source: None,
    })
}

#[derive(Debug)]
#[allow(dead_code)]
struct MethodSignature {
    pub args: Vec<MyString>,
    pub params: Vec<JType>,
    pub ret: JType,
}
fn parse_method_signature_info(
    sig: &MyString,
) -> Result<(usize, MethodSignature), ClassParserError> {
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
            let (npos, _) = parse_field_type(content, pos)?;
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
            let (npos, ty) = parse_field_type(content, pos)?;
            params.push(ty);
            pos = npos;
        }
    }
    let (pos, ret) = parse_field_type(content, pos)?;
    Ok((pos, MethodSignature { args, params, ret }))
}
#[derive(Debug)]
#[allow(dead_code)]
struct ClassSignature {
    pub args: Vec<MyString>,
    pub ret: JType,
}
#[allow(dead_code)]
fn parse_class_signature_info(sig: &MyString) -> Result<(usize, ClassSignature), ClassParserError> {
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
            let (npos, _) = parse_field_type(content, pos)?;
            pos = npos;
        }
    }
    let (pos, ret) = parse_field_type(content, pos)?;
    Ok((pos, ClassSignature { args, ret }))
}

fn assert_char(content: &[u8], pos: usize, p: u8) -> Result<usize, ClassParserError> {
    let Some(c) = content.get(pos) else {
        return Err(ClassParserError::EOF);
    };

    if *c != p {
        // let expected = char::from_u32(p as u32);
        // let got = char::from_u32(*c as u32);
        // eprintln!("expected: {expected:?}, got: {got:?}");
        return Err(ClassParserError::ExpectedOther { pos });
    }

    Ok(pos + 1)
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
        return Err(ClassParserError::NotAsExpected { pos, len });
    }
    Ok(pos + len)
}

fn parse_used_classes(
    c: &ClassFile,
    code_attribute: CodeAttribute,
) -> Result<Vec<MyString>, ClassParserError> {
    let mut out = Vec::new();

    for attribute in code_attribute.attributes {
        let attribute_name = lookup_string(c, attribute.attribute_name_index)?;
        if attribute_name == "LocalVariableTable" {
            let (_, info) =
                classfile_parser::code_attribute::local_variable_table_parser(&attribute.info)
                    .map_err(|_| ClassParserError::LocalVariableTable)?;
            for f in info.items {
                let field_desc = lookup_string(c, f.descriptor_index)?;
                let (_, field_desc) = parse_field_type(field_desc.as_bytes(), 0)?;
                let types = jtype_class_names(field_desc);
                out.extend(types);
            }
        }
    }
    Ok(out)
}

fn jtype_class_names(i: JType) -> Vec<MyString> {
    match i {
        JType::Class(class) => vec![class],
        JType::Array(jtype) => jtype_class_names(*jtype),
        JType::Generic(class, jtypes) => {
            let mut out: Vec<MyString> = jtypes.into_iter().flat_map(jtype_class_names).collect();
            out.push(class);
            out
        }
        _ => vec![],
    }
}

fn parse_code_attribute(
    c: &ClassFile,
    attributes: &[AttributeInfo],
) -> Result<Option<CodeAttribute>, ClassParserError> {
    for a in attributes {
        let attribute_name = lookup_string(c, a.attribute_name_index)?;
        if attribute_name == "Code" {
            let (_, out) = classfile_parser::attribute_info::code_attribute_parser(&a.info)
                .map_err(|_| ClassParserError::CodeAttribute)?;
            return Ok(Some(out));
        }
    }
    Ok(None)
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
        let (pos, return_type) = parse_field_type(content, pos)?;
        return Ok((
            pos,
            MethodDescriptor {
                param_types,
                return_type,
            },
        ));
    }
    let (pos, return_type) = parse_field_type(content, pos)?;
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
        let (npos, filed_type) = parse_field_type(content, pos)?;
        out.push(filed_type);
        pos = npos;
    }
    Ok((pos, out))
}

fn parse_field_type(content: &[u8], pos: usize) -> Result<(usize, JType), ClassParserError> {
    let c = content.get(pos).ok_or(ClassParserError::EOF)?;
    match c {
        b'B' => Ok((pos + 1, JType::Byte)),
        b'C' => Ok((pos + 1, JType::Char)),
        b'D' => Ok((pos + 1, JType::Double)),
        b'F' => Ok((pos + 1, JType::Float)),
        b'I' => Ok((pos + 1, JType::Int)),
        b'J' => Ok((pos + 1, JType::Long)),
        b'S' => Ok((pos + 1, JType::Short)),
        b'Z' => Ok((pos + 1, JType::Boolean)),
        b'V' => Ok((pos + 1, JType::Void)),
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
            Ok((pos, JType::Parameter(param.finish())))
        }
        b'L' => {
            let mut pos = pos + 1;
            let mut class_name = SmolStrBuilder::new();
            let mut args = Vec::new();
            loop {
                if let Some(c) = content.get(pos) {
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
                                if let Ok((npos, arg)) = parse_field_type(content, pos) {
                                    args.push(arg);
                                    pos = npos;
                                    if let Ok(npos) = assert_char(content, pos, b';') {
                                        pos = npos;
                                    }
                                }
                                continue;
                            }
                            let (npos, arg) = parse_field_type(content, pos)?;
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
            }
            let class_name = class_name.finish().replace_smolstr("/", ".");
            if !args.is_empty() {
                return Ok((pos, JType::Generic(class_name, args)));
            }
            Ok((pos, JType::Class(class_name)))
        }
        b'[' => {
            let (npos, inner) = parse_field_type(content, pos + 1)?;
            Ok((npos, JType::Array(Box::new(inner))))
        }
        c => {
            let got = char::from_u32(u32::from(*c));
            Err(ClassParserError::UnknownType(got))
        }
    }
}
#[derive(Debug)]
pub struct ModuleInfo {
    pub exports: Vec<MyString>,
}

pub fn load_module(bytes: &[u8]) -> Result<ModuleInfo, ClassParserError> {
    let (_, c) = class_parser(bytes).map_err(|_| ClassParserError::Module)?;
    for a in &c.attributes {
        let name = lookup_string(&c, a.attribute_name_index)?;
        if name == "Module" {
            let (_, module) = classfile_parser::attribute_info::module_attribute_parser(&a.info)
                .map_err(|_| ClassParserError::ModuleAttribute)?;
            let module_name = lookup_string(&c, module.module_name_index)?;
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

fn parse_class_access(flags: ClassAccessFlags, deprecated: bool) -> Access {
    let mut access = Access::empty();
    if deprecated {
        access.insert(Access::Deprecated);
    }
    if flags == ClassAccessFlags::PUBLIC {
        access.insert(Access::Public);
    }
    if flags == ClassAccessFlags::FINAL {
        access.insert(Access::Final);
    }
    if flags == ClassAccessFlags::SUPER {
        access.insert(Access::Super);
    }
    if flags == ClassAccessFlags::INTERFACE {
        access.insert(Access::Interface);
    }
    if flags == ClassAccessFlags::SYNTHETIC {
        access.insert(Access::Synthetic);
    }
    if flags == ClassAccessFlags::ANNOTATION {
        access.insert(Access::Annotation);
    }
    if flags == ClassAccessFlags::ENUM {
        access.insert(Access::Enum);
    }
    access
}

fn parse_method_access(
    method: &classfile_parser::method_info::MethodInfo,
    deprecated: bool,
) -> Access {
    let mut access = Access::empty();
    if deprecated {
        access.insert(Access::Deprecated);
    }
    if method.access_flags == MethodAccessFlags::PUBLIC {
        access.insert(Access::Public);
    }
    if method.access_flags == MethodAccessFlags::PRIVATE {
        access.insert(Access::Private);
    }
    if method.access_flags == MethodAccessFlags::PROTECTED {
        access.insert(Access::Protected);
    }
    if method.access_flags == MethodAccessFlags::STATIC {
        access.insert(Access::Static);
    }
    if method.access_flags == MethodAccessFlags::FINAL {
        access.insert(Access::Final);
    }
    if method.access_flags == MethodAccessFlags::ABSTRACT {
        access.insert(Access::Abstract);
    }
    if method.access_flags == MethodAccessFlags::SYNTHETIC {
        access.insert(Access::Synthetic);
    }
    access
}

fn parse_field_access(method: &FieldInfo) -> Access {
    let mut access = Access::empty();
    if method.access_flags == FieldAccessFlags::PUBLIC {
        access.insert(Access::Public);
    }
    if method.access_flags == FieldAccessFlags::PRIVATE {
        access.insert(Access::Private);
    }
    if method.access_flags == FieldAccessFlags::PROTECTED {
        access.insert(Access::Protected);
    }
    if method.access_flags == FieldAccessFlags::STATIC {
        access.insert(Access::Static);
    }
    if method.access_flags == FieldAccessFlags::FINAL {
        access.insert(Access::Final);
    }
    if method.access_flags == FieldAccessFlags::SYNTHETIC {
        access.insert(Access::Synthetic);
    }
    access
}
fn lookup_string(c: &ClassFile, index: u16) -> Result<MyString, ClassParserError> {
    lookup_string_inner(c, index, 0)
}

fn lookup_string_inner(c: &ClassFile, index: u16, depth: u8) -> Result<MyString, ClassParserError> {
    if depth == 5 {
        return Err(ClassParserError::NameRecursion);
    }
    if index == 0 {
        return Err(ClassParserError::StringIndexZero);
    }
    let con = &c.const_pool.get((index - 1) as usize);
    match con {
        Some(ConstantInfo::Utf8(utf8)) => Ok(utf8.utf8_string.to_smolstr()),
        Some(ConstantInfo::Module(m)) => lookup_string_inner(c, m.name_index, depth + 1),
        Some(ConstantInfo::Package(p)) => lookup_string_inner(c, p.name_index, depth + 1),
        Some(ConstantInfo::Class(p)) => lookup_string_inner(c, p.name_index, depth + 1),
        _ => Err(ClassParserError::ExpectedString),
    }
}
#[cfg(test)]
mod tests {
    use crate::class::{load_class, load_module};
    use dto::SourceDestination;
    use my_string::smol_str::SmolStr;

    #[cfg(not(windows))]
    #[test]
    fn relative_source() {
        use my_string::smol_str::SmolStr;

        let result = load_class(
            include_bytes!("../test/Everything.class"),
            SmolStr::new("ch.emilycares.Everything"),
            SourceDestination::None,
            false,
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn everything() {
        let result = load_class(
            include_bytes!("../test/Everything.class"),
            SmolStr::new("ch.emilycares.Everything"),
            SourceDestination::None,
            false,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn super_base() {
        let result = load_class(
            include_bytes!("../test/Super.class"),
            SmolStr::new_inline("ch.emilycares.Super"),
            SourceDestination::None,
            false,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn thrower() {
        let result = load_class(
            include_bytes!("../test/Thrower.class"),
            SmolStr::new_inline("ch.emilycares.Thrower"),
            SourceDestination::None,
            false,
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn super_interfaces() {
        let result = load_class(
            include_bytes!("../test/SuperInterface.class"),
            SmolStr::new("ch.emilycares.SuperInterface"),
            SourceDestination::None,
            false,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn variables() {
        let result = load_class(
            include_bytes!("../test/LocalVariableTable.class"),
            SmolStr::new("ch.emilycares.LocalVariableTable"),
            SourceDestination::None,
            false,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn module_java_desktop() {
        let result = load_module(include_bytes!("../test/module-info-java-desktop.class"));
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn module_jakarta() {
        let result = load_module(include_bytes!("../test/module-info-jakarta.class"));
        insta::assert_debug_snapshot!(result.unwrap());
    }
}
