use parser::{dto::ImportUnit, java::parse_import_declarations};
use tree_sitter::Tree;
use tree_sitter_util::CommentSkiper;

use crate::Document;

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
                if class_path.starts_with(p) {
                    return true;
                }
            }
            ImportUnit::StaticPrefix(p) => {
                if class_path.starts_with(p) {
                    return true;
                }
            }
            ImportUnit::Package(p) => {
                if class_path.starts_with(p) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn imports(document: &Document) -> Vec<ImportUnit> {
    let tree = &document.tree;
    let bytes = document.as_bytes();
    get_imported_classpaths(bytes, tree)
}

#[allow(dead_code)]
fn get_imported_classpaths(bytes: &[u8], tree: &Tree) -> Vec<ImportUnit> {
    let mut out = vec![];
    let mut cursor = tree.walk();
    cursor.first_child();

    cursor.first_child();
    cursor.sibling();
    let package = cursor
        .node()
        .utf8_text(bytes)
        .unwrap_or_default()
        .to_string();
    out.push(ImportUnit::Package(package));
    cursor.parent();

    cursor.sibling();
    out.extend(parse_import_declarations(bytes, &mut cursor));

    out
}

#[cfg(test)]
mod tests {
    use crate::imports::ImportUnit;
    use pretty_assertions::assert_eq;

    use super::get_imported_classpaths;

    #[test]
    fn classes() {
        let demo = "package heh.haha;

import java.util.List;
import java.util.stream.Collectors;
import static org.junit.jupiter.api.Assertions;
import static org.junit.jupiter.api.Assertions.assertEquals;
 
public class Controller {}";
        let (_, tree) = tree_sitter_util::parse(demo).unwrap();
        assert_eq!(
            get_imported_classpaths(demo.as_bytes(), &tree),
            vec![
                ImportUnit::Package("heh.haha".to_string()),
                ImportUnit::Class("java.util.List".to_string()),
                ImportUnit::Class("java.util.stream.Collectors".to_string()),
                ImportUnit::StaticClass("org.junit.jupiter.api.Assertions".to_string()),
                ImportUnit::StaticClassMethod(
                    "org.junit.jupiter.api.Assertions".to_string(),
                    "assertEquals".to_string()
                )
            ]
        );
    }

    #[test]
    fn stat() {
        let demo = "package heh.haha;

import java.util.*;
import static java.util.*;

public class Controller {}";
        let (_, tree) = tree_sitter_util::parse(demo).unwrap();
        assert_eq!(
            get_imported_classpaths(demo.as_bytes(), &tree),
            vec![
                ImportUnit::Package("heh.haha".to_string()),
                ImportUnit::Prefix("java.util".to_string()),
                ImportUnit::StaticPrefix("java.util".to_string())
            ]
        );
    }
}
