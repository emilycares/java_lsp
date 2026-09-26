#![deny(clippy::nursery)]
#![deny(clippy::perf)]
#![deny(clippy::arithmetic_side_effects)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::collections::HashSet;

use ast::types::{
    AstAnnotated, AstAnnotatedParameterKind, AstAvailability, AstBaseExpression, AstBlock,
    AstClass, AstClassBlock, AstClassMethod, AstClassVariable, AstDouble, AstEnumeration,
    AstExpressionIdentifier, AstExpressionKind, AstExpressionOperator, AstFile, AstIdentifier,
    AstImport, AstImportUnit, AstInt, AstJType, AstJTypeKind, AstMethodHeader, AstMethodParameter,
    AstMethodParameterFlags, AstMethodParameters, AstPackage, AstRange, AstThing,
    AstThingAttributes, AstThrowsDeclaration, AstTopLevel, AstValueNuget, AstVolatileTransient,
};
use bitflags::bitflags;
use my_string::{NuVec, NuVecBuilder};

const U8_LEN: usize = 1;
const U16_LEN: usize = 2;
#[derive(Debug)]
pub enum DecompilerError {
    EOF,
    ExpectedOther,
    Ignoring,
    StringIndexZero,
    ExpectedString,
    InvalidName,
    NotEnogthParams,
    NoModuleAttribute,
    UnknownType,
    GenericParameterName,
    InvalidAttributeIndex,
    NotAsExpected,
    NotAClass,
    NameRecursion,
    Number,
    UnknownConstant,
    Mutf8,
    InvalidUtf8,
}

pub fn decompile_class(data: &[u8], class_path: &NuVec) -> Result<AstFile, DecompilerError> {
    let (c, _) = parser_base(data, 0)?;

    let name = lookup_class_name(&c, c.this_class.into())?;

    let mut used_classes = HashSet::new();
    let mut methods = Vec::new();
    let mut class_signature = None;
    let mut annotated = Vec::new();

    for a in &c.attributes {
        if a.name == 0 {
            continue;
        }
        let attribute_name = lookup_string(&c.const_pool, a.name)?;

        if attribute_name == "Signature" {
            let info = a.lookup(data)?;
            let (sig, _) = get_u16(info, 0)?;
            let sig = lookup_string(&c.const_pool, sig)?;
            let (sig, _) = parse_class_signature_info(&sig)?;
            class_signature = Some(sig);
        } else if attribute_name == "Code" {
            let info = a.lookup(data)?;
            let (out, _) = parse_code_attribute(info, 0, a.start, a.end)?;
            parse_used_classes(&c, data, &out, &mut used_classes)?;
        } else if attribute_name == "Deprecated" {
            annotated.push(AstAnnotated {
                range: AstRange::default(),
                name: ntoident(NuVec::Static(b"Deprecated")),
                parameters: AstAnnotatedParameterKind::None,
            });
        }
    }
    let _ = class_signature;

    for m in &c.methods {
        let method = parse_method(&c, data, m);
        let (method, code_attribute) = method?;
        if let Some(code_attribute) = code_attribute {
            parse_used_classes(&c, data, &code_attribute, &mut used_classes)?;
        }
        jtype_class_names(method.header.jtype.clone(), &mut used_classes);
        methods.push(method);
    }
    for f in &c.fields {
        jtype_class_names(f.jtype.clone(), &mut used_classes);
    }

    let mut file = AstFile { top: Vec::new() };
    let package = class_path
        .trim_end_matches(name.as_bytes())
        .trim_end_matches_byte(b'.');
    file.top.push(AstTopLevel::Package(AstPackage {
        range: AstRange::default(),
        annotated: Vec::new(),
        name: ntoident(package),
    }));
    let mut imports: Vec<&NuVec> = used_classes.iter().filter(|i| *i != class_path).collect();
    imports.sort();
    file.top.extend(imports.into_iter().map(|i| {
        AstTopLevel::Import(AstImport {
            range: AstRange::default(),
            unit: AstImportUnit::Class(ntoident(i.clone())),
        })
    }));

    let mut availability = AstAvailability::empty();
    if c.class_access_flags.intersects(ClassAccessFlags::Public) {
        availability |= AstAvailability::Public;
    }
    if c.class_access_flags.intersects(ClassAccessFlags::Final) {
        availability |= AstAvailability::Final;
    }
    if c.class_access_flags.intersects(ClassAccessFlags::Abstract) {
        availability |= AstAvailability::Abstract;
    }

    if c.class_access_flags.intersects(ClassAccessFlags::Enum) {
        let enu = AstEnumeration {
            range: AstRange::default(),
            availability,
            attributes: AstThingAttributes::empty(),
            annotated,
            name: ntoident(name),
            implements: Vec::new(),
            permits: Vec::new(),
            superclass: Vec::new(),
            variants: Vec::new(),
            methods,
            variables: c.fields,
            constructors: Vec::new(),
            static_blocks: Vec::new(),
            inner: Vec::new(),
        };
        let enu_thing = AstTopLevel::Thing(Box::new(AstThing::Enumeration(enu)));
        file.top.push(enu_thing);
    } else {
        let class = AstClass {
            range: AstRange::default(),
            availability,
            attributes: AstThingAttributes::empty(),
            annotated,
            name: ntoident(name),
            type_parameters: None,
            superclass: Vec::new(),
            implements: Vec::new(),
            permits: Vec::new(),
            block: AstClassBlock {
                range: AstRange::default(),
                variables: c.fields,
                methods,
                constructors: Vec::new(),
                static_blocks: Vec::new(),
                inner: Vec::new(),
                blocks: Vec::new(),
            },
        };
        let class_thing = AstTopLevel::Thing(Box::new(AstThing::Class(class)));
        file.top.push(class_thing);
    }

    Ok(file)
}

fn ntoident(value: NuVec) -> AstIdentifier {
    AstIdentifier {
        value,
        range: AstRange::default(),
    }
}

