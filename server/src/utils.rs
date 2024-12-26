use tree_sitter::{Point, Range};

#[allow(dead_code)]
pub fn to_lsp_position(point: Point) -> tower_lsp::lsp_types::Position {
    tower_lsp::lsp_types::Position::new(
        point.row.try_into().unwrap_or_default(),
        point.column.try_into().unwrap_or_default(),
    )
}

pub fn to_treesitter_point(position: tower_lsp::lsp_types::Position) -> Point {
    Point::new(
        position.line.try_into().unwrap_or_default(),
        position.character.try_into().unwrap_or_default(),
    )
}

pub fn to_lsp_range(range: Range) -> tower_lsp::lsp_types::Range {
    tower_lsp::lsp_types::Range {
        start: to_lsp_position(range.start_point),
        end: to_lsp_position(range.end_point),
    }
}
