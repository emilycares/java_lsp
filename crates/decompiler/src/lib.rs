#![deny(clippy::nursery)]
#![deny(clippy::perf)]
#![deny(clippy::arithmetic_side_effects)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::collections::HashSet;

use ast::types::{
    AstAnnotated, AstAnnotatedParameter, AstAnnotatedParameterKind, AstAvailability,
    AstBaseExpression, AstBlock, AstBoolean, AstClass, AstClassBlock, AstClassMethod,
    AstClassVariable, AstDouble, AstEnumeration, AstEnumerationVariant, AstExpression,
    AstExpressionIdentifier, AstExpressionKind, AstExpressionOperator, AstExpressionOrAnnotated,
    AstFile, AstIdentifier, AstImport, AstImportUnit, AstInt, AstInterface, AstInterfaceConstant,
    AstInterfaceMethod, AstInterfaceMethodDefault, AstJType, AstJTypeExpression, AstJTypeKind,
    AstMethodHeader, AstMethodParameter, AstMethodParameterFlags, AstMethodParameters, AstPackage,
    AstPoint, AstRange, AstRecord, AstRecordEntries, AstRecordEntry, AstSuperClass, AstThing,
    AstThingAttributes, AstThrowsDeclaration, AstTopLevel, AstTypeParameter, AstTypeParameters,
    AstValueNuget, AstValuesWithAnnotated, AstVolatileTransient,
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
    ExpectedClass,
    ElementValueUnknowntag,
    ExpectedInteger,
    ExpectedLong,
    ExpectedDouble,
    ExpectedFloat,
}

