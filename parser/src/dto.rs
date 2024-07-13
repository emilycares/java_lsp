use thiserror::Error;
use tree_sitter::LanguageError;

#[derive(Error, Debug)]
pub enum ClassError {
    #[error("IO error")]
    IO(#[from] std::io::Error),
    #[error("Treesitter language error")]
    Language(#[from] LanguageError),
    #[error("unknown")]
    Unknown,
    #[error("Parse error")]
    ParseError,
}

#[derive(Debug, PartialEq)]
pub struct Class {
    pub access: Vec<Access>,
    pub name: String,
    pub methods: Vec<Method>,
}

#[derive(Debug, PartialEq, Clone)]
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
    Enum,
    Interface,
    Abstract,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub struct Method {
    pub access: Vec<Access>,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub ret: JType,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub jtype: JType,
}

#[derive(Debug, PartialEq, Clone)]
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
