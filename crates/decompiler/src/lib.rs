#![deny(clippy::nursery)]
#![deny(clippy::perf)]
#![deny(clippy::arithmetic_side_effects)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::collections::HashSet;
use std::fmt::Write;

use ast::types::{
    AstAnnotated, AstAnnotatedParameter, AstAnnotatedParameterKind, AstAvailability,
    AstBaseExpression, AstBlock, AstBlockEntry, AstBoolean, AstClass, AstClassBlock,
    AstClassMethod, AstClassVariable, AstDouble, AstEnumeration, AstEnumerationVariant,
    AstExpression, AstExpressionIdentifier, AstExpressionKind, AstExpressionOperator,
    AstExpressionOrAnnotated, AstFile, AstIdentifier, AstImport, AstImportUnit, AstInt,
    AstInterface, AstInterfaceConstant, AstInterfaceMethod, AstInterfaceMethodDefault, AstJType,
    AstJTypeExpression, AstJTypeKind, AstLineComment, AstMethodHeader, AstMethodParameter,
    AstMethodParameterFlags, AstMethodParameters, AstPackage, AstPoint, AstRange, AstRecord,
    AstRecordEntries, AstRecordEntry, AstSuperClass, AstThing, AstThingAttributes,
    AstThrowsDeclaration, AstTopLevel, AstTypeParameter, AstTypeParameters, AstValueNuget,
    AstValuesWithAnnotated, AstVolatileTransient,
};
use bitflags::bitflags;
use my_string::{NuVec, NuVecBuilder};

const U8_LEN: usize = 1;
const U16_LEN: usize = 2;
#[derive(Debug, PartialEq, Eq)]
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
    UnknownInstructionWide,
    UnknownInstruction,
    ToMeanyInstructions,
    DebugInstruction,
    ConstEntryNotFound,
}

pub fn decompile_class(
    data: &[u8],
    class_path: Option<&NuVec>,
) -> Result<AstFile, DecompilerError> {
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
    if let Some(class_path) = class_path {
        let package = class_path
            .trim_end_matches(class_name.as_bytes())
            .trim_end_matches_byte(b'.');
        file.top.push(AstTopLevel::Package(AstPackage {
            range: RANGE,
            annotated: Vec::new(),
            name: ntoident(package),
        }));
    }
    let mut imports: Vec<&NuVec> = class_path.map_or_else(
        || used_classes.iter().collect(),
        |class_path| used_classes.iter().filter(|i| *i != class_path).collect(),
    );
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
    const_pool: &ConstPool,
    data: &[u8],
    availability: AstAvailability,
    name: u16,
    descriptor: u16,
    attributes: Vec<Attribute>,
    class_name: &NuVec,
) -> Result<(AstClassMethod, Option<CodeAttribute>), DecompilerError> {
    let lname = lookup_string(const_pool, name)?;
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
        let attribute_name = lookup_string(const_pool, attribute.name)?;
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
            let mut entries = Vec::new();
            for c in &ca.instructions {
                let content = debug_instruction(c, const_pool)?;
                entries.push(AstBlockEntry::LineComment(AstLineComment {
                    range: RANGE,
                    content,
                }));
            }
            code_attribute = Some(ca);
            out.block = Some(AstBlock {
                range: RANGE,
                entries,
            });
        } else if attribute_name == "RuntimeVisibleAnnotations"
            || attribute_name == "RuntimeInvisibleAnnotations"
        {
            let info = attribute.lookup(data)?;
            parse_runtime_visible_annotations(info, 0, const_pool, &mut out.header.annotated)?;
            is_annotated = true;
        }
    }
    let no_parameter_names_and_signature =
        method_parameter_index.is_none() && signature_index.is_none();
    if no_parameter_names_and_signature {
        let desc = lookup_string(const_pool, descriptor)?;
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
                let name = lookup_string(const_pool, p.name_index)
                    .ok()
                    .filter(|i| !i.is_empty());
                parameter_names.push(name);
            }
        } else {
            let (_, md) = parse_method_descriptor(&lookup_string(const_pool, descriptor)?)?;
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
                } else if let Some(name) = lookup_string(const_pool, p.name_index)
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
        let sig = lookup_string(const_pool, sig)?;
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
                let class_name = lookup_string(const_pool, exception)?;
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

fn debug_instruction(c: &Instruction, const_pool: &ConstPool) -> Result<NuVec, DecompilerError> {
    let mut b = NuVecBuilder::new();
    match c {
        Instruction::Getstatic(index) => {
            b.pusha(b"Getstatic(");
            debug_ref(const_pool, &mut b, index)?;
            b.push(b')');
        }
        Instruction::Invokevirtual(index) => {
            b.pusha(b"Invokevirtual(");
            debug_ref(const_pool, &mut b, index)?;
            b.push(b')');
        }
        Instruction::Invokestatic(index) => {
            b.pusha(b"Invokestatic(");
            debug_ref(const_pool, &mut b, index)?;
            b.push(b')');
        }
        Instruction::Invokespecial(index) => {
            b.pusha(b"Invokespecial(");
            debug_ref(const_pool, &mut b, index)?;
            b.push(b')');
        }
        Instruction::Invokeinterface { index, count } => {
            b.pusha(b"Invokeinterface(");
            debug_ref(const_pool, &mut b, index)?;
            write!(b, ", count: {count}").map_err(|_| DecompilerError::DebugInstruction)?;
            b.push(b')');
        }
        Instruction::Putfield(index) => {
            b.pusha(b"PutField(");
            debug_ref(const_pool, &mut b, index)?;
            b.push(b')');
        }
        Instruction::Ldc(index) => {
            b.pusha(b"Ldc(");
            let index = &(*index as u16);
            ldc_helper(const_pool, &mut b, index)?;
            b.push(b')');
        }
        Instruction::LdcW(index) => {
            b.pusha(b"LdcW(");
            ldc_helper(const_pool, &mut b, index)?;
            b.push(b')');
        }
        Instruction::New(index) => {
            let lookup = const_pool.get(index)?;
            b.pusha(b"New(");
            if let ConstEntry::Class { name } = lookup {
                let name = lookup_string(const_pool, *name)?;
                write!(b, "{:?}", name).map_err(|_| DecompilerError::DebugInstruction)?;
            } else {
                write!(b, "{:?}", lookup).map_err(|_| DecompilerError::DebugInstruction)?;
            }
            b.push(b')');
        }
        _ => {
            write!(b, "{c:?}").map_err(|_| DecompilerError::DebugInstruction)?;
        }
    }
    Ok(b.finish())
}

