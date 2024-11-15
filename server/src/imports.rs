use tree_sitter::{Parser, Tree};
use tree_sitter_util::CommentSkiper;

#[allow(dead_code)]
fn get_tree(content: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    let language = tree_sitter_java::language();
    parser
        .set_language(&language)
        .expect("Error loading java grammar");
    let Some(tree) = parser.parse(content, None) else {
        return None;
    };
    Some(tree)
}

pub fn get_imported_classpaths<'a>(content: &'a [u8], tree: &Tree) -> Vec<&'a str> {
    let mut out = vec![];
    let mut cursor = tree.walk();
    cursor.first_child();
    cursor.sibling();
    loop {
        match cursor.node().kind() {
            "import_declaration" => {
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
            _ => {
                break;
            }
        }
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
