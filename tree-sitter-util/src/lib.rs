use thiserror::Error;
use tree_sitter::{Node, Point, Range, Tree, TreeCursor};

#[derive(Error, Debug, PartialEq)]
pub enum TreesitterError {
    #[error("No node was found for location")]
    NoNodeFound,
}

pub fn get_node_at_point(tree: &Tree, point: Point) -> Result<Node, TreesitterError> {
    let root = tree.root_node();
    let mut cursor = root.walk();
    loop {
        let Some(_node_index) = cursor.goto_first_child_for_point(point) else {
            return Err(TreesitterError::NoNodeFound);
        };
        let node = cursor.node();

        // Do not loop forever
        if node.child_count() == 0 {
            break;
        }
    }

    Ok(cursor.node())
}

pub fn is_point_in_range(point: &Point, range: &Range) -> bool {
    let start = range.start_point;
    let end = range.end_point;

    if *point >= start {
        if *point <= end {
            return true;
        }
    }
    false
}

pub trait CommentSkiper {
    fn parent(&mut self) -> bool;
    fn previous_sibling(&mut self) -> bool;
    fn sibling(&mut self) -> bool;
    fn first_child(&mut self) -> bool;
}

impl CommentSkiper for TreeCursor<'_> {
    fn parent(&mut self) -> bool {
        if self.goto_parent() {
            return skip_comments(self);
        }
        false
    }
    fn previous_sibling(&mut self) -> bool {
        if self.goto_previous_sibling() {
            return skip_comments(self);
        }
        false
    }
    fn sibling(&mut self) -> bool {
        if self.goto_next_sibling() {
            return skip_comments(self);
        }
        false
    }
    fn first_child(&mut self) -> bool {
        if self.goto_first_child() {
            return skip_comments(self);
        }
        false
    }
}

fn skip_comments(cursor: &mut TreeCursor<'_>) -> bool {
    match cursor.node().kind() {
        "block_comment" | "line_comment" => {
            if !cursor.goto_next_sibling() {
                return false;
            }
            skip_comments(cursor)
        }
        _ => true,
    }
}
/// Return string under cursor
pub fn get_string(cursor: &tree_sitter::TreeCursor<'_>, bytes: &[u8]) -> String {
    cursor
        .node()
        .utf8_text(bytes)
        .unwrap_or_default()
        .to_owned()
}
/// Return string under node
pub fn get_string_node(node: &Node, bytes: &[u8]) -> String {
    node.utf8_text(bytes).unwrap_or_default().to_owned()
}

#[allow(dead_code)]
pub fn tdbc(cursor: &TreeCursor, bytes: &[u8]) {
    eprintln!(
        "{} - kind:{} - text:\"{}\"",
        cursor.node().to_sexp(),
        cursor.node().kind(),
        get_string(cursor, bytes)
    );
}
