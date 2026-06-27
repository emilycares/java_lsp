#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::enum_glob_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use dto::{
    Access, CFC_VERSION, Class, ClassFolder, ClassSignature, Field, ImportUnit, JType, Method,
    Parameter, SourceDestination, SuperClass,
};
use my_string::{MyString, smol_str::ToSmolStr};

#[derive(Debug)]
pub enum DtoRwError {
    EOF,
    Number,
    InvalidCfcCache,
    Str,
    SourceDestination,
    Access,
    Import,
    JType,
    SuperClass,
}
const HEADER: &[u8; 4] = &[0xAA, 0xCC, 0xFF, 0xCC];

#[must_use]
pub fn write(class_folder: &ClassFolder) -> Vec<u8> {
    let mut out = Vec::new();
    write_header(&mut out);
    write_classes(&class_folder.classes, &mut out);
    out
}
pub fn parse(data: &[u8]) -> Result<ClassFolder, DtoRwError> {
    let pos = 0;
    let pos = parse_header(data, pos)?;
    let (classes, _) = parse_classes(data, pos)?;

    Ok(ClassFolder { classes })
}

fn write_header(out: &mut Vec<u8>) {
    out.extend(HEADER);
    write_usize(CFC_VERSION, out);
}
fn parse_header(data: &[u8], pos: usize) -> Result<usize, DtoRwError> {
    let pos = {
        let len = HEADER.len();
        let Some(get) = data.get(pos..pos + len) else {
            return Err(DtoRwError::EOF);
        };

        let cond = get != HEADER;
        if cond {
            return Err(DtoRwError::InvalidCfcCache);
        }
        Ok(pos + len)
    }?;
    let (version, pos) = parse_usize(data, pos)?;
    if version != CFC_VERSION {
        return Err(DtoRwError::InvalidCfcCache);
    }
    Ok(pos)
}

fn write_string(st: &MyString, out: &mut Vec<u8>) {
    write_usize(st.len(), out);
    out.extend(st.as_bytes());
}
fn parse_string(data: &[u8], pos: usize) -> Result<(MyString, usize), DtoRwError> {
    let (len, pos) = parse_usize(data, pos)?;
    let Some(get) = data.get(pos..pos.saturating_add(len)) else {
        return Err(DtoRwError::EOF);
    };
    let st = str::from_utf8(get).map_err(|_| DtoRwError::Str)?;

    Ok((st.to_smolstr(), pos + len))
}

fn write_classes(classes: &[Class], out: &mut Vec<u8>) {
    write_usize(classes.len(), out);
    for c in classes {
        write_class(c, out);
    }
}
fn parse_classes(data: &[u8], pos: usize) -> Result<(Vec<Class>, usize), DtoRwError> {
    let (len, pos) = parse_usize(data, pos)?;
    let mut i = 0;
    let mut pos = pos;
    let mut out = Vec::new();
    while i != len {
        let (class, npos) = parse_class(data, pos)?;
        pos = npos;
        out.push(class);
        i += 1;
    }

    Ok((out, pos))
}

fn write_class(class: &Class, out: &mut Vec<u8>) {
    write_string(&class.class_path, out);
    write_source_destination(&class.source, out);
    write_access(&class.access, out);
    write_imports(&class.imports, out);
    write_class_signature(class.signature.as_ref(), out);
    write_string(&class.name, out);
    write_methods(&class.methods, out);
    write_fields(&class.fields, out);
    write_super_class(&class.super_class, out);
    write_super_classes(&class.super_interfaces, out);
}
fn parse_class(data: &[u8], pos: usize) -> Result<(Class, usize), DtoRwError> {
    let (class_path, pos) = parse_string(data, pos)?;
    let (source, pos) = parse_source_destination(data, pos)?;
    let (access, pos) = parse_access(data, pos)?;
    let (imports, pos) = parse_imports(data, pos)?;
    let (signature, pos) = parse_class_signature(data, pos)?;
    let (name, pos) = parse_string(data, pos)?;
    let (methods, pos) = parse_methods(data, pos)?;
    let (fields, pos) = parse_fields(data, pos)?;
    let (super_class, pos) = parse_super_class(data, pos)?;
    let (super_interfaces, pos) = parse_super_classes(data, pos)?;

    Ok((
        Class {
            class_path,
            source,
            access,
            imports,
            signature,
            name,
            methods,
            fields,
            super_class,
            super_interfaces,
        },
        pos,
    ))
}