pub fn decompile_class(data: &[u8], class_path: &NuVec) -> Result<AstFile, DecompilerError> {
    let pos =
        expect_data(data, 0, &[0xCA, 0xFE, 0xBA, 0xBE]).map_err(|_| DecompilerError::NotAClass)?;

    let pos = pos.saturating_add(U16_LEN + U16_LEN);

    let (const_pool, pos) = parse_const_pool(data, pos)?;

    let (class_access_flags, pos) = parse_class_access_flags(data, pos)?;
    let is_interface = class_access_flags.intersects(ClassAccessFlags::Interface);
    let (this_class, pos) = get_u16(data, pos)?;
    let class_name = lookup_class_name(&const_pool, this_class.into())?;
    let (super_class, pos) = get_u16(data, pos)?;

    let (_, pos) = parse_interfaces(data, pos)?;
    let (fields, enum_variants, pos) = parse_fields(data, pos, &const_pool)?;
    let mut used_classes = HashSet::new();
    let (methods, pos) = parse_methods(
        data,
        pos,
        &const_pool,
        is_interface,
        &mut used_classes,
        &class_name,
    )?;
    let (attributes, _) = parse_attributes(data, pos)?;

    let mut annotated = Vec::new();
    let mut superclass = Vec::new();
    let mut type_parameters = None;
    let mut record_entries = None;
    let mut permits = Vec::new();

    if super_class != 0 {
        let c = lookup_class_name(&const_pool, super_class as usize)?;
        if c != "Object" && c != "java.lang.Object" {
            superclass.push(AstSuperClass::Name(ntoident(c)));
        }
    }

    let mut deprecated = false;
    let mut is_annotated = false;

    for a in &attributes {
        if a.name == 0 {
            continue;
        }
        let attribute_name = lookup_string(&const_pool, a.name)?;

        if attribute_name == "Signature" {
            let info = a.lookup(data)?;
            let (sig, _) = get_u16(info, 0)?;
            let sig = lookup_string(&const_pool, sig)?;
            let (sig, _) = parse_class_signature_info(&sig)?;
            if !sig.args.is_empty() {
                let mut parameters = Vec::with_capacity(sig.args.len());
                for p in sig.args {
                    parameters.push(AstTypeParameter {
                        range: RANGE,
                        annotated: Vec::new(),
                        name: ntoident(p),
                        supperclass: None,
                    });
                }
                type_parameters = Some(AstTypeParameters {
                    range: RANGE,
                    parameters,
                });
            }
            for e in sig.extends {
                if let AstJTypeKind::Class(AstIdentifier { ref value, .. }) = e.value
                    && (value == "Object" || value == "java.lang.Object")
                {
                    continue;
                }
                superclass.push(AstSuperClass::JType(e));
            }
        } else if attribute_name == "Code" {
            let info = a.lookup(data)?;
            let (out, _) = parse_code_attribute(info, 0, a.start, a.end)?;
            parse_used_classes(&const_pool, data, &out, &mut used_classes)?;
        } else if attribute_name == "Deprecated" {
            deprecated = true;
        } else if attribute_name == "Record" {
            let info = a.lookup(data)?;
            record_entries = Some(parse_record_attribute(info, 0, &const_pool)?.0);
        } else if attribute_name == "PermittedSubclasses" {
            let info = a.lookup(data)?;
            permits = parse_permitted_subclasses_attribute(info, 0, &const_pool)?.0;
        } else if attribute_name == "RuntimeVisibleAnnotations"
            || attribute_name == "RuntimeInvisibleAnnotations"
        {
            let info = a.lookup(data)?;
            parse_runtime_visible_annotations(info, 0, &const_pool, &mut annotated)?;
            is_annotated = true;
        } else if attribute_name == "SourceFile" {
            // Not interesting
        }
    }

    if !is_annotated && deprecated {
        annotated.push(AstAnnotated {
            range: RANGE,
            name: ntoident(NuVec::Static(b"Deprecated")),
            parameters: AstAnnotatedParameterKind::None,
        });
    }

    for f in &fields {
        jtype_class_names(f.jtype.clone(), &mut used_classes);
    }

    let mut file = AstFile { top: Vec::new() };
    let package = class_path
        .trim_end_matches(class_name.as_bytes())
        .trim_end_matches_byte(b'.');
    file.top.push(AstTopLevel::Package(AstPackage {
        range: RANGE,
        annotated: Vec::new(),
        name: ntoident(package),
    }));
    let mut imports: Vec<&NuVec> = used_classes.iter().filter(|i| *i != class_path).collect();
    imports.sort();
    file.top.extend(imports.into_iter().map(|i| {
        AstTopLevel::Import(AstImport {
            range: RANGE,
            unit: AstImportUnit::Class(ntoident(i.clone())),
        })
    }));

    let mut availability = AstAvailability::empty();
    if class_access_flags.intersects(ClassAccessFlags::Public) {
        availability |= AstAvailability::Public;
    }
    if class_access_flags.intersects(ClassAccessFlags::Final) {
        availability |= AstAvailability::Final;
    }
    if !is_interface && class_access_flags.intersects(ClassAccessFlags::Abstract) {
        availability |= AstAvailability::Abstract;
    }

    if class_access_flags.intersects(ClassAccessFlags::Enum) {
        let enu = AstEnumeration {
            range: RANGE,
            availability,
            attributes: AstThingAttributes::empty(),
            annotated,
            name: ntoident(class_name),
            implements: Vec::new(),
            permits,
            superclass,
            variants: enum_variants,
            methods,
            variables: fields,
            constructors: Vec::new(),
            static_blocks: Vec::new(),
            inner: Vec::new(),
        };
        let thing = AstTopLevel::Thing(Box::new(AstThing::Enumeration(enu)));
        file.top.push(thing);
    } else if is_interface {
        let int = AstInterface {
            range: RANGE,
            availability,
            attributes: AstThingAttributes::empty(),
            annotated,
            name: ntoident(class_name),
            type_parameters,
            extends: None,
            constants: fields
                .into_iter()
                .map(|i| AstInterfaceConstant {
                    range: RANGE,
                    annotated: i.annotated,
                    availability: i.availability,
                    name: i.name,
                    jtype: i.jtype,
                    expression: i.expression,
                })
                .collect(),
            methods: methods
                .clone()
                .into_iter()
                .filter(|i| i.block.is_none())
                .map(|i| AstInterfaceMethod {
                    range: RANGE,
                    header: i.header,
                })
                .collect(),
            default_methods: methods
                .into_iter()
                .filter_map(|mut i| {
                    if let Some(block) = i.block {
                        i.header.default = true;
                        return Some(AstInterfaceMethodDefault {
                            range: RANGE,
                            header: i.header,
                            block,
                        });
                    }
                    None
                })
                .collect(),
            inner: Vec::new(),
            permits,
        };
        let thing = AstTopLevel::Thing(Box::new(AstThing::Interface(int)));
        file.top.push(thing);
    } else if let Some(record_entries) = record_entries {
        let record = AstRecord {
            range: RANGE,
            availability,
            attributes: AstThingAttributes::empty(),
            annotated,
            name: ntoident(class_name),
            type_parameters,
            superclass,
            implements: Vec::new(),
            block: AstClassBlock {
                range: RANGE,
                variables: fields,
                methods,
                constructors: Vec::new(),
                static_blocks: Vec::new(),
                inner: Vec::new(),
                blocks: Vec::new(),
            },
            record_entries,
        };
        let thing = AstTopLevel::Thing(Box::new(AstThing::Record(record)));
        file.top.push(thing);
    } else {
        let class = AstClass {
            range: RANGE,
            availability,
            attributes: AstThingAttributes::empty(),
            annotated,
            name: ntoident(class_name),
            type_parameters,
            superclass,
            implements: Vec::new(),
            permits,
            block: AstClassBlock {
                range: RANGE,
                variables: fields,
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

const RANGE: AstRange = AstRange {
    start: AstPoint { line: 0, col: 0 },
    end: AstPoint { line: 0, col: 0 },
};

const fn ntoident(value: NuVec) -> AstIdentifier {
    AstIdentifier {
        value,
        range: RANGE,
    }
}

fn lookup_class_name(c: &ConstPool, index: usize) -> Result<NuVec, DecompilerError> {
    match c.pool.get(index.saturating_sub(1)) {
        Some(ConstEntry::Class { name }) => Ok(lookup_string(c, *name)?
            .to_str()
            .split('/')
            .next_back()
            .map(Into::into)
            .ok_or(DecompilerError::InvalidName)?),
        _ => Err(DecompilerError::ExpectedClass),
    }
}

const NO_NAME_CHARS: [&[u8; 1]; 26] = [
    b"a", b"b", b"c", b"d", b"e", b"f", b"g", b"h", b"i", b"j", b"k", b"l", b"m", b"n", b"o", b"p",
    b"q", b"r", b"s", b"t", b"u", b"v", b"w", b"x", b"y", b"z",
];
fn no_name(i: usize) -> NuVec {
    #[allow(clippy::arithmetic_side_effects)]
    let div = i / NO_NAME_CHARS.len();
    if div == 0 {
        #[allow(clippy::arithmetic_side_effects)]
        let i = i % NO_NAME_CHARS.len();
        NuVec::Static(NO_NAME_CHARS[i])
    } else {
        let mut out = NuVecBuilder::new();
        for _ in 0..div {
            #[allow(clippy::arithmetic_side_effects)]
            let i = i % NO_NAME_CHARS.len();
            out.pusha(NO_NAME_CHARS[i]);
        }
        out.finish()
    }
}

fn parse_method(
    c: &ConstPool,
    data: &[u8],
    availability: AstAvailability,
    name: u16,
    descriptor: u16,
    attributes: Vec<Attribute>,
    class_name: &NuVec,
) -> Result<(AstClassMethod, Option<CodeAttribute>), DecompilerError> {
    let lname = lookup_string(c, name)?;
    let lname = match lname.as_bytes() {
        b"<init>" => class_name.clone(),
        b"<clinit>" => NuVec::Static(b"clinit"),
        _ => lname,
    };
    let mut out = AstClassMethod {
        range: RANGE,
        header: AstMethodHeader {
            range: RANGE,
            availability,
            name: ntoident(lname),
            jtype: AstJType {
                range: RANGE,
                annotated: Vec::new(),
                value: AstJTypeKind::Void,
            },
            parameters: AstMethodParameters {
                range: RANGE,
                parameters: Vec::new(),
            },
            throws: None,
            type_parameters: None,
            annotated: Vec::new(),
            default: false,
        },
        block: None,
    };

    let mut code_attribute = None;

    let mut parameter_names = Vec::new();
    let mut deprecated = false;
    let mut signature_index = None;
    let mut method_parameter_index = None;
    let mut exception_index = None;
    let mut ret = AstJType {
        range: RANGE,
        value: AstJTypeKind::Void,
        annotated: Vec::new(),
    };
    let mut is_annotated = false;

    for (index, attribute) in attributes.iter().enumerate() {
        let attribute_name = lookup_string(c, attribute.name)?;
        if attribute_name == "Signature" {
            signature_index = Some(index);
        } else if attribute_name == "MethodParameters" {
            method_parameter_index = Some(index);
        } else if attribute_name == "Exceptions" {
            exception_index = Some(index);
        } else if attribute_name == "Deprecated" {
            deprecated = true;
        } else if attribute_name == "Code" {
            let info = attribute.lookup(data)?;
            let (ca, _) = parse_code_attribute(info, 0, attribute.start, attribute.end)?;
            code_attribute = Some(ca);
            out.block = Some(AstBlock {
                range: RANGE,
                entries: Vec::new(),
            });
        } else if attribute_name == "RuntimeVisibleAnnotations"
            || attribute_name == "RuntimeInvisibleAnnotations"
        {
            let info = attribute.lookup(data)?;
            parse_runtime_visible_annotations(info, 0, c, &mut out.header.annotated)?;
            is_annotated = true;
        }
    }
    let no_parameter_names_and_signature =
        method_parameter_index.is_none() && signature_index.is_none();
    if no_parameter_names_and_signature {
        let desc = lookup_string(c, descriptor)?;
        let (_, md) = parse_method_descriptor(&desc)?;
        ret = md.return_type;
        for (i, p) in md.param_types.into_iter().enumerate() {
            out.header.parameters.parameters.push(AstMethodParameter {
                range: RANGE,
                annotated: Vec::new(),
                jtype: p,
                name: ntoident(no_name(i)),
                flags: AstMethodParameterFlags::empty(),
            });
        }
    }

    if let Some(index) = method_parameter_index {
        let attribute = attributes
            .get(index)
            .ok_or(DecompilerError::InvalidAttributeIndex)?;

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
            let (_, md) = parse_method_descriptor(&lookup_string(c, descriptor)?)?;
            ret = md.return_type;
            let mut params = md.param_types.into_iter();
            for (i, p) in info.iter().enumerate() {
                let jtype = params.next().ok_or(DecompilerError::NotEnogthParams)?;
                if p.name_index == 0 {
                    out.header.parameters.parameters.push(AstMethodParameter {
                        range: RANGE,
                        annotated: Vec::new(),
                        jtype,
                        name: ntoident(no_name(i)),
                        flags: AstMethodParameterFlags::empty(),
                    });
                } else if let Some(name) = lookup_string(c, p.name_index)
                    .ok()
                    .filter(|i| !i.is_empty())
                {
                    // parameters.push(Parameter { name, jtype });
                    out.header.parameters.parameters.push(AstMethodParameter {
                        range: RANGE,
                        annotated: Vec::new(),
                        jtype,
                        name: ntoident(name),
                        flags: AstMethodParameterFlags::empty(),
                    });
                } else {
                    out.header.parameters.parameters.push(AstMethodParameter {
                        range: RANGE,
                        annotated: Vec::new(),
                        jtype,
                        name: ntoident(no_name(i)),
                        flags: AstMethodParameterFlags::empty(),
                    });
                }
            }
        }
    }

    if let Some(index) = signature_index {
        let attribute = attributes
            .get(index)
            .ok_or(DecompilerError::InvalidAttributeIndex)?;

        let info = attribute.lookup(data)?;
        let (sig, _) = get_u16(info, 0)?;
        let sig = lookup_string(c, sig)?;
        let (sig, _) = parse_method_signature_info(&sig)?;
        let mut name_iter = parameter_names.into_iter();
        sig.params.iter().enumerate().for_each(|(i, jtype)| {
            if let Some(name) = name_iter.next().flatten() {
                out.header.parameters.parameters.push(AstMethodParameter {
                    range: RANGE,
                    annotated: Vec::new(),
                    jtype: jtype.clone(),
                    name: ntoident(name),
                    flags: AstMethodParameterFlags::empty(),
                });
            } else {
                out.header.parameters.parameters.push(AstMethodParameter {
                    range: RANGE,
                    annotated: Vec::new(),
                    jtype: jtype.clone(),
                    name: ntoident(no_name(i)),
                    flags: AstMethodParameterFlags::empty(),
                });
            }
        });

        ret = sig.ret;
    }

    if let Some(index) = exception_index {
        let attribute = attributes
            .get(index)
            .ok_or(DecompilerError::InvalidAttributeIndex)?;
        let info = attribute.lookup(data)?;
        let (info, _) = parse_exceptions_attribute(info, 0)?;

        if !info.is_empty() {
            let mut throws = Vec::new();
            for exception in info {
                let class_name = lookup_string(c, exception)?;
                throws.push(AstJType {
                    range: RANGE,
                    annotated: Vec::new(),
                    value: AstJTypeKind::Class(ntoident(class_name.replace_byte(b'/', b'.'))),
                });
            }
            out.header.throws = Some(AstThrowsDeclaration {
                range: RANGE,
                parameters: throws,
            });
        }
    }
    out.header.jtype = ret;
    if !is_annotated && deprecated {
        out.header.annotated.push(AstAnnotated {
            range: RANGE,
            name: ntoident(NuVec::Static(b"Deprecated")),
            parameters: AstAnnotatedParameterKind::None,
        });
    }

    Ok((out, code_attribute))
}

fn parse_runtime_visible_annotations(
    data: &[u8],
    pos: usize,
    c: &ConstPool,
    annotated: &mut Vec<AstAnnotated>,
) -> Result<((), usize), DecompilerError> {
    let (count, pos) = get_u16(data, pos)?;
    let mut pos = pos;
    for _ in 0..count {
        let (ann, ipos) = parse_annotation(data, pos, c)?;
        annotated.push(ann);

        pos = ipos;
    }
    Ok(((), pos))
}

fn parse_annotation(
    data: &[u8],
    pos: usize,
    c: &ConstPool,
) -> Result<(AstAnnotated, usize), DecompilerError> {
    let (type_index, pos) = get_u16(data, pos)?;
    let content = lookup_string(c, type_index)?;
    let ty = parse_field_type(content.as_bytes(), 0)?;
    let name = match ty.0.value {
        AstJTypeKind::Class(ident) => Ok(ident),
        _ => Err(DecompilerError::ExpectedClass),
    }?;
    let (num_element_value_pairs, pos) = get_u16(data, pos)?;
    let mut pos = pos;
    let mut element_values = Vec::new();
    for _ in 0..num_element_value_pairs {
        let (element_name_index, ipos) = get_u16(data, pos)?;
        let name = lookup_string(c, element_name_index)?;
        let (value, ipos) = parse_element_value(data, ipos, c)?;
        let p = element_value_to_annotated_parameter(name, value, c)?;
        element_values.push(p);

        pos = ipos;
    }
    if element_values.is_empty() {
        return Ok((
            AstAnnotated {
                range: RANGE,
                name,
                parameters: AstAnnotatedParameterKind::None,
            },
            pos,
        ));
    }
    Ok((
        AstAnnotated {
            range: RANGE,
            name,
            parameters: AstAnnotatedParameterKind::Parameter(element_values),
        },
        pos,
    ))
}

fn element_value_to_annotated_parameter(
    name: NuVec,
    value: ElementValue,
    c: &ConstPool,
) -> Result<AstAnnotatedParameter, DecompilerError> {
    match value {
        ElementValue::ConstValueIndexInteger(i) => {
            let Some(ConstEntry::Integer(i)) = c.pool.get(i.saturating_sub(1) as usize) else {
                return Err(DecompilerError::ExpectedInteger)?;
            };
            let value = NuVec::new(i.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Int(AstInt {
                    range: RANGE,
                    value,
                }))),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(AstAnnotatedParameter::NamedExpression {
                range: RANGE,
                name: ntoident(name),
                expression: expr,
            })
        }
        ElementValue::ConstValueIndexChar(i) => {
            let Some(ConstEntry::Integer(i)) = c.pool.get(i.saturating_sub(1) as usize) else {
                return Err(DecompilerError::ExpectedInteger)?;
            };
            let value = NuVec::new(i.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::CharLiteral {
                    value,
                    range: RANGE,
                })),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(AstAnnotatedParameter::NamedExpression {
                range: RANGE,
                name: ntoident(name),
                expression: expr,
            })
        }
        ElementValue::ConstValueIndexDouble(i) => {
            let Some(ConstEntry::Double(i)) = c.pool.get(i.saturating_sub(1) as usize) else {
                return Err(DecompilerError::ExpectedDouble)?;
            };
            let value = NuVec::new(i.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Double(
                    AstDouble {
                        range: RANGE,
                        value,
                    },
                ))),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(AstAnnotatedParameter::NamedExpression {
                range: RANGE,
                name: ntoident(name),
                expression: expr,
            })
        }
        ElementValue::ConstValueIndexFloat(i) => {
            let Some(ConstEntry::Float(i)) = c.pool.get(i.saturating_sub(1) as usize) else {
                return Err(DecompilerError::ExpectedFloat)?;
            };
            let value = NuVec::new(i.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Long(
                    AstInt {
                        range: RANGE,
                        value,
                    },
                ))),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(AstAnnotatedParameter::NamedExpression {
                range: RANGE,
                name: ntoident(name),
                expression: expr,
            })
        }
        ElementValue::ConstValueIndexLong(i) => {
            let Some(ConstEntry::Long(i)) = c.pool.get(i.saturating_sub(1) as usize) else {
                return Err(DecompilerError::ExpectedLong)?;
            };
            let value = NuVec::new(i.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Long(
                    AstInt {
                        range: RANGE,
                        value,
                    },
                ))),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(AstAnnotatedParameter::NamedExpression {
                range: RANGE,
                name: ntoident(name),
                expression: expr,
            })
        }
        ElementValue::ConstValueIndexBoolean(i) => {
            let Some(ConstEntry::Integer(i)) = c.pool.get(i.saturating_sub(1) as usize) else {
                return Err(DecompilerError::ExpectedInteger)?;
            };
            let value = *i != 0;
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(
                    AstValueNuget::BooleanLiteral(AstBoolean {
                        range: RANGE,
                        value,
                    }),
                )),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(AstAnnotatedParameter::NamedExpression {
                range: RANGE,
                name: ntoident(name),
                expression: expr,
            })
        }
        ElementValue::ConstValueIndexString(index) => {
            let value = lookup_string(c, index)?;
            Ok(AstAnnotatedParameter::NamedExpression {
                range: RANGE,
                name: ntoident(name),
                expression: vec![AstExpressionKind::Base(AstBaseExpression {
                    range: RANGE,
                    ident: Some(AstExpressionIdentifier::Nuget(
                        AstValueNuget::StringLiteral {
                            range: RANGE,
                            value,
                        },
                    )),
                    values: None,
                    operator: AstExpressionOperator::None,
                })],
            })
        }
        ElementValue::EnumConstValue {
            type_name_index,
            const_name_index,
        } => {
            let content = lookup_string(c, type_name_index)?;
            let ty = parse_field_type(content.as_bytes(), 0)?;
            let con = lookup_string(c, const_name_index)?;
            Ok(AstAnnotatedParameter::NamedExpression {
                range: RANGE,
                name: ntoident(name),
                expression: vec![
                    AstExpressionKind::JType(AstJTypeExpression {
                        range: RANGE,
                        jtype: ty.0,
                    }),
                    AstExpressionKind::Base(AstBaseExpression {
                        range: RANGE,
                        ident: None,
                        values: None,
                        operator: AstExpressionOperator::Dot(RANGE),
                    }),
                    AstExpressionKind::Base(AstBaseExpression {
                        range: RANGE,
                        ident: Some(AstExpressionIdentifier::Identifier(ntoident(con))),
                        values: None,
                        operator: AstExpressionOperator::None,
                    }),
                ],
            })
        }
        ElementValue::ClassInfoIndex(i) => {
            let cname = lookup_class_name(c, i as usize)?;
            Ok(AstAnnotatedParameter::NamedExpression {
                range: RANGE,
                name: ntoident(name),
                expression: vec![AstExpressionKind::Base(AstBaseExpression {
                    range: RANGE,
                    ident: Some(AstExpressionIdentifier::Identifier(ntoident(cname))),
                    values: None,
                    operator: AstExpressionOperator::None,
                })],
            })
        }
        ElementValue::Annotation(annotated) => Ok(AstAnnotatedParameter::NamedAnnotated {
            range: RANGE,
            name: ntoident(name),
            annotated,
        }),
        ElementValue::ArrayValue(arr) => Ok(AstAnnotatedParameter::NamedArray {
            range: RANGE,
            name: ntoident(name),
            values: AstValuesWithAnnotated {
                range: RANGE,
                values: arr
                    .into_iter()
                    .flat_map(|i| element_value_to_expression_or_annotated(i, c))
                    .collect(),
            },
        }),
    }
}
fn element_value_to_expression_or_annotated(
    value: ElementValue,
    c: &ConstPool,
) -> Result<AstExpressionOrAnnotated, DecompilerError> {
    match value {
        ElementValue::ConstValueIndexInteger(i) => {
            let Some(ConstEntry::Integer(i)) = c.pool.get(i.saturating_sub(1) as usize) else {
                return Err(DecompilerError::ExpectedInteger)?;
            };
            let value = NuVec::new(i.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Int(AstInt {
                    range: RANGE,
                    value,
                }))),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(AstExpressionOrAnnotated::Expression(expr))
        }
        ElementValue::ConstValueIndexChar(i) => {
            let Some(ConstEntry::Integer(i)) = c.pool.get(i.saturating_sub(1) as usize) else {
                return Err(DecompilerError::ExpectedInteger)?;
            };
            let value = NuVec::new(i.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::CharLiteral {
                    value,
                    range: RANGE,
                })),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(AstExpressionOrAnnotated::Expression(expr))
        }
        ElementValue::ConstValueIndexDouble(i) => {
            let Some(ConstEntry::Double(i)) = c.pool.get(i.saturating_sub(1) as usize) else {
                return Err(DecompilerError::ExpectedDouble)?;
            };
            let value = NuVec::new(i.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Double(
                    AstDouble {
                        range: RANGE,
                        value,
                    },
                ))),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(AstExpressionOrAnnotated::Expression(expr))
        }
        ElementValue::ConstValueIndexFloat(i) => {
            let Some(ConstEntry::Float(i)) = c.pool.get(i.saturating_sub(1) as usize) else {
                return Err(DecompilerError::ExpectedFloat)?;
            };
            let value = NuVec::new(i.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Long(
                    AstInt {
                        range: RANGE,
                        value,
                    },
                ))),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(AstExpressionOrAnnotated::Expression(expr))
        }
        ElementValue::ConstValueIndexLong(i) => {
            let Some(ConstEntry::Long(i)) = c.pool.get(i.saturating_sub(1) as usize) else {
                return Err(DecompilerError::ExpectedLong)?;
            };
            let value = NuVec::new(i.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Long(
                    AstInt {
                        range: RANGE,
                        value,
                    },
                ))),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(AstExpressionOrAnnotated::Expression(expr))
        }
        ElementValue::ConstValueIndexBoolean(i) => {
            let Some(ConstEntry::Integer(i)) = c.pool.get(i.saturating_sub(1) as usize) else {
                return Err(DecompilerError::ExpectedInteger)?;
            };
            let value = *i != 0;
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(
                    AstValueNuget::BooleanLiteral(AstBoolean {
                        range: RANGE,
                        value,
                    }),
                )),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(AstExpressionOrAnnotated::Expression(expr))
        }
        ElementValue::ConstValueIndexString(index) => {
            let value = lookup_string(c, index)?;
            Ok(AstExpressionOrAnnotated::Expression(vec![
                AstExpressionKind::Base(AstBaseExpression {
                    range: RANGE,
                    ident: Some(AstExpressionIdentifier::Nuget(
                        AstValueNuget::StringLiteral {
                            range: RANGE,
                            value,
                        },
                    )),
                    values: None,
                    operator: AstExpressionOperator::None,
                }),
            ]))
        }
        ElementValue::EnumConstValue {
            type_name_index,
            const_name_index,
        } => {
            let content = lookup_string(c, type_name_index)?;
            let ty = parse_field_type(content.as_bytes(), 0)?;
            let ty = match ty.0.value {
                AstJTypeKind::Class(ident) => Ok(ident),
                _ => Err(DecompilerError::ExpectedClass),
            }?;
            let con = lookup_string(c, const_name_index)?;
            Ok(AstExpressionOrAnnotated::Expression(vec![
                AstExpressionKind::Base(AstBaseExpression {
                    range: RANGE,
                    ident: Some(AstExpressionIdentifier::Identifier(ty)),
                    values: None,
                    operator: AstExpressionOperator::None,
                }),
                AstExpressionKind::Base(AstBaseExpression {
                    range: RANGE,
                    ident: None,
                    values: None,
                    operator: AstExpressionOperator::Dot(RANGE),
                }),
                AstExpressionKind::Base(AstBaseExpression {
                    range: RANGE,
                    ident: Some(AstExpressionIdentifier::Identifier(ntoident(con))),
                    values: None,
                    operator: AstExpressionOperator::None,
                }),
            ]))
        }
        ElementValue::ClassInfoIndex(i) => {
            let cname = lookup_class_name(c, i as usize)?;
            Ok(AstExpressionOrAnnotated::Expression(vec![
                AstExpressionKind::Base(AstBaseExpression {
                    range: RANGE,
                    ident: Some(AstExpressionIdentifier::Identifier(ntoident(cname))),
                    values: None,
                    operator: AstExpressionOperator::None,
                }),
            ]))
        }
        ElementValue::Annotation(ast_annotated) => {
            Ok(AstExpressionOrAnnotated::Annotated(ast_annotated))
        }
        ElementValue::ArrayValue(_) => Ok(AstExpressionOrAnnotated::Annotated(AstAnnotated {
            range: RANGE,
            name: ntoident(NuVec::Static(b"TODO")),
            parameters: AstAnnotatedParameterKind::None,
        })),
    }
}

