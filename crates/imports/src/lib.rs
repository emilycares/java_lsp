#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use ast::types::AstFile;
use parser::dto::ImportUnit;

#[must_use]
pub fn is_imported(imports: &[ImportUnit], class_path: &str) -> bool {
    if class_path.starts_with("java.lang") {
        return true;
    }
    for inp in imports {
        match inp {
            ImportUnit::StaticClassMethod(c, _)
            | ImportUnit::Class(c)
            | ImportUnit::StaticClass(c) => {
                if *c == class_path {
                    return true;
                }
            }
            ImportUnit::Prefix(p) | ImportUnit::StaticPrefix(p) | ImportUnit::Package(p) => {
                if class_path.starts_with(p.as_str()) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn imports(ast: &AstFile) -> Vec<ImportUnit> {
    let mut out = vec![];
    if let Some(package) = &ast.package {
        out.push(ImportUnit::Package(package.name.value.clone()));
    }
    if let Some(imports) = &ast.imports {
        out.extend(imports.imports.iter().map(ImportUnit::from));
    }
    out
}
