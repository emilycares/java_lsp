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
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ClassFolder {
    pub classes: Vec<Class>,
}

impl ClassFolder {
    pub fn new() -> Self {
        Self {
            classes: vec![]
        }
    }

    pub fn append(&mut self, other: Self) {
        self.classes.extend(other.classes);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Class {
    pub class_path: String,
    pub access: Vec<Access>,
    pub name: String,
    pub methods: Vec<Method>,
    pub fields: Vec<Field>,
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Method {
    pub access: Vec<Access>,
    pub name: String,
    pub parameters: Vec<Parameter>,
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum JType {
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
            JType::Class(c) => write!(f, "{}", c),
            JType::Array(i) => write!(f, "{}[]", i),
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
