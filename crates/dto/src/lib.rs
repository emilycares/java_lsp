use std::{
    fmt::Display,
    path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR},
};

use bitflags::bitflags;
use my_string::{MyString, smol_str::format_smolstr};

pub const CFC_VERSION: usize = 19;

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
    pub args: Vec<MyString>,
    pub ret: JType,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Class {
    pub class_path: MyString,
    pub source: SourceDestination,
    pub access: Access,
    pub imports: Vec<ImportUnit>,
    pub signature: Option<ClassSignature>,
    pub name: MyString,
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
    pub fn get_source(&self) -> Option<MyString> {
        match &self.source {
            SourceDestination::RelativeInFolder(e) => Some(format_smolstr!(
                "{}{}{}.java",
                e,
                MAIN_SEPARATOR,
                &self.class_path.replace('.', MAIN_SEPARATOR_STR)
            )),
            SourceDestination::RelativeInFolderLang(e, lang) => Some(format_smolstr!(
                "{}{}{}.{}",
                e,
                MAIN_SEPARATOR,
                &self.class_path.replace('.', MAIN_SEPARATOR_STR),
                lang
            )),
            SourceDestination::Here(e) => Some(e.clone()),
            SourceDestination::None => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SourceDestination {
    #[default]
    None,
    Here(MyString),
    RelativeInFolder(MyString),
    RelativeInFolderLang(MyString, MyString),
}

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub enum SuperClass {
    #[default]
    None,
    Name(MyString),
    ClassPath(MyString),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImportUnit {
    Package(MyString),
    Class(MyString),
    StaticClass(MyString),
    StaticClassMethod(MyString, MyString),
    Prefix(MyString),
    StaticPrefix(MyString),
}
impl ImportUnit {
    #[must_use]
    pub fn class_path_get_class_name(class_path: &str) -> Option<&str> {
        if let Some((_, c)) = class_path.rsplit_once('.') {
            return Some(c);
        }
        None
    }
    #[must_use]
    pub fn class_path_match_class_name(class_path: &str, name: &str) -> bool {
        Self::class_path_get_class_name(class_path)
            .iter()
            .any(|i| *i == name)
    }
    #[must_use]
    pub fn get_imported_class_package(&self, name: &str) -> Option<MyString> {
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
    pub name: Option<MyString>,
    pub parameters: Vec<Parameter>,
    pub throws: Vec<JType>,
    pub ret: JType,
    /// When None then it is in the class
    pub source: Option<MyString>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Field {
    pub access: Access,
    pub name: MyString,
    pub jtype: JType,
    /// When None then it is in the class
    pub source: Option<MyString>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Parameter {
    pub name: Option<MyString>,
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
    Class(MyString),
    ClassOrPackage(MyString),
    Array(Box<Self>),
    Generic(MyString, Vec<Self>),
    Parameter(MyString),
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
                if c.starts_with("java.lang.") {
                    return write!(f, "{}", c.trim_start_matches("java.lang."));
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