fn lookup_class_name(c: &Base, index: usize) -> Result<NuVec, DecompilerError> {
    match c.const_pool.pool.get(index.saturating_sub(1)) {
        Some(ConstEntry::Class { name }) => Ok(lookup_string(&c.const_pool, *name)?
            .to_str()
            .split('/')
            .next_back()
            .map(Into::into)
            .ok_or(DecompilerError::InvalidName)?),
        _ => Err(DecompilerError::ExpectedString),
    }
}

const UNKNOWN: &[u8] = b"";

fn parse_method(
    c: &Base,
    data: &[u8],
    method: &Method,
) -> Result<(AstClassMethod, Option<CodeAttribute>), DecompilerError> {
    let lname = lookup_string(&c.const_pool, method.name)?;
    // let name = if lname == "<init>" { None } else { Some(lname) };
    let mut out = AstClassMethod {
        range: AstRange::default(),
        header: AstMethodHeader {
            range: AstRange::default(),
            availability: method.availability.clone(),
            name: ntoident(lname),
            jtype: AstJType {
                range: AstRange::default(),
                annotated: Vec::new(),
                value: AstJTypeKind::Void,
            },
            parameters: AstMethodParameters {
                range: AstRange::default(),
                parameters: Vec::new(),
            },
            throws: None,
            type_parameters: None,
            annotated: Vec::new(),
            default: false,
        },
        block: Some(AstBlock {
            range: AstRange::default(),
            entries: Vec::new(),
        }),
    };

    let mut code_attribute = None;

    let mut parameter_names = Vec::new();
    let mut deprecated = false;
    let mut signature_index = None;
    let mut method_parameter_index = None;
    let mut exception_index = None;
    let mut ret = AstJType {
        range: AstRange::default(),
        value: AstJTypeKind::Void,
        annotated: Vec::new(),
    };

    for (index, attribute) in method.attributes.iter().enumerate() {
        let name = lookup_string(&c.const_pool, attribute.name)?;
        if name == "Signature" {
            signature_index = Some(index);
        } else if name == "MethodParameters" {
            method_parameter_index = Some(index);
        } else if name == "Exceptions" {
            exception_index = Some(index);
        } else if name == "Deprecated" {
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
        let desc = lookup_string(&c.const_pool, method.descriptor)?;
        let (_, md) = parse_method_descriptor(&desc)?;
        ret = md.return_type;
        for p in md.param_types {
            out.header.parameters.parameters.push(AstMethodParameter {
                range: AstRange::default(),
                annotated: Vec::new(),
                jtype: p,
                name: ntoident(NuVec::Static(UNKNOWN)),
                flags: AstMethodParameterFlags::empty(),
            });
        }
    }

    if let Some(index) = method_parameter_index {
        let attribute = method
            .attributes
            .get(index)
            .ok_or(DecompilerError::InvalidAttributeIndex)?;

        let info = attribute.lookup(data)?;
        let (info, _) = parse_method_parameters_attribute(info, 0)?;
        if signature_index.is_some() {
            for p in info {
                let name = lookup_string(&c.const_pool, p.name_index)
                    .ok()
                    .filter(|i| !i.is_empty());
                parameter_names.push(name);
            }
        } else {
            let (_, md) =
                parse_method_descriptor(&lookup_string(&c.const_pool, method.descriptor)?)?;
            ret = md.return_type;
            let mut params = md.param_types.into_iter();
            for p in info {
                let jtype = params.next().ok_or(DecompilerError::NotEnogthParams)?;
                if p.name_index == 0 {
                    out.header.parameters.parameters.push(AstMethodParameter {
                        range: AstRange::default(),
                        annotated: Vec::new(),
                        jtype,
                        name: ntoident(NuVec::Static(UNKNOWN)),
                        flags: AstMethodParameterFlags::empty(),
                    });
                } else if let Some(name) = lookup_string(&c.const_pool, p.name_index)
                    .ok()
                    .filter(|i| !i.is_empty())
                {
                    // parameters.push(Parameter { name, jtype });
                    out.header.parameters.parameters.push(AstMethodParameter {
                        range: AstRange::default(),
                        annotated: Vec::new(),
                        jtype,
                        name: ntoident(name),
                        flags: AstMethodParameterFlags::empty(),
                    });
                } else {
                    out.header.parameters.parameters.push(AstMethodParameter {
                        range: AstRange::default(),
                        annotated: Vec::new(),
                        jtype,
                        name: ntoident(NuVec::Static(UNKNOWN)),
                        flags: AstMethodParameterFlags::empty(),
                    });
                }
            }
        }
    }

    if let Some(index) = signature_index {
        let attribute = method
            .attributes
            .get(index)
            .ok_or(DecompilerError::InvalidAttributeIndex)?;

        let info = attribute.lookup(data)?;
        let (sig, _) = get_u16(info, 0)?;
        let sig = lookup_string(&c.const_pool, sig)?;
        let (sig, _) = parse_method_signature_info(&sig)?;
        let mut name_iter = parameter_names.into_iter();
        sig.params.iter().for_each(|jtype| {
            if let Some(name) = name_iter.next().flatten() {
                out.header.parameters.parameters.push(AstMethodParameter {
                    range: AstRange::default(),
                    annotated: Vec::new(),
                    jtype: jtype.clone(),
                    name: ntoident(name),
                    flags: AstMethodParameterFlags::empty(),
                });
            } else {
                out.header.parameters.parameters.push(AstMethodParameter {
                    range: AstRange::default(),
                    annotated: Vec::new(),
                    jtype: jtype.clone(),
                    name: ntoident(NuVec::Static(UNKNOWN)),
                    flags: AstMethodParameterFlags::empty(),
                });
            }
        });

        ret = sig.ret;
    }

    if let Some(index) = exception_index {
        let attribute = method
            .attributes
            .get(index)
            .ok_or(DecompilerError::InvalidAttributeIndex)?;
        let info = attribute.lookup(data)?;
        let (info, _) = parse_exceptions_attribute(info, 0)?;

        if !info.is_empty() {
            let mut throws = Vec::new();
            for exception in info {
                let class_name = lookup_string(&c.const_pool, exception)?;
                throws.push(AstJType {
                    range: AstRange::default(),
                    annotated: Vec::new(),
                    value: AstJTypeKind::Class(ntoident(class_name.replace_byte(b'/', b'.'))),
                });
            }
            out.header.throws = Some(AstThrowsDeclaration {
                range: AstRange::default(),
                parameters: throws,
            });
        }
    }
    out.header.jtype = ret;
    if deprecated {
        out.header.annotated.push(AstAnnotated {
            range: AstRange::default(),
            name: ntoident(NuVec::Static(b"Deprecated")),
            parameters: AstAnnotatedParameterKind::None,
        });
    }

    Ok((out, code_attribute))
}

fn parse_exceptions_attribute(
    data: &[u8],
    pos: usize,
) -> Result<(Vec<u16>, usize), DecompilerError> {
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
#[derive(Debug)]
pub struct ClassSignature {
    // Generics defined on class level
    pub args: Vec<NuVec>,
    pub ret: AstJType,
}

fn parse_method_parameters_attribute(
    data: &[u8],
    pos: usize,
) -> Result<(Vec<MethodParametersAttribute>, usize), DecompilerError> {
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
) -> Result<(MethodParametersAttribute, usize), DecompilerError> {
    let (name_index, pos) = get_u16(data, pos)?;
    let pos = pos.saturating_add(2);
    Ok((MethodParametersAttribute { name_index }, pos))
}

#[derive(Debug)]
#[allow(dead_code)]
struct MethodSignature {
    pub args: Vec<NuVec>,
    pub params: Vec<AstJType>,
    pub ret: AstJType,
}
fn parse_method_signature_info(sig: &NuVec) -> Result<(MethodSignature, usize), DecompilerError> {
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
            let mut arg = NuVecBuilder::new();
            loop {
                let v = content.get(pos).ok_or(DecompilerError::EOF)?;
                if *v == b':' {
                    break;
                }
                arg.push(*v);
                pos = pos.saturating_add(1);
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
fn parse_class_signature_info(sig: &NuVec) -> Result<(ClassSignature, usize), DecompilerError> {
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
            let mut arg = NuVecBuilder::new();
            loop {
                let v = content.get(pos).ok_or(DecompilerError::EOF)?;
                if *v == b':' {
                    break;
                }
                arg.push(*v);
                pos = pos.saturating_add(1);
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
    let mut init = false;
    let mut ret = AstJType {
        range: AstRange::default(),
        annotated: Vec::new(),
        value: AstJTypeKind::Void,
    };
    let end = sig.len();
    while pos < end
        && let Ok((nret, npos)) = parse_field_type(content, pos)
    {
        pos = npos;

        if init {
        } else {
            ret = nret;
            init = true;
        }
    }

    Ok((ClassSignature { args, ret }, pos))
}

fn assert_char(content: &[u8], pos: usize, p: u8) -> Result<usize, DecompilerError> {
    let Some(c) = content.get(pos) else {
        return Err(DecompilerError::EOF);
    };

    if *c != p {
        // let expected = char::from_u32(p as u32);
        // let got = char::from_u32(*c as u32);
        // eprintln!("expected: {expected:?}, got: {got:?}");
        return Err(DecompilerError::ExpectedOther);
    }

    Ok(pos.saturating_add(1))
}

struct CodeAttribute {
    start: usize,
    end: usize,
    pub attributes: Vec<Attribute>,
}
impl CodeAttribute {
    pub fn lookup<'a>(&'a self, data: &'a [u8]) -> Result<&'a [u8], DecompilerError> {
        data.get(self.start..self.end).ok_or(DecompilerError::EOF)
    }
}

fn parse_code_attribute(
    data: &[u8],
    pos: usize,
    start: usize,
    end: usize,
) -> Result<(CodeAttribute, usize), DecompilerError> {
    let (_max_stack, pos) = get_u16(data, pos)?;
    let (_max_locals, pos) = get_u16(data, pos)?;

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
) -> Result<(Vec<u16>, usize), DecompilerError> {
    let (table_length, pos) = get_u16(data, pos)?;
    let mut out = Vec::with_capacity(table_length as usize);
    let mut pos = pos;

    for _ in 0..table_length {
        pos = pos.saturating_add(3_usize.saturating_mul(U16_LEN));
        let (descriptor_index, npos) = get_u16(data, pos)?;
        pos = npos;
        out.push(descriptor_index);
        // skip u16
        pos = pos.saturating_add(U16_LEN);
    }

    Ok((out, pos))
}

fn parse_used_classes(
    c: &Base,
    data: &[u8],
    code_attribute: &CodeAttribute,
    used_classes: &mut HashSet<NuVec>,
) -> Result<(), DecompilerError> {
    let info = code_attribute.lookup(data)?;

    for attribute in &code_attribute.attributes {
        let attribute_name = lookup_string(&c.const_pool, attribute.name)?;
        if attribute_name == "LocalVariableTable" {
            let info = attribute.lookup(info)?;
            let (descriptors, _) = parse_local_variable_table_attribute(info, 0)?;
            for f in descriptors {
                let field_desc = lookup_string(&c.const_pool, f)?;
                let (field_desc, _) = parse_field_type(field_desc.as_bytes(), 0)?;
                jtype_class_names(field_desc, used_classes);
            }
        }
    }
    Ok(())
}

fn jtype_class_names(i: AstJType, used_classes: &mut HashSet<NuVec>) {
    match i.value {
        AstJTypeKind::WildcardImplements(ast_jtype)
        | AstJTypeKind::WildcardExtends(ast_jtype)
        | AstJTypeKind::WildcardSuper(ast_jtype)
        | AstJTypeKind::Array(ast_jtype) => jtype_class_names(*ast_jtype, used_classes),
        AstJTypeKind::Class(ast_identifier)
        | AstJTypeKind::ClassOrPackage(ast_identifier)
        | AstJTypeKind::Generic(ast_identifier, _) => {
            used_classes.insert(ast_identifier.value);
        }
        AstJTypeKind::Access { base, inner } => {
            jtype_class_names(*base, used_classes);
            jtype_class_names(*inner, used_classes);
        }
        _ => (),
    }
    // match i {
    //     JType::Class(class) => {
    //         used_classes.push(class);
    //     }
    //     JType::Array(jtype) => jtype_class_names(*jtype, used_classes),
    //     JType::Generic(class, jtypes) => {
    //         for j in jtypes {
    //             jtype_class_names(j, used_classes);
    //         }
    //         used_classes.push(class);
    //     }
    //     _ => (),
    // }
}

#[derive(Debug)]
struct MethodDescriptor {
    param_types: Vec<AstJType>,
    return_type: AstJType,
}

fn parse_method_descriptor(
    descriptor: &NuVec,
) -> Result<(usize, MethodDescriptor), DecompilerError> {
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

fn parse_param_types(
    content: &[u8],
    pos: usize,
) -> Result<(usize, Vec<AstJType>), DecompilerError> {
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

fn parse_field_type(content: &[u8], pos: usize) -> Result<(AstJType, usize), DecompilerError> {
    let c = content.get(pos).ok_or(DecompilerError::EOF)?;

    match c {
        b'B' => Ok((
            AstJType {
                range: AstRange::default(),
                annotated: Vec::new(),
                value: AstJTypeKind::Byte,
            },
            pos.saturating_add(1),
        )),
        b'C' => Ok((
            AstJType {
                range: AstRange::default(),
                annotated: Vec::new(),
                value: AstJTypeKind::Char,
            },
            pos.saturating_add(1),
        )),
        b'D' => Ok((
            AstJType {
                range: AstRange::default(),
                annotated: Vec::new(),
                value: AstJTypeKind::Double,
            },
            pos.saturating_add(1),
        )),
        b'F' => Ok((
            AstJType {
                range: AstRange::default(),
                annotated: Vec::new(),
                value: AstJTypeKind::Float,
            },
            pos.saturating_add(1),
        )),
        b'I' => Ok((
            AstJType {
                range: AstRange::default(),
                annotated: Vec::new(),
                value: AstJTypeKind::Int,
            },
            pos.saturating_add(1),
        )),
        b'J' => Ok((
            AstJType {
                range: AstRange::default(),
                annotated: Vec::new(),
                value: AstJTypeKind::Long,
            },
            pos.saturating_add(1),
        )),
        b'S' => Ok((
            AstJType {
                range: AstRange::default(),
                annotated: Vec::new(),
                value: AstJTypeKind::Short,
            },
            pos.saturating_add(1),
        )),
        b'Z' => Ok((
            AstJType {
                range: AstRange::default(),
                annotated: Vec::new(),
                value: AstJTypeKind::Boolean,
            },
            pos.saturating_add(1),
        )),
        b'V' => Ok((
            AstJType {
                range: AstRange::default(),
                annotated: Vec::new(),
                value: AstJTypeKind::Void,
            },
            pos.saturating_add(1),
        )),
        b'T' => {
            let mut pos = pos.saturating_add(1);
            let mut param = NuVecBuilder::new();
            loop {
                let v = content.get(pos).ok_or(DecompilerError::EOF)?;
                if *v == b';' {
                    break;
                }
                param.push(*v);
                pos = pos.saturating_add(1);
            }
            Ok((
                AstJType {
                    range: AstRange::default(),
                    annotated: Vec::new(),
                    value: AstJTypeKind::Class(ntoident(param.finish())),
                },
                pos,
            ))
        }
        b'L' => {
            let pos = pos.saturating_add(1);
            let (mut pos, mut out) = parse_jtype_class_name(content, pos)?;
            while let Some(next) = content.get(pos)
                && next == &b'.'
            {
                let (npos, inner) = parse_jtype_class_name(content, pos.saturating_add(1))?;
                pos = npos;
                out = AstJType {
                    range: AstRange::default(),
                    annotated: Vec::new(),
                    value: AstJTypeKind::Access {
                        base: Box::new(out),
                        inner: Box::new(inner),
                    },
                }
            }
            Ok((out, pos))
        }
        b'[' => {
            let (inner, npos) = parse_field_type(content, pos.saturating_add(1))?;
            Ok((
                AstJType {
                    range: AstRange::default(),
                    annotated: Vec::new(),
                    value: AstJTypeKind::Array(Box::new(inner)),
                },
                npos,
            ))
        }
        _ => {
            // let got = char::from_u32(u32::from(*c));
            Err(DecompilerError::UnknownType)
        }
    }
}

fn parse_jtype_class_name(
    content: &[u8],
    mut pos: usize,
) -> Result<(usize, AstJType), DecompilerError> {
    let mut class_name = NuVecBuilder::new();
    let mut args = Vec::new();
    while let Some(c) = content.get(pos) {
        if c == &b'<' {
            pos = pos.saturating_add(1);
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
            pos = pos.saturating_add(1);
            break;
        }
        class_name.push(*c);
        pos = pos.saturating_add(1);
    }
    let class_name = class_name.finish().replace_byte(b'/', b'.');
    if !args.is_empty() {
        return Ok((
            pos,
            AstJType {
                range: AstRange::default(),
                annotated: Vec::new(),
                value: AstJTypeKind::Generic(ntoident(class_name), args),
            },
        ));
    }
    Ok((
        pos,
        AstJType {
            range: AstRange::default(),
            annotated: Vec::new(),
            value: AstJTypeKind::Class(ntoident(class_name)),
        },
    ))
}

fn lookup_string(c: &ConstPool, index: u16) -> Result<NuVec, DecompilerError> {
    lookup_string_inner(c, index, 0)
}

fn lookup_string_inner(c: &ConstPool, index: u16, depth: u8) -> Result<NuVec, DecompilerError> {
    if depth == 5 {
        return Err(DecompilerError::NameRecursion);
    }
    if index == 0 {
        return Err(DecompilerError::StringIndexZero);
    }
    let con = &c.pool.get((index.saturating_sub(1)) as usize);
    match con {
        Some(ConstEntry::Utf8(utf8)) => Ok(utf8.clone()),
        Some(
            ConstEntry::Module { name } | ConstEntry::Package { name } | ConstEntry::Class { name },
        ) => lookup_string_inner(c, *name, depth.saturating_add(1)),
        _ => Err(DecompilerError::ExpectedString),
    }
}

struct Base {
    pub const_pool: ConstPool,
    pub class_access_flags: ClassAccessFlags,
    pub this_class: u16,
    pub fields: Vec<AstClassVariable>,
    pub methods: Vec<Method>,
    pub attributes: Vec<Attribute>,
}

struct Method {
    pub availability: AstAvailability,
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
    pub fn lookup<'a>(&'a self, data: &'a [u8]) -> Result<&'a [u8], DecompilerError> {
        data.get(self.start..self.end).ok_or(DecompilerError::EOF)
    }
}

fn parser_base(data: &[u8], pos: usize) -> Result<(Base, usize), DecompilerError> {
    let pos = expect_data(data, pos, &[0xCA, 0xFE, 0xBA, 0xBE])
        .map_err(|_| DecompilerError::NotAClass)?;

    let pos = pos.saturating_add(U16_LEN + U16_LEN);

    let (const_pool, pos) = parse_const_pool(data, pos)?;

    let (class_access_flags, pos) = parse_class_access_flags(data, pos)?;
    let (this_class, pos) = get_u16(data, pos)?;
    let (_, pos) = get_u16(data, pos)?;

    let (_, pos) = parse_interfaces(data, pos)?;
    let (fields, pos) = parse_fields(data, pos, &const_pool)?;
    let (methods, pos) = parse_methods(data, pos)?;
    let (attributes, pos) = parse_attributes(data, pos)?;

    Ok((
        Base {
            const_pool,

            class_access_flags,
            this_class,

            fields,
            methods,
            attributes,
        },
        pos,
    ))
}

bitflags! {
   #[derive(Clone, Eq, PartialEq, Debug, Default)]
   pub struct ClassAccessFlags: u16 {
       const Public = 0x0001;
       const Final = 0x0010;
       const Super = 0x0020;
       const Interface = 0x0200;
       const Abstract = 0x0400;
       const Synthetic = 0x1000;
       const Annotation = 0x2000;
       const Enum = 0x4000;
       const Module = 0x8000;
   }
}
fn parse_class_access_flags(
    data: &[u8],
    pos: usize,
) -> Result<(ClassAccessFlags, usize), DecompilerError> {
    let (flags, pos) = get_u16(data, pos)?;
    let out = ClassAccessFlags::from_bits_retain(flags);

    Ok((out, pos))
}

fn parse_fields(
    data: &[u8],
    pos: usize,
    c: &ConstPool,
) -> Result<(Vec<AstClassVariable>, usize), DecompilerError> {
    let (size, pos) = get_u16(data, pos)?;
    let mut pos = pos;
    let mut out = Vec::with_capacity(size as usize);

    for _ in 0..size {
        let (field, npos) = parse_class_field(data, pos, c)?;
        pos = npos;
        out.push(field);
    }

    Ok((out, pos))
}

fn parse_class_field(
    data: &[u8],
    pos: usize,
    c: &ConstPool,
) -> Result<(AstClassVariable, usize), DecompilerError> {
    let (access_flags, volatile_transient, pos) = parse_field_access_flags(data, pos)?;
    let (name, pos) = get_u16(data, pos)?;
    let (descriptor, pos) = get_u16(data, pos)?;
    let (attributes, pos) = parse_attributes(data, pos)?;
    let mut expression = None;

    for a in attributes {
        if a.name == 0 {
            continue;
        }
        let attribute_name = lookup_string(c, a.name)?;
        if attribute_name == "ConstantValue" {
            let info = a.lookup(data)?;
            let (constant_value_index, _) = get_u16(info, 0)?;
            match c
                .pool
                .get((constant_value_index.saturating_sub(1)) as usize)
            {
                Some(ConstEntry::Float(c)) => {
                    let value = NuVec::new(c.to_string().as_bytes());
                    let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                        range: AstRange::default(),
                        ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Float(
                            AstDouble {
                                range: AstRange::default(),
                                value,
                            },
                        ))),
                        values: None,
                        operator: AstExpressionOperator::None,
                    })];
                    expression = Some(expr);
                }
                Some(ConstEntry::Long(c)) => {
                    let value = NuVec::new(c.to_string().as_bytes());
                    let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                        range: AstRange::default(),
                        ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Long(
                            AstInt {
                                range: AstRange::default(),
                                value,
                            },
                        ))),
                        values: None,
                        operator: AstExpressionOperator::None,
                    })];
                    expression = Some(expr);
                }
                Some(ConstEntry::Double(c)) => {
                    let value = NuVec::new(c.to_string().as_bytes());
                    let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                        range: AstRange::default(),
                        ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Double(
                            AstDouble {
                                range: AstRange::default(),
                                value,
                            },
                        ))),
                        values: None,
                        operator: AstExpressionOperator::None,
                    })];
                    expression = Some(expr);
                }
                Some(ConstEntry::String { name }) => {
                    let value = lookup_string(c, *name)?;
                    let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                        range: AstRange::default(),
                        ident: Some(AstExpressionIdentifier::Nuget(
                            AstValueNuget::StringLiteral {
                                range: AstRange::default(),
                                value,
                            },
                        )),
                        values: None,
                        operator: AstExpressionOperator::None,
                    })];
                    expression = Some(expr);
                }
                Some(_) | None => {}
            }
        }
    }

    Ok((
        AstClassVariable {
            range: AstRange::default(),
            availability: access_flags,
            annotated: Vec::new(),
            name: ntoident(lookup_string(c, name)?),
            jtype: parse_field_type(lookup_string(c, descriptor)?.as_bytes(), 0)?.0,
            expression,
            volatile_transient,
        },
        pos,
    ))
}

