use std::{
    fmt::Display,
    path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR},
};

use bitflags::bitflags;
use my_string::{
    MyString,
    smol_str::{SmolStr, format_smolstr},
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum ClassError {
    IO(std::io::Error),
    Asm,
    Unknown,
    ParseError,
    ClassParser(ClassParserError),
    UnknownClassName,
    UnknownClassPath,
    InvalidClassPath,
    NoModuleAttribute,
}

#[derive(Debug)]
pub enum ClassParserError {
    EOF,
    ExpectedOther { pos: usize },
    IgnoringLambda,
    StringIndexZero,
    ExpectedString,
    Private,
    InvalidName,
    NotEnogthParams,
    NoModuleAttribute,
    UnknownType(Option<char>),
    GenericParameterName,
    InvalidAttributeIndex,
    CodeAttribute,
    BaseParser,
    MethodParameters,
    SignatureAttribute,
    ExceptionsAttribute,
    LocalVariableTable,
    Module,
    ModuleAttribute,
    NotAsExpected { pos: usize, len: usize },
    NotAClass,
}

pub const CFC_VERSION: usize = 10;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct ClassFolder {
    pub version: usize,
    pub classes: Vec<Class>,
}

impl ClassFolder {
    pub fn append(&mut self, other: Self) {
        self.classes.extend(other.classes);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Class {
    pub class_path: MyString,
    pub source: SourceDestination,
    pub access: Access,
    pub imports: Vec<ImportUnit>,
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
    pub fn get_source(&self) -> MyString {
        match &self.source {
            SourceDestination::RelativeInFolder(e) => format_smolstr!(
                "{}{}{}.java",
                e,
                MAIN_SEPARATOR,
                &self.class_path.replace('.', MAIN_SEPARATOR_STR)
            ),
            SourceDestination::Here(e) => e.clone(),
            SourceDestination::None => SmolStr::new(""),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SourceDestination {
    Here(MyString),
    RelativeInFolder(MyString),
    #[default]
    None,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Default)]
pub enum SuperClass {
    #[default]
    None,
    Name(MyString),
    ClassPath(MyString),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
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
   #[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug, Default)]
   pub struct Access: u16 {
     const Public       = 0b0000_0000_0000_0001;
     const Private      = 0b0000_0000_0000_0010;
     const Protected    = 0b0000_0000_0000_0100;
     const Static       = 0b0000_0000_0000_1000;
     const Final        = 0b0000_0000_0010_0000;
     const Super        = 0b0000_0000_0100_0000;
     const Volatile     = 0b0000_0000_1000_0000;
     const Transient    = 0b0000_0001_0000_0000;
     const Synthetic    = 0b0000_0010_0000_0000;
     const Annotation   = 0b0000_0100_0000_0000;
     const Enum         = 0b0000_1000_0000_0000;
     const Interface    = 0b0001_0000_0000_0000;
     const Abstract     = 0b0010_0000_0000_0000;
     const Synchronized = 0b0100_0000_0000_0000;
     const Deprecated   = 0b1000_0000_0000_0000;
   }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Method {
    pub access: Access,
    pub name: Option<MyString>,
    pub parameters: Vec<Parameter>,
    pub throws: Vec<JType>,
    pub ret: JType,
    /// When None then it is in the class
    pub source: Option<MyString>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Field {
    pub access: Access,
    pub name: MyString,
    pub jtype: JType,
    /// When None then it is in the class
    pub source: Option<MyString>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Parameter {
    pub name: Option<MyString>,
    pub jtype: JType,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
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
    Array(Box<Self>),
    Generic(MyString, Vec<Self>),
    Parameter(MyString),
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
            Self::Class(c) => {
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
        }
    }
}
