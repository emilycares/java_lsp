#![deny(missing_docs)]
//! A java ast
use annotation::parse_annotation;
use class::parse_class;
use enumeration::parse_enumeration;
use error::{AstError, ExpectedToken, InvalidToken, assert_semicolon, assert_token};
use interface::parse_interface;
use lexer::{PositionToken, Token};
use smol_str::{SmolStr, SmolStrBuilder};
use types::{
    AstAnnotated, AstAvailability, AstBaseExpression, AstBlock, AstBlockAssign, AstBlockBreak,
    AstBlockContinue, AstBlockEntry, AstBlockExpression, AstBlockReturn, AstBlockVariable,
    AstBlockVariableMutliType, AstBoolean, AstCastedExpression, AstDouble, AstExpressionIdentifier,
    AstExpressionOperator, AstExtends, AstFile, AstFor, AstForEnhanced, AstIdentifier, AstIf,
    AstIfContent, AstImport, AstImportUnit, AstImports, AstInt, AstJType, AstJTypeKind, AstLambda,
    AstLambdaParameters, AstMethodHeader, AstMethodParamerter, AstMethodParamerters, AstPoint,
    AstRange, AstRecursiveExpression, AstSuperClass, AstSwitch, AstSwitchCase, AstSwitchDefault,
    AstThing, AstThrow, AstThrowsDeclaration, AstTryCatch, AstTryCatchCase, AstTypeParameters,
    AstValue, AstValueNewClass, AstValueNuget, AstValues, AstWhile,
};

use crate::{
    class::parse_class_block,
    types::{AstBlockYield, AstSwitchCaseArrow},
};

pub mod annotation;
pub mod class;
pub mod enumeration;
pub mod error;
pub mod interface;
pub mod lexer;
pub mod range;
pub mod types;

///` package ch.emilycares; import .... public class ...`
pub fn parse_file(tokens: &[PositionToken]) -> Result<AstFile, AstError> {
    let (package_name, pos) = parse_package(tokens, 0)?;
    let (imports, pos) = parse_imports(tokens, pos)?;
    let (thing, _pos) = parse_thing(tokens, pos)?;
    dbg!("endfild");

    Ok(AstFile {
        package: package_name,
        imports,
        thing,
    })
}

///` package ch.emilycares;`
fn parse_package(tokens: &[PositionToken], pos: usize) -> Result<(AstIdentifier, usize), AstError> {
    let pos = assert_token(tokens, pos, Token::Package)?;
    let (package_name, pos) = parse_identifier(tokens, pos)?;
    let pos = assert_semicolon(tokens, pos);
    Ok((package_name, pos))
}
///`  import java.io.IOException;`
///`  import java.net.Socket;`
fn parse_imports(tokens: &[PositionToken], pos: usize) -> Result<(AstImports, usize), AstError> {
    let mut pos = pos;
    let mut imports = vec![];

    let start = tokens.get(pos).ok_or(AstError::eof())?;
    while let Ok((import, new_pos)) = parse_import(tokens, pos) {
        pos = new_pos;
        imports.push(import);
    }
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstImports {
            range: AstRange {
                start: start.start_point(),
                end: end.end_point(),
            },
            imports,
        },
        pos,
    ))
}

///`  import java.io.IOException;`
fn parse_import(tokens: &[PositionToken], pos: usize) -> Result<(AstImport, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Import)?;
    let mut pos = pos;
    let mut stat = false;
    let mut prefix = false;
    if let Ok(npos) = assert_token(tokens, pos, Token::Static) {
        pos = npos;
        stat = true;
    }
    let (ident, pos) = parse_identifier(tokens, pos)?;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Star) {
        pos = npos;
        prefix = true;
    }
    let pos = assert_semicolon(tokens, pos);
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstImport {
            range: AstRange::from_position_token(start, end),
            unit: match (stat, prefix) {
                (true, true) => AstImportUnit::StaticPrefix(ident),
                (true, false) => match ident.value.rsplit_once(".") {
                    Some((class, method)) => {
                        match method.chars().next().unwrap_or_default().is_lowercase() {
                            true => AstImportUnit::StaticClassMethod(
                                AstIdentifier {
                                    range: AstRange {
                                        start: ident.range.start,
                                        end: AstPoint {
                                            line: ident.range.start.line,
                                            col: ident.range.end.line - method.len(),
                                        },
                                    },
                                    value: class.into(),
                                },
                                AstIdentifier {
                                    range: AstRange {
                                        start: AstPoint {
                                            line: ident.range.start.line,
                                            col: ident.range.start.col - class.len(),
                                        },
                                        end: ident.range.end,
                                    },
                                    value: method.into(),
                                },
                            ),
                            false => AstImportUnit::StaticClass(ident),
                        }
                    }
                    None => AstImportUnit::StaticPrefix(ident),
                },
                (false, true) => AstImportUnit::Prefix(ident),
                (false, false) => AstImportUnit::Class(ident),
            },
        },
        pos,
    ))
}

///`  public class Everything { ...`
///`  public interface Constants { ...`
fn parse_thing(tokens: &[PositionToken], pos: usize) -> Result<(AstThing, usize), AstError> {
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let (avaliability, pos) = parse_avaliability(tokens, pos)?;
    match tokens.get(pos) {
        Some(t) => match t {
            PositionToken {
                token: Token::Class,
                line: _,
                col: _,
            } => parse_class(tokens, pos + 1, avaliability, annotated),
            PositionToken {
                token: Token::Interface,
                line: _,
                col: _,
            } => parse_interface(tokens, pos + 1, avaliability, annotated),
            PositionToken {
                token: Token::Enum,
                line: _,
                col: _,
            } => parse_enumeration(tokens, pos + 1, avaliability, annotated),
            PositionToken {
                token: Token::At,
                line: _,
                col: _,
            } => parse_annotation(tokens, pos + 1, avaliability, annotated),
            found => Err(AstError::ExpectedToken(ExpectedToken::from(
                found,
                Token::Class,
            ))),
        },
        None => Err(AstError::eof()),
    }
}

