//! Parse functions for module-info.java
use crate::{
    error::{AstError, GetStartEnd, assert_semicolon, assert_token},
    lexer::{PositionToken, Token},
    parse_annotated_list, parse_name_dot_logical,
    types::{
        AstModule, AstModuleExports, AstModuleOpens, AstModuleProvides, AstModuleRequires,
        AstModuleRequiresFlags, AstModuleUses, AstRange,
    },
};

/// module java.base { ... }
pub fn parse_module(tokens: &[PositionToken], pos: usize) -> Result<(AstModule, usize), AstError> {
    let start = tokens.start(pos)?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;

    let mut open = false;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Open) {
        open = true;
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::Module)?;
    let (name, pos) = parse_name_dot_logical(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut errors = vec![];
    let mut pos = pos;
    let mut exports = vec![];
    let mut opens = vec![];
    let mut uses = vec![];
    let mut provides = vec![];
    let mut requires = vec![];

    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            break;
        };
        errors.clear();
        match parse_exports(tokens, pos) {
            Ok((ex, npos)) => {
                pos = npos;
                exports.push(ex);
                continue;
            }
            Err(e) => {
                errors.push(("exports".into(), e));
            }
        }
        match parse_opens(tokens, pos) {
            Ok((op, npos)) => {
                pos = npos;
                opens.push(op);
                continue;
            }
            Err(e) => {
                errors.push(("opens".into(), e));
            }
        }
        match parse_uses(tokens, pos) {
            Ok((u, npos)) => {
                pos = npos;
                uses.push(u);
                continue;
            }
            Err(e) => {
                errors.push(("uses".into(), e));
            }
        }
        match parse_provides(tokens, pos) {
            Ok((p, npos)) => {
                pos = npos;
                provides.push(p);
                continue;
            }
            Err(e) => {
                errors.push(("provides".into(), e));
            }
        }
        match parse_requires(tokens, pos) {
            Ok((p, npos)) => {
                pos = npos;
                requires.push(p);
                continue;
            }
            Err(e) => {
                errors.push(("requires".into(), e));
            }
        }
        return Err(AstError::AllChildrenFailed {
            parent: "module".into(),
            errors,
        });
    }

    let end = tokens.end(pos)?;
    Ok((
        AstModule {
            range: AstRange::from_position_token(start, end),
            annotated,
            open,
            name,
            exports,
            opens,
            uses,
            provides,
            requires,
        },
        pos,
    ))
}
fn parse_exports(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstModuleExports, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Exports)?;
    let (name, pos) = parse_name_dot_logical(tokens, pos)?;
    let mut to = vec![];
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::To) {
        let (t, npos) = parse_name_dot_logical(tokens, npos)?;
        to.push(t);
        pos = npos;
        while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            let (t, npos) = parse_name_dot_logical(tokens, npos)?;
            to.push(t);
            pos = npos;
        }
    }
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstModuleExports {
            range: AstRange::from_position_token(start, end),
            name,
            to,
        },
        pos,
    ))
}
fn parse_opens(tokens: &[PositionToken], pos: usize) -> Result<(AstModuleOpens, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Opens)?;
    let (name, pos) = parse_name_dot_logical(tokens, pos)?;
    let mut to = vec![];
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::To) {
        let (t, npos) = parse_name_dot_logical(tokens, npos)?;
        to.push(t);
        pos = npos;
    }
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstModuleOpens {
            range: AstRange::from_position_token(start, end),
            name,
            to,
        },
        pos,
    ))
}
fn parse_uses(tokens: &[PositionToken], pos: usize) -> Result<(AstModuleUses, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Uses)?;
    let (name, pos) = parse_name_dot_logical(tokens, pos)?;
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstModuleUses {
            range: AstRange::from_position_token(start, end),
            name,
        },
        pos,
    ))
}
fn parse_requires(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstModuleRequires, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut pos = assert_token(tokens, pos, Token::Requires)?;
    let mut flags = AstModuleRequiresFlags::empty();
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match t.token {
            Token::Transitive => flags |= AstModuleRequiresFlags::Transitive,
            Token::Static => flags |= AstModuleRequiresFlags::Static,
            _ => break,
        }
        pos += 1;
    }
    let (name, pos) = parse_name_dot_logical(tokens, pos)?;
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstModuleRequires {
            range: AstRange::from_position_token(start, end),
            name,
            flags,
        },
        pos,
    ))
}
fn parse_provides(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstModuleProvides, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Provides)?;
    let (name, pos) = parse_name_dot_logical(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::With)?;
    let (wi, pos) = parse_name_dot_logical(tokens, pos)?;
    let mut with = vec![wi];
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
        let (wi, npos) = parse_name_dot_logical(tokens, npos)?;
        with.push(wi);
        pos = npos;
    }
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstModuleProvides {
            range: AstRange::from_position_token(start, end),
            name,
            with,
        },
        pos,
    ))
}