fn parse_field_access_flags(
    data: &[u8],
    pos: usize,
) -> Result<(AstAvailability, AstVolatileTransient, usize), DecompilerError> {
    let (flags, pos) = get_u16(data, pos)?;
    let mut out = AstAvailability::empty();
    let mut vt = AstVolatileTransient::empty();
    if (flags & 0x0001) != 0 {
        out |= AstAvailability::Public;
    }
    if (flags & 0x0002) != 0 {
        out |= AstAvailability::Private;
    }
    if (flags & 0x0004) != 0 {
        out |= AstAvailability::Protected;
    }
    if (flags & 0x0008) != 0 {
        out |= AstAvailability::Static;
    }
    if (flags & 0x0010) != 0 {
        out |= AstAvailability::Final;
    }
    if (flags & 0x0040) != 0 {
        vt |= AstVolatileTransient::Volatile;
    }
    if (flags & 0x0080) != 0 {
        vt |= AstVolatileTransient::Transient;
    }

    Ok((out, vt, pos))
}
fn parse_methods(data: &[u8], pos: usize) -> Result<(Vec<Method>, usize), DecompilerError> {
    let (size, pos) = get_u16(data, pos)?;
    let mut pos = pos;
    let mut out = Vec::with_capacity(size as usize);

    for _ in 0..size {
        let (method, npos) = parse_class_method(data, pos)?;
        pos = npos;
        out.push(method);
    }

    Ok((out, pos))
}
fn parse_class_method(data: &[u8], pos: usize) -> Result<(Method, usize), DecompilerError> {
    let (access_flags, pos) = parse_method_access_flags(data, pos)?;
    let (name, pos) = get_u16(data, pos)?;
    let (descriptor, pos) = get_u16(data, pos)?;
    let (attributes, pos) = parse_attributes(data, pos)?;
    Ok((
        Method {
            availability: access_flags,
            name,
            descriptor,
            attributes,
        },
        pos,
    ))
}

