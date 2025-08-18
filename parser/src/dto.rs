use std::fmt::Display;

use ast::types::{AstAvailability, AstImport, AstImportUnit, AstJType, AstJTypeKind};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Debug)]
pub enum ClassError {
    IO(std::io::Error),
    Asm,
    Unknown,
    ParseError,
    Postcard(postcard::Error),
    UnknownClassName,
    UnknownClassPath,
    InvalidClassPath,
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
    pub class_path: SmolStr,
    pub source: SmolStr,
    pub access: Vec<Access>,
    pub imports: Vec<ImportUnit>,
    pub name: SmolStr,
    pub methods: Vec<Method>,
    pub fields: Vec<Field>,
    pub super_class: SuperClass,
    pub super_interfaces: Vec<SuperClass>,
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
    Name(SmolStr),
    ClassPath(SmolStr),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImportUnit {
    Package(SmolStr),
    Class(SmolStr),
    StaticClass(SmolStr),
    StaticClassMethod(SmolStr, SmolStr),
    Prefix(SmolStr),
    StaticPrefix(SmolStr),
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
    pub fn get_imported_class_package(&self, name: &str) -> Option<SmolStr> {
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

impl From<AstImport> for ImportUnit {
    fn from(value: AstImport) -> Self {
        match value.unit {
            AstImportUnit::Class(ast_identifier) => ImportUnit::Class(ast_identifier.into()),
            AstImportUnit::StaticClass(ast_identifier) => Self::StaticClass(ast_identifier.into()),
            AstImportUnit::StaticClassMethod(ast_identifier, ast_identifier1) => {
                Self::StaticClassMethod(ast_identifier.into(), ast_identifier1.into())
            }
            AstImportUnit::Prefix(ast_identifier) => ImportUnit::Prefix(ast_identifier.into()),
            AstImportUnit::StaticPrefix(ast_identifier) => {
                Self::StaticPrefix(ast_identifier.into())
            }
        }
    }
}

impl From<&AstImport> for ImportUnit {
    fn from(value: &AstImport) -> Self {
        match &value.unit {
            AstImportUnit::Class(ast_identifier) => ImportUnit::Class(ast_identifier.into()),
            AstImportUnit::StaticClass(ast_identifier) => Self::StaticClass(ast_identifier.into()),
            AstImportUnit::StaticClassMethod(ast_identifier, ast_identifier1) => {
                Self::StaticClassMethod(ast_identifier.into(), ast_identifier1.into())
            }
            AstImportUnit::Prefix(ast_identifier) => ImportUnit::Prefix(ast_identifier.into()),
            AstImportUnit::StaticPrefix(ast_identifier) => {
                Self::StaticPrefix(ast_identifier.into())
            }
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

impl Access {
    pub fn from(value: &AstAvailability, def: Access) -> Vec<Self> {
        match value {
            AstAvailability::Public => vec![Access::Public],
            AstAvailability::Private => vec![Access::Private],
            AstAvailability::Protected => vec![Access::Protected],
            AstAvailability::Undefined => vec![def],
            AstAvailability::PublicStatic => vec![Access::Static, Access::Public],
            AstAvailability::PrivateStatic => vec![Access::Static, Access::Private],
            AstAvailability::ProtectedStatic => vec![Access::Static, Access::Protected],
            AstAvailability::UndefinedStatic => vec![Access::Static, def],
            AstAvailability::PublicFinal => vec![Access::Public, Access::Final],
            AstAvailability::PublicStaticFinal => vec![Access::Static, Access::Final],
            AstAvailability::PrivateFinal => vec![Access::Private, Access::Final],
            AstAvailability::PrivateStaticFinal => {
                vec![Access::Private, Access::Static, Access::Final]
            }
            AstAvailability::ProtectedFinal => vec![Access::Protected, Access::Final],
            AstAvailability::ProtectedStaticFinal => {
                vec![Access::Protected, Access::Static, Access::Final]
            }
            AstAvailability::UndefinedFinal => vec![def, Access::Final],
            AstAvailability::UndefinedStaticFinal => vec![def, Access::Static, Access::Final],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Method {
    pub access: Vec<Access>,
    pub name: SmolStr,
    pub parameters: Vec<Parameter>,
    pub throws: Vec<JType>,
    pub ret: JType,
    /// When None then it is in the class
    pub source: Option<SmolStr>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Field {
    pub access: Vec<Access>,
    pub name: SmolStr,
    pub jtype: JType,
    /// When None then it is in the class
    pub source: Option<SmolStr>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Parameter {
    pub name: Option<SmolStr>,
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
    Class(SmolStr),
    Array(Box<JType>),
    Generic(SmolStr, Vec<JType>),
    Parameter(SmolStr),
    Var,
}
impl From<&AstJType> for JType {
    fn from(value: &AstJType) -> Self {
        match &value.value {
            AstJTypeKind::Void => JType::Void,
            AstJTypeKind::Byte => JType::Byte,
            AstJTypeKind::Char => JType::Char,
            AstJTypeKind::Double => JType::Double,
            AstJTypeKind::Float => JType::Float,
            AstJTypeKind::Int => JType::Int,
            AstJTypeKind::Long => JType::Long,
            AstJTypeKind::Short => JType::Short,
            AstJTypeKind::Boolean => JType::Boolean,
            AstJTypeKind::Wildcard => JType::Wildcard,
            AstJTypeKind::Class(ast_identifier) => JType::Class(ast_identifier.into()),
            AstJTypeKind::Array(ast_jtype) => JType::Array(Box::new(ast_jtype.as_ref().into())),
            AstJTypeKind::Generic(ast_identifier, ast_jtypes) => JType::Generic(
                ast_identifier.into(),
                ast_jtypes.iter().map(|i| i.into()).collect(),
            ),
            AstJTypeKind::Parameter(ast_identifier) => JType::Parameter(ast_identifier.into()),
            AstJTypeKind::Var => JType::Var,
        }
    }
}
impl From<AstJType> for JType {
    fn from(value: AstJType) -> Self {
        match value.value {
            AstJTypeKind::Void => JType::Void,
            AstJTypeKind::Byte => JType::Byte,
            AstJTypeKind::Char => JType::Char,
            AstJTypeKind::Double => JType::Double,
            AstJTypeKind::Float => JType::Float,
            AstJTypeKind::Int => JType::Int,
            AstJTypeKind::Long => JType::Long,
            AstJTypeKind::Short => JType::Short,
            AstJTypeKind::Boolean => JType::Boolean,
            AstJTypeKind::Wildcard => JType::Wildcard,
            AstJTypeKind::Class(ast_identifier) => JType::Class(ast_identifier.into()),
            AstJTypeKind::Array(ast_jtype) => JType::Array(Box::new(ast_jtype.as_ref().into())),
            AstJTypeKind::Generic(ast_identifier, ast_jtypes) => JType::Generic(
                ast_identifier.into(),
                ast_jtypes.iter().map(|i| i.into()).collect(),
            ),
            AstJTypeKind::Parameter(ast_identifier) => JType::Parameter(ast_identifier.into()),
            AstJTypeKind::Var => Self::Var,
        }
    }
}

impl Display for JType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JType::Void => write!(f, "void"),
            JType::Byte => write!(f, "byte"),
            JType::Char => write!(f, "char"),
            JType::Double => write!(f, "double"),
            JType::Float => write!(f, "float"),
            JType::Int => write!(f, "int"),
            JType::Long => write!(f, "long"),
            JType::Short => write!(f, "short"),
            JType::Boolean => write!(f, "boolean"),
            JType::Wildcard => write!(f, "?"),
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
            JType::Parameter(p) => write!(f, "<{}>", p),
            JType::Var => write!(f, "var"),
        }
    }
}

impl PartialEq<AstJType> for JType {
    fn eq(&self, other: &AstJType) -> bool {
        Into::<JType>::into(other) == *self
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use ast::types::{AstIdentifier, AstJType, AstJTypeKind, AstPoint, AstRange};

    use super::JType;

    #[test]
    fn ser() {
        let inp = JType::Void;
        let ser: Vec<u8> = postcard::to_allocvec(&inp).unwrap();
        let out: JType = postcard::from_bytes(ser.deref()).unwrap();

        assert_eq!(inp, out);
    }

    #[test]
    fn jtype_map() {
        let inp = AstJType {
            range: AstRange::default(),
            value: AstJTypeKind::Generic(
                AstIdentifier {
                    range: AstRange {
                        start: AstPoint { line: 6, col: 27 },
                        end: AstPoint { line: 6, col: 38 },
                    },
                    value: "IntFunction".into(),
                },
                vec![
                    AstJType {
                        range: AstRange {
                            start: AstPoint { line: 6, col: 39 },
                            end: AstPoint { line: 6, col: 50 },
                        },
                        value: AstJTypeKind::Wildcard,
                    },
                    AstJType {
                        range: AstRange {
                            start: AstPoint { line: 6, col: 49 },
                            end: AstPoint { line: 6, col: 50 },
                        },
                        value: AstJTypeKind::Class(AstIdentifier {
                            range: AstRange {
                                start: AstPoint { line: 6, col: 49 },
                                end: AstPoint { line: 6, col: 50 },
                            },
                            value: "U".into(),
                        }),
                    },
                ],
            ),
        };
        let out: JType = (&inp).into();
        assert_eq!(
            JType::Generic(
                "IntFunction".into(),
                vec![JType::Wildcard, JType::Class("U".into())]
            ),
            out
        );
    }
}
