#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
use document::Document;
use parser::dto::ImportUnit;

pub fn is_imported(imports: &[ImportUnit], class_path: &str) -> bool {
    for inp in imports {
        match inp {
            ImportUnit::Class(c) => {
                if *c == class_path {
                    return true;
                }
            }
            ImportUnit::StaticClass(c) => {
                if *c == class_path {
                    return true;
                }
            }
            ImportUnit::StaticClassMethod(c, _) => {
                if *c == class_path {
                    return true;
                }
            }
            ImportUnit::Prefix(p) => {
                if class_path.starts_with(p.as_str()) {
                    return true;
                }
            }
            ImportUnit::StaticPrefix(p) => {
                if class_path.starts_with(p.as_str()) {
                    return true;
                }
            }
            ImportUnit::Package(p) => {
                if class_path.starts_with(p.as_str()) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn imports(document: &Document) -> Vec<ImportUnit> {
    if let Some(imports) = &document.ast.imports {
        return imports.imports.iter().map(ImportUnit::from).collect();
    }
    vec![]
}