fn parse_method_access_flags(
    data: &[u8],
    pos: usize,
) -> Result<(AstAvailability, usize), DecompilerError> {
    let (flags, pos) = get_u16(data, pos)?;
    let mut out = AstAvailability::empty();
    if (flags & 0x0001) != 0 {
        out |= AstAvailability::Public;
    }
    if (flags & 0x0002) != 0 {
        out |= AstAvailability::Private;
    }
    if (flags & 0x0004) != 0 {
        out |= AstAvailability::Protected;
    }
    if (flags & 0x0008) != 0 {
        out |= AstAvailability::Static;
    }
    if (flags & 0x0010) != 0 {
        out |= AstAvailability::Final;
    }
    if (flags & 0x0400) != 0 {
        out |= AstAvailability::Abstract;
    }

    Ok((out, pos))
}

fn parse_attributes(data: &[u8], pos: usize) -> Result<(Vec<Attribute>, usize), DecompilerError> {
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
fn parse_attribute(data: &[u8], pos: usize) -> Result<(Attribute, usize), DecompilerError> {
    let (name, pos) = get_u16(data, pos)?;
    let (length, pos) = get_u32(data, pos)?;
    let start = pos;
    let end = pos.saturating_add(length as usize);
    Ok((Attribute { name, start, end }, end))
}

fn parse_interfaces(data: &[u8], pos: usize) -> Result<(Vec<u16>, usize), DecompilerError> {
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

fn parse_const_pool(data: &[u8], pos: usize) -> Result<(ConstPool, usize), DecompilerError> {
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

        idx = idx.saturating_add(len);
    }
    Ok((ConstPool { pool }, pos))
}

#[derive(Debug)]
pub enum ConstEntry {
    Empty,
    Utf8(NuVec),
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
    Float(i32),
    Double(i64),
    Integer(i32),
    Long(i64),
    MehthodHandle,
    MethodType,
}

/// Returns the constant, next pos, how meany slots the constant takes
fn parse_constant(data: &[u8], pos: usize) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    let (kind, pos) = get_u8(data, pos)?;

    match kind {
        1 => parse_utf8_const(data, pos),
        3 => parse_integer_const(data, pos),
        4 => parse_float_const(data, pos),
        5 => parse_long_const(data, pos),
        6 => parse_double_const(data, pos),
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
        _ => Err(DecompilerError::UnknownConstant),
    }
}