fn write_source_destination(source_destination: &SourceDestination, out: &mut Vec<u8>) {
    match source_destination {
        SourceDestination::Here(smol_str) => {
            write_u8(1, out);
            write_string(smol_str, out);
        }
        SourceDestination::RelativeInFolder(smol_str) => {
            write_u8(2, out);
            write_string(smol_str, out);
        }
        SourceDestination::RelativeInFolderLang(smol_str, lang) => {
            write_u8(3, out);
            write_string(smol_str, out);
            write_string(lang, out);
        }
        SourceDestination::None => write_u8(0, out),
    }
}
fn parse_source_destination(
    data: &[u8],
    pos: usize,
) -> Result<(SourceDestination, usize), DtoRwError> {
    let (variant, pos) = parse_u8(data, pos)?;
    match variant {
        0 => Ok((SourceDestination::None, pos)),
        1 => {
            let (st, pos) = parse_string(data, pos)?;
            Ok((SourceDestination::Here(st), pos))
        }
        2 => {
            let (st, pos) = parse_string(data, pos)?;
            Ok((SourceDestination::RelativeInFolder(st), pos))
        }
        3 => {
            let (st, pos) = parse_string(data, pos)?;
            let (lang, pos) = parse_string(data, pos)?;
            Ok((SourceDestination::RelativeInFolderLang(st, lang), pos))
        }
        _ => Err(DtoRwError::SourceDestination),
    }
}

fn write_access(st: &Access, out: &mut Vec<u8>) {
    write_u16(st.bits(), out);
}
fn parse_access(data: &[u8], pos: usize) -> Result<(Access, usize), DtoRwError> {
    let (bits, pos) = parse_u16(data, pos)?;
    let out = Access::from_bits(bits).ok_or(DtoRwError::Access)?;
    Ok((out, pos))
}

fn write_imports(im: &[ImportUnit], out: &mut Vec<u8>) {
    write_usize(im.len(), out);
    for c in im {
        write_import(c, out);
    }
}
fn parse_imports(data: &[u8], pos: usize) -> Result<(Vec<ImportUnit>, usize), DtoRwError> {
    let (len, pos) = parse_usize(data, pos)?;
    let mut i = 0;
    let mut pos = pos;
    let mut out = Vec::new();
    while i < len {
        let (im, npos) = parse_import(data, pos)?;
        pos = npos;
        out.push(im);
        i += 1;
    }
    Ok((out, pos))
}

fn write_import(im: &ImportUnit, out: &mut Vec<u8>) {
    match im {
        ImportUnit::Package(smol_str) => {
            write_u8(0, out);
            write_string(smol_str, out);
        }
        ImportUnit::Class(smol_str) => {
            write_u8(1, out);
            write_string(smol_str, out);
        }
        ImportUnit::StaticClass(smol_str) => {
            write_u8(2, out);
            write_string(smol_str, out);
        }
        ImportUnit::StaticClassMethod(smol_str, smol_str1) => {
            write_u8(3, out);
            write_string(smol_str, out);
            write_string(smol_str1, out);
        }
        ImportUnit::Prefix(smol_str) => {
            write_u8(4, out);
            write_string(smol_str, out);
        }
        ImportUnit::StaticPrefix(smol_str) => {
            write_u8(5, out);
            write_string(smol_str, out);
        }
    }
}
fn parse_import(data: &[u8], pos: usize) -> Result<(ImportUnit, usize), DtoRwError> {
    let (variant, pos) = parse_u8(data, pos)?;
    match variant {
        0 => {
            let (s, pos) = parse_string(data, pos)?;
            Ok((ImportUnit::Package(s), pos))
        }
        1 => {
            let (s, pos) = parse_string(data, pos)?;
            Ok((ImportUnit::Class(s), pos))
        }
        2 => {
            let (s, pos) = parse_string(data, pos)?;
            Ok((ImportUnit::StaticClass(s), pos))
        }
        3 => {
            let (s, pos) = parse_string(data, pos)?;
            let (method, pos) = parse_string(data, pos)?;
            Ok((ImportUnit::StaticClassMethod(s, method), pos))
        }
        4 => {
            let (s, pos) = parse_string(data, pos)?;
            Ok((ImportUnit::Prefix(s), pos))
        }
        5 => {
            let (s, pos) = parse_string(data, pos)?;
            Ok((ImportUnit::StaticPrefix(s), pos))
        }
        _ => Err(DtoRwError::Import),
    }
}

