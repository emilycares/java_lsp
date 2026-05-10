#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use ast::types::{AstFile, AstTopLevel};
use dto::ImportUnit;

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

#[must_use]
pub fn imports(ast: &AstFile) -> Vec<ImportUnit> {
    let mut out = vec![];
    for t in &ast.top {
        match t {
            AstTopLevel::Package(package) => {
                out.push(ImportUnit::Package(package.name.value.clone()));
            }
            AstTopLevel::Import(ast_import) => {
                out.push(ast_import.into());
            }
            AstTopLevel::Thing(_) | AstTopLevel::Module(_) => (),
        }
    }
    out
}