#[derive(Debug)]
enum ElementValue {
    ConstValueIndexInteger(u16),
    ConstValueIndexChar(u16),
    ConstValueIndexDouble(u16),
    ConstValueIndexFloat(u16),
    ConstValueIndexLong(u16),
    ConstValueIndexBoolean(u16),
    ConstValueIndexString(u16),
    EnumConstValue {
        type_name_index: u16,
        const_name_index: u16,
    },
    ClassInfoIndex(u16),
    Annotation(AstAnnotated),
    ArrayValue(Vec<Self>),
}
fn parse_element_value(
    data: &[u8],
    pos: usize,
    c: &ConstPool,
) -> Result<(ElementValue, usize), DecompilerError> {
    let (tag, pos) = get_u8(data, pos)?;
    match tag {
        b'B' => {
            let (index, pos) = get_u16(data, pos)?;
            Ok((ElementValue::ConstValueIndexInteger(index), pos))
        }
        b'C' => {
            let (index, pos) = get_u16(data, pos)?;
            Ok((ElementValue::ConstValueIndexChar(index), pos))
        }
        b'D' => {
            let (index, pos) = get_u16(data, pos)?;
            Ok((ElementValue::ConstValueIndexDouble(index), pos))
        }
        b'F' => {
            let (index, pos) = get_u16(data, pos)?;
            Ok((ElementValue::ConstValueIndexFloat(index), pos))
        }
        b'J' => {
            let (index, pos) = get_u16(data, pos)?;
            Ok((ElementValue::ConstValueIndexLong(index), pos))
        }
        b'S' => {
            let (index, pos) = get_u16(data, pos)?;
            Ok((ElementValue::ConstValueIndexInteger(index), pos))
        }
        b'Z' => {
            let (index, pos) = get_u16(data, pos)?;
            Ok((ElementValue::ConstValueIndexBoolean(index), pos))
        }
        b's' => {
            let (index, pos) = get_u16(data, pos)?;
            Ok((ElementValue::ConstValueIndexString(index), pos))
        }
        b'e' => {
            let (type_name_index, pos) = get_u16(data, pos)?;
            let (const_name_index, pos) = get_u16(data, pos)?;
            Ok((
                ElementValue::EnumConstValue {
                    type_name_index,
                    const_name_index,
                },
                pos,
            ))
        }
        b'c' => {
            let (index, pos) = get_u16(data, pos)?;
            Ok((ElementValue::ClassInfoIndex(index), pos))
        }
        b'@' => {
            let (an, pos) = parse_annotation(data, pos, c)?;
            Ok((ElementValue::Annotation(an), pos))
        }
        b'[' => {
            let (count, pos) = get_u16(data, pos)?;
            let mut pos = pos;
            let mut out = Vec::new();
            for _ in 0..count {
                let (o, ipos) = parse_element_value(data, pos, c)?;
                out.push(o);
                pos = ipos;
            }
            Ok((ElementValue::ArrayValue(out), pos))
        }
        _ => Err(DecompilerError::ElementValueUnknowntag),
    }
}