fn write_class_signature(si: Option<&ClassSignature>, out: &mut Vec<u8>) {
    if let Some(c) = si {
        write_u8(1, out);
        write_usize(c.args.len(), out);
        for a in &c.args {
            write_string(a, out);
        }
        write_jtype(&c.ret, out);
    } else {
        write_u8(0, out);
    }
}
fn parse_class_signature(
    data: &[u8],
    pos: usize,
) -> Result<(Option<ClassSignature>, usize), DtoRwError> {
    let (v, pos) = parse_u8(data, pos)?;
    if v == 0 {
        return Ok((None, pos));
    }
    let (len, pos) = parse_usize(data, pos)?;
    let mut i = 0;
    let mut pos = pos;
    let mut args = Vec::new();
    while i < len {
        let (a, npos) = parse_string(data, pos)?;
        pos = npos;
        args.push(a);
        i += 1;
    }
    let (ret, pos) = parse_jtype(data, pos)?;
    Ok((Some(ClassSignature { args, ret }), pos))
}

fn write_jtype(t: &JType, out: &mut Vec<u8>) {
    match t {
        JType::Void => write_u8(0, out),
        JType::Byte => write_u8(1, out),
        JType::Char => write_u8(2, out),
        JType::Double => write_u8(3, out),
        JType::Float => write_u8(4, out),
        JType::Int => write_u8(5, out),
        JType::Long => write_u8(6, out),
        JType::Short => write_u8(7, out),
        JType::Boolean => write_u8(8, out),
        JType::Wildcard => write_u8(9, out),
        JType::Var => write_u8(10, out),
        JType::Class(smol_str) => {
            write_u8(11, out);
            write_string(smol_str, out);
        }
        JType::ClassOrPackage(smol_str) => {
            write_u8(12, out);
            write_string(smol_str, out);
        }
        JType::Array(jtype) => {
            write_u8(13, out);
            write_jtype(jtype, out);
        }
        JType::Generic(smol_str, jtypes) => {
            write_u8(14, out);
            write_string(smol_str, out);
            write_usize(jtypes.len(), out);
            for t in jtypes {
                write_jtype(t, out);
            }
        }
        JType::Parameter(smol_str) => {
            write_u8(15, out);
            write_string(smol_str, out);
        }
        JType::Extends { base, extends } => {
            write_u8(16, out);
            write_jtype(base, out);
            write_jtype(extends, out);
        }
        JType::Access { base, inner } => {
            write_u8(17, out);
            write_jtype(base, out);
            write_jtype(inner, out);
        }
    }
}
fn parse_jtype(data: &[u8], pos: usize) -> Result<(JType, usize), DtoRwError> {
    let (variant, pos) = parse_u8(data, pos)?;
    match variant {
        0 => Ok((JType::Void, pos)),
        1 => Ok((JType::Byte, pos)),
        2 => Ok((JType::Char, pos)),
        3 => Ok((JType::Double, pos)),
        4 => Ok((JType::Float, pos)),
        5 => Ok((JType::Int, pos)),
        6 => Ok((JType::Long, pos)),
        7 => Ok((JType::Short, pos)),
        8 => Ok((JType::Boolean, pos)),
        9 => Ok((JType::Wildcard, pos)),
        10 => Ok((JType::Var, pos)),
        11 => {
            let (name, pos) = parse_string(data, pos)?;
            Ok((JType::Class(name), pos))
        }
        12 => {
            let (name, pos) = parse_string(data, pos)?;
            Ok((JType::ClassOrPackage(name), pos))
        }
        13 => {
            let (inner, pos) = parse_jtype(data, pos)?;
            Ok((JType::Array(Box::new(inner)), pos))
        }
        14 => {
            let (name, pos) = parse_string(data, pos)?;
            let (len, pos) = parse_usize(data, pos)?;
            let mut i = 0;
            let mut pos = pos;
            let mut args = Vec::new();
            while i != len {
                let (a, npos) = parse_jtype(data, pos)?;
                pos = npos;
                args.push(a);
                i += 1;
            }
            Ok((JType::Generic(name, args), pos))
        }
        15 => {
            let (name, pos) = parse_string(data, pos)?;
            Ok((JType::Parameter(name), pos))
        }
        16 => {
            let (base, pos) = parse_jtype(data, pos)?;
            let (extends, pos) = parse_jtype(data, pos)?;
            Ok((
                JType::Extends {
                    base: Box::new(base),
                    extends: Box::new(extends),
                },
                pos,
            ))
        }
        17 => {
            let (base, pos) = parse_jtype(data, pos)?;
            let (inner, pos) = parse_jtype(data, pos)?;
            Ok((
                JType::Access {
                    base: Box::new(base),
                    inner: Box::new(inner),
                },
                pos,
            ))
        }
        _ => Err(DtoRwError::JType),
    }
}