fn parse_utf8_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    let (len, pos) = get_u16(data, pos)?;
    let len = len as usize;
    let end = pos.saturating_add(len);
    let inner = data.get(pos..end).ok_or(DecompilerError::EOF)?;
    let mu = mutf8::mutf8_to_utf8(inner).map_err(|_| DecompilerError::Mutf8)?;
    Ok((ConstEntry::Utf8(NuVec::new(&mu)), end, 1))
}

fn parse_class_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    let (name, pos) = get_u16(data, pos)?;
    Ok((ConstEntry::Class { name }, pos, 1))
}
fn parse_string_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    let (name, pos) = get_u16(data, pos)?;
    Ok((ConstEntry::String { name }, pos, 1))
}
fn parse_module_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    let (name, pos) = get_u16(data, pos)?;
    Ok((ConstEntry::Module { name }, pos, 1))
}
fn parse_package_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    let (name, pos) = get_u16(data, pos)?;
    Ok((ConstEntry::Package { name }, pos, 1))
}
const fn parse_field_ref_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (
        ConstEntry::FieldRef,
        pos.saturating_add(U16_LEN + U16_LEN),
        1,
    )
}

const fn parse_method_ref_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (
        ConstEntry::MethodRef,
        pos.saturating_add(U16_LEN + U16_LEN),
        1,
    )
}
fn parse_integer_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    let (content, pos) = get_i32(data, pos)?;
    Ok((ConstEntry::Integer(content), pos, 1))
}
fn parse_float_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    let (content, pos) = get_i32(data, pos)?;
    Ok((ConstEntry::Float(content), pos, 1))
}
fn parse_long_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    let (content, pos) = get_i64(data, pos)?;
    Ok((ConstEntry::Long(content), pos, 2))
}
fn parse_double_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    let (content, pos) = get_i64(data, pos)?;
    Ok((ConstEntry::Double(content), pos, 2))
}
const fn parse_interface_method_ref_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (
        ConstEntry::InterfaceMethodRef,
        pos.saturating_add(U16_LEN + U16_LEN),
        1,
    )
}
const fn parse_name_and_type_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (
        ConstEntry::NameAndType,
        pos.saturating_add(U16_LEN + U16_LEN),
        1,
    )
}
const fn parse_dynamic_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (
        ConstEntry::Dynamic,
        pos.saturating_add(U16_LEN + U16_LEN),
        1,
    )
}
const fn parse_method_handle_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (
        ConstEntry::MehthodHandle,
        pos.saturating_add(U8_LEN + U16_LEN),
        1,
    )
}
const fn parse_method_type_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 1 * u16
    (ConstEntry::MethodType, pos.saturating_add(U16_LEN), 1)
}
const fn parse_invoke_dynamic_const(pos: usize) -> (ConstEntry, usize, usize) {
    // content 2 * u16
    (
        ConstEntry::InvokeDynamic,
        pos.saturating_add(U16_LEN + U16_LEN),
        1,
    )
}

