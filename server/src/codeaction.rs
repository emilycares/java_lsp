use std::collections::HashMap;

use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Position, Range, TextEdit, Url, WorkspaceEdit,
};
use tree_sitter::{Point, Tree};

use crate::tyres;

pub fn import_jtype<'a>(
    tree: &Tree,
    bytes: &'a [u8],
    point: Point,
    imports: &Vec<&str>,
    current_file: &Url,
    class_map: &'a dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<Vec<CodeActionOrCommand>> {
    if let Ok(n) = tree_sitter_util::get_node_at_point(tree, point) {
        if n.kind() == "type_identifier" {
            if let Ok(jtype) = n.utf8_text(bytes) {
                if !tyres::is_imported(jtype, &imports) {
                    let i = tyres::resolve_import(jtype, class_map)
                        .iter()
                        .map(|a| import_to_code_action(current_file, a))
                        .collect();
                    return Some(i);
                }
            }
        }
    }
    None
}
fn import_to_code_action(current_file: &Url, classpath: &str) -> CodeActionOrCommand {
    let mut changes = HashMap::new();
    changes.insert(
        current_file.to_owned(),
        vec![TextEdit {
            range: Range::new(
                Position {
                    line: 2,
                    character: 0,
                },
                Position {
                    line: 2,
                    character: 0,
                },
            ),
            new_text: format!("import {};\n", classpath),
        }],
    );
    CodeActionOrCommand::CodeAction(CodeAction {
        kind: Some(CodeActionKind::QUICKFIX),
        title: format!("Import {}", classpath),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: None,
        disabled: None,
        data: None,
    })
}
