use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClassError {
    #[error("IO error")]
    IO(#[from] std::io::Error),
    #[error("unknown")]
    Unknown,
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
    pub ret: crate::JType,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub jtype: crate::JType,
}
