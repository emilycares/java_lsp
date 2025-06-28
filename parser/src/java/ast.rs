use super::{
    error::{AstError, ExpectedToken, InvalidToken, assert_token},
    lexer::{PositionToken, Token},
};

#[derive(Debug, PartialEq)]
pub struct AstFile {
    package: String,
    imports: Vec<String>,
    thing: AstThing,
}
#[derive(Debug, PartialEq)]
pub enum AstAvailability {
    Public,
    Private,
    Protected,
}

#[derive(Debug, PartialEq)]
pub struct AstClass {
    pub avaliability: AstAvailability,
}

#[derive(Debug, PartialEq)]
pub struct AstInterface {
    pub avaliability: AstAvailability,
}

#[derive(Debug, PartialEq)]
pub enum AstThing {
    Class(AstClass),
    Interface(AstInterface),
    None,
}

pub fn parse_file(tokens: &[PositionToken], pos: usize) -> Result<AstFile, AstError> {
    let (package_name, pos) = parse_package(tokens, pos)?;
    let (imports, pos) = parse_imports(tokens, pos)?;
    let (thing, _pos) = parse_thing(tokens, pos)?;

    Ok(AstFile {
        package: package_name,
        imports,
        thing,
    })
}

/// package ch.emilycares;
fn parse_package(tokens: &[PositionToken], pos: usize) -> Result<(String, usize), AstError> {
    let mut next_pos = pos;
    let mut iter = tokens.iter();
    assert_token(&mut iter, Token::Package)?;
    let mut package_name = String::new();
    for t in iter {
        next_pos += 1;
        match &t.token {
            Token::Identifier(id) => package_name.push_str(id),
            Token::Dot => package_name.push('.'),
            Token::Semicolon => break,
            _ => continue,
        }
    }
    Ok((package_name, next_pos))
}

///  import java.io.IOException;
///  import java.net.Socket;
fn parse_imports(tokens: &[PositionToken], pos: usize) -> Result<(Vec<String>, usize), AstError> {
    let mut next_pos = pos + 1;
    let mut imports = vec![];

    while let Ok((import, new_pos)) = parse_import(tokens, next_pos) {
        next_pos = new_pos;
        imports.push(import);
    }

    Ok((imports, next_pos))
}

///  import java.io.IOException;
fn parse_import(tokens: &[PositionToken], pos: usize) -> Result<(String, usize), AstError> {
    let mut iter = tokens.iter().skip(pos);
    assert_token(&mut iter, Token::Import)?;

    let mut pos = pos;
    let mut import_name = String::new();
    for t in iter {
        pos += 1;
        match &t.token {
            Token::Identifier(id) => import_name.push_str(id),
            Token::Dot => import_name.push('.'),
            Token::Star => import_name.push('*'),
            Token::Semicolon => {
                pos += 1;
                break;
            }
            _ => continue,
        }
    }
    Ok((import_name, pos))
}

///  public class Everything { ...
///  public interface Constants { ...
fn parse_thing(tokens: &[PositionToken], pos: usize) -> Result<(AstThing, usize), AstError> {
    let mut pos = pos;
    let mut iter = tokens.iter().skip(pos);
    pos += 1;
    let avaliability = parse_avaliability(iter.next())?;
    match iter.next() {
        Some(t) => match t {
            PositionToken {
                token: Token::Class,
                line: _,
                col: _,
            } => Ok((AstThing::Class(AstClass { avaliability }), pos)),
            PositionToken {
                token: Token::Interface,
                line: _,
                col: _,
            } => Ok((AstThing::Interface(AstInterface { avaliability }), pos)),
            found => Err(AstError::ExpectedToken(ExpectedToken::from(
                found,
                Token::Class,
            ))),
        },
        None => Err(AstError::UnexpectedEOF),
    }
}

fn parse_avaliability(token: Option<&PositionToken>) -> Result<AstAvailability, AstError> {
    let Some(token) = token else {
        return Err(AstError::UnexpectedEOF);
    };
    match token {
        PositionToken {
            token: Token::Public,
            line: _,
            col: _,
        } => Ok(AstAvailability::Public),
        PositionToken {
            token: Token::Private,
            line: _,
            col: _,
        } => Ok(AstAvailability::Private),
        PositionToken {
            token: Token::Protected,
            line: _,
            col: _,
        } => Ok(AstAvailability::Protected),
        e => Err(AstError::InvalidAvailability(InvalidToken::from(e))),
    }
}

#[cfg(test)]
pub mod tests {
    use pretty_assertions::assert_eq;

    use crate::java::error::PrintErr;
    use crate::java::{
        ast::{AstAvailability, AstClass, AstFile, AstInterface, AstThing, parse_file},
        lexer,
    };

    #[test]
    fn everything() {
        let content = include_str!("../../test/Everything.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens, 0);
        parsed.print_err(content);
        assert_eq!(
            parsed.unwrap(),
            AstFile {
                package: "ch.emilycares".to_string(),
                imports: vec![],
                thing: AstThing::Class(AstClass {
                    avaliability: AstAvailability::Public
                })
            }
        );
    }

    #[test]
    fn superee() {
        let content = include_str!("../../test/Super.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens, 0);
        parsed.print_err(content);
        assert_eq!(
            parsed.unwrap(),
            AstFile {
                package: "ch.emilycares".to_string(),
                imports: vec!["java.io.IOException".to_string()],
                thing: AstThing::Class(AstClass {
                    avaliability: AstAvailability::Public
                })
            }
        );
    }

    #[test]
    fn constants() {
        let content = include_str!("../../test/Constants.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens, 0);
        parsed.print_err(content);
        assert_eq!(
            parsed.unwrap(),
            AstFile {
                package: "ch.emilycares".to_string(),
                imports: vec![
                    "jdk.net.Sockets".to_string(),
                    "java.io.IOException".to_string(),
                    "java.net.Socket".to_string()
                ],
                thing: AstThing::Interface(AstInterface {
                    avaliability: AstAvailability::Public
                })
            }
        );
    }
}
