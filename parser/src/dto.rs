use std::fmt::Display;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tree_sitter::LanguageError;

#[derive(Error, Debug)]
pub enum ClassError {
    #[error("IO error")]
    IO(#[from] std::io::Error),
    #[error("Treesitter language error")]
    Language(#[from] LanguageError),
    #[error("java_asm error")]
    Asm(),
    #[error("unknown")]
    Unknown,
    #[error("Parse error")]
    ParseError,
    #[error("Serialize error")]
    SerializeError(#[from] postcard::Error),
    #[error("Could not find the class name")]
    UnknownClassName,
    #[error("Could not find the class path")]
    UnknownClassPath,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct ClassFolder {
    pub classes: Vec<Class>,
}

impl ClassFolder {
    pub fn new(classes: Vec<Class>) -> Self {
        Self { classes }
    }

    pub fn append(&mut self, other: Self) {
        self.classes.extend(other.classes);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Class {
    pub class_path: String,
    pub source: String,
    pub access: Vec<Access>,
    pub imports: Vec<ImportUnit>,
    pub name: String,
    pub methods: Vec<Method>,
    pub fields: Vec<Field>,
    pub super_class: SuperClass,
}
impl Class {
    pub fn no_imports(mut self) -> Self {
        self.imports = vec![];
        self
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub enum SuperClass {
    #[default]
    None,
    Name(String),
    ClassPath(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImportUnit {
    Package(String),
    Class(String),
    StaticClass(String),
    StaticClassMethod(String, String),
    Prefix(String),
    StaticPrefix(String),
}
impl ImportUnit {
    pub fn class_path_get_class_name(class_path: &str) -> Option<&str> {
        if let Some((_, c)) = class_path.rsplit_once(".") {
            return Some(c);
        }
        None
    }
    pub fn class_path_match_class_name(class_path: &str, name: &str) -> bool {
        ImportUnit::class_path_get_class_name(class_path)
            .iter()
            .any(|i| *i == name)
    }
    pub fn get_imported_class_package(&self, name: &str) -> Option<String> {
        match self {
            ImportUnit::Class(class_path) | ImportUnit::StaticClass(class_path) => {
                if Self::class_path_match_class_name(class_path, name) {
                    return Some(class_path.clone());
                }
                None
            }
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum SourceKind {
    Jdk(String),
    Maven(String),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Access {
    Public,
    Private,
    Protected,
    Static,
    Final,
    Super,
    Volatile,
    Transient,
    Synthetic,
    Annotation,
    Enum,
    Interface,
    Abstract,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Method {
    pub access: Vec<Access>,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub throws: Vec<JType>,
    pub ret: JType,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Field {
    pub access: Vec<Access>,
    pub name: String,
    pub jtype: JType,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Parameter {
    pub name: Option<String>,
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
    Class(String),
    Array(Box<JType>),
    Generic(String, Vec<JType>),
}

impl Display for JType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JType::Void => write!(f, ""),
            JType::Byte => write!(f, "byte"),
            JType::Char => write!(f, "char"),
            JType::Double => write!(f, "double"),
            JType::Float => write!(f, "float"),
            JType::Int => write!(f, "int"),
            JType::Long => write!(f, "long"),
            JType::Short => write!(f, "short"),
            JType::Boolean => write!(f, "boolean"),
            JType::Class(c) => {
                if c.starts_with("java.lang.") {
                    return write!(f, "{}", c.trim_start_matches("java.lang."));
                }
                write!(f, "{}", c)
            }
            JType::Array(i) => write!(f, "{}[]", i),
            JType::Generic(class, vec) => {
                let v = vec
                    .iter()
                    .map(|i| format!("{}", i))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{}<{}>", class, v)
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use super::JType;

    #[test]
    fn ser() {
        let inp = JType::Void;
        let ser: Vec<u8> = postcard::to_allocvec(&inp).unwrap();
        let out: JType = postcard::from_bytes(ser.deref()).unwrap();

        assert_eq!(inp, out);
    }
}
