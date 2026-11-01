#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(clippy::perf)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::enum_glob_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
//! Ast to dto types
use dto::{Access, ImportUnit, JType};

use ast::types::{AstAvailability, AstImport, AstImportUnit, AstJType, AstJTypeKind};

#[must_use]
/// impl manual into
pub fn ast_import_to_import_unit(value: AstImport) -> ImportUnit {
    match value.unit {
        AstImportUnit::Class(ast_identifier) => ImportUnit::Class(ast_identifier.into()),
        AstImportUnit::StaticClass(ast_identifier) => {
            ImportUnit::StaticClass(ast_identifier.into())
        }
        AstImportUnit::StaticClassMethod(ast_identifier, ast_identifier1) => {
            ImportUnit::StaticClassMethod(ast_identifier.into(), ast_identifier1.into())
        }
        AstImportUnit::Prefix(ast_identifier) => ImportUnit::Prefix(ast_identifier.into()),
        AstImportUnit::StaticPrefix(ast_identifier) => {
            ImportUnit::StaticPrefix(ast_identifier.into())
        }
    }
}

#[must_use]
/// impl manual into
pub fn ast_import_ref_to_import_unit(value: &AstImport) -> ImportUnit {
    match &value.unit {
        AstImportUnit::Class(ast_identifier) => ImportUnit::Class(ast_identifier.into()),
        AstImportUnit::StaticClass(ast_identifier) => {
            ImportUnit::StaticClass(ast_identifier.into())
        }
        AstImportUnit::StaticClassMethod(ast_identifier, ast_identifier1) => {
            ImportUnit::StaticClassMethod(ast_identifier.into(), ast_identifier1.into())
        }
        AstImportUnit::Prefix(ast_identifier) => ImportUnit::Prefix(ast_identifier.into()),
        AstImportUnit::StaticPrefix(ast_identifier) => {
            ImportUnit::StaticPrefix(ast_identifier.into())
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

#[must_use]
/// impl manual into
pub fn ast_jtype_ref_to_jtype(value: &AstJType) -> JType {
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
        AstJTypeKind::ClassOrPackage(ast_identifier) => {
            JType::ClassOrPackage(ast_identifier.into())
        }
        AstJTypeKind::Array(ast_jtype) => {
            JType::Array(Box::new(ast_jtype_ref_to_jtype(ast_jtype.as_ref())))
        }
        AstJTypeKind::Generic(ast_identifier, ast_jtypes) => JType::Generic(
            ast_identifier.into(),
            ast_jtypes.iter().map(ast_jtype_ref_to_jtype).collect(),
        ),
        AstJTypeKind::Var => JType::Var,
        AstJTypeKind::Access { base, inner } => JType::Access {
            base: Box::new(ast_jtype_ref_to_jtype(base)),
            inner: Box::new(ast_jtype_ref_to_jtype(inner)),
        },
        AstJTypeKind::WildcardImplements(ast_jtype)
        | AstJTypeKind::WildcardExtends(ast_jtype)
        | AstJTypeKind::WildcardSuper(ast_jtype) => ast_jtype_ref_to_jtype(ast_jtype),
    }
}

#[must_use]
/// impl manual into
pub fn ast_jtype_to_jtype(value: AstJType) -> JType {
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
        AstJTypeKind::ClassOrPackage(ast_identifier) => {
            JType::ClassOrPackage(ast_identifier.into())
        }
        AstJTypeKind::Array(ast_jtype) => {
            JType::Array(Box::new(ast_jtype_ref_to_jtype(ast_jtype.as_ref())))
        }
        AstJTypeKind::Generic(ast_identifier, ast_jtypes) => JType::Generic(
            ast_identifier.into(),
            ast_jtypes.iter().map(ast_jtype_ref_to_jtype).collect(),
        ),
        AstJTypeKind::Var => JType::Var,
        AstJTypeKind::Access { base, inner } => JType::Access {
            base: Box::new(ast_jtype_ref_to_jtype(base.as_ref())),
            inner: Box::new(ast_jtype_ref_to_jtype(inner.as_ref())),
        },
        AstJTypeKind::WildcardImplements(ast_jtype)
        | AstJTypeKind::WildcardExtends(ast_jtype)
        | AstJTypeKind::WildcardSuper(ast_jtype) => ast_jtype_ref_to_jtype(ast_jtype.as_ref()),
    }
}

#[cfg(test)]
mod tests {

    use dto::JType;
    use my_string::NuVec;

    use ast::types::{AstIdentifier, AstJType, AstJTypeKind, AstPoint, AstRange};

    use crate::ast_jtype_ref_to_jtype;

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
                    value: NuVec::new_static(b"IntFunction"),
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
                            value: NuVec::new_static(b"U"),
                        }),
                    },
                ],
            ),
        };
        let out: JType = ast_jtype_ref_to_jtype(&inp);
        assert_eq!(
            JType::Generic(
                NuVec::new_static(b"IntFunction"),
                vec![JType::Wildcard, JType::Class(NuVec::new_static(b"U"))]
            ),
            out
        );
    }
}