fn write_methods(ms: &[Method], out: &mut Vec<u8>) {
    write_usize(ms.len(), out);
    for m in ms {
        write_method(m, out);
    }
}
fn parse_methods(data: &[u8], pos: usize) -> Result<(Vec<Method>, usize), DtoRwError> {
    let (len, pos) = parse_usize(data, pos)?;
    let mut i = 0;
    let mut pos = pos;
    let mut out = Vec::new();
    while i < len {
        let (im, npos) = parse_method(data, pos)?;
        pos = npos;
        out.push(im);
        i += 1;
    }
    Ok((out, pos))
}

fn write_method(m: &Method, out: &mut Vec<u8>) {
    write_access(&m.access, out);
    if let Some(n) = &m.name {
        write_u8(1, out);
        write_string(n, out);
    } else {
        write_u8(0, out);
    }
    write_usize(m.parameters.len(), out);
    for p in &m.parameters {
        write_parameter(p, out);
    }
    write_usize(m.throws.len(), out);
    for t in &m.throws {
        write_jtype(t, out);
    }
    write_jtype(&m.ret, out);
    if let Some(n) = &m.source {
        write_u8(1, out);
        write_string(n, out);
    } else {
        write_u8(0, out);
    }
}
fn parse_method(data: &[u8], pos: usize) -> Result<(Method, usize), DtoRwError> {
    let (access, pos) = parse_access(data, pos)?;

    let (variant, pos) = parse_u8(data, pos)?;
    let (name, pos) = if variant == 1 {
        let (name, pos) = parse_string(data, pos)?;
        (Some(name), pos)
    } else {
        (None, pos)
    };

    let (len, pos) = parse_usize(data, pos)?;
    let mut pos = pos;
    let mut parameters = Vec::new();
    let mut i = 0;
    while i < len {
        let (im, npos) = parse_parameter(data, pos)?;
        pos = npos;
        parameters.push(im);
        i += 1;
    }

    let (len, pos) = parse_usize(data, pos)?;
    let mut pos = pos;
    let mut throws = Vec::new();
    let mut i = 0;
    while i < len {
        let (im, npos) = parse_jtype(data, pos)?;
        pos = npos;
        throws.push(im);
        i += 1;
    }

    let (ret, pos) = parse_jtype(data, pos)?;

    let (variant, pos) = parse_u8(data, pos)?;
    let (source, pos) = if variant == 1 {
        let (source, pos) = parse_string(data, pos)?;
        (Some(source), pos)
    } else {
        (None, pos)
    };
    Ok((
        Method {
            access,
            name,
            parameters,
            throws,
            ret,
            source,
        },
        pos,
    ))
}

fn write_parameter(p: &Parameter, out: &mut Vec<u8>) {
    if let Some(n) = &p.name {
        write_u8(1, out);
        write_string(n, out);
    } else {
        write_u8(0, out);
    }
    write_jtype(&p.jtype, out);
}
fn parse_parameter(data: &[u8], pos: usize) -> Result<(Parameter, usize), DtoRwError> {
    let (variant, pos) = parse_u8(data, pos)?;
    let (name, pos) = if variant == 1 {
        let (name, pos) = parse_string(data, pos)?;
        (Some(name), pos)
    } else {
        (None, pos)
    };
    let (jtype, pos) = parse_jtype(data, pos)?;
    Ok((Parameter { name, jtype }, pos))
}

fn write_fields(ms: &[Field], out: &mut Vec<u8>) {
    write_usize(ms.len(), out);
    for m in ms {
        write_field(m, out);
    }
}
fn parse_fields(data: &[u8], pos: usize) -> Result<(Vec<Field>, usize), DtoRwError> {
    let (len, pos) = parse_usize(data, pos)?;
    let mut i = 0;
    let mut pos = pos;
    let mut out = Vec::new();
    while i < len {
        let (im, npos) = parse_field(data, pos)?;
        pos = npos;
        out.push(im);
        i += 1;
    }
    Ok((out, pos))
}