fn ldc_helper(
    const_pool: &ConstPool,
    b: &mut NuVecBuilder,
    index: &u16,
) -> Result<(), DecompilerError> {
    let lookup = const_pool.get(index)?;
    match lookup {
        ConstEntry::String { name } => {
            let st = lookup_string(const_pool, *name)?;
            write!(b, "String {:?}", st).map_err(|_| DecompilerError::DebugInstruction)?;
        }
        ConstEntry::Class { name } => {
            let st = lookup_string(const_pool, *name)?;
            write!(b, "Class {:?}", st).map_err(|_| DecompilerError::DebugInstruction)?;
        }
        _ => {
            write!(b, "Entry {:?}", lookup).map_err(|_| DecompilerError::DebugInstruction)?;
        }
    }
    Ok(())
}

fn debug_ref(
    const_pool: &ConstPool,
    b: &mut NuVecBuilder,
    index: &u16,
) -> Result<(), DecompilerError> {
    let lookup = const_pool.get(index)?;
    match lookup {
        ConstEntry::FieldRef {
            class,
            name_and_type,
        } => {
            b.pusha(b"FieldRef");
            debug_field_or_method_ref(const_pool, b, class, name_and_type)?;
        }
        ConstEntry::MethodRef {
            class,
            name_and_type,
        } => {
            b.pusha(b"MethodRef");
            debug_field_or_method_ref(const_pool, b, class, name_and_type)?;
        }
        ConstEntry::InterfaceMethodRef {
            class,
            name_and_type,
        } => {
            b.pusha(b"InterfaceMethodRef");
            debug_field_or_method_ref(const_pool, b, class, name_and_type)?;
        }
        _ => {
            write!(b, "{lookup:?}").map_err(|_| DecompilerError::DebugInstruction)?;
        }
    }
    Ok(())
}

