use std::collections::HashMap;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Position, Range, TextEdit, Uri, WorkspaceEdit,
};
use parser::dto::ImportUnit;
use tree_sitter::{Point, Tree};
use tree_sitter_util::{lsp::to_lsp_position, CommentSkiper};

pub fn import_jtype<'a>(
    tree: &Tree,
    bytes: &'a [u8],
    point: Point,
    imports: &[ImportUnit],
    current_file: &Uri,
    class_map: &'a dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<Vec<CodeActionOrCommand>> {
    if let Ok(n) = tree_sitter_util::get_node_at_point(tree, point) {
        if n.kind() == "type_identifier" {
            if let Ok(jtype) = n.utf8_text(bytes) {
                if !tyres::is_imported_class_name(jtype, imports, class_map) {
                    let i = tyres::resolve_import(jtype, class_map)
                        .iter()
                        .map(|a| import_to_code_action(current_file, a, tree))
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
