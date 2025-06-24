use std::collections::HashMap;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Position, Range, TextEdit, Uri, WorkspaceEdit,
};
use parser::{
    dto::{self, ImportUnit},
    java::parse_jtype,
};
use tree_sitter::{Point, Tree};
use tree_sitter_util::{CommentSkiper, lsp::to_lsp_position};
use tyres::TyresError;
use variables::LocalVariable;

pub struct CodeActionContext<'a> {
    pub point: &'a Point,
    pub imports: &'a [ImportUnit],
    pub class_map: &'a dashmap::DashMap<String, parser::dto::Class>,
    pub class: &'a dto::Class,
    pub vars: &'a [LocalVariable],
    pub current_file: &'a Uri,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum CodeActionError {
    NoCallCain,
    ParseJava(parser::java::ParseJavaError),
    Tyres(TyresError),
}

pub fn replace_with_value_type(
    tree: &tree_sitter::Tree,
    bytes: &[u8],
    context: &CodeActionContext,
) -> Result<Option<CodeActionOrCommand>, CodeActionError> {
    let Ok(node) =
        tree_sitter_util::digg_until_kind(tree, *context.point, "local_variable_declaration")
    else {
        return Ok(None);
    };
    let mut cursor = node.walk();
    cursor.first_child();
    let current_type_range = cursor.node().range();
    let current_type =
        parse_jtype(&cursor.node(), bytes, &vec![]).map_err(CodeActionError::ParseJava)?;
    cursor.sibling();
    cursor.first_child();
    cursor.sibling();
    cursor.sibling();
    let mut point = cursor.node().range().end_point;
    point.column -= 1;
    // value here
    let Some(call_chain) = call_chain::get_call_chain(tree, bytes, &point) else {
        return Err(CodeActionError::NoCallCain);
    };
    let value_resolve_state = tyres::resolve_call_chain_value(
        &call_chain,
        context.vars,
        context.imports,
        context.class,
        context.class_map,
    )
    .map_err(CodeActionError::Tyres)?;

    if current_type != value_resolve_state.jtype {
        // Required by lsp types
        #[allow(clippy::mutable_key_type)]
        let mut changes = HashMap::new();
        changes.insert(
            context.current_file.to_owned(),
            vec![TextEdit {
                range: tree_sitter_util::lsp::to_lsp_range(current_type_range),
                new_text: value_resolve_state.class.name.clone(),
            }],
        );
        let action = CodeActionOrCommand::CodeAction(CodeAction {
            kind: Some(CodeActionKind::QUICKFIX),
            title: format!("Replace variable type with: {}", value_resolve_state.jtype),
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }),
            ..Default::default()
        });
        return Ok(Some(action));
    }

    Ok(None)
}

pub fn import_jtype(
    tree: &Tree,
    bytes: &[u8],
    context: &CodeActionContext,
) -> Option<Vec<CodeActionOrCommand>> {
    if let Ok(n) = tree_sitter_util::get_node_at_point(tree, *context.point) {
        if n.kind() == "type_identifier" {
            if let Ok(jtype) = n.utf8_text(bytes) {
                if !tyres::is_imported_class_name(jtype, context.imports, context.class_map) {
                    let i = tyres::resolve_import(jtype, context.class_map)
                        .iter()
                        .map(|a| import_to_code_action(context.current_file, a, tree))
                        .collect();
                    return Some(i);
                }
            }
        }
    }
    None
}

pub fn get_import_position(tree: &Tree) -> Option<Position> {
    let mut cursor = tree.walk();
    cursor.first_child();
    cursor.sibling();
    if cursor.node().kind() == "import_declaration" {
        return Some(to_lsp_position(cursor.node().end_position()));
    }
    None
}
pub fn import_text_edit(classpath: &str, tree: &Tree) -> Vec<TextEdit> {
    let mut pos = Position {
        line: 2,
        character: 0,
    };
    if let Some(npos) = get_import_position(tree) {
        pos = npos;
    }

    vec![TextEdit {
        range: Range::new(pos, pos),
        new_text: format!("\nimport {};", classpath),
    }]
}
pub fn import_to_code_action(
    current_file: &Uri,
    classpath: &str,
    tree: &Tree,
) -> CodeActionOrCommand {
    // Required by lsp types
    #[allow(clippy::mutable_key_type)]
    let mut changes = HashMap::new();
    changes.insert(current_file.to_owned(), import_text_edit(classpath, tree));
    CodeActionOrCommand::CodeAction(CodeAction {
        kind: Some(CodeActionKind::QUICKFIX),
        title: format!("Import {}", classpath),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }),
        ..Default::default()
    })
}
#[cfg(test)]
pub mod tests {
    use std::{path::PathBuf, str::FromStr};

