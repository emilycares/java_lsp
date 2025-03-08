use tree_sitter::Tree;
use tree_sitter_util::CommentSkiper;

use crate::Document;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ImportUnit<'a> {
    Package(&'a str),
    Class(&'a str),
    StaticClass(&'a str),
    StaticClassMethod(&'a str, &'a str),
    Prefix(&'a str),
    StaticPrefix(&'a str),
}

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
    return get_imported_classpaths(bytes, tree);
}

#[allow(dead_code)]
fn get_imported_classpaths<'a>(bytes: &'a [u8], tree: &Tree) -> Vec<ImportUnit<'a>> {
    let mut out = vec![];
    let mut cursor = tree.walk();
    cursor.first_child();

    cursor.first_child();
    cursor.sibling();
    let package = cursor.node().utf8_text(bytes).unwrap_or_default();
    out.push(ImportUnit::Package(package));
    cursor.parent();

    cursor.sibling();
    while let "import_declaration" = cursor.node().kind() {
        cursor.first_child();
        cursor.sibling();
        let mut stat = false;
        let mut prefix = false;
        if cursor.node().kind() == "static" {
            stat = true;
            cursor.sibling();
        }

        // skip import when not correctly formated
        if cursor.node().kind() == "scoped_identifier" {
            let class_path = cursor.node().utf8_text(bytes).unwrap_or_default();
            if cursor.sibling() {
                if cursor.node().kind() == "." {
                    cursor.sibling();
                }
                if cursor.node().kind() == "asterisk" {
                    prefix = true;
                }
            }

            let imp = match (stat, prefix) {
                (true, true) => ImportUnit::StaticPrefix(class_path),
                (true, false) => match class_path.rsplit_once(".") {
                    Some((class, method)) => {
                        match method.chars().next().unwrap_or_default().is_lowercase() {
                            true => ImportUnit::StaticClassMethod(class, method),
                            false => ImportUnit::StaticClass(class_path),
                        }
                    }
                    None => ImportUnit::StaticClass(class_path),
                },
                (false, true) => ImportUnit::Prefix(class_path),
                (false, false) => ImportUnit::Class(class_path),
            };
            out.push(imp);
        }
        cursor.parent();
        cursor.sibling();
    }

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
                ImportUnit::Package("heh.haha"),
                ImportUnit::Class("java.util.List"),
                ImportUnit::Class("java.util.stream.Collectors"),
                ImportUnit::StaticClass("org.junit.jupiter.api.Assertions"),
                ImportUnit::StaticClassMethod("org.junit.jupiter.api.Assertions", "assertEquals")
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
                ImportUnit::Package("heh.haha"),
                ImportUnit::Prefix("java.util"),
                ImportUnit::StaticPrefix("java.util")
            ]
        );
    }
}