fn write_field(f: &Field, out: &mut Vec<u8>) {
    write_access(&f.access, out);
    write_string(&f.name, out);
    write_jtype(&f.jtype, out);
    if let Some(n) = &f.source {
        write_u8(1, out);
        write_string(n, out);
    } else {
        write_u8(0, out);
    }
}
fn parse_field(data: &[u8], pos: usize) -> Result<(Field, usize), DtoRwError> {
    let (access, pos) = parse_access(data, pos)?;
    let (name, pos) = parse_string(data, pos)?;
    let (jtype, pos) = parse_jtype(data, pos)?;

    let (variant, pos) = parse_u8(data, pos)?;
    let (source, pos) = if variant == 1 {
        let (source, pos) = parse_string(data, pos)?;
        (Some(source), pos)
    } else {
        (None, pos)
    };
    Ok((
        Field {
            access,
            name,
            jtype,
            source,
        },
        pos,
    ))
}

fn write_super_class(super_class: &SuperClass, out: &mut Vec<u8>) {
    match super_class {
        SuperClass::None => write_u8(0, out),
        SuperClass::Name(smol_str) => {
            write_u8(1, out);
            write_string(smol_str, out);
        }
        SuperClass::ClassPath(smol_str) => {
            write_u8(2, out);
            write_string(smol_str, out);
        }
    }
}
fn parse_super_class(data: &[u8], pos: usize) -> Result<(SuperClass, usize), DtoRwError> {
    let (variant, pos) = parse_u8(data, pos)?;
    match variant {
        0 => Ok((SuperClass::None, pos)),
        1 => {
            let (st, pos) = parse_string(data, pos)?;
            Ok((SuperClass::Name(st), pos))
        }
        2 => {
            let (st, pos) = parse_string(data, pos)?;
            Ok((SuperClass::ClassPath(st), pos))
        }
        _ => Err(DtoRwError::SuperClass),
    }
}

fn write_super_classes(ms: &[SuperClass], out: &mut Vec<u8>) {
    write_usize(ms.len(), out);
    for m in ms {
        write_super_class(m, out);
    }
}
fn parse_super_classes(data: &[u8], pos: usize) -> Result<(Vec<SuperClass>, usize), DtoRwError> {
    let (len, pos) = parse_usize(data, pos)?;
    let mut i = 0;
    let mut pos = pos;
    let mut out = Vec::new();
    while i < len {
        let (im, npos) = parse_super_class(data, pos)?;
        pos = npos;
        out.push(im);
        i += 1;
    }
    Ok((out, pos))
}

fn write_usize(n: usize, out: &mut Vec<u8>) {
    out.extend(n.to_le_bytes());
}
fn parse_usize(data: &[u8], pos: usize) -> Result<(usize, usize), DtoRwError> {
    let next = pos + 8;
    let items = data.get(pos..next).ok_or(DtoRwError::EOF)?;
    let get = <[u8; 8]>::try_from(items).map_err(|_| DtoRwError::Number)?;
    let out = usize::from_le_bytes(get);

    Ok((out, next))
}

fn write_u16(n: u16, out: &mut Vec<u8>) {
    out.extend(n.to_le_bytes());
}
fn parse_u16(data: &[u8], pos: usize) -> Result<(u16, usize), DtoRwError> {
    let next = pos + 2;
    let items = data.get(pos..next).ok_or(DtoRwError::EOF)?;
    let get = <[u8; 2]>::try_from(items).map_err(|_| DtoRwError::Number)?;
    let out = u16::from_le_bytes(get);

    Ok((out, next))
}

fn write_u8(n: u8, out: &mut Vec<u8>) {
    out.push(n);
}
fn parse_u8(data: &[u8], pos: usize) -> Result<(u8, usize), DtoRwError> {
    let Some(get) = data.get(pos) else {
        return Err(DtoRwError::EOF);
    };

    Ok((*get, pos + 1))
}

#[cfg(test)]
mod tests {
    use dto::ClassSignature;
    use my_string::{MyString, smol_str::SmolStr};

    use super::*;

    #[test]
    fn header() {
        let mut data = Vec::new();
        write_header(&mut data);
        let pos = parse_header(&data, 0).unwrap();
        assert_eq!(pos, 12);
    }

    #[test]
    fn string() {
        let mut data = Vec::new();
        let input = MyString::new_inline("java_lsp");
        write_string(&input, &mut data);
        let (out, pos) = parse_string(&data, 0).unwrap();
        assert_eq!(input, out);
        assert_eq!(pos, 16);
    }

    #[test]
    fn source_none() {
        let mut data = Vec::new();
        let input = SourceDestination::None;
        write_source_destination(&input, &mut data);
        let (out, pos) = parse_source_destination(&data, 0).unwrap();
        assert_eq!(input, out);
        assert_eq!(pos, 1);
    }