fn parse_value(tokens: &[PositionToken], pos: usize) -> Result<(AstValue, usize), AstError> {
    let mut errors = vec![];
    match parse_boolean_literal(tokens, pos) {
        Ok((nuget, pos)) => return Ok((nuget, pos)),
        Err(e) => errors.push(("value boolean".into(), e)),
    }
    match parse_value_new_class(tokens, pos) {
        Ok((new, pos)) => return Ok((AstValue::NewClass(new), pos)),
        Err(e) => errors.push(("value new class".into(), e)),
    };
    match parse_value_array(tokens, pos) {
        Ok(v) => return Ok(v),
        Err(e) => errors.push(("value array".into(), e)),
    };
    match parse_value_nuget(tokens, pos) {
        Ok((nuget, pos)) => return Ok((nuget, pos)),
        Err(e) => errors.push(("value nuget".into(), e)),
    }
    Err(AstError::AllChildrenFailed {
        parent: "value".into(),
        errors,
    })
}
fn parse_annotated_list(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstAnnotated>, usize), AstError> {
    let mut out = vec![];
    let mut pos = pos;
    loop {
        if let Ok((a, npos)) = parse_annotated(tokens, pos) {
            out.push(a);
            pos = npos;
        } else {
            break;
        }
    }
    Ok((out, pos))
}
fn parse_annotated(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstAnnotated, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::At)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let mut parameters = vec![];
    let mut pos = pos;
    if let Ok((params, npos)) = parse_expression_parameters(tokens, pos) {
        parameters.extend(params);
        pos = npos;
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstAnnotated {
            range: AstRange::from_position_token(start, end),
            name,
            parameters,
        },
        pos,
    ))
}
///` (a) -> a.doThing()`
///` a -> a.doThing()`
///` a -> {a.doThing();}`
pub fn parse_lambda(tokens: &[PositionToken], pos: usize) -> Result<(AstLambda, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let parameters;
    if let Ok((lparams, npos)) = parse_lambda_parameters(tokens, pos) {
        parameters = lparams;
        pos = npos;
    } else {
        let (n, npos) = parse_name(tokens, pos)?;
        parameters = AstLambdaParameters {
            range: n.range,
            values: vec![n],
        };
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::Dash)?;
    let pos = assert_token(tokens, pos, Token::Gt)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstLambda {
            range: AstRange::from_position_token(start, end),
            parameters,
            block,
        },
        pos,
    ))
}
/// `(a) -> a.length`
///  ^^^
pub fn parse_lambda_parameters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstLambdaParameters, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let mut pos = pos;
    let mut values = vec![];

    loop {
        if let Ok((n, npos)) = parse_name(tokens, pos) {
            values.push(n);

            pos = npos;
            if let Ok(npos) = assert_token(tokens, npos, Token::Comma) {
                pos = npos;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstLambdaParameters {
            range: AstRange::from_position_token(start, end),
            values,
        },
        pos,
    ))
}

fn parse_value_array(tokens: &[PositionToken], pos: usize) -> Result<(AstValue, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut values = vec![];
    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            break;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
        let (value, npos) = parse_expression(tokens, pos)?;
        pos = npos;
        values.push(value);
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            break;
        }
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstValue::Array(AstValues {
            range: AstRange::from_position_token(start, end),
            values,
        }),
        pos,
    ))
}

fn parse_value_nuget(tokens: &[PositionToken], pos: usize) -> Result<(AstValue, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    match &start.token {
        Token::Identifier(name) => Ok((
            AstValue::Variable(AstIdentifier {
                range: AstRange {
                    start: start.start_point(),
                    end: start.end_point(),
                },
                value: name.clone(),
            }),
            pos + 1,
        )),
        Token::Number(num) => {
            if let Ok(pos) = assert_token(tokens, pos + 1, Token::Dot) {
                let current = tokens.get(pos).ok_or(AstError::eof())?;
                if let Token::Number(n) = current.token {
                    let value: f64 = format!("{num}.{n}")
                        .parse()
                        .map_err(|_| AstError::InvalidDouble(*num, n))?;
                    let pos = pos + 1;
                    let current = tokens.get(pos).ok_or(AstError::eof())?;
                    let pos = pos + 1;
                    match &current.token {
                        Token::Identifier(val) if val == "d" => {
                            return Ok((
                                AstValue::Nuget(AstValueNuget::Double(AstDouble {
                                    range: AstRange::from_position_token(start, start),
                                    value,
                                })),
                                pos,
                            ));
                        }
                        Token::Identifier(val) if val == "f" => {
                            return Ok((
                                AstValue::Nuget(AstValueNuget::Float(AstDouble {
                                    range: AstRange::from_position_token(start, start),
                                    value,
                                })),
                                pos,
                            ));
                        }
                        _ => {
                            return Err(AstError::InvalidNuget(InvalidToken::from(
                                current,
                                pos - 1,
                            )));
                        }
                    }
                }
            }
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Identifier("l".into())) {
                return Ok((
                    AstValue::Nuget(AstValueNuget::Int(AstInt {
                        range: AstRange::from_position_token(start, start),
                        value: *num,
                    })),
                    npos,
                ));
            }
            Ok((
                AstValue::Nuget(AstValueNuget::Int(AstInt {
                    range: AstRange::from_position_token(start, start),
                    value: *num,
                })),
                pos + 1,
            ))
        }
        Token::DoubleQuote => {
            parse_string_literal(tokens, pos).map(|i| (AstValue::Nuget(i.0), i.1))
        }
        Token::SingleQuote => parse_char_literal(tokens, pos),
        Token::True => parse_boolean_literal_input(tokens, pos, true),
        Token::False => parse_boolean_literal_input(tokens, pos, false),
        _ => Err(AstError::InvalidNuget(InvalidToken::from(start, pos))),
    }
}