fn parse_permitted_subclasses_attribute(
    data: &[u8],
    pos: usize,
    c: &ConstPool,
) -> Result<(Vec<AstJType>, usize), DecompilerError> {
    let (count, pos) = get_u16(data, pos)?;
    let mut pos = pos;
    let mut permits = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let (idx, npos) = get_u16(data, pos)?;
        let name = lookup_class_name(c, idx as usize)?;
        permits.push(AstJType {
            annotated: Vec::new(),
            range: RANGE,
            value: AstJTypeKind::Class(ntoident(name)),
        });
        pos = npos;
    }
    Ok((permits, pos))
}

fn parse_record_attribute(
    data: &[u8],
    pos: usize,
    c: &ConstPool,
) -> Result<(AstRecordEntries, usize), DecompilerError> {
    let (count, pos) = get_u16(data, pos)?;
    let mut pos = pos;
    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let (o, npos) = parse_record_attribute_inner(data, pos, c)?;
        entries.push(o);
        pos = npos;
    }
    Ok((
        AstRecordEntries {
            range: RANGE,
            entries,
        },
        pos,
    ))
}
fn parse_record_attribute_inner(
    data: &[u8],
    pos: usize,
    c: &ConstPool,
) -> Result<(AstRecordEntry, usize), DecompilerError> {
    let (name_index, pos) = get_u16(data, pos)?;
    let (descriptor_index, pos) = get_u16(data, pos)?;
    let (attributes, pos) = parse_attributes(data, pos)?;

    let mut jtype = AstJType {
        annotated: Vec::new(),
        range: RANGE,
        value: AstJTypeKind::Var,
    };
    let mut annotated = Vec::new();

    for a in attributes {
        if a.name == 0 {
            continue;
        }
        let attribute_name = lookup_string(c, a.name)?;
        if attribute_name == "Signature" {
            let info = a.lookup(data)?;
            let (sig, _) = get_u16(info, 0)?;
            let sig = lookup_string(c, sig)?;
            let (sig, _) = parse_class_signature_info(&sig)?;
            if let Some(first) = sig.extends.first() {
                jtype = first.clone();
            }
        } else if attribute_name == "RuntimeVisibleAnnotations"
            || attribute_name == "RuntimeInvisibleAnnotations"
        {
            let info = a.lookup(data)?;
            parse_runtime_visible_annotations(info, 0, c, &mut annotated)?;
        }
    }

    if matches!(jtype.value, AstJTypeKind::Var) {
        let sig = lookup_string(c, descriptor_index)?;
        let (sig, _) = parse_class_signature_info(&sig)?;
        if let Some(first) = sig.extends.first() {
            jtype = first.clone();
        }
    }

    let name = lookup_string(c, name_index)?;
    Ok((
        AstRecordEntry {
            range: RANGE,
            annotated,
            jtype,
            variadic: false,
            name: ntoident(name),
        },
        pos,
    ))
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
    pub extends: Vec<AstJType>,
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
    let end = sig.len();
    let mut extends = Vec::new();
    while pos < end
        && let Ok((nret, npos)) = parse_field_type(content, pos)
    {
        pos = npos;
        extends.push(nret);
    }

    Ok((ClassSignature { args, extends }, pos))
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
    c: &ConstPool,
    data: &[u8],
    code_attribute: &CodeAttribute,
    used_classes: &mut HashSet<NuVec>,
) -> Result<(), DecompilerError> {
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
                range: RANGE,
                annotated: Vec::new(),
                value: AstJTypeKind::Byte,
            },
            pos.saturating_add(1),
        )),
        b'C' => Ok((
            AstJType {
                range: RANGE,
                annotated: Vec::new(),
                value: AstJTypeKind::Char,
            },
            pos.saturating_add(1),
        )),
        b'D' => Ok((
            AstJType {
                range: RANGE,
                annotated: Vec::new(),
                value: AstJTypeKind::Double,
            },
            pos.saturating_add(1),
        )),
        b'F' => Ok((
            AstJType {
                range: RANGE,
                annotated: Vec::new(),
                value: AstJTypeKind::Float,
            },
            pos.saturating_add(1),
        )),
        b'I' => Ok((
            AstJType {
                range: RANGE,
                annotated: Vec::new(),
                value: AstJTypeKind::Int,
            },
            pos.saturating_add(1),
        )),
        b'J' => Ok((
            AstJType {
                range: RANGE,
                annotated: Vec::new(),
                value: AstJTypeKind::Long,
            },
            pos.saturating_add(1),
        )),
        b'S' => Ok((
            AstJType {
                range: RANGE,
                annotated: Vec::new(),
                value: AstJTypeKind::Short,
            },
            pos.saturating_add(1),
        )),
        b'Z' => Ok((
            AstJType {
                range: RANGE,
                annotated: Vec::new(),
                value: AstJTypeKind::Boolean,
            },
            pos.saturating_add(1),
        )),
        b'V' => Ok((
            AstJType {
                range: RANGE,
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
                    range: RANGE,
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
                    range: RANGE,
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
                    range: RANGE,
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
                range: RANGE,
                annotated: Vec::new(),
                value: AstJTypeKind::Generic(ntoident(class_name), args),
            },
        ));
    }
    Ok((
        pos,
        AstJType {
            range: RANGE,
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
    let con = c.pool.get((index.saturating_sub(1)) as usize);
    match con {
        Some(ConstEntry::Utf8(utf8)) => Ok(utf8.clone()),
        Some(
            ConstEntry::Module { name } | ConstEntry::Package { name } | ConstEntry::Class { name },
        ) => lookup_string_inner(c, *name, depth.saturating_add(1)),
        _ => Err(DecompilerError::ExpectedString),
    }
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
) -> Result<(Vec<AstClassVariable>, Vec<AstEnumerationVariant>, usize), DecompilerError> {
    let (size, pos) = get_u16(data, pos)?;
    let mut pos = pos;
    let mut out = Vec::with_capacity(size as usize);
    let mut enum_variants = Vec::new();

    for _ in 0..size {
        let (access_flags, volatile_transient, enu, ipos) = parse_field_access_flags(data, pos)?;
        let (name, ipos) = get_u16(data, ipos)?;
        let (descriptor, ipos) = get_u16(data, ipos)?;
        let (attributes, ipos) = parse_attributes(data, ipos)?;
        pos = ipos;

        let name = ntoident(lookup_string(c, name)?);
        if !enu {
            let mut expression = None;
            let mut jtype = AstJType {
                annotated: Vec::new(),
                range: RANGE,
                value: AstJTypeKind::Var,
            };
            let mut annotated = Vec::new();

            let mut constant_value_attribute = None;
            let mut deprecated = false;
            let mut is_annotated = false;

            for a in attributes {
                if a.name == 0 {
                    continue;
                }
                let attribute_name = lookup_string(c, a.name)?;
                if attribute_name == "ConstantValue" {
                    constant_value_attribute = Some(a);
                } else if attribute_name == "Deprecated" {
                    deprecated = true;
                } else if attribute_name == "Signature" {
                    let info = a.lookup(data)?;
                    let (sig, _) = get_u16(info, 0)?;
                    let sig = lookup_string(c, sig)?;
                    let (sig, _) = parse_class_signature_info(&sig)?;
                    if let Some(first) = sig.extends.first() {
                        jtype = first.clone();
                    }
                } else if attribute_name == "RuntimeVisibleAnnotations"
                    || attribute_name == "RuntimeInvisibleAnnotations"
                {
                    let info = a.lookup(data)?;
                    parse_runtime_visible_annotations(info, 0, c, &mut annotated)?;
                    is_annotated = true;
                }
            }

            if !is_annotated && deprecated {
                annotated.push(AstAnnotated {
                    range: RANGE,
                    name: ntoident(NuVec::Static(b"Deprecated")),
                    parameters: AstAnnotatedParameterKind::None,
                });
            }

            // Lookup type via descriptor, When no Signature attribute exists
            if matches!(jtype.value, AstJTypeKind::Var) {
                jtype = parse_field_type(lookup_string(c, descriptor)?.as_bytes(), 0)?.0;
            }

            if let Some(a) = constant_value_attribute {
                let out = field_constant_value_attribute(data, c, a, &jtype)?;
                if let Some(o) = out {
                    expression = Some(o);
                }
            }

            out.push(AstClassVariable {
                range: RANGE,
                availability: access_flags,
                annotated,
                name,
                jtype,
                expression,
                volatile_transient,
            });
        } else {
            enum_variants.push(AstEnumerationVariant {
                range: RANGE,
                annotated: Vec::new(),
                name,
                parameters: Vec::new(),
            });
        }
    }

    Ok((out, enum_variants, pos))
}

fn field_constant_value_attribute(
    data: &[u8],
    c: &ConstPool,
    a: Attribute,
    jtype: &AstJType,
) -> Result<Option<AstExpression>, DecompilerError> {
    let info = a.lookup(data)?;
    let (constant_value_index, _) = get_u16(info, 0)?;
    match c
        .pool
        .get((constant_value_index.saturating_sub(1)) as usize)
    {
        Some(ConstEntry::Float(c)) => {
            let value = NuVec::new(c.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Float(
                    AstDouble {
                        range: RANGE,
                        value,
                    },
                ))),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(Some(expr))
        }
        Some(ConstEntry::Long(c)) => {
            let value = NuVec::new(c.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Long(
                    AstInt {
                        range: RANGE,
                        value,
                    },
                ))),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(Some(expr))
        }
        Some(ConstEntry::Double(c)) => {
            let value = NuVec::new(c.to_string().as_bytes());
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Double(
                    AstDouble {
                        range: RANGE,
                        value,
                    },
                ))),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(Some(expr))
        }
        Some(ConstEntry::String { name }) => {
            let value = lookup_string(c, *name)?;
            let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                range: RANGE,
                ident: Some(AstExpressionIdentifier::Nuget(
                    AstValueNuget::StringLiteral {
                        range: RANGE,
                        value,
                    },
                )),
                values: None,
                operator: AstExpressionOperator::None,
            })];
            Ok(Some(expr))
        }
        Some(ConstEntry::Integer(value)) => match jtype.value {
            AstJTypeKind::Char => {
                let value = NuVec::new(value.to_string().as_bytes());
                let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                    range: RANGE,
                    ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::CharLiteral {
                        value,
                        range: RANGE,
                    })),
                    values: None,
                    operator: AstExpressionOperator::None,
                })];
                Ok(Some(expr))
            }
            AstJTypeKind::Long => {
                let value = NuVec::new(value.to_string().as_bytes());
                let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                    range: RANGE,
                    ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Long(
                        AstInt {
                            range: RANGE,
                            value,
                        },
                    ))),
                    values: None,
                    operator: AstExpressionOperator::None,
                })];
                Ok(Some(expr))
            }
            AstJTypeKind::Int | AstJTypeKind::Short | AstJTypeKind::Byte => {
                let value = NuVec::new(value.to_string().as_bytes());
                let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                    range: RANGE,
                    ident: Some(AstExpressionIdentifier::Nuget(AstValueNuget::Int(AstInt {
                        range: RANGE,
                        value,
                    }))),
                    values: None,
                    operator: AstExpressionOperator::None,
                })];
                Ok(Some(expr))
            }
            AstJTypeKind::Boolean => {
                let value = *value != 0;
                let expr = vec![AstExpressionKind::Base(AstBaseExpression {
                    range: RANGE,
                    ident: Some(AstExpressionIdentifier::Nuget(
                        AstValueNuget::BooleanLiteral(AstBoolean {
                            range: RANGE,
                            value,
                        }),
                    )),
                    values: None,
                    operator: AstExpressionOperator::None,
                })];
                Ok(Some(expr))
            }
            _ => Ok(None),
        },
        Some(_) | None => Ok(None),
    }
}

