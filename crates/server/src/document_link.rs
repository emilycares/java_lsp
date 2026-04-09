use std::path::MAIN_SEPARATOR;

use document::Document;
use lsp_extra::{source_to_uri, to_lsp_range};
use lsp_types::{DocumentLink, Uri};
use position::PositionSymbol;

pub fn get_document_link(uri: &Uri, document: &Document) -> Option<Vec<DocumentLink>> {
    const TEST_JAVA: &str = "Test.java";
    const JAVA: &str = ".java";
    let mut symbols = vec![];
    position::get_class_position_ast(&document.ast, None, &mut symbols);
    let PositionSymbol { range, .. } = symbols.first()?;

    let path = uri.path().as_str();
    let is_test = path.ends_with(TEST_JAVA);
    let target = if is_test {
        let Ok(t) = source_to_uri(
            &path
                .replacen(
                    &format!("src{MAIN_SEPARATOR}test{MAIN_SEPARATOR}java"),
                    &format!("src{MAIN_SEPARATOR}main{MAIN_SEPARATOR}java"),
                    1,
                )
                .replacen(TEST_JAVA, JAVA, 1),
        ) else {
            return None;
        };
        t
    } else {
        let Ok(t) = source_to_uri(
            &path
                .replacen(
                    &format!("src{MAIN_SEPARATOR}main{MAIN_SEPARATOR}java"),
                    &format!("src{MAIN_SEPARATOR}test{MAIN_SEPARATOR}java"),
                    1,
                )
                .replacen(JAVA, TEST_JAVA, 1),
        ) else {
            return None;
        };
        t
    };

    let tooltip = if is_test {
        "To Implementation"
    } else {
        "To Test"
    }
    .to_string();
    Some(vec![DocumentLink {
        range: to_lsp_range(range).ok()?,
        target: Some(target),
        tooltip: Some(tooltip),
        data: None,
    }])
}
#[cfg(test)]
mod tests {
    use lsp_types::{Position, Range};

    use super::*;
    use std::{path::PathBuf, str::FromStr};

    #[test]
    fn to_test() {
        let cont = r#"
package ch.emilycares;
public class Thing {}
        "#;
        let document = Document::setup(cont, PathBuf::from_str("/Thing.java").unwrap()).unwrap();
        let s = format!("file:///src{MAIN_SEPARATOR}test{MAIN_SEPARATOR}java/Thing.java",);
        let uri = Uri::from_str(&s).unwrap();
        let out = get_document_link(&uri, &document).unwrap();
        let out = out.first().unwrap();
        assert_eq!(
            out.range,
            Range {
                start: Position {
                    line: 2,
                    character: 13
                },
                end: Position {
                    line: 2,
                    character: 18
                }
            }
        );
        assert_eq!(
            out.target
                .as_ref()
                .unwrap()
                .path()
                .as_str()
                .replace("\\", "/"),
            "/src/test/java/ThingTest.java"
        );
        assert_eq!(out.tooltip, Some("To Test".to_string()));
    }

    #[test]
    fn to_impl() {
        let cont = r#"
package ch.emilycares;
public class ThingTest {}
        "#;
        let document =
            Document::setup(cont, PathBuf::from_str("/ThingTest.java").unwrap()).unwrap();
        let s = format!("file:///src{MAIN_SEPARATOR}test{MAIN_SEPARATOR}java/ThingTest.java",);
        let uri = Uri::from_str(&s).unwrap();
        let out = get_document_link(&uri, &document).unwrap();
        let out = out.first().unwrap();
        assert_eq!(
            out.range,
            Range {
                start: Position {
                    line: 2,
                    character: 13
                },
                end: Position {
                    line: 2,
                    character: 22
                }
            }
        );
        assert_eq!(
            out.target
                .as_ref()
                .unwrap()
                .path()
                .as_str()
                .replace("\\", "/"),
            "/src/main/java/Thing.java"
        );
        assert_eq!(out.tooltip, Some("To Implementation".to_string()));
    }
}