fn get_u8(data: &[u8], pos: usize) -> Result<(u8, usize), DecompilerError> {
    let Some(get) = data.get(pos) else {
        return Err(DecompilerError::EOF);
    };

    Ok((*get, pos.saturating_add(1)))
}
fn get_u16(data: &[u8], pos: usize) -> Result<(u16, usize), DecompilerError> {
    let next = pos.saturating_add(2);
    let items = data.get(pos..next).ok_or(DecompilerError::EOF)?;
    let get = <[u8; 2]>::try_from(items).map_err(|_| DecompilerError::Number)?;
    let out = u16::from_be_bytes(get);

    Ok((out, next))
}
fn get_u32(data: &[u8], pos: usize) -> Result<(u32, usize), DecompilerError> {
    let next = pos.saturating_add(4);
    let items = data.get(pos..next).ok_or(DecompilerError::EOF)?;
    let get = <[u8; 4]>::try_from(items).map_err(|_| DecompilerError::Number)?;
    let out = u32::from_be_bytes(get);

    Ok((out, next))
}
fn get_i32(data: &[u8], pos: usize) -> Result<(i32, usize), DecompilerError> {
    let next = pos.saturating_add(4);
    let items = data.get(pos..next).ok_or(DecompilerError::EOF)?;
    let get = <[u8; 4]>::try_from(items).map_err(|_| DecompilerError::Number)?;
    let out = i32::from_be_bytes(get);

    Ok((out, next))
}
fn get_i64(data: &[u8], pos: usize) -> Result<(i64, usize), DecompilerError> {
    let next = pos.saturating_add(8);
    let items = data.get(pos..next).ok_or(DecompilerError::EOF)?;
    let get = <[u8; 8]>::try_from(items).map_err(|_| DecompilerError::Number)?;
    let out = i64::from_be_bytes(get);

    Ok((out, next))
}