fn parse_field_access_flags(
    data: &[u8],
    pos: usize,
) -> Result<(AstAvailability, AstVolatileTransient, bool, usize), DecompilerError> {
    let (flags, pos) = get_u16(data, pos)?;
    let mut out = AstAvailability::empty();
    let mut vt = AstVolatileTransient::empty();
    let mut enu = false;
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
    if (flags & 0x4000) != 0 {
        enu = true;
    }
    if (flags & 0x0040) != 0 {
        vt |= AstVolatileTransient::Volatile;
    }
    if (flags & 0x0080) != 0 {
        vt |= AstVolatileTransient::Transient;
    }

    Ok((out, vt, enu, pos))
}
fn parse_methods(
    data: &[u8],
    pos: usize,
    c: &ConstPool,
    is_interface: bool,
    used_classes: &mut HashSet<NuVec>,
    class_name: &NuVec,
) -> Result<(Vec<AstClassMethod>, usize), DecompilerError> {
    let (size, pos) = get_u16(data, pos)?;
    let mut pos = pos;
    let mut out = Vec::with_capacity(size as usize);

    for _ in 0..size {
        let (access_flags, ipos) = parse_method_access_flags(data, pos, is_interface)?;
        let (name, ipos) = get_u16(data, ipos)?;
        let (descriptor, ipos) = get_u16(data, ipos)?;
        let (attributes, ipos) = parse_attributes(data, ipos)?;

        let (m, code) = parse_method(
            c,
            data,
            access_flags,
            name,
            descriptor,
            attributes,
            class_name,
        )?;
        if let Some(code_attribute) = code {
            parse_used_classes(c, data, &code_attribute, used_classes)?;
        }
        jtype_class_names(m.header.jtype.clone(), used_classes);

        out.push(m);
        pos = ipos;
    }

    Ok((out, pos))
}

