use tree_sitter::Parser;
use tree_sitter_util::CommentSkiper;

pub fn get_classes_to_load<'a>(content: &'a str) -> Vec<&'a str> {
    let mut parser = Parser::new();
    let language = tree_sitter_java::language();
    parser
        .set_language(&language)
        .expect("Error loading java grammar");
    let Some(tree) = parser.parse(content, None) else {
        return vec![];
    };
    let mut out = vec![];
    let mut cursor = tree.walk();
    cursor.first_child();
    cursor.sibling();
    loop {
        match cursor.node().kind() {
            "import_declaration" => {
                cursor.first_child();
                cursor.sibling();

                //dbg!(cursor.node().kind());
                //dbg!(cursor.node().utf8_text(content.as_bytes()).unwrap());
                assert_eq!(cursor.node().kind(), "scoped_identifier");
                let class_path = cursor.node().utf8_text(content.as_bytes()).unwrap();
                if !class_path.contains('{') {
                    out.push(class_path);
                } else {
                    unimplemented!();
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
    use super::get_classes_to_load;

    #[test]
    fn classes() {
        let demo = "package heh.haha;

import java.util.List;
import java.util.stream.Collectors;

public class Controller {}";
        assert_eq!(
            get_classes_to_load(demo),
            vec!["java.util.List", "java.util.stream.Collectors"]
        );
    }
}