fn parse_char_literal(tokens: &[PositionToken], pos: usize) -> Result<(AstValue, usize), AstError> {
    let pos = assert_token(tokens, pos, Token::SingleQuote)?;
    let (char, pos) = parse_name(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::SingleQuote)?;
    Ok((AstValue::Nuget(AstValueNuget::CharLiteral(char)), pos))
}

fn parse_boolean_literal_input(
    tokens: &[PositionToken],
    pos: usize,
    value: bool,
) -> Result<(AstValue, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstValue::Nuget(AstValueNuget::BooleanLiteral(AstBoolean {
            range: AstRange::from_position_token(start, start),
            value,
        })),
        pos + 1,
    ))
}

fn parse_boolean_literal(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValue, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let value = match start.token {
        Token::True => true,
        Token::False => false,
        _ => return Err(AstError::InvalidBoolean(InvalidToken::from(start, pos))),
    };
    Ok((
        AstValue::Nuget(AstValueNuget::BooleanLiteral(AstBoolean {
            range: AstRange::from_position_token(start, start),
            value,
        })),
        pos + 1,
    ))
}
/// `"some string"`
pub fn parse_string_literal(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValueNuget, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = assert_token(tokens, pos, Token::DoubleQuote)?;
    let mut multiline = false;
    if let Ok(npos) = assert_token(tokens, pos, Token::DoubleQuote)
        && let Ok(npos) = assert_token(tokens, npos, Token::DoubleQuote)
    {
        pos = npos;
        multiline = true;
    }
    let mut value = SmolStrBuilder::new();
    loop {
        let token = tokens.get(pos).ok_or(AstError::eof())?;
        match &token.token {
            Token::DoubleQuote => {
                let peek = tokens.get(pos - 1).ok_or(AstError::eof())?;
                if peek.token == Token::BackSlash {
                    value.push_str("\\\"");
                } else if !multiline {
                    pos += 1;
                    break;
                } else if let Ok(npos) = assert_token(tokens, pos, Token::DoubleQuote)
                    && let Ok(npos) = assert_token(tokens, npos, Token::DoubleQuote)
                {
                    pos = npos;
                    break;
                }
            }
            cot => {
                value.push_str(&cot.to_string());
            }
        }
        pos += 1;
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstValueNuget::StringLiteral(AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value: value.finish(),
        }),
        pos,
    ))
}