fn parse_method_access_flags(
    data: &[u8],
    pos: usize,
    is_interface: bool,
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
    if !is_interface && (flags & 0x0400) != 0 {
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
                public void Everything() {}
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
                public void Everything() {}
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
            public interface Constants {
                public static final java.lang.String CONSTANT_A = "A";
                public static final java.lang.String CONSTANT_B = "B";
                public static final java.lang.String CONSTANT_C = "C";
                public void display();
                public java.net.Socket createSocket(java.lang.String hostname, int port) throws java.io.IOException;
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
                java.util.List<java.lang.String> one_list;
                java.util.Map<java.lang.Integer, java.lang.String> one_map;
                public void Types() {}
                public static void main(java.lang.String[] a) {}
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
            public class Super extends IOException {
                public void Super() {}
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
                public void Thrower() {}
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
            public interface SuperInterface<E> {
                default public java.util.stream.Stream<E> stream() {}
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
                private java.util.HashSet<java.lang.String> a;
                public void LocalVariableTable() {}
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
                A,
                B,
                C;
                private final java.lang.String tag;
                private static final ch.emilycares.Variants[] $VALUES;
                public static ch.emilycares.Variants[] values() {}
                public static ch.emilycares.Variants valueOf(java.lang.String name) {}
                private void Variants(java.lang.String $enum$name) {}
                public java.lang.String getTag() {}
                private static ch.emilycares.Variants[] $values() {}
                static void clinit() {}
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
                extends: [
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
                                value: "java.lang.Object",
                            },
                        ),
                    },
                    AstJType {
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
                                value: "java.util.SequencedCollection",
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
                                            value: "E",
                                        },
                                    ),
                                },
                            ],
                        ),
                    },
                ],
            }
        "#]];
        expected.assert_debug_eq(&result.0);
    }
}
