use tree_sitter::{Parser, Tree};
use tree_sitter_util::CommentSkiper;

use crate::Document;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ImportUnit<'a> {
    Class(&'a str),
    StaticClass(&'a str),
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
        }
    }
    false
}

#[allow(dead_code)]
fn get_tree(content: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    let language = tree_sitter_java::LANGUAGE;
    parser
        .set_language(&language.into())
        .expect("Error loading java grammar");
    let tree = parser.parse(content, None)?;
    Some(tree)
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
    cursor.sibling();
    while let "import_declaration" = cursor.node().kind() {
        cursor.first_child();
        cursor.sibling();
        let mut done = false;
        let mut stat = false;
        if cursor.node().kind() == "static" {
            stat = true;
            cursor.sibling();
        }

        // skip import when not correctly formated
        if cursor.node().kind() == "scoped_identifier" {
            let class_path = cursor.node().utf8_text(bytes).unwrap();
            if cursor.sibling() {
                if cursor.node().kind() == "." {
                    cursor.sibling();
                }
                if cursor.node().kind() == "asterisk" {
                    if stat {
                        out.push(ImportUnit::StaticPrefix(class_path));
                    } else {
                        out.push(ImportUnit::Prefix(class_path));
                    }

                    done = true;
                }
            }
            if !done {
                if stat {
                    out.push(ImportUnit::StaticClass(class_path));
                } else {
                    out.push(ImportUnit::Class(class_path));
                }
            }
        }
        cursor.parent();
        cursor.sibling();
    }

    out
}

#[cfg(test)]
mod tests {
    use crate::imports::{get_tree, ImportUnit};

    use super::get_imported_classpaths;

    #[test]
    fn classes() {
        let demo = "package heh.haha;

import java.util.List;
import java.util.stream.Collectors;
import static org.junit.jupiter.api.Assertions;

public class Controller {}";
        assert_eq!(
            get_imported_classpaths(demo.as_bytes(), &get_tree(demo).unwrap()),
            vec![
                ImportUnit::Class("java.util.List"),
                ImportUnit::Class("java.util.stream.Collectors"),
                ImportUnit::StaticClass("org.junit.jupiter.api.Assertions")
            ]
        );
    }

    #[test]
    fn star() {
        let demo = "package heh.haha;

import java.util.*;

public class Controller {}";
        assert_eq!(
            get_imported_classpaths(demo.as_bytes(), &get_tree(demo).unwrap()),
            vec![ImportUnit::Prefix("java.util")]
        );
    }
}