    #[test]
    fn source_here() {
        let mut data = Vec::new();
        let input = SourceDestination::Here(SmolStr::new_inline("/data/Here.java"));
        write_source_destination(&input, &mut data);
        let (out, pos) = parse_source_destination(&data, 0).unwrap();
        assert_eq!(input, out);
        assert_eq!(pos, 24);
    }

    #[test]
    fn source_relative() {
        let mut data = Vec::new();
        let input = SourceDestination::RelativeInFolder(SmolStr::new_inline("/data"));
        write_source_destination(&input, &mut data);
        let (out, pos) = parse_source_destination(&data, 0).unwrap();
        assert_eq!(input, out);
        assert_eq!(pos, 14);
    }

    #[test]
    fn access() {
        let mut data = Vec::new();
        let input = Access::Public | Access::Interface;
        write_access(&input, &mut data);
        let (out, pos) = parse_access(&data, 0).unwrap();
        assert_eq!(input, out);
        assert_eq!(pos, 2);
    }

    #[test]
    fn imports() {
        let mut data = Vec::new();
        let input = vec![
            ImportUnit::Package(SmolStr::new_inline("emily")),
            ImportUnit::Class(SmolStr::new_inline("emily")),
            ImportUnit::StaticClass(SmolStr::new_inline("emily")),
            ImportUnit::StaticClassMethod(
                SmolStr::new_inline("emily"),
                SmolStr::new_inline("emily"),
            ),
            ImportUnit::Prefix(SmolStr::new_inline("emily")),
            ImportUnit::StaticPrefix(SmolStr::new_inline("emily")),
        ];
        write_imports(&input, &mut data);
        let (out, pos) = parse_imports(&data, 0).unwrap();
        assert_eq!(input, out);
        assert_eq!(pos, 105);
    }

    #[test]
    fn class_signature_none() {
        let mut data = Vec::new();
        let input = None;
        write_class_signature(input, &mut data);
        let (out, pos) = parse_class_signature(&data, 0).unwrap();
        assert_eq!(input, out.as_ref());
        assert_eq!(pos, 1);
    }

    #[test]
    fn class_signature() {
        let mut data = Vec::new();
        let input = Some(ClassSignature {
            args: vec![SmolStr::new_inline("T")],
            ret: JType::Generic(
                SmolStr::new_inline("List"),
                vec![JType::Parameter(SmolStr::new_inline("T"))],
            ),
        });
        write_class_signature(input.as_ref(), &mut data);
        let (out, pos) = parse_class_signature(&data, 0).unwrap();
        assert_eq!(input, out);
        assert_eq!(pos, 49);
    }

    #[test]
    fn base() {
        let input = ClassFolder {
            classes: vec![Class {
                class_path: SmolStr::new_inline("eu.emily.String"),
                source: SourceDestination::None,
                access: Access::Public,
                imports: vec![],
                signature: None,
                name: SmolStr::new_inline("String"),
                methods: vec![
                    Method {
                        access: Access::Public,
                        name: None,
                        parameters: vec![
                            Parameter {
                                name: Some(SmolStr::new_inline("a")),
                                jtype: JType::Char,
                            },
                            Parameter {
                                name: None,
                                jtype: JType::Char,
                            },
                        ],
                        throws: vec![JType::Class(SmolStr::new_inline("IOException"))],
                        ret: JType::Void,
                        source: None,
                    },
                    Method {
                        access: Access::Public,
                        name: Some(SmolStr::new_inline("haha")),
                        parameters: vec![],
                        throws: vec![],
                        ret: JType::Void,
                        source: Some(SmolStr::new_inline("/data/String.java")),
                    },
                ],
                fields: vec![
                    Field {
                        access: Access::Public,
                        name: SmolStr::new_inline("a"),
                        jtype: JType::Int,
                        source: None,
                    },
                    Field {
                        access: Access::Public,
                        name: SmolStr::new_inline("a"),
                        jtype: JType::Int,
                        source: Some(SmolStr::new_inline("/data/String.java")),
                    },
                ],
                super_class: SuperClass::None,
                super_interfaces: vec![
                    SuperClass::Name(SmolStr::new_inline("String")),
                    SuperClass::ClassPath(SmolStr::new_inline("eu.emily.String")),
                ],
            }],
        };
        let data = write(&input);
        let out = parse(&data).unwrap();
        assert_eq!(input, out);
    }
}
