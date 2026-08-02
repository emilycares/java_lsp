use std::{fmt::Display, path::MAIN_SEPARATOR};

use bitflags::bitflags;
use my_string::{NuVec, NuVecBuilder};

pub const CFC_VERSION: usize = 20;

#[derive(Debug)]
pub enum ClassParserError {
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

#[derive(Debug, PartialEq, Clone, Default)]
pub struct ClassFolder {
    pub classes: Vec<Class>,
}

impl ClassFolder {
    pub fn append(&mut self, other: Self) {
        self.classes.extend(other.classes);
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct ClassSignature {
    // Generics defined on class level
    pub args: Vec<NuVec>,
    pub ret: JType,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Class {
    pub class_path: NuVec,
    pub source: SourceDestination,
    pub access: Access,
    pub imports: Vec<ImportUnit>,
    pub signature: Option<ClassSignature>,
    pub name: NuVec,
    pub methods: Vec<Method>,
    pub fields: Vec<Field>,
    pub super_class: SuperClass,
    pub super_interfaces: Vec<SuperClass>,
}
impl Class {
    #[must_use]
    pub fn no_imports(mut self) -> Self {
        self.imports = vec![];
        self
    }

    #[must_use]
    pub fn get_source(&self) -> Option<NuVec> {
        match &self.source {
            SourceDestination::RelativeInFolder(e) => {
                let s = MAIN_SEPARATOR as u8;
                let mut b = NuVecBuilder::new();
                b.extend(e);
                b.push(s);
                b.extend(&self.class_path.replace_byte(b'.', s));
                b.pusha(b".java");
                Some(b.finish())
            }
            SourceDestination::RelativeInFolderLang(e, lang) => {
                let separator = MAIN_SEPARATOR as u8;
                let mut b = NuVecBuilder::new();
                b.extend(e);
                b.push(separator);
                b.extend(&self.class_path.replace_byte(b'.', separator));
                b.push(b'.');
                b.extend(lang);
                Some(b.finish())
            }
            SourceDestination::Here(e) => Some(e.clone()),
            SourceDestination::None => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SourceDestination {
    #[default]
    None,
    Here(NuVec),
    RelativeInFolder(NuVec),
    RelativeInFolderLang(NuVec, NuVec),
}

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub enum SuperClass {
    #[default]
    None,
    Name(NuVec),
    ClassPath(NuVec),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImportUnit {
    Package(NuVec),
    Class(NuVec),
    StaticClass(NuVec),
    StaticClassMethod(NuVec, NuVec),
    Prefix(NuVec),
    StaticPrefix(NuVec),
}
impl ImportUnit {
    #[must_use]
    pub fn class_path_get_class_name(class_path: &NuVec) -> Option<NuVec> {
        if let Some((_, c)) = class_path.rsplit_once_byte(b'.') {
            return Some(c);
        }
        None
    }
    #[must_use]
    pub fn class_path_match_class_name(class_path: &NuVec, name: &NuVec) -> bool {
        Self::class_path_get_class_name(class_path)
            .iter()
            .any(|i| i == name)
    }
    #[must_use]
    pub fn get_imported_class_package(&self, name: &NuVec) -> Option<NuVec> {
        match self {
            Self::Class(class_path) | Self::StaticClass(class_path) => {
                if Self::class_path_match_class_name(class_path, name) {
                    return Some(class_path.clone());
                }
                None
            }
            _ => None,
        }
    }
}

bitflags! {
   #[derive(Clone, Eq, PartialEq, Debug, Default)]
   pub struct Access: u16 {
     const Public       = 0b0000_0000_0000_0001;
     const Private      = 0b0000_0000_0000_0010;
     const Protected    = 0b0000_0000_0000_0100;
     const Static       = 0b0000_0000_0000_1000;
     const Final        = 0b0000_0000_0001_0000;
     const Super        = 0b0000_0000_0010_0000;
     const Volatile     = 0b0000_0000_0100_0000;
     const Transient    = 0b0000_0000_1000_0000;
     const Synthetic    = 0b0000_0001_0000_0000;
     const Annotation   = 0b0000_0010_0000_0000;
     const Enum         = 0b0000_0100_0000_0000;
     const Interface    = 0b0000_1000_0000_0000;
     const Abstract     = 0b0001_0000_0000_0000;
     const Synchronized = 0b0010_0000_0000_0000;
     const Deprecated   = 0b0100_0000_0000_0000;
   }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Method {
    pub access: Access,
    pub name: Option<NuVec>,
    pub parameters: Vec<Parameter>,
    pub throws: Vec<JType>,
    pub ret: JType,
    /// When None then it is in the class
    pub source: Option<NuVec>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Field {
    pub access: Access,
    pub name: NuVec,
    pub jtype: JType,
    /// When None then it is in the class
    pub source: Option<NuVec>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Parameter {
    pub name: Option<NuVec>,
    pub jtype: JType,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub enum JType {
    #[default]
    Void,
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Short,
    Boolean,
    Wildcard,
    Class(NuVec),
    ClassOrPackage(NuVec),
    Array(Box<Self>),
    Generic(NuVec, Vec<Self>),
    Parameter(NuVec),
    Extends {
        base: Box<Self>,
        extends: Box<Self>,
    },
    Var,
    Access {
        base: Box<Self>,
        inner: Box<Self>,
    },
}

impl JType {
    pub fn to_nuvec(&self) -> NuVec {
        match self {
            JType::Void => NuVec::new_static(b"void"),
            JType::Byte => NuVec::new_static(b"byte"),
            JType::Char => NuVec::new_static(b"char"),
            JType::Double => NuVec::new_static(b"double"),
            JType::Float => NuVec::new_static(b"float"),
            JType::Int => NuVec::new_static(b"int"),
            JType::Long => NuVec::new_static(b"long"),
            JType::Short => NuVec::new_static(b"short"),
            JType::Boolean => NuVec::new_static(b"boolean"),
            JType::Wildcard => NuVec::new_static(b"?"),
            JType::Var => NuVec::new_static(b"var"),
            JType::Class(nu_vec) | JType::ClassOrPackage(nu_vec) => class_name_hover(nu_vec),
            JType::Array(jtype) => {
                let mut b = NuVecBuilder::new();
                b.extend(&jtype.to_nuvec());
                b.pusha(b"[]");
                b.finish()
            }
            JType::Generic(nu_vec, jtypes) => {
                let mut b = NuVecBuilder::new();
                b.extend(nu_vec);
                b.push(b'<');
                let mut first = true;
                for j in jtypes {
                    if !first {
                        b.pusha(b", ");
                    }
                    first = false;
                    b.extend(&j.to_nuvec());
                }
                b.push(b'>');
                b.finish()
            }
            JType::Parameter(nu_vec) => {
                let mut b = NuVecBuilder::new();
                b.push(b'<');
                b.extend(nu_vec);
                b.push(b'>');
                b.finish()
            }
            JType::Extends { base, .. } => base.to_nuvec(),
            JType::Access { base, inner } => {
                let mut b = NuVecBuilder::new();
                b.extend(&base.to_nuvec());
                b.push(b'.');
                b.extend(&inner.to_nuvec());
                b.finish()
            }
        }
    }
}
fn class_name_hover(s: &NuVec) -> NuVec {
    if let Some((_, s)) = s.rsplit_once_byte(b'.') {
        return s.replace_byte(b'$', b'.');
    }
    s.clone()
}

impl Display for JType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Void => write!(f, "void"),
            Self::Byte => write!(f, "byte"),
            Self::Char => write!(f, "char"),
            Self::Double => write!(f, "double"),
            Self::Float => write!(f, "float"),
            Self::Int => write!(f, "int"),
            Self::Long => write!(f, "long"),
            Self::Short => write!(f, "short"),
            Self::Boolean => write!(f, "boolean"),
            Self::Wildcard => write!(f, "?"),
            Self::Class(c) | Self::ClassOrPackage(c) => {
                if c.starts_with(b"java.lang.") {
                    return write!(f, "{}", c.trim_start_matches(b"java.lang."));
                }
                write!(f, "{c}")
            }
            Self::Array(i) => write!(f, "{i}[]"),
            Self::Generic(class, vec) => {
                let v = vec
                    .iter()
                    .map(|i| format!("{i}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{class}<{v}>")
            }
            Self::Parameter(p) => write!(f, "<{p}>"),
            Self::Var => write!(f, "var"),
            Self::Access { base, inner } => {
                write!(f, "{}.{}", **base, **inner)
            }
            Self::Extends { base, .. } => {
                write!(f, "{}", **base)
            }
        }
    }
}