    use dashmap::DashMap;
    use document::Document;
    use lsp_types::Uri;
    use parser::dto::{self, ImportUnit};
    use pretty_assertions::assert_eq;
    use tree_sitter::Point;

    use crate::codeaction::replace_with_value_type;

    use super::CodeActionContext;

    #[test]
    fn replace_type_base() {
        let content = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        int local = "";
    }
}
        "#;
        let point = Point::new(4, 10);
        let doc = Document::setup(
            content,
            PathBuf::from_str("./").unwrap(),
            "ch.emilycares.Test".to_string(),
        )
        .unwrap();
        let imports = vec![];
        let class = parser::java::load_java_tree(
            content.as_bytes(),
            parser::loader::SourceDestination::None,
            &doc.tree,
        )
        .unwrap();
        let uri = Uri::from_str("file:///a").unwrap();
        let context = CodeActionContext {
            point: &point,
            imports: &imports,
            class_map: &get_class_map(),
            class: &class,
            vars: &variables::get_vars(&doc, &point).unwrap(),
            current_file: &uri,
        };
        let out = replace_with_value_type(&doc.tree, content.as_bytes(), &context);
        let result = out.unwrap().unwrap();
        match result {
            lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
                assert_eq!(code_action.title, "Replace variable type with: String");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn replace_type_with_argument() {
        let content = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        int local = new FileInputStream(new File(""));
    }
}
        "#;
        let point = Point::new(4, 10);
        let doc = Document::setup(
            content,
            PathBuf::from_str("./").unwrap(),
            "ch.emilycares.Test".to_string(),
        )
        .unwrap();
        let imports = vec![ImportUnit::Class("java.io.FileInputStream".to_string())];
        let class = parser::java::load_java_tree(
            content.as_bytes(),
            parser::loader::SourceDestination::None,
            &doc.tree,
        )
        .unwrap();
        let uri = Uri::from_str("file:///a").unwrap();
        let context = CodeActionContext {
            point: &point,
            imports: &imports,
            class_map: &get_class_map(),
            class: &class,
            vars: &variables::get_vars(&doc, &point).unwrap(),
            current_file: &uri,
        };
        let out = replace_with_value_type(&doc.tree, content.as_bytes(), &context);
        let result = out.unwrap().unwrap();
        match result {
            lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
                assert_eq!(
                    code_action.title,
                    "Replace variable type with: java.io.FileInputStream"
                );
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn replace_type_method() {
        let content = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        String a = "";
        String local = a.length();
    }
}
        "#;
        let point = Point::new(5, 10);
        let doc = Document::setup(
            content,
            PathBuf::from_str("./").unwrap(),
            "ch.emilycares.Test".to_string(),
        )
        .unwrap();
        let imports = vec![];
        let class = parser::java::load_java_tree(
            content.as_bytes(),
            parser::loader::SourceDestination::None,
            &doc.tree,
        )
        .unwrap();
        let uri = Uri::from_str("file:///a").unwrap();
        let context = CodeActionContext {
            point: &point,
            imports: &imports,
            class_map: &get_class_map(),
            class: &class,
            vars: &variables::get_vars(&doc, &point).unwrap(),
            current_file: &uri,
        };
        let out = replace_with_value_type(&doc.tree, content.as_bytes(), &context);
        let result = out.unwrap().unwrap();
        match result {
            lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
                assert_eq!(code_action.title, "Replace variable type with: int");
            }
            _ => unreachable!(),
        }
    }
    fn get_class_map() -> DashMap<String, dto::Class> {
        let class_map: DashMap<String, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".to_string(),
            dto::Class {
                access: vec![dto::Access::Public],
                name: "String".to_string(),
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "length".to_string(),
                    ret: dto::JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            "java.io.FileInputStream".to_string(),
            dto::Class {
                access: vec![dto::Access::Public],
                name: "FileInputStream".to_string(),
                methods: vec![],
                ..Default::default()
            },
        );
        class_map
    }
}