fn debug_field_or_method_ref(
    const_pool: &ConstPool,
    b: &mut NuVecBuilder,
    class: &u16,
    name_and_type: &u16,
) -> Result<(), DecompilerError> {
    let class = lookup_string(const_pool, *class)?;
    let ntl = const_pool.get(name_and_type)?;
    if let ConstEntry::NameAndType { name, descriptor } = ntl {
        let name = lookup_string(const_pool, *name)?;
        let descriptor = lookup_string(const_pool, *descriptor)?;
        write!(
            b,
            "(class: {class}, name_and_type: (name: \"{name}\", descriptor: \"{descriptor}\"))"
        )
        .map_err(|_| DecompilerError::DebugInstruction)?;
    } else {
        return Err(DecompilerError::ExpectedOther);
    }
    Ok(())
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

#[derive(Debug)]
struct CodeAttribute {
    start: usize,
    end: usize,
    pub attributes: Vec<Attribute>,
    pub instructions: Vec<Instruction>,
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
    if code_length > 65536 {
        return Err(DecompilerError::ToMeanyInstructions);
    }
    let code_length = code_length as usize;

    let idata_end = pos.saturating_add(code_length);
    let idata = &data[pos..idata_end];
    let mut ipos = 0;
    let mut instructions = Vec::with_capacity(6);
    loop {
        let (instruction, npos) = parse_instruction(idata, ipos)?;
        instructions.push(instruction);
        ipos = npos;
        if ipos >= code_length {
            break;
        }
    }
    let pos = pos.saturating_add(code_length);

    let (exception_table_length, pos) = get_u16(data, pos)?;
    let exception_table = (exception_table_length as usize).saturating_mul(8);
    let pos = pos.saturating_add(exception_table);

    let (attributes, pos) = parse_attributes(data, pos)?;
    // let mut line_number_table = Vec::new();
    // for attribute in &attributes {
    //     let attribute_name = lookup_string(c, attribute.name)?;
    //     if attribute_name == "LineNumberTable" {
    //         let info = attribute.lookup(data)?;
    //         line_number_table = parse_line_number_table(info)?;
    //     }
    // }

    Ok((
        CodeAttribute {
            start,
            end,
            attributes,
            instructions,
        },
        pos,
    ))
}

#[derive(Debug)]
#[allow(unused)]
struct LineNumber {
    pub start_pc: u16,
    pub line_number: u16,
}

#[allow(unused)]
fn parse_line_number_table(info: &[u8]) -> Result<Vec<LineNumber>, DecompilerError> {
    let pos = 0;
    let (table_len, pos) = get_u16(info, pos)?;
    let mut out = Vec::with_capacity(table_len as usize);
    let mut pos = pos;
    for _ in 0..table_len {
        let (start_pc, ipos) = get_u16(info, pos)?;
        let (line_number, ipos) = get_u16(info, ipos)?;
        pos = ipos;
        out.push(LineNumber {
            start_pc,
            line_number,
        });
    }
    Ok(out)
}

#[derive(Debug, Eq, PartialEq)]
pub enum Instruction {
    Aaload,
    Aastore,
    Aconstnull,
    Aload(u8),
    AloadWide(u16),
    Aload0,
    Aload1,
    Aload2,
    Aload3,
    Anewarray(u16),
    Areturn,
    Arraylength,
    Astore(u8),
    AstoreWide(u16),
    Astore0,
    Astore1,
    Astore2,
    Astore3,
    Athrow,
    Baload,
    Bastore,
    Bipush(i8),
    Caload,
    Castore,
    Checkcast(u16),
    D2f,
    D2i,
    D2l,
    Dadd,
    Daload,
    Dastore,
    Dcmpg,
    Dcmpl,
    Dconst0,
    Dconst1,
    Ddiv,
    Dload(u8),
    DloadWide(u16),
    Dload0,
    Dload1,
    Dload2,
    Dload3,
    Dmul,
    Dneg,
    Drem,
    Dreturn,
    Dstore(u8),
    DstoreWide(u16),
    Dstore0,
    Dstore1,
    Dstore2,
    Dstore3,
    Dsub,
    Dup,
    Dupx1,
    Dupx2,
    Dup2,
    Dup2x1,
    Dup2x2,
    F2d,
    F2i,
    F2l,
    Fadd,
    Faload,
    Fastore,
    Fcmpg,
    Fcmpl,
    Fconst0,
    Fconst1,
    Fconst2,
    Fdiv,
    Fload(u8),
    FloadWide(u16),
    Fload0,
    Fload1,
    Fload2,
    Fload3,
    Fmul,
    Fneg,
    Frem,
    Freturn,
    Fstore(u8),
    FstoreWide(u16),
    Fstore0,
    Fstore1,
    Fstore2,
    Fstore3,
    Fsub,
    Getfield(u16),
    Getstatic(u16),
    Goto(i16),
    GotoW(i32),
    I2b,
    I2c,
    I2d,
    I2f,
    I2l,
    I2s,
    Iadd,
    Iaload,
    Iand,
    Iastore,
    Iconstm1,
    Iconst0,
    Iconst1,
    Iconst2,
    Iconst3,
    Iconst4,
    Iconst5,
    Idiv,
    IfAcmpeq(i16),
    IfAcmpne(i16),
    IfIcmpeq(i16),
    IfIcmpne(i16),
    IfIcmplt(i16),
    IfIcmpge(i16),
    IfIcmpgt(i16),
    IfIcmple(i16),
    Ifeq(i16),
    Ifne(i16),
    Iflt(i16),
    Ifge(i16),
    Ifgt(i16),
    Ifle(i16),
    Ifnonnull(i16),
    Ifnull(i16),
    Iinc {
        index: u8,
        value: i8,
    },
    IincWide {
        index: u16,
        value: i16,
    },
    Iload(u8),
    IloadWide(u16),
    Iload0,
    Iload1,
    Iload2,
    Iload3,
    Imul,
    Ineg,
    Instanceof(u16),
    Invokedynamic(u16),
    Invokeinterface {
        index: u16,
        count: u8,
    },
    Invokespecial(u16),
    Invokestatic(u16),
    Invokevirtual(u16),
    Ior,
    Irem,
    Ireturn,
    Ishl,
    Ishr,
    Istore(u8),
    IstoreWide(u16),
    Istore0,
    Istore1,
    Istore2,
    Istore3,
    Isub,
    Iushr,
    Ixor,
    Jsr(i16),
    JsrW(i32),
    L2d,
    L2f,
    L2i,
    Ladd,
    Laload,
    Land,
    Lastore,
    Lcmp,
    Lconst0,
    Lconst1,
    Ldc(u8),
    LdcW(u16),
    Ldc2W(u16),
    Ldiv,
    Lload(u8),
    LloadWide(u16),
    Lload0,
    Lload1,
    Lload2,
    Lload3,
    Lmul,
    Lneg,
    Lookupswitch {
        default: i32,
        pairs: Vec<(i32, i32)>,
    },
    Lor,
    Lrem,
    Lreturn,
    Lshl,
    Lshr,
    Lstore(u8),
    LstoreWide(u16),
    Lstore0,
    Lstore1,
    Lstore2,
    Lstore3,
    Lsub,
    Lushr,
    Lxor,
    Monitorenter,
    Monitorexit,
    Multianewarray {
        index: u16,
        dimensions: u8,
    },
    New(u16),
    Newarray(u8),
    Nop,
    Pop,
    Pop2,
    Putfield(u16),
    Putstatic(u16),
    Ret(u8),
    RetWide(u16),
    Return,
    Saload,
    Sastore,
    Sipush(i16),
    Swap,
    Tableswitch {
        default: i32,
        low: i32,
        high: i32,
        offsets: Vec<i32>,
    },
}

fn parse_instruction(data: &[u8], pos: usize) -> Result<(Instruction, usize), DecompilerError> {
    let (kind, pos) = get_u8(data, pos)?;
    match kind {
        0x32 => Ok((Instruction::Aaload, pos)),
        0x53 => Ok((Instruction::Aastore, pos)),
        0x01 => Ok((Instruction::Aconstnull, pos)),
        0x19 => get_u8(data, pos).map(|(i, pos)| (Instruction::Aload(i), pos)),
        0x2a => Ok((Instruction::Aload0, pos)),
        0x2b => Ok((Instruction::Aload1, pos)),
        0x2c => Ok((Instruction::Aload2, pos)),
        0x2d => Ok((Instruction::Aload3, pos)),
        0xbd => get_u16(data, pos).map(|(i, pos)| (Instruction::Anewarray(i), pos)),
        0xb0 => Ok((Instruction::Areturn, pos)),
        0xbe => Ok((Instruction::Arraylength, pos)),
        0x3a => get_u8(data, pos).map(|(i, pos)| (Instruction::Astore(i), pos)),
        0x4b => Ok((Instruction::Astore0, pos)),
        0x4c => Ok((Instruction::Astore1, pos)),
        0x4d => Ok((Instruction::Astore2, pos)),
        0x4e => Ok((Instruction::Astore3, pos)),
        0xbf => Ok((Instruction::Athrow, pos)),
        0x33 => Ok((Instruction::Baload, pos)),
        0x54 => Ok((Instruction::Bastore, pos)),
        0x10 => get_i8(data, pos).map(|(i, pos)| (Instruction::Bipush(i), pos)),
        0x34 => Ok((Instruction::Caload, pos)),
        0x55 => Ok((Instruction::Castore, pos)),
        0xc0 => get_u16(data, pos).map(|(i, pos)| (Instruction::Checkcast(i), pos)),
        0x90 => Ok((Instruction::D2f, pos)),
        0x8e => Ok((Instruction::D2i, pos)),
        0x8f => Ok((Instruction::D2l, pos)),
        0x63 => Ok((Instruction::Dadd, pos)),
        0x31 => Ok((Instruction::Daload, pos)),
        0x52 => Ok((Instruction::Dastore, pos)),
        0x98 => Ok((Instruction::Dcmpg, pos)),
        0x97 => Ok((Instruction::Dcmpl, pos)),
        0x0e => Ok((Instruction::Dconst0, pos)),
        0x0f => Ok((Instruction::Dconst1, pos)),
        0x6f => Ok((Instruction::Ddiv, pos)),
        0x18 => get_u8(data, pos).map(|(i, pos)| (Instruction::Dload(i), pos)),
        0x26 => Ok((Instruction::Dload0, pos)),
        0x27 => Ok((Instruction::Dload1, pos)),
        0x28 => Ok((Instruction::Dload2, pos)),
        0x29 => Ok((Instruction::Dload3, pos)),
        0x6b => Ok((Instruction::Dmul, pos)),
        0x77 => Ok((Instruction::Dneg, pos)),
        0x73 => Ok((Instruction::Drem, pos)),
        0xaf => Ok((Instruction::Dreturn, pos)),
        0x39 => get_u8(data, pos).map(|(i, pos)| (Instruction::Dstore(i), pos)),
        0x47 => Ok((Instruction::Dstore0, pos)),
        0x48 => Ok((Instruction::Dstore1, pos)),
        0x49 => Ok((Instruction::Dstore2, pos)),
        0x4a => Ok((Instruction::Dstore3, pos)),
        0x67 => Ok((Instruction::Dsub, pos)),
        0x59 => Ok((Instruction::Dup, pos)),
        0x5a => Ok((Instruction::Dupx1, pos)),
        0x5b => Ok((Instruction::Dupx2, pos)),
        0x5c => Ok((Instruction::Dup2, pos)),
        0x5d => Ok((Instruction::Dup2x1, pos)),
        0x5e => Ok((Instruction::Dup2x2, pos)),
        0x8d => Ok((Instruction::F2d, pos)),
        0x8b => Ok((Instruction::F2i, pos)),
        0x8c => Ok((Instruction::F2l, pos)),
        0x62 => Ok((Instruction::Fadd, pos)),
        0x30 => Ok((Instruction::Faload, pos)),
        0x51 => Ok((Instruction::Fastore, pos)),
        0x96 => Ok((Instruction::Fcmpg, pos)),
        0x95 => Ok((Instruction::Fcmpl, pos)),
        0x0b => Ok((Instruction::Fconst0, pos)),
        0x0c => Ok((Instruction::Fconst1, pos)),
        0x0d => Ok((Instruction::Fconst2, pos)),
        0x6e => Ok((Instruction::Fdiv, pos)),
        0x17 => get_u8(data, pos).map(|(i, pos)| (Instruction::Fload(i), pos)),
        0x22 => Ok((Instruction::Fload0, pos)),
        0x23 => Ok((Instruction::Fload1, pos)),
        0x24 => Ok((Instruction::Fload2, pos)),
        0x25 => Ok((Instruction::Fload3, pos)),
        0x6a => Ok((Instruction::Fmul, pos)),
        0x76 => Ok((Instruction::Fneg, pos)),
        0x72 => Ok((Instruction::Frem, pos)),
        0xae => Ok((Instruction::Freturn, pos)),
        0x38 => get_u8(data, pos).map(|(i, pos)| (Instruction::Fstore(i), pos)),
        0x43 => Ok((Instruction::Fstore0, pos)),
        0x44 => Ok((Instruction::Fstore1, pos)),
        0x45 => Ok((Instruction::Fstore2, pos)),
        0x46 => Ok((Instruction::Fstore3, pos)),
        0x66 => Ok((Instruction::Fsub, pos)),
        0xb4 => get_u16(data, pos).map(|(i, pos)| (Instruction::Getfield(i), pos)),
        0xb2 => get_u16(data, pos).map(|(i, pos)| (Instruction::Getstatic(i), pos)),
        0xa7 => get_i16(data, pos).map(|(i, pos)| (Instruction::Goto(i), pos)),
        0xc8 => get_i32(data, pos).map(|(i, pos)| (Instruction::GotoW(i), pos)),
        0x91 => Ok((Instruction::I2b, pos)),
        0x92 => Ok((Instruction::I2c, pos)),
        0x87 => Ok((Instruction::I2d, pos)),
        0x86 => Ok((Instruction::I2f, pos)),
        0x85 => Ok((Instruction::I2l, pos)),
        0x93 => Ok((Instruction::I2s, pos)),
        0x60 => Ok((Instruction::Iadd, pos)),
        0x2e => Ok((Instruction::Iaload, pos)),
        0x7e => Ok((Instruction::Iand, pos)),
        0x4f => Ok((Instruction::Iastore, pos)),
        0x02 => Ok((Instruction::Iconstm1, pos)),
        0x03 => Ok((Instruction::Iconst0, pos)),
        0x04 => Ok((Instruction::Iconst1, pos)),
        0x05 => Ok((Instruction::Iconst2, pos)),
        0x06 => Ok((Instruction::Iconst3, pos)),
        0x07 => Ok((Instruction::Iconst4, pos)),
        0x08 => Ok((Instruction::Iconst5, pos)),
        0x6c => Ok((Instruction::Idiv, pos)),
        0xa5 => get_i16(data, pos).map(|(i, pos)| (Instruction::IfAcmpeq(i), pos)),
        0xa6 => get_i16(data, pos).map(|(i, pos)| (Instruction::IfAcmpne(i), pos)),
        0x9f => get_i16(data, pos).map(|(i, pos)| (Instruction::IfIcmpeq(i), pos)),
        0xa0 => get_i16(data, pos).map(|(i, pos)| (Instruction::IfIcmpne(i), pos)),
        0xa1 => get_i16(data, pos).map(|(i, pos)| (Instruction::IfIcmplt(i), pos)),
        0xa2 => get_i16(data, pos).map(|(i, pos)| (Instruction::IfIcmpge(i), pos)),
        0xa3 => get_i16(data, pos).map(|(i, pos)| (Instruction::IfIcmpgt(i), pos)),
        0xa4 => get_i16(data, pos).map(|(i, pos)| (Instruction::IfIcmple(i), pos)),
        0x99 => get_i16(data, pos).map(|(i, pos)| (Instruction::Ifeq(i), pos)),
        0x9a => get_i16(data, pos).map(|(i, pos)| (Instruction::Ifne(i), pos)),
        0x9b => get_i16(data, pos).map(|(i, pos)| (Instruction::Iflt(i), pos)),
        0x9c => get_i16(data, pos).map(|(i, pos)| (Instruction::Ifge(i), pos)),
        0x9d => get_i16(data, pos).map(|(i, pos)| (Instruction::Ifgt(i), pos)),
        0x9e => get_i16(data, pos).map(|(i, pos)| (Instruction::Ifle(i), pos)),
        0xc7 => get_i16(data, pos).map(|(i, pos)| (Instruction::Ifnonnull(i), pos)),
        0xc6 => get_i16(data, pos).map(|(i, pos)| (Instruction::Ifnull(i), pos)),
        0x84 => {
            let (index, pos) = get_u8(data, pos)?;
            let (value, pos) = get_i8(data, pos)?;
            Ok((Instruction::Iinc { index, value }, pos))
        }
        0x15 => get_u8(data, pos).map(|(i, pos)| (Instruction::Iload(i), pos)),
        0x1a => Ok((Instruction::Iload0, pos)),
        0x1b => Ok((Instruction::Iload1, pos)),
        0x1c => Ok((Instruction::Iload2, pos)),
        0x1d => Ok((Instruction::Iload3, pos)),
        0x68 => Ok((Instruction::Imul, pos)),
        0x74 => Ok((Instruction::Ineg, pos)),
        0xc1 => get_u16(data, pos).map(|(i, pos)| (Instruction::Instanceof(i), pos)),
        0xba => {
            let (index, pos) = get_u16(data, pos)?;
            let pos = expect_data(data, pos, &[0, 0])?;

            Ok((Instruction::Invokedynamic(index), pos))
        }
        0xb9 => {
            let (index, pos) = get_u16(data, pos)?;
            let (count, pos) = get_u8(data, pos)?;
            let pos = expect_data(data, pos, &[0])?;

            Ok((Instruction::Invokeinterface { index, count }, pos))
        }
        0xb7 => get_u16(data, pos).map(|(i, pos)| (Instruction::Invokespecial(i), pos)),
        0xb8 => get_u16(data, pos).map(|(i, pos)| (Instruction::Invokestatic(i), pos)),
        0xb6 => get_u16(data, pos).map(|(i, pos)| (Instruction::Invokevirtual(i), pos)),
        0x80 => Ok((Instruction::Ior, pos)),
        0x70 => Ok((Instruction::Irem, pos)),
        0xac => Ok((Instruction::Ireturn, pos)),
        0x78 => Ok((Instruction::Ishl, pos)),
        0x7a => Ok((Instruction::Ishr, pos)),
        0x36 => get_u8(data, pos).map(|(i, pos)| (Instruction::Istore(i), pos)),
        0x3b => Ok((Instruction::Istore0, pos)),
        0x3c => Ok((Instruction::Istore1, pos)),
        0x3d => Ok((Instruction::Istore2, pos)),
        0x3e => Ok((Instruction::Istore3, pos)),
        0x64 => Ok((Instruction::Isub, pos)),
        0x7c => Ok((Instruction::Iushr, pos)),
        0x82 => Ok((Instruction::Ixor, pos)),
        0xa8 => get_i16(data, pos).map(|(i, pos)| (Instruction::Jsr(i), pos)),
        0xc9 => get_i32(data, pos).map(|(i, pos)| (Instruction::JsrW(i), pos)),
        0x8a => Ok((Instruction::L2d, pos)),
        0x89 => Ok((Instruction::L2f, pos)),
        0x88 => Ok((Instruction::L2i, pos)),
        0x61 => Ok((Instruction::Ladd, pos)),
        0x2f => Ok((Instruction::Laload, pos)),
        0x7f => Ok((Instruction::Land, pos)),
        0x50 => Ok((Instruction::Lastore, pos)),
        0x94 => Ok((Instruction::Lcmp, pos)),
        0x09 => Ok((Instruction::Lconst0, pos)),
        0x0a => Ok((Instruction::Lconst1, pos)),
        0x12 => get_u8(data, pos).map(|(i, pos)| (Instruction::Ldc(i), pos)),
        0x13 => get_u16(data, pos).map(|(i, pos)| (Instruction::LdcW(i), pos)),
        0x14 => get_u16(data, pos).map(|(i, pos)| (Instruction::Ldc2W(i), pos)),
        0x6d => Ok((Instruction::Ldiv, pos)),
        0x16 => get_u8(data, pos).map(|(i, pos)| (Instruction::Lload(i), pos)),
        0x1e => Ok((Instruction::Lload0, pos)),
        0x1f => Ok((Instruction::Lload1, pos)),
        0x20 => Ok((Instruction::Lload2, pos)),
        0x21 => Ok((Instruction::Lload3, pos)),
        0x69 => Ok((Instruction::Lmul, pos)),
        0x75 => Ok((Instruction::Lneg, pos)),
        0xab => {
            let aligend = align_4byte(pos);
            lookupswitch_parser(data, pos.saturating_add(aligend))
        }
        0x81 => Ok((Instruction::Lor, pos)),
        0x71 => Ok((Instruction::Lrem, pos)),
        0xad => Ok((Instruction::Lreturn, pos)),
        0x79 => Ok((Instruction::Lshl, pos)),
        0x7b => Ok((Instruction::Lshr, pos)),
        0x37 => get_u8(data, pos).map(|(i, pos)| (Instruction::Lstore(i), pos)),
        0x3f => Ok((Instruction::Lstore0, pos)),
        0x40 => Ok((Instruction::Lstore1, pos)),
        0x41 => Ok((Instruction::Lstore2, pos)),
        0x42 => Ok((Instruction::Lstore3, pos)),
        0x65 => Ok((Instruction::Lsub, pos)),
        0x7d => Ok((Instruction::Lushr, pos)),
        0x83 => Ok((Instruction::Lxor, pos)),
        0xc2 => Ok((Instruction::Monitorenter, pos)),
        0xc3 => Ok((Instruction::Monitorexit, pos)),
        0xc5 => {
            let (index, pos) = get_u16(data, pos)?;
            let (dimensions, pos) = get_u8(data, pos)?;
            Ok((Instruction::Multianewarray { index, dimensions }, pos))
        }
        0xbb => get_u16(data, pos).map(|(i, pos)| (Instruction::New(i), pos)),
        0xbc => get_u8(data, pos).map(|(i, pos)| (Instruction::Newarray(i), pos)),
        0x00 => Ok((Instruction::Nop, pos)),
        0x57 => Ok((Instruction::Pop, pos)),
        0x58 => Ok((Instruction::Pop2, pos)),
        0xb5 => get_u16(data, pos).map(|(i, pos)| (Instruction::Putfield(i), pos)),
        0xb3 => get_u16(data, pos).map(|(i, pos)| (Instruction::Putstatic(i), pos)),
        0xa9 => get_u8(data, pos).map(|(i, pos)| (Instruction::Ret(i), pos)),
        0xb1 => Ok((Instruction::Return, pos)),
        0x35 => Ok((Instruction::Saload, pos)),
        0x56 => Ok((Instruction::Sastore, pos)),
        0x11 => get_i16(data, pos).map(|(i, pos)| (Instruction::Sipush(i), pos)),
        0x5f => Ok((Instruction::Swap, pos)),
        0xaa => {
            let aligend = align_4byte(pos);
            tableswitch_parser(data, pos.saturating_add(aligend))
        }
        0xc4 => {
            let (b, pos) = get_u8(data, pos)?;
            match b {
                0x19 => get_u16(data, pos).map(|(i, pos)| (Instruction::AloadWide(i), pos)),
                0x3a => get_u16(data, pos).map(|(i, pos)| (Instruction::AstoreWide(i), pos)),
                0x18 => get_u16(data, pos).map(|(i, pos)| (Instruction::DloadWide(i), pos)),
                0x39 => get_u16(data, pos).map(|(i, pos)| (Instruction::DstoreWide(i), pos)),
                0x17 => get_u16(data, pos).map(|(i, pos)| (Instruction::FloadWide(i), pos)),
                0x38 => get_u16(data, pos).map(|(i, pos)| (Instruction::FstoreWide(i), pos)),
                0x15 => get_u16(data, pos).map(|(i, pos)| (Instruction::IloadWide(i), pos)),
                0x36 => get_u16(data, pos).map(|(i, pos)| (Instruction::IstoreWide(i), pos)),
                0x16 => get_u16(data, pos).map(|(i, pos)| (Instruction::LloadWide(i), pos)),
                0x37 => get_u16(data, pos).map(|(i, pos)| (Instruction::LstoreWide(i), pos)),
                0xa9 => get_u16(data, pos).map(|(i, pos)| (Instruction::RetWide(i), pos)),
                0x84 => {
                    let (index, pos) = get_u16(data, pos)?;
                    let (value, pos) = get_i16(data, pos)?;
                    Ok((Instruction::IincWide { index, value }, pos))
                }
                _ => Err(DecompilerError::UnknownInstructionWide),
            }
        }
        _ => {
            eprintln!("kind: {kind:#04x}");
            Err(DecompilerError::UnknownInstruction)
        }
    }
}

fn tableswitch_parser(data: &[u8], pos: usize) -> Result<(Instruction, usize), DecompilerError> {
    let (default, pos) = get_i32(data, pos)?;
    let (low, pos) = get_i32(data, pos)?;
    let (high, pos) = get_i32(data, pos)?;
    let noffsets = ((high.saturating_sub(low)).saturating_add(1)) as usize;
    let mut offsets = Vec::new();
    let mut pos = pos;
    for _ in 0..noffsets {
        let (c, ipos) = get_i32(data, pos)?;
        offsets.push(c);
        pos = ipos;
    }
    Ok((
        Instruction::Tableswitch {
            default,
            low,
            high,
            offsets,
        },
        pos,
    ))
}
fn lookupswitch_parser(data: &[u8], pos: usize) -> Result<(Instruction, usize), DecompilerError> {
    let (default, pos) = get_i32(data, pos)?;
    let (np, pos) = get_i32(data, pos)?;
    let mut pairs = Vec::new();
    let mut pos = pos;
    for _ in 0..np {
        let (a, ipos) = get_i32(data, pos)?;
        let (b, ipos) = get_i32(data, ipos)?;
        pairs.push((a, b));
        pos = ipos;
    }
    Ok((Instruction::Lookupswitch { default, pairs }, pos))
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

impl ConstPool {
    pub fn get(&self, index: &u16) -> Result<&ConstEntry, DecompilerError> {
        self.pool
            .get(index.saturating_sub(1) as usize)
            .ok_or(DecompilerError::ConstEntryNotFound)
    }
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
    MethodRef { class: u16, name_and_type: u16 },
    NameAndType { name: u16, descriptor: u16 },
    InterfaceMethodRef { class: u16, name_and_type: u16 },
    FieldRef { class: u16, name_and_type: u16 },
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
        9 => parse_field_ref_const(data, pos),
        10 => parse_method_ref_const(data, pos),
        11 => parse_interface_method_ref_const(data, pos),
        12 => parse_name_and_type_const(data, pos),
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
fn parse_field_ref_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    // content 2 * u16
    let (class, pos) = get_u16(data, pos)?;
    let (name_and_type, pos) = get_u16(data, pos)?;
    Ok((
        ConstEntry::FieldRef {
            class,
            name_and_type,
        },
        pos,
        1,
    ))
}

fn parse_method_ref_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    // content 2 * u16
    let (class, pos) = get_u16(data, pos)?;
    let (name_and_type, pos) = get_u16(data, pos)?;

    Ok((
        ConstEntry::MethodRef {
            class,
            name_and_type,
        },
        pos,
        1,
    ))
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
fn parse_interface_method_ref_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    // content 2 * u16
    let (class, pos) = get_u16(data, pos)?;
    let (name_and_type, pos) = get_u16(data, pos)?;
    Ok((
        ConstEntry::InterfaceMethodRef {
            class,
            name_and_type,
        },
        pos,
        1,
    ))
}
fn parse_name_and_type_const(
    data: &[u8],
    pos: usize,
) -> Result<(ConstEntry, usize, usize), DecompilerError> {
    // content 2 * u16
    let (name, pos) = get_u16(data, pos)?;
    let (descriptor, pos) = get_u16(data, pos)?;
    Ok((ConstEntry::NameAndType { name, descriptor }, pos, 1))
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
fn get_i8(data: &[u8], pos: usize) -> Result<(i8, usize), DecompilerError> {
    let Some(get) = data.get(pos) else {
        return Err(DecompilerError::EOF);
    };
    let out = *get as i8;

    Ok((out, pos.saturating_add(1)))
}
fn get_u16(data: &[u8], pos: usize) -> Result<(u16, usize), DecompilerError> {
    let next = pos.saturating_add(2);
    let items = data.get(pos..next).ok_or(DecompilerError::EOF)?;
    let get = <[u8; 2]>::try_from(items).map_err(|_| DecompilerError::Number)?;
    let out = u16::from_be_bytes(get);

    Ok((out, next))
}
fn get_i16(data: &[u8], pos: usize) -> Result<(i16, usize), DecompilerError> {
    let next = pos.saturating_add(2);
    let items = data.get(pos..next).ok_or(DecompilerError::EOF)?;
    let get = <[u8; 2]>::try_from(items).map_err(|_| DecompilerError::Number)?;
    let out = i16::from_be_bytes(get);

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
const fn align_4byte(pos: usize) -> usize {
    (4_usize.saturating_sub(pos % 4)) % 4
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
            Some(&NuVec::new(b"ch.emilycares.Everything")),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();
        let expected = expect![[r#"
            package ch.emilycares;
            public class Everything {
                int noprop;
                public int publicproperty;
                private int privateproperty;
                public void Everything() {
                    // Aload0
                    // Invokespecial(MethodRef(class: java/lang/Object, name_and_type: (name: "<init>", descriptor: "()V")))
                    // Return
                }
                void method() {
                    // Return
                }
                public void public_method() {
                    // Return
                }
                private void private_method() {
                    // Return
                }
                int out() {
                    // Iconst0
                    // Ireturn
                }
                int add(int a, int b) {
                    // Iload1
                    // Iload2
                    // Iadd
                    // Ireturn
                }
                static int sadd(int a, int b) {
                    // Iload0
                    // Iload1
                    // Iadd
                    // Ireturn
                }
            }
        "#]];
        expected.assert_eq(out);
    }

    #[test]
    fn everything() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/Everything.class"),
            Some(&NuVec::new_static(b"ch.emilycares.Everything")),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r#"
            package ch.emilycares;
            public class Everything {
                int noprop;
                public int publicproperty;
                private int privateproperty;
                public void Everything() {
                    // Aload0
                    // Invokespecial(MethodRef(class: java/lang/Object, name_and_type: (name: "<init>", descriptor: "()V")))
                    // Return
                }
                void method() {
                    // Return
                }
                public void public_method() {
                    // Return
                }
                private void private_method() {
                    // Return
                }
                int out() {
                    // Iconst0
                    // Ireturn
                }
                int add(int a, int b) {
                    // Iload1
                    // Iload2
                    // Iadd
                    // Ireturn
                }
                static int sadd(int a, int b) {
                    // Iload0
                    // Iload1
                    // Iadd
                    // Ireturn
                }
            }
        "#]];
        expected.assert_eq(out);
    }

    #[test]
    fn constants() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/Constants.class"),
            Some(&NuVec::new_static(b"ch.emilycares.Constants")),
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
            Some(&NuVec::new_static(b"ch.emilycares.Types")),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r#"
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
                public void Types() {
                    // Aload0
                    // Invokespecial(MethodRef(class: java/lang/Object, name_and_type: (name: "<init>", descriptor: "()V")))
                    // Aload0
                    // Ldc(String "Types")
                    // Invokestatic(MethodRef(class: java/util/logging/Logger, name_and_type: (name: "getLogger", descriptor: "(Ljava/lang/String;)Ljava/util/logging/Logger;")))
                    // PutField(FieldRef(class: a/test/Types, name_and_type: (name: "LOG", descriptor: "Ljava/util/logging/Logger;")))
                    // Aload0
                    // Iconst1
                    // PutField(FieldRef(class: a/test/Types, name_and_type: (name: "IS_ACTIVE", descriptor: "Z")))
                    // Aload0
                    // Iconst0
                    // PutField(FieldRef(class: a/test/Types, name_and_type: (name: "one_byte", descriptor: "B")))
                    // Aload0
                    // Iconst0
                    // PutField(FieldRef(class: a/test/Types, name_and_type: (name: "one_int", descriptor: "I")))
                    // Aload0
                    // Iconst0
                    // PutField(FieldRef(class: a/test/Types, name_and_type: (name: "one_short", descriptor: "S")))
                    // Aload0
                    // Ldc2W(37)
                    // PutField(FieldRef(class: a/test/Types, name_and_type: (name: "one_long", descriptor: "J")))
                    // Aload0
                    // Dconst0
                    // PutField(FieldRef(class: a/test/Types, name_and_type: (name: "one_double", descriptor: "D")))
                    // Aload0
                    // Ldc(Entry Float(1066275963))
                    // PutField(FieldRef(class: a/test/Types, name_and_type: (name: "one_float", descriptor: "F")))
                    // Aload0
                    // Bipush(97)
                    // PutField(FieldRef(class: a/test/Types, name_and_type: (name: "one_char", descriptor: "C")))
                    // Aload0
                    // Ldc(String "hihi")
                    // PutField(FieldRef(class: a/test/Types, name_and_type: (name: "one_string", descriptor: "Ljava/lang/String;")))
                    // Aload0
                    // Ldc(String "haha")
                    // Invokestatic(InterfaceMethodRef(class: java/util/List, name_and_type: (name: "of", descriptor: "(Ljava/lang/Object;)Ljava/util/List;")))
                    // PutField(FieldRef(class: a/test/Types, name_and_type: (name: "one_list", descriptor: "Ljava/util/List;")))
                    // Aload0
                    // New("java/util/HashMap")
                    // Dup
                    // Invokespecial(MethodRef(class: java/util/HashMap, name_and_type: (name: "<init>", descriptor: "()V")))
                    // PutField(FieldRef(class: a/test/Types, name_and_type: (name: "one_map", descriptor: "Ljava/util/Map;")))
                    // Return
                }
                public static void main(java.lang.String[] a) {
                    // Return
                }
            }
        "#]];
        expected.assert_eq(out);
    }

    #[test]
    fn super_base() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/Super.class"),
            Some(&NuVec::new_static(b"ch.emilycares.Super")),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r#"
            package ch.emilycares;
            public class Super extends IOException {
                public void Super() {
                    // Aload0
                    // Invokespecial(MethodRef(class: java/io/IOException, name_and_type: (name: "<init>", descriptor: "()V")))
                    // Return
                }
            }
        "#]];
        expected.assert_eq(out);
    }
    #[test]
    fn thrower() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/Thrower.class"),
            Some(&NuVec::new_static(b"ch.emilycares.Thrower")),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();
        let expected = expect![[r#"
            package ch.emilycares;
            public class Thrower {
                public void Thrower() {
                    // Aload0
                    // Invokespecial(MethodRef(class: java/lang/Object, name_and_type: (name: "<init>", descriptor: "()V")))
                    // Return
                }
                public void ioThrower() throws java.io.IOException {
                    // Return
                }
                public void ioThrower(int a) throws java.io.IOException, java.io.IOException {
                    // Return
                }
            }
        "#]];
        expected.assert_eq(out);
    }
    #[test]
    fn super_interfaces() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/SuperInterface.class"),
            Some(&NuVec::new_static(b"ch.emilycares.SuperInterface")),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r#"
            package ch.emilycares;
            import java.util.stream.Stream;
            public interface SuperInterface<E> {
                default public java.util.stream.Stream<E> stream() {
                    // Aload0
                    // Invokeinterface(InterfaceMethodRef(class: ch/emilycares/SuperInterface, name_and_type: (name: "spliterator", descriptor: "()Ljava/util/Spliterator;")), count: 1)
                    // Iconst0
                    // Invokestatic(MethodRef(class: java/util/stream/StreamSupport, name_and_type: (name: "stream", descriptor: "(Ljava/util/Spliterator;Z)Ljava/util/stream/Stream;")))
                    // Areturn
                }
            }
        "#]];
        expected.assert_eq(out);
    }
    #[test]
    fn variables() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/LocalVariableTable.class"),
            Some(&NuVec::new_static(b"ch.emilycares.LocalVariableTable")),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r#"
            package ch.emilycares;
            import java.util.HashMap;
            import java.util.HashSet;
            public class LocalVariableTable {
                private java.util.HashSet<java.lang.String> a;
                public void LocalVariableTable() {
                    // Aload0
                    // Invokespecial(MethodRef(class: java/lang/Object, name_and_type: (name: "<init>", descriptor: "()V")))
                    // Aload0
                    // New("java/util/HashSet")
                    // Dup
                    // Invokespecial(MethodRef(class: java/util/HashSet, name_and_type: (name: "<init>", descriptor: "()V")))
                    // PutField(FieldRef(class: ch/emilycares/LocalVariableTable, name_and_type: (name: "a", descriptor: "Ljava/util/HashSet;")))
                    // Return
                }
                public void hereIsCode() {
                    // New("java/util/HashMap")
                    // Dup
                    // Invokespecial(MethodRef(class: java/util/HashMap, name_and_type: (name: "<init>", descriptor: "()V")))
                    // Astore1
                    // Aload1
                    // Iconst1
                    // Invokestatic(MethodRef(class: java/lang/Integer, name_and_type: (name: "valueOf", descriptor: "(I)Ljava/lang/Integer;")))
                    // Ldc(String "")
                    // Invokevirtual(MethodRef(class: java/util/HashMap, name_and_type: (name: "put", descriptor: "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;")))
                    // Pop
                    // Return
                }
                public int hereIsCode(int a, int b) {
                    // Iload1
                    // Iload2
                    // Iadd
                    // Istore3
                    // Iload3
                    // Iconst1
                    // Isub
                    // Ireturn
                }
            }
        "#]];
        expected.assert_eq(out);
    }
    #[test]
    fn variants() {
        let ast = decompile_class(
            include_bytes!("../../parser/test/Variants.class"),
            Some(&NuVec::new_static(b"ch.emilycares.Variants")),
        )
        .unwrap();
        let formatted = formatter::internal(&ast, b"", &EditorConfigFilled::default()).unwrap();
        let out = str::from_utf8(&formatted).unwrap();

        let expected = expect![[r#"
            package ch.emilycares;
            import java.lang.String;
            public final enum Variants {
                A,
                B,
                C;
                private final java.lang.String tag;
                private static final ch.emilycares.Variants[] $VALUES;
                public static ch.emilycares.Variants[] values() {
                    // Getstatic(FieldRef(class: ch/emilycares/Variants, name_and_type: (name: "$VALUES", descriptor: "[Lch/emilycares/Variants;")))
                    // Invokevirtual(MethodRef(class: [Lch/emilycares/Variants;, name_and_type: (name: "clone", descriptor: "()Ljava/lang/Object;")))
                    // Checkcast(18)
                    // Areturn
                }
                public static ch.emilycares.Variants valueOf(java.lang.String name) {
                    // Ldc(Class "ch/emilycares/Variants")
                    // Aload0
                    // Invokestatic(MethodRef(class: java/lang/Enum, name_and_type: (name: "valueOf", descriptor: "(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;")))
                    // Checkcast(1)
                    // Areturn
                }
                private void Variants(java.lang.String $enum$name) {
                    // Aload0
                    // Aload1
                    // Iload2
                    // Invokespecial(MethodRef(class: java/lang/Enum, name_and_type: (name: "<init>", descriptor: "(Ljava/lang/String;I)V")))
                    // Aload0
                    // Aload3
                    // PutField(FieldRef(class: ch/emilycares/Variants, name_and_type: (name: "tag", descriptor: "Ljava/lang/String;")))
                    // Return
                }
                public java.lang.String getTag() {
                    // Aload0
                    // Getfield(32)
                    // Areturn
                }
                private static ch.emilycares.Variants[] $values() {
                    // Iconst3
                    // Anewarray(1)
                    // Dup
                    // Iconst0
                    // Getstatic(FieldRef(class: ch/emilycares/Variants, name_and_type: (name: "A", descriptor: "Lch/emilycares/Variants;")))
                    // Aastore
                    // Dup
                    // Iconst1
                    // Getstatic(FieldRef(class: ch/emilycares/Variants, name_and_type: (name: "B", descriptor: "Lch/emilycares/Variants;")))
                    // Aastore
                    // Dup
                    // Iconst2
                    // Getstatic(FieldRef(class: ch/emilycares/Variants, name_and_type: (name: "C", descriptor: "Lch/emilycares/Variants;")))
                    // Aastore
                    // Areturn
                }
                static void clinit() {
                    // New("ch/emilycares/Variants")
                    // Dup
                    // Ldc(String "A")
                    // Iconst0
                    // Ldc(String "a")
                    // Invokespecial(MethodRef(class: ch/emilycares/Variants, name_and_type: (name: "<init>", descriptor: "(Ljava/lang/String;ILjava/lang/String;)V")))
                    // Putstatic(3)
                    // New("ch/emilycares/Variants")
                    // Dup
                    // Ldc(String "B")
                    // Iconst1
                    // Ldc(String "b")
                    // Invokespecial(MethodRef(class: ch/emilycares/Variants, name_and_type: (name: "<init>", descriptor: "(Ljava/lang/String;ILjava/lang/String;)V")))
                    // Putstatic(7)
                    // New("ch/emilycares/Variants")
                    // Dup
                    // Ldc(String "C")
                    // Iconst2
                    // Ldc(String "c")
                    // Invokespecial(MethodRef(class: ch/emilycares/Variants, name_and_type: (name: "<init>", descriptor: "(Ljava/lang/String;ILjava/lang/String;)V")))
                    // Putstatic(10)
                    // Invokestatic(MethodRef(class: ch/emilycares/Variants, name_and_type: (name: "$values", descriptor: "()[Lch/emilycares/Variants;")))
                    // Putstatic(13)
                    // Return
                }
            }
        "#]];
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