fn parse_value_operator(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstExpressionOperator, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    match &start.token {
        Token::Plus => {
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Plus) {
                let end = tokens.get(npos).ok_or(AstError::eof())?;
                Ok((
                    AstExpressionOperator::PlusPlus(AstRange::from_position_token(start, end)),
                    npos,
                ))
            } else {
                Ok((
                    AstExpressionOperator::Plus(AstRange::from_position_token(start, start)),
                    pos + 1,
                ))
            }
        }
        Token::Dash => {
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Dash) {
                let end = tokens.get(npos).ok_or(AstError::eof())?;
                Ok((
                    AstExpressionOperator::MinusMinus(AstRange::from_position_token(start, end)),
                    npos,
                ))
            } else {
                Ok((
                    AstExpressionOperator::Minus(AstRange::from_position_token(start, start)),
                    pos + 1,
                ))
            }
        }
        Token::Ampersand => {
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Ampersand) {
                let end = tokens.get(npos).ok_or(AstError::eof())?;
                Ok((
                    AstExpressionOperator::AmpersandAmpersand(AstRange::from_position_token(
                        start, end,
                    )),
                    npos,
                ))
            } else {
                Ok((
                    AstExpressionOperator::Ampersand(AstRange::from_position_token(start, start)),
                    pos + 1,
                ))
            }
        }
        Token::VerticalBar => {
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::VerticalBar) {
                let end = tokens.get(npos).ok_or(AstError::eof())?;
                Ok((
                    AstExpressionOperator::VerticalBarVerticalBar(AstRange::from_position_token(
                        start, end,
                    )),
                    npos,
                ))
            } else {
                Ok((
                    AstExpressionOperator::VerticalBar(AstRange::from_position_token(start, start)),
                    pos + 1,
                ))
            }
        }
        Token::QuestionMark => Ok((
            AstExpressionOperator::QuestionMark(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Colon => Ok((
            AstExpressionOperator::Colon(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Dot => Ok((
            AstExpressionOperator::Dot(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::ExclamationMark => Ok((
            AstExpressionOperator::ExclemationMark(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Star => Ok((
            AstExpressionOperator::Multiply(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Slash => Ok((
            AstExpressionOperator::Devide(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Percent => Ok((
            AstExpressionOperator::Modulo(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::EqualDouble => Ok((
            AstExpressionOperator::Equal(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Ne => Ok((
            AstExpressionOperator::NotEqual(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Gt => Ok((
            AstExpressionOperator::Gt(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Ge => Ok((
            AstExpressionOperator::Ge(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Lt => Ok((
            AstExpressionOperator::Lt(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Le => Ok((
            AstExpressionOperator::Le(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        _ => Err(AstError::InvalidNuget(InvalidToken::from(start, pos))),
    }
}

fn parse_expression_parameters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstBaseExpression>, usize), AstError> {
    let mut pos = assert_token(tokens, pos, Token::LeftParen)?;
    let mut out = vec![];
    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParen) {
            pos = npos;
            break;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
        let (expression, npos) = parse_expression(tokens, pos)?;
        pos = npos;
        out.push(expression);
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParen) {
            pos = npos;
            break;
        }
    }
    Ok((out, pos))
}

fn parse_value_new_class(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValueNewClass, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::New)?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (parameters, pos) = parse_expression_parameters(tokens, pos)?;
    let mut pos = pos;
    let mut block = None;
    if let Ok((b, npos)) = parse_class_block(tokens, pos) {
        pos = npos;
        block = Some(b);
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstValueNewClass {
            range: AstRange::from_position_token(start, end),
            jtype,
            parameters,
            block,
        },
        pos,
    ))
}
fn parse_expression(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBaseExpression, usize), AstError> {
    let mut errors = vec![];
    match parse_lambda(tokens, pos) {
        Ok((lambda, pos)) => {
            return Ok((AstBaseExpression::Lambda(lambda), pos));
        }
        Err(e) => errors.push(("lambda".into(), e)),
    }
    match parse_switch(tokens, pos) {
        Ok((casted, pos)) => {
            return Ok((AstBaseExpression::InlineSwitch(casted), pos));
        }
        Err(e) => errors.push(("inline switch".into(), e)),
    }
    match parse_casted_expression(tokens, pos) {
        Ok((casted, pos)) => {
            return Ok((AstBaseExpression::Casted(casted), pos));
        }
        Err(e) => errors.push(("casted".into(), e)),
    }
    match parse_recursive_expression(tokens, pos) {
        Ok((recursive, pos)) => {
            return Ok((AstBaseExpression::Recursive(recursive), pos));
        }
        Err(e) => errors.push(("recursive".into(), e)),
    }
    Err(AstError::AllChildrenFailed {
        parent: "expression".into(),
        errors,
    })
}
fn parse_casted_expression(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstCastedExpression, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (cast, pos) = parse_jtype(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let (expression, pos) = parse_recursive_expression(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstCastedExpression {
            range: AstRange::from_position_token(start, end),
            cast,
            expression,
        },
        pos,
    ))
}
/// `a.b.c("a".length)`
pub fn parse_recursive_expression(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstRecursiveExpression, usize), AstError> {
    let mut ident = None;
    let mut values = None;
    let mut next = None;
    let mut operator = AstExpressionOperator::None;

    let mut pos = pos;
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    match start.token {
        Token::Identifier(_) | Token::Class | Token::This => {
            let (id, npos) = parse_expression_lhs(tokens, pos)?;
            pos = npos;
            ident = Some(AstExpressionIdentifier::Identifier(id));
            if let Ok((exp, npos)) = parse_recursive_expression(tokens, pos) {
                pos = npos;
                if exp.has_content() {
                    next = Some(Box::new(exp));
                }
            }
        }
        Token::LeftParenSquare => {
            pos += 1;
            let (value, npos) = parse_value(tokens, pos)?;
            let npos = assert_token(tokens, npos, Token::RightParenSquare)?;
            ident = Some(AstExpressionIdentifier::ArrayAccess(value));
            pos = npos;
            if let Ok((exp, npos)) = parse_recursive_expression(tokens, pos) {
                pos = npos;
                if exp.has_content() {
                    next = Some(Box::new(exp));
                }
            }
        }
        Token::LeftParen => {
            let values_start = tokens.get(pos).ok_or(AstError::eof())?;
            let (vals, npos) = parse_expression_parameters(tokens, pos)?;
            pos = npos;
            let values_end = tokens.get(pos - 1).ok_or(AstError::eof())?;
            values = Some(types::AstValues {
                range: AstRange::from_position_token(values_start, values_end),
                values: vals,
            });
            if let Ok((exp, npos)) = parse_recursive_expression(tokens, pos) {
                pos = npos;
                if exp.has_content() {
                    next = Some(Box::new(exp));
                }
            }
        }
        _ => {
            let mut errors: Vec<(SmolStr, AstError)> = vec![];
            match parse_value_operator(tokens, pos) {
                Ok((op, npos)) => {
                    pos = npos;
                    operator = op;
                    if let Ok((exp, npos)) = parse_recursive_expression(tokens, pos) {
                        pos = npos;
                        if exp.has_content() {
                            next = Some(Box::new(exp));
                        }
                    }
                }
                Err(e) => errors.push(("operator".into(), e)),
            }
            match parse_value(tokens, pos) {
                Ok((value, npos)) => {
                    pos = npos;
                    ident = Some(AstExpressionIdentifier::Value(value));
                    if let Ok((exp, npos)) = parse_recursive_expression(tokens, pos) {
                        pos = npos;
                        if exp.has_content() {
                            next = Some(Box::new(exp));
                        }
                    }
                }
                Err(e) => errors.push(("value".into(), e)),
            }
            if errors.len() >= 2 {
                return Err(AstError::AllChildrenFailed {
                    parent: "expression".into(),
                    errors,
                });
            }
        }
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstRecursiveExpression {
            range: AstRange::from_position_token(start, end),
            ident,
            next,
            values,
            operator,
        },
        pos,
    ))
}

fn parse_expression_lhs(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    match parse_name(tokens, pos) {
        Ok((ident, npos)) => Ok((ident, npos)),
        Err(AstError::InvalidName(e)) => match e.found {
            Token::Class => {
                let start = tokens.get(pos).ok_or(AstError::eof())?;
                Ok((
                    AstIdentifier {
                        range: AstRange::from_position_token(start, start),
                        value: "class".into(),
                    },
                    pos + 1,
                ))
            }
            Token::This => {
                let start = tokens.get(pos).ok_or(AstError::eof())?;
                Ok((
                    AstIdentifier {
                        range: AstRange::from_position_token(start, start),
                        value: "this".into(),
                    },
                    pos + 1,
                ))
            }
            _ => Err(AstError::InvalidName(e)),
        },
        Err(e) => Err(e),
    }
}

/// `String value = "a";`
pub fn parse_block_variable(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockVariable, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (block_variable, pos) = parse_block_variable_no_semicolon(tokens, pos)?;
    let mut block_variable = block_variable;
    let pos = assert_semicolon(tokens, pos);
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    block_variable.range = AstRange::from_position_token(start, end);
    Ok((block_variable, pos))
}
fn parse_block_variable_no_semicolon(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockVariable, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let mut expression = None;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Equal) {
        let (aexpression, npos) = parse_expression(tokens, npos)?;
        pos = npos;
        expression = Some(aexpression);
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    Ok((
        AstBlockVariable {
            name,
            jtype,
            expression,
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}
fn parse_block_variable_multi_type_no_semicolon(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockVariableMutliType, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut jtypes = vec![];
    loop {
        let (jtype, npos) = parse_jtype(tokens, pos)?;
        jtypes.push(jtype);
        pos = npos;
        if let Ok(npos) = assert_token(tokens, pos, Token::VerticalBar) {
            pos = npos;
            continue;
        } else {
            break;
        }
    }
    let (name, pos) = parse_name(tokens, pos)?;
    let mut expression = None;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Equal) {
        let (aexpression, npos) = parse_expression(tokens, npos)?;
        pos = npos;
        expression = Some(aexpression);
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    Ok((
        AstBlockVariableMutliType {
            name,
            jtypes,
            expression,
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}

fn parse_block_return(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockReturn, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Return)?;
    let mut pos = pos;
    let mut expression = None;
    if let Ok((nexpression, npos)) = parse_recursive_expression(tokens, pos) {
        pos = npos;
        expression = Some(nexpression);
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstBlockReturn {
            range: AstRange::from_position_token(start, end),
            expression,
        },
        pos,
    ))
}
fn parse_block_yield(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockYield, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Yield)?;
    let mut pos = pos;
    let mut expression = None;
    if let Ok((nexpression, npos)) = parse_recursive_expression(tokens, pos) {
        pos = npos;
        expression = Some(nexpression);
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstBlockYield {
            range: AstRange::from_position_token(start, end),
            expression,
        },
        pos,
    ))
}
fn parse_block_break(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockBreak, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Break)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstBlockBreak {
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}
fn parse_block_continue(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockContinue, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Continue)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstBlockContinue {
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}
fn parse_block_expression(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockExpression, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (value, pos) = parse_recursive_expression(tokens, pos)?;
    let pos = assert_semicolon(tokens, pos);
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstBlockExpression {
            range: AstRange::from_position_token(start, end),
            value,
        },
        pos,
    ))
}

fn parse_block_assign(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockAssign, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (key, pos) = parse_recursive_expression(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Equal)?;
    let mut expression = None;
    let mut pos = pos;
    if let Ok((expr, npos)) = parse_recursive_expression(tokens, pos) {
        expression = Some(expr);
        pos = npos;
    }
    let pos = assert_semicolon(tokens, pos);
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstBlockAssign {
            range: AstRange::from_position_token(start, end),
            key,
            expression,
        },
        pos,
    ))
}

fn parse_method_header(
    tokens: &[PositionToken],
    pos: usize,
    default_availability: AstAvailability,
) -> Result<(AstMethodHeader, usize), AstError> {
    let mut pos = pos;
    let mut avaliability = default_availability;
    let mut type_parameters = None;
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    if let Ok((avav, npos)) = parse_avaliability(tokens, pos) {
        avaliability = avav;
        pos = npos;
    };

    if let Ok(npos) = assert_token(tokens, pos, Token::Static) {
        avaliability = avaliability.to_static();
        pos = npos;
    }

    if let Ok((type_params, npos)) = parse_type_parameters(tokens, pos) {
        type_parameters = Some(type_params);
        pos = npos;
    };

    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let (parameters, pos) = parse_method_paramerters(tokens, pos)?;
    let mut pos = pos;
    let mut throws = None;
    if let Ok((nthrows, npos)) = parse_throws_declaration(tokens, pos) {
        throws = Some(nthrows);
        pos = npos;
    }
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstMethodHeader {
            range: AstRange::from_position_token(start, end),
            avaliability,
            type_parameters,
            name,
            jtype,
            parameters,
            throws,
        },
        pos,
    ))
}
fn parse_throws_declaration(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstThrowsDeclaration, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Throws)?;
    let (parameters, pos) = parse_type_list(tokens, pos)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstThrowsDeclaration {
            range: AstRange::from_position_token(start, end),
            parameters,
        },
        pos,
    ))
}

/// `public void <T> do() { ... }`
///              ^^^
pub fn parse_type_parameters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstTypeParameters, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Lt)?;
    let mut pos = pos;
    let mut parameters = vec![];
    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::Gt) {
            pos = npos;
            break;
        }
        let (name, npos) = parse_name(tokens, pos)?;
        pos = npos;
        parameters.push(name);
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Gt) {
            pos = npos;
            break;
        }
    }
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstTypeParameters {
            range: AstRange::from_position_token(start, end),
            parameters,
        },
        pos,
    ))
}

fn parse_extends(tokens: &[PositionToken], pos: usize) -> Result<(AstExtends, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Extends)?;
    let (parameters, pos) = parse_type_list(tokens, pos)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstExtends {
            range: AstRange::from_position_token(start, end),
            parameters,
        },
        pos,
    ))
}

fn parse_type_list(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstJType>, usize), AstError> {
    let mut pos = pos;
    let mut parameters = vec![];
    while let Ok((name, npos)) = parse_jtype(tokens, pos) {
        pos = npos;
        parameters.push(name);
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
    }
    Ok((parameters, pos))
}

fn parse_block(tokens: &[PositionToken], pos: usize) -> Result<(AstBlock, usize), AstError> {
    parse_block_brackets(tokens, pos, Token::LeftParenCurly, Token::RightParenCurly)
}
fn parse_block_brackets(
    tokens: &[PositionToken],
    pos: usize,
    left: Token,
    right: Token,
) -> Result<(AstBlock, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, left)?;
    let mut pos = pos;
    let mut entries = vec![];
    let mut errors = vec![];
    loop {
        errors.clear();
        if let Ok(npos) = assert_token(tokens, pos, right.clone()) {
            pos = npos;
            break;
        };
        match parse_block_variable(tokens, pos) {
            Ok((variable, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Variable(variable));
                continue;
            }
            Err(e) => {
                errors.push(("block variable".into(), e));
            }
        }
        match parse_block_return(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Return(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block return".into(), e));
            }
        }
        match parse_block_yield(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Yield(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block yield".into(), e));
            }
        }
        match parse_block_break(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Break(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block break".into(), e));
            }
        }
        match parse_block_continue(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Continue(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block continue".into(), e));
            }
        }
        match parse_if(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::If(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block if".into(), e));
            }
        }
        match parse_while(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::While(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block while".into(), e));
            }
        }
        match parse_do_while(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::While(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block do while".into(), e));
            }
        }
        match parse_for(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::For(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block for".into(), e));
            }
        }
        match parse_switch(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Switch(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block switch".into(), e));
            }
        }
        match parse_switch_case(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::SwitchCase(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block switch case".into(), e));
            }
        }
        match parse_switch_case_arrow(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::SwitchCaseArrow(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block switch case arrow".into(), e));
            }
        }
        match parse_switch_default(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::SwitchDefault(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block switch default".into(), e));
            }
        }
        match parse_for_enhanced(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::ForEnhanced(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block for enhanced".into(), e));
            }
        }
        match parse_try_catch(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::TryCatch(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block try catch".into(), e));
            }
        }
        match parse_throw(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Throw(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block throw".into(), e));
            }
        }
        match parse_block_assign(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Assign(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block assign".into(), e));
            }
        }
        match parse_block_expression(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Expression(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block expression".into(), e));
            }
        }
        return Err(AstError::AllChildrenFailed {
            parent: "block".into(),
            errors,
        });
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof()).unwrap();
    Ok((
        AstBlock {
            range: AstRange::from_position_token(start, end),
            entries,
        },
        pos,
    ))
}

fn parse_while(tokens: &[PositionToken], pos: usize) -> Result<(AstWhile, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut lable = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        lable = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::While)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (control, pos) = parse_recursive_expression(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstWhile {
            range: AstRange::from_position_token(start, end),
            control,
            block,
            lable,
        },
        pos,
    ))
}
fn parse_do_while(tokens: &[PositionToken], pos: usize) -> Result<(AstWhile, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut lable = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        lable = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::Do)?;
    let (block, pos) = parse_block(tokens, pos)?;

    let pos = assert_token(tokens, pos, Token::While)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (control, pos) = parse_recursive_expression(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstWhile {
            range: AstRange::from_position_token(start, end),
            control,
            block,
            lable,
        },
        pos,
    ))
}
fn parse_for(tokens: &[PositionToken], pos: usize) -> Result<(AstFor, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut lable = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        lable = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::For)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (var, pos) = parse_block_variable(tokens, pos)?;
    let (check, pos) = parse_recursive_expression(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let (change, pos) = parse_recursive_expression(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstFor {
            range: AstRange::from_position_token(start, end),
            block,
            lable,
            var,
            check,
            change,
        },
        pos,
    ))
}

fn parse_switch(tokens: &[PositionToken], pos: usize) -> Result<(AstSwitch, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Switch)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (check, pos) = parse_recursive_expression(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstSwitch {
            range: AstRange::from_position_token(start, end),
            block,
            check,
        },
        pos,
    ))
}
fn parse_switch_case(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchCase, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Case)?;
    let (value, pos) = parse_value(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Colon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstSwitchCase {
            range: AstRange::from_position_token(start, end),
            value,
        },
        pos,
    ))
}
fn parse_switch_case_arrow(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchCaseArrow, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = assert_token(tokens, pos, Token::Case)?;
    let mut values = vec![];
    loop {
        let (value, npos) = parse_value(tokens, pos)?;
        values.push(value);
        pos = npos;

        match assert_token(tokens, pos, Token::Comma) {
            Ok(npos) => {
                pos = npos;
            }
            Err(_) => break,
        }
    }
    let pos = assert_token(tokens, pos, Token::Dash)?;
    let pos = assert_token(tokens, pos, Token::Gt)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstSwitchCaseArrow {
            range: AstRange::from_position_token(start, end),
            values,
            block,
        },
        pos,
    ))
}
fn parse_switch_default(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchDefault, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Default)?;
    let pos = assert_token(tokens, pos, Token::Colon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstSwitchDefault {
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}

fn parse_for_enhanced(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstForEnhanced, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut lable = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        lable = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::For)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (var, pos) = parse_block_variable_no_semicolon(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Colon)?;
    let (rhs, pos) = parse_recursive_expression(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstForEnhanced {
            range: AstRange::from_position_token(start, end),
            block,
            lable,
            var,
            rhs,
        },
        pos,
    ))
}

fn parse_if(tokens: &[PositionToken], pos: usize) -> Result<(AstIf, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::If)?;
    let start_control = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (control, pos) = parse_recursive_expression(tokens, pos)?;
    let end_control = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let mut pos = pos;
    let mut content = AstIfContent::None;
    if let Ok((block, npos)) = parse_block(tokens, pos) {
        content = AstIfContent::Block(block);
        pos = npos;
    } else if let Ok((expression, npos)) = parse_recursive_expression(tokens, pos) {
        content = AstIfContent::Expression(expression);
        let npos = assert_semicolon(tokens, npos);
        pos = npos;
    }
    let mut el = None;
    if let Ok(npos) = assert_token(tokens, pos, Token::Else) {
        pos = npos;

        if let Ok((i, npos)) = parse_if(tokens, pos) {
            el = Some(Box::new(i));
            pos = npos;
        }
        if let Ok((i, npos)) = parse_block(tokens, pos) {
            el = Some(Box::new(AstIf::Else {
                range: i.range,
                content: AstIfContent::Block(i),
            }));
            pos = npos;
        }
    }

    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstIf::If {
            range: AstRange::from_position_token(start, end),
            control,
            control_range: AstRange::from_position_token(start_control, end_control),
            content,
            el,
        },
        pos,
    ))
}
fn parse_throw(tokens: &[PositionToken], pos: usize) -> Result<(AstThrow, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Throw)?;
    let (value, pos) = parse_value_new_class(tokens, pos)?;
    let pos = assert_semicolon(tokens, pos);
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstThrow {
            range: AstRange::from_position_token(start, end),
            value,
        },
        pos,
    ))
}

