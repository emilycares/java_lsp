use tree_sitter::{Parser, Tree};
use tree_sitter_util::CommentSkiper;

use crate::Document;

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
pub fn imports(document: &Document) -> Vec<&str> {
    let tree = &document.tree;
    let bytes = document
        .text
        .slice(..)
        .as_str()
        .unwrap_or_default()
        .as_bytes();
    return get_imported_classpaths(bytes, tree);
}

#[allow(dead_code)]
fn get_imported_classpaths<'a>(content: &'a [u8], tree: &Tree) -> Vec<&'a str> {
    let mut out = vec![];
    let mut cursor = tree.walk();
    cursor.first_child();
    cursor.sibling();
    while let "import_declaration" = cursor.node().kind() {
        cursor.first_child();
        cursor.sibling();

        // skip import when not correctly formated
        if cursor.node().kind() == "scoped_identifier" {
            let class_path = cursor.node().utf8_text(content).unwrap();
            if class_path.contains('{') {
                unimplemented!();
            }
            if class_path.contains('*') {
                unimplemented!();
            }

            out.push(class_path);
        }
        cursor.parent();
        cursor.sibling();
    }

    out
}

#[cfg(test)]
mod tests {
    use crate::imports::get_tree;

    use super::get_imported_classpaths;

    #[test]
    fn classes() {
        let demo = "package heh.haha;

import java.util.List;
import java.util.stream.Collectors;

public class Controller {}";
        assert_eq!(
            get_imported_classpaths(demo.as_bytes(), &get_tree(demo).unwrap()),
            vec!["java.util.List", "java.util.stream.Collectors"]
        );
    }
}
