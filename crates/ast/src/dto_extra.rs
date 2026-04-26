#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::enum_glob_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
//! Ast to dto types
use dto::{Access, ImportUnit, JType};

use crate::types::{AstAvailability, AstImport, AstImportUnit, AstJType, AstJTypeKind};

impl From<AstImport> for ImportUnit {
    fn from(value: AstImport) -> Self {
        match value.unit {
            AstImportUnit::Class(ast_identifier) => Self::Class(ast_identifier.into()),
            AstImportUnit::StaticClass(ast_identifier) => Self::StaticClass(ast_identifier.into()),
            AstImportUnit::StaticClassMethod(ast_identifier, ast_identifier1) => {
                Self::StaticClassMethod(ast_identifier.into(), ast_identifier1.into())
            }
            AstImportUnit::Prefix(ast_identifier) => Self::Prefix(ast_identifier.into()),
            AstImportUnit::StaticPrefix(ast_identifier) => {
                Self::StaticPrefix(ast_identifier.into())
            }
        }
    }
}

impl From<&AstImport> for ImportUnit {
    fn from(value: &AstImport) -> Self {
        match &value.unit {
            AstImportUnit::Class(ast_identifier) => Self::Class(ast_identifier.into()),
            AstImportUnit::StaticClass(ast_identifier) => Self::StaticClass(ast_identifier.into()),
            AstImportUnit::StaticClassMethod(ast_identifier, ast_identifier1) => {
                Self::StaticClassMethod(ast_identifier.into(), ast_identifier1.into())
            }
            AstImportUnit::Prefix(ast_identifier) => Self::Prefix(ast_identifier.into()),
            AstImportUnit::StaticPrefix(ast_identifier) => {
                Self::StaticPrefix(ast_identifier.into())
            }
        }
    }
}

#[must_use]
/// `dto::Access` from `AstAvailability`
pub fn access_from_availability(value: &AstAvailability, def: Access) -> Access {
    let mut out = Access::empty();

    if value.intersects(AstAvailability::Public) {
        out.insert(Access::Public);
    }
    if value.intersects(AstAvailability::Private) {
        out.insert(Access::Private);
    }
    if value.intersects(AstAvailability::Protected) {
        out.insert(Access::Protected);
    }
    if !value
        .intersects(AstAvailability::Public | AstAvailability::Private | AstAvailability::Protected)
    {
        out.insert(def);
    }

    if value.intersects(AstAvailability::Synchronized) {
        out.insert(Access::Synchronized);
    }
    if value.intersects(AstAvailability::Final) {
        out.insert(Access::Final);
    }
    if value.intersects(AstAvailability::Static) {
        out.insert(Access::Static);
    }
    out
}

impl From<&AstJType> for JType {
    fn from(value: &AstJType) -> Self {
        match &value.value {
            AstJTypeKind::Void => Self::Void,
            AstJTypeKind::Byte => Self::Byte,
            AstJTypeKind::Char => Self::Char,
            AstJTypeKind::Double => Self::Double,
            AstJTypeKind::Float => Self::Float,
            AstJTypeKind::Int => Self::Int,
            AstJTypeKind::Long => Self::Long,
            AstJTypeKind::Short => Self::Short,
            AstJTypeKind::Boolean => Self::Boolean,
            AstJTypeKind::Wildcard => Self::Wildcard,
            AstJTypeKind::Class(ast_identifier) => Self::Class(ast_identifier.into()),
            AstJTypeKind::Array(ast_jtype) => Self::Array(Box::new(ast_jtype.as_ref().into())),
            AstJTypeKind::Generic(ast_identifier, ast_jtypes) => Self::Generic(
                ast_identifier.into(),
                ast_jtypes.iter().map(Into::into).collect(),
            ),
            AstJTypeKind::Var => Self::Var,
            AstJTypeKind::Access { base, inner } => Self::Access {
                base: Box::new((&**base).into()),
                inner: Box::new((&**inner).into()),
            },
        }
    }
}

impl PartialEq<AstJType> for JType {
    fn eq(&self, other: &AstJType) -> bool {
        Into::<Self>::into(other) == *self
    }
}

#[cfg(test)]
mod tests {

    use dto::JType;
    use my_string::smol_str::SmolStr;

    use crate::types::{AstIdentifier, AstJType, AstJTypeKind, AstPoint, AstRange};

    #[test]
    fn jtype_map() {
        let inp = AstJType {
            annotated: Vec::new(),
            range: AstRange::default(),
            value: AstJTypeKind::Generic(
                AstIdentifier {
                    range: AstRange {
                        start: AstPoint { line: 6, col: 27 },
                        end: AstPoint { line: 6, col: 38 },
                    },
                    value: SmolStr::new_inline("IntFunction"),
                },
                vec![
                    AstJType {
                        annotated: Vec::new(),
                        range: AstRange {
                            start: AstPoint { line: 6, col: 39 },
                            end: AstPoint { line: 6, col: 50 },
                        },
                        value: AstJTypeKind::Wildcard,
                    },
                    AstJType {
                        annotated: Vec::new(),
                        range: AstRange {
                            start: AstPoint { line: 6, col: 49 },
                            end: AstPoint { line: 6, col: 50 },
                        },
                        value: AstJTypeKind::Class(AstIdentifier {
                            range: AstRange {
                                start: AstPoint { line: 6, col: 49 },
                                end: AstPoint { line: 6, col: 50 },
                            },
                            value: SmolStr::new_inline("U"),
                        }),
                    },
                ],
            ),
        };
        let out: JType = (&inp).into();
        assert_eq!(
            JType::Generic(
                SmolStr::new_inline("IntFunction"),
                vec![JType::Wildcard, JType::Class(SmolStr::new_inline("U"))]
            ),
            out
        );
    }
}