fn parse_try_catch(tokens: &[PositionToken], pos: usize) -> Result<(AstTryCatch, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Try)?;
    let mut resources_block = None;
    let mut pos = pos;
    if let Ok((res, npos)) = parse_block_brackets(tokens, pos, Token::LeftParen, Token::RightParen)
    {
        resources_block = Some(res);
        pos = npos;
    }
    let (block, pos) = parse_block(tokens, pos)?;
    let mut pos = pos;
    let mut finally_block = None;
    let mut cases = vec![];
    while assert_token(tokens, pos, Token::Catch).is_ok() {
        pos += 1;
        let start = tokens.get(pos).ok_or(AstError::eof())?;
        let npos = assert_token(tokens, pos, Token::LeftParen)?;
        let (variable, npos) = parse_block_variable_multi_type_no_semicolon(tokens, npos)?;
        let npos = assert_token(tokens, npos, Token::RightParen)?;
        let (block, npos) = parse_block(tokens, npos)?;
        pos = npos;
        let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
        cases.push(AstTryCatchCase {
            range: AstRange::from_position_token(start, end),
            variable,
            block,
        });
    }
    if let Ok(npos) = assert_token(tokens, pos, Token::Finally) {
        let (f_block, npos) = parse_block(tokens, npos)?;
        finally_block = Some(f_block);
        pos = npos;
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstTryCatch {
            range: AstRange::from_position_token(start, end),
            resources_block,
            block,
            cases,
            finally_block,
        },
        pos,
    ))
}
fn parse_method_paramerters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstMethodParamerters, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let mut parameters = vec![];
    let mut pos = pos;
    let mut early_exit = false;
    'l: loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParen) {
            pos = npos;
            early_exit = true;
            break 'l;
        }
        match parse_method_paramerter(tokens, pos) {
            Ok((parameter, npos)) => {
                parameters.push(parameter);
                pos = npos;
            }
            Err(e) => return Err(e),
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
        } else {
            break 'l;
        }
    }
    if !early_exit {
        let npos = assert_token(tokens, pos, Token::RightParen)?;
        pos = npos;
    }
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstMethodParamerters {
            range: AstRange::from_position_token(start, end),
            parameters,
        },
        pos,
    ))
}
fn parse_method_paramerter(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstMethodParamerter, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut fin = false;
    if let Ok(npos) = assert_token(tokens, pos, Token::Final) {
        fin = true;
        pos = npos;
    }
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstMethodParamerter {
            range: AstRange::from_position_token(start, end),
            jtype,
            name,
            fin,
        },
        pos,
    ))
}