#[track_caller]
#[inline]
fn expect_data(data: &[u8], pos: usize, expected: &[u8]) -> Result<usize, DecompilerError> {
    let len = expected.len();
    let Some(get) = data.get(pos..pos.saturating_add(len)) else {
        return Err(DecompilerError::EOF);
    };

    let cond = get != expected;
    if cond {
        return Err(DecompilerError::NotAsExpected);
    }
    Ok(pos.saturating_add(len))
}

#[cfg(test)]
mod tests {
    use crate::{decompile_class, parse_class_signature_info, parse_field_type};
    use editorconfig::EditorConfigFilled;
    use expect_test::expect;
    use my_string::NuVec;

    #[cfg(not(windows))]
    #[test]
    fn relative_source() {
        use expect_test::expect;
        use my_string::NuVec;

        let ast = decompile_class(
            include_bytes!("../../parser/test/Everything.class"),
            &NuVec::new(b"ch.emilycares.Everything"),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();
        let expected = expect![[r"
            package ch.emilycares;
            public class Everything {
                int noprop;
                public int publicproperty;
                private int privateproperty;
                public void <init>() {}
                void method() {}
                public void public_method() {}
                private void private_method() {}
                int out() {}
                int add(int a, int b) {}
                static int sadd(int a, int b) {}
            }
        "]];
        expected.assert_eq(out);
    }

    #[test]
    fn everything() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/Everything.class"),
            &NuVec::new_static(b"ch.emilycares.Everything"),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r"
            package ch.emilycares;
            public class Everything {
                int noprop;
                public int publicproperty;
                private int privateproperty;
                public void <init>() {}
                void method() {}
                public void public_method() {}
                private void private_method() {}
                int out() {}
                int add(int a, int b) {}
                static int sadd(int a, int b) {}
            }
        "]];
        expected.assert_eq(out);
    }

    #[test]
    fn constants() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/Constants.class"),
            &NuVec::new_static(b"ch.emilycares.Constants"),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r#"
            package ch.emilycares;
            import java.net.Socket;
            import java.lang.String;
            public abstract class Constants {
                public static final java.lang.String CONSTANT_A = "A";
                public static final java.lang.String CONSTANT_B = "B";
                public static final java.lang.String CONSTANT_C = "C";
                public abstract void display() {}
                public abstract java.net.Socket createSocket(java.lang.String hostname, int port) throws java.io.IOException {}
            }
        "#]];
        expected.assert_eq(out);
    }

    #[test]
    fn types() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/Types.class"),
            &NuVec::new_static(b"ch.emilycares.Types"),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r"
            package ch.emilycares;
            import java.util.Map;
            import java.util.List;
            import java.lang.String;
            import java.util.logging.Logger;
            public class Types {
                java.util.logging.Logger LOG;
                boolean IS_ACTIVE;
                byte one_byte;
                int one_int;
                short one_short;
                long one_long;
                double one_double;
                float one_float;
                char one_char;
                java.lang.String one_string;
                java.util.List one_list;
                java.util.Map one_map;
                public void <init>() {}
                public static void main(java.lang.String[] ) {}
            }
        "]];
        expected.assert_eq(out);
    }

