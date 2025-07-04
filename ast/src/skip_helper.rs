use crate::{
    error::AstError,
    lexer::{PositionToken, Token},
};

/// Get token at position and skipping comments and newline
#[track_caller]
pub fn skip(tokens: &[PositionToken], pos: usize) -> Result<usize, AstError> {
    let pos = skip_newline(tokens, pos)?;
    let pos = skip_line_comment(tokens, pos)?;
    let pos = skip_block_comment(tokens, pos)?;

    Ok(pos)
}

#[track_caller]
fn skip_newline(tokens: &[PositionToken], pos: usize) -> Result<usize, AstError> {
    let mut pos = pos;
    loop {
        let Some(token) = tokens.get(pos) else {
            break;
        };
        if token.token == Token::EOF {
            return Err(AstError::eof());
        }
        if token.token == Token::NewLine {
            pos += 1;
        } else {
            break;
        }
    }

    Ok(pos)
}
fn skip_line_comment(tokens: &[PositionToken], pos: usize) -> Result<usize, AstError> {
    let initial_pos = pos;
    let Some(token) = tokens.get(pos) else {
        return Err(AstError::eof());
    };
    if token.token != Token::Slash {
        return Ok(initial_pos);
    }
    let pos = pos + 1;
    let Some(token) = tokens.get(pos) else {
        return Err(AstError::eof());
    };
    if token.token != Token::Slash {
        return Ok(initial_pos);
    }
    let mut pos = pos;
    loop {
        let Some(token) = tokens.get(pos) else {
            return Err(AstError::eof());
        };
        if token.token == Token::NewLine {
            break;
        }

        pos += 1;
    }
    pos += 1;

    Ok(pos)
}

fn skip_block_comment(tokens: &[PositionToken], pos: usize) -> Result<usize, AstError> {
    let initial_pos = pos;
    let Some(token) = tokens.get(pos) else {
        return Err(AstError::eof());
    };
    if token.token != Token::Slash {
        return Ok(initial_pos);
    }
    let pos = pos + 1;
    let Some(token) = tokens.get(pos) else {
        return Err(AstError::eof());
    };
    if token.token != Token::Star {
        return Ok(initial_pos);
    }
    let mut pos = pos;
    loop {
        let Some(token) = tokens.get(pos) else {
            return Err(AstError::eof());
        };
        if token.token != Token::Star {
            pos += 1;
            continue;
        }
        pos += 1;
        let Some(token) = tokens.get(pos) else {
            return Err(AstError::eof());
        };
        if token.token == Token::Slash {
            break;
        }

        pos += 1;
    }

    pos += 1;

    Ok(pos)
}