/// `thing1`
pub fn parse_name(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let init_pos = pos;
    let mut pos = pos;
    let mut ident = SmolStrBuilder::new();
    loop {
        let Ok(t) = tokens.get(pos).ok_or(AstError::eof()) else {
            break;
        };
        match &t.token {
            Token::Identifier(id) => {
                ident.push_str(id);
                pos += 1;
            }
            Token::Number(id) => {
                ident.push_str(&id.to_string());
                pos += 1;
            }
            _ => {
                if pos == init_pos
                    && let Ok(t) = tokens.get(pos).ok_or(AstError::eof())
                {
                    return Err(AstError::InvalidName(InvalidToken::from(t, pos)));
                }
                break;
            }
        }
    }
    let ident = ident.finish();
    if ident.is_empty() {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t, pos)));
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value: ident.clone(),
        },
        pos,
    ))
}
/// `thing`
pub fn parse_name_single(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut ident = None;
    let t = tokens.get(pos).ok_or(AstError::eof())?;
    match &t.token {
        Token::Identifier(id) => {
            ident = Some(id);
            pos += 1;
        }
        _ => {
            if let Ok(t) = tokens.get(pos).ok_or(AstError::eof()) {
                return Err(AstError::InvalidName(InvalidToken::from(t, pos)));
            }
        }
    }
    let Some(ident) = ident else {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t, pos)));
    };
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value: ident.clone(),
        },
        pos,
    ))
}

