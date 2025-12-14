use std::num::TryFromIntError;

use ast::types::AstRange;
use lsp_types::{Position, Range};

#[derive(Debug)]
pub enum ToLspRangeError {
    Int(TryFromIntError),
}
pub fn to_lsp_range(range: &AstRange) -> Result<Range, ToLspRangeError> {
    let sl = u32::try_from(range.start.line).map_err(ToLspRangeError::Int)?;
    let sc = u32::try_from(range.start.col).map_err(ToLspRangeError::Int)?;
    let el = u32::try_from(range.end.line).map_err(ToLspRangeError::Int)?;
    let ec = u32::try_from(range.end.col).map_err(ToLspRangeError::Int)?;

    Ok(Range {
        start: Position {
            line: sl,
            character: sc,
        },
        end: Position {
            line: el,
            character: ec,
        },
    })
}
