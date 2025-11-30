use std::fmt::Display;

use ast::types::{AstAvailability, AstImport, AstImportUnit, AstJType, AstJTypeKind};
use bitflags::bitflags;
use my_string::MyString;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum ClassError {
    IO(std::io::Error),
    Asm,
    Unknown,
    ParseError,
    // Postcard(postcard::Error),
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
    pub class_path: MyString,
    pub source: MyString,
    pub access: Access,
    pub imports: Vec<ImportUnit>,
    pub name: MyString,
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
    pub fn get_imported_class_package(&self, name: &str) -> Option<MyString> {
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

bitflags! {
   #[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
   pub struct Access: u16 {
     const Public       = 0b0000000000000001;
     const Private      = 0b0000000000000010;
     const Protected    = 0b0000000000000100;
     const Static       = 0b0000000000001000;
     const Final        = 0b0000000000100000;
     const Super        = 0b0000000001000000;
     const Volatile     = 0b0000000010000000;
     const Transient    = 0b0000000100000000;
     const Synthetic    = 0b0000001000000000;
     const Annotation   = 0b0000010000000000;
     const Enum         = 0b0000100000000000;
     const Interface    = 0b0001000000000000;
     const Abstract     = 0b0010000000000000;
     const Synchronized = 0b0100000000000000;
   }
}
impl Access {
    pub fn from(value: &AstAvailability, def: Access) -> Self {
        let mut out = Access::empty();

        if value.contains(AstAvailability::Public) {
            out.insert(Access::Public);
        }
        if value.contains(AstAvailability::Private) {
            out.insert(Access::Private);
        }
        if value.contains(AstAvailability::Protected) {
            out.insert(Access::Protected);
        }
        if !value.intersects(
            AstAvailability::Public | AstAvailability::Private | AstAvailability::Protected,
        ) {
            out.insert(def);
        }

        if value.contains(AstAvailability::Synchronized) {
            out.insert(Access::Synchronized);
        }
        if value.contains(AstAvailability::Final) {
            out.insert(Access::Final);
        }
        if value.contains(AstAvailability::Static) {
            out.insert(Access::Static);
        }
        out
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Method {
    pub access: Access,
    pub name: MyString,
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
    Array(Box<JType>),
    Generic(MyString, Vec<JType>),
    Parameter(MyString),
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
            AstJTypeKind::Access { base: _, inner: _ } => todo!(),
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