// Conatins Token::Identifier, Token::Dot
fn parse_identifier(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut ident = SmolStrBuilder::new();
    let mut modded = false;
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match &t.token {
            Token::Identifier(id) => {
                modded = true;
                ident.push_str(id);
                pos += 1;
            }
            Token::Dot => {
                modded = true;
                ident.push('.');
                pos += 1;
            }
            _ => break,
        }
    }
    if !modded {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t, pos)));
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value: ident.finish(),
        },
        pos,
    ))
}

fn parse_superclass(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSuperClass, usize), AstError> {
    let Ok(pos) = assert_token(tokens, pos, Token::Extends) else {
        return Ok((AstSuperClass::None, pos));
    };
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let sp = match jtype.value {
        AstJTypeKind::Class(c) | AstJTypeKind::Generic(c, _) => AstSuperClass::Name(c),
        _ => AstSuperClass::None,
    };

    Ok((sp, pos))
}

fn parse_jtype(tokens: &[PositionToken], pos: usize) -> Result<(AstJType, usize), AstError> {
    let out_pos = pos + 1;
    let current = tokens.get(pos).ok_or(AstError::eof())?;
    let base = match &current.token {
        Token::Int => Ok((
            AstJType {
                value: AstJTypeKind::Int,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Long => Ok((
            AstJType {
                value: AstJTypeKind::Long,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Short => Ok((
            AstJType {
                value: AstJTypeKind::Short,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Byte => Ok((
            AstJType {
                value: AstJTypeKind::Byte,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Char => Ok((
            AstJType {
                value: AstJTypeKind::Char,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Double => Ok((
            AstJType {
                value: AstJTypeKind::Double,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Float => Ok((
            AstJType {
                value: AstJTypeKind::Float,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Boolean => Ok((
            AstJType {
                value: AstJTypeKind::Boolean,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Void => Ok((
            AstJType {
                value: AstJTypeKind::Void,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::QuestionMark => Ok((
            AstJType {
                value: AstJTypeKind::Wildcard,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Var => Ok((
            AstJType {
                range: AstRange::from_position_token(current, current),
                value: AstJTypeKind::Var,
            },
            out_pos,
        )),
        Token::Identifier(ident) => {
            let token = tokens.get(pos + 1).ok_or(AstError::eof())?;
            let range = AstRange::from_position_token(current, current);
            let ident = AstIdentifier {
                value: ident.clone(),
                range,
            };
            match token.token {
                Token::Lt => {
                    let out_pos;
                    let mut args = vec![];
                    let mut pos = pos + 2;
                    loop {
                        // If there are no type arguments
                        if let Ok(npos) = assert_token(tokens, pos, Token::Gt) {
                            out_pos = npos;
                            break;
                        }
                        if let Ok(npos) = assert_token(tokens, pos, Token::QuestionMark) {
                            let wstart = tokens.get(pos).ok_or(AstError::eof())?;

                            if let Ok(npos) = assert_token(tokens, npos, Token::Implements) {
                                pos = npos;
                                let wend = tokens.get(pos).ok_or(AstError::eof())?;
                                args.push(AstJType {
                                    range: AstRange::from_position_token(wstart, wend),
                                    value: AstJTypeKind::Wildcard,
                                });
                            }
                            if let Ok(npos) = assert_token(tokens, npos, Token::Extends) {
                                pos = npos;
                                let wend = tokens.get(pos).ok_or(AstError::eof())?;
                                args.push(AstJType {
                                    range: AstRange::from_position_token(wstart, wend),
                                    value: AstJTypeKind::Wildcard,
                                });
                            }
                        }
                        let (jtype, npos) = parse_jtype(tokens, pos)?;
                        pos = npos;
                        args.push(jtype);
                        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
                            pos = npos;
                            continue;
                        }
                        if let Ok(npos) = assert_token(tokens, pos, Token::Gt) {
                            out_pos = npos;
                            break;
                        }
                        pos += 1;
                    }
                    let end = tokens.get(pos).ok_or(AstError::eof())?;
                    Ok((
                        AstJType {
                            value: AstJTypeKind::Generic(ident, args),
                            range: AstRange::from_position_token(current, end),
                        },
                        out_pos,
                    ))
                }
                _ => Ok((
                    AstJType {
                        value: AstJTypeKind::Class(ident),
                        range,
                    },
                    out_pos,
                )),
            }
        }
        _ => Err(AstError::InvalidJtype(InvalidToken::from(current, pos))),
    };
    let (base, pos) = base?;

    let mut pos = pos;
    let mut base = base;

    while let Ok(npos) = assert_token(tokens, pos, Token::LeftParenSquare) {
        if let Ok(npos) = assert_token(tokens, npos, Token::RightParenSquare) {
            pos = npos;
            base = AstJType {
                range: AstRange::from_position_token(current, current),
                value: AstJTypeKind::Array(Box::new(base)),
            };
        } else {
            break;
        }
    }
    Ok((base, pos))
}

fn parse_avaliability(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstAvailability, usize), AstError> {
    let Some(token) = tokens.get(pos) else {
        return Err(AstError::eof());
    };
    match token {
        PositionToken {
            token: Token::Public,
            line: _,
            col: _,
        } => Ok((AstAvailability::Public, pos + 1)),
        PositionToken {
            token: Token::Private,
            line: _,
            col: _,
        } => Ok((AstAvailability::Private, pos + 1)),
        PositionToken {
            token: Token::Protected,
            line: _,
            col: _,
        } => Ok((AstAvailability::Protected, pos + 1)),
        _ => Ok((AstAvailability::Undefined, pos)),
    }
}