    #[test]
    fn super_base() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/Super.class"),
            &NuVec::new_static(b"ch.emilycares.Super"),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r"
            package ch.emilycares;
            public class Super {
                public void <init>() {}
            }
        "]];
        expected.assert_eq(out);
    }
    #[test]
    fn thrower() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/Thrower.class"),
            &NuVec::new_static(b"ch.emilycares.Thrower"),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();
        let expected = expect![[r"
            package ch.emilycares;
            public class Thrower {
                public void <init>() {}
                public void ioThrower() throws java.io.IOException {}
                public void ioThrower(int a) throws java.io.IOException, java.io.IOException {}
            }
        "]];
        expected.assert_eq(out);
    }
    #[test]
    fn super_interfaces() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/SuperInterface.class"),
            &NuVec::new_static(b"ch.emilycares.SuperInterface"),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r"
            package ch.emilycares;
            import java.util.stream.Stream;
            public abstract class SuperInterface {
                public java.util.stream.Stream<E> stream() {}
            }
        "]];
        expected.assert_eq(out);
    }
    #[test]
    fn variables() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/LocalVariableTable.class"),
            &NuVec::new_static(b"ch.emilycares.LocalVariableTable"),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r"
            package ch.emilycares;
            import java.util.HashMap;
            import java.util.HashSet;
            public class LocalVariableTable {
                private java.util.HashSet a;
                public void <init>() {}
                public void hereIsCode() {}
                public int hereIsCode(int a, int b) {}
            }
        "]];
        expected.assert_eq(out);
    }
    #[test]
    fn variants() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/Variants.class"),
            &NuVec::new_static(b"ch.emilycares.Variants"),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r"
            package ch.emilycares;
            import java.lang.String;
            public final enum Variants {
                public static final ch.emilycares.Variants A;
                public static final ch.emilycares.Variants B;
                public static final ch.emilycares.Variants C;
                private final java.lang.String tag;
                private static final ch.emilycares.Variants[] $VALUES;
                public static ch.emilycares.Variants[] values() {}
                public static ch.emilycares.Variants valueOf(java.lang.String name) {}
                private void <init>(java.lang.String $enum$name) {}
                public java.lang.String getTag() {}
                private static ch.emilycares.Variants[] $values() {}
                static void <clinit>() {}
            }
        "]];
        expected.assert_eq(out);
    }

    #[test]
    fn jtype_access() {
        let content = b"Ljava/util/HashMap<LA;LB;>.Factory";
        let result = parse_field_type(content, 0).unwrap();
        assert_eq!(content.len(), result.1);
        let expected = expect![[r#"
            AstJType {
                annotated: [],
                range: AstRange {
                    start: AstPoint { 0:0 },
                    end: AstPoint { 0:0 },
                },
                value: Access {
                    base: AstJType {
                        annotated: [],
                        range: AstRange {
                            start: AstPoint { 0:0 },
                            end: AstPoint { 0:0 },
                        },
                        value: Generic(
                            AstIdentifier {
                                range: AstRange {
                                    start: AstPoint { 0:0 },
                                    end: AstPoint { 0:0 },
                                },
                                value: "java.util.HashMap",
                            },
                            [
                                AstJType {
                                    annotated: [],
                                    range: AstRange {
                                        start: AstPoint { 0:0 },
                                        end: AstPoint { 0:0 },
                                    },
                                    value: Class(
                                        AstIdentifier {
                                            range: AstRange {
                                                start: AstPoint { 0:0 },
                                                end: AstPoint { 0:0 },
                                            },
                                            value: "A",
                                        },
                                    ),
                                },
                                AstJType {
                                    annotated: [],
                                    range: AstRange {
                                        start: AstPoint { 0:0 },
                                        end: AstPoint { 0:0 },
                                    },
                                    value: Class(
                                        AstIdentifier {
                                            range: AstRange {
                                                start: AstPoint { 0:0 },
                                                end: AstPoint { 0:0 },
                                            },
                                            value: "B",
                                        },
                                    ),
                                },
                            ],
                        ),
                    },
                    inner: AstJType {
                        annotated: [],
                        range: AstRange {
                            start: AstPoint { 0:0 },
                            end: AstPoint { 0:0 },
                        },
                        value: Class(
                            AstIdentifier {
                                range: AstRange {
                                    start: AstPoint { 0:0 },
                                    end: AstPoint { 0:0 },
                                },
                                value: "Factory",
                            },
                        ),
                    },
                },
            }
        "#]];
        expected.assert_debug_eq(&result.0);
    }

    #[test]
    fn class_signature() {
        let content = NuVec::new_static(
            b"<E:Ljava/lang/Object;>Ljava/lang/Object;Ljava/util/SequencedCollection<TE;>;",
        );
        let result = parse_class_signature_info(&content).unwrap();
        let expected = expect![[r#"
            ClassSignature {
                args: [
                    "E",
                ],
                ret: AstJType {
                    annotated: [],
                    range: AstRange {
                        start: AstPoint { 0:0 },
                        end: AstPoint { 0:0 },
                    },
                    value: Class(
                        AstIdentifier {
                            range: AstRange {
                                start: AstPoint { 0:0 },
                                end: AstPoint { 0:0 },
                            },
                            value: "java.lang.Object",
                        },
                    ),
                },
            }
        "#]];
        expected.assert_debug_eq(&result.0);
    }
}
