use tree_sitter::Point;

pub fn tlp(point: Point) -> tower_lsp::lsp_types::Position {
    tower_lsp::lsp_types::Position::new(
        point.row.try_into().unwrap_or_default(),
        point.column.try_into().unwrap_or_default(),
    )
}

pub fn ttp(position: tower_lsp::lsp_types::Position) -> Point {
    Point::new(
        position.line.try_into().unwrap(),
        position.character.try_into().unwrap(),
    )
}
