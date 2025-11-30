#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
//! A java ast
use annotation::parse_annotation;
use class::parse_class;
use enumeration::parse_enumeration;
use error::{AstError, ExpectedToken, InvalidToken, assert_semicolon, assert_token};
use interface::parse_interface;
use lexer::{PositionToken, Token};
use my_string::MyString;
use types::{
    AstAnnotated, AstAvailability, AstBlock, AstBlockAssign, AstBlockBreak, AstBlockContinue,
    AstBlockEntry, AstBlockExpression, AstBlockReturn, AstBlockVariable, AstBlockVariableMutliType,
    AstBoolean, AstCastedExpression, AstDouble, AstExpression, AstExpressionIdentifier,
    AstExpressionOperator, AstExtends, AstFile, AstFor, AstForEnhanced, AstIdentifier, AstIf,
    AstIfContent, AstImport, AstImportUnit, AstImports, AstInt, AstJType, AstJTypeKind, AstLambda,
    AstLambdaParameters, AstMethodHeader, AstMethodParamerter, AstMethodParamerters, AstNewClass,
    AstPoint, AstRange, AstRecursiveExpression, AstSuperClass, AstSwitch, AstSwitchCase, AstThing,
    AstThrow, AstThrowsDeclaration, AstTryCatch, AstTryCatchCase, AstTypeParameters, AstValue,
    AstValueNuget, AstValues, AstWhile,
};

use crate::{
    class::parse_class_block,
    error::{GetStartEnd, assert_semicolon_options},
    module::parse_module,
    record::parse_record,
    types::{
        AstAnnotatedParameter, AstAnnotatedParameterKind, AstBinaryLiteral, AstBlockAssert,
        AstBlockYield, AstClassAccess, AstConstructorHeader, AstExpressionKind,
        AstExpressionOrDefault, AstExpressionOrValue, AstExpresssionOrAnnotated, AstForContent,
        AstGenerics, AstHexLiteral, AstInlineBlock, AstInstanceOf, AstLambdaParameter,
        AstLambdaRhs, AstNewRhs, AstPackage, AstSwitchCaseArrowContent, AstSwitchCaseArrowDefault,
        AstSwitchCaseArrowType, AstSwitchCaseArrowValues, AstSwitchCaseArrowVar, AstSwitchDefault,
        AstSynchronizedBlock, AstThingAttributes, AstTypeParameter, AstValuesWithAnnotated,
        AstWhileContent,
    },
};

pub mod annotation;
pub mod class;
pub mod enumeration;
pub mod error;
pub mod interface;
pub mod lexer;
pub mod module;
pub mod range;
pub mod record;
pub mod types;

///` package ch.emilycares; import .... public class ...`
pub fn parse_file(tokens: &[PositionToken]) -> Result<AstFile, AstError> {
    let mut pos = 0;
    let mut package = None;
    if let Ok((p, npos)) = parse_package(tokens, pos) {
        package = Some(p);
        pos = npos;
    }
    let mut imports = None;
    if let Ok((imp, npos)) = parse_imports(tokens, pos) {
        imports = Some(imp);
        pos = npos;
    }
    let mut things = vec![];
    let mut modules = vec![];
    let mut errors = vec![];
    while tokens.get(pos).is_some() {
        errors.clear();
        match assert_token(tokens, pos, Token::Semicolon) {
            Ok(npos) => {
                pos = npos;
                continue;
            }
            Err(e) => {
                errors.push(("semicolon".into(), e));
            }
        }
        match parse_thing(tokens, pos) {
            Ok((thing, npos)) => {
                pos = npos;
                things.push(thing);
                continue;
            }
            Err(e) => {
                errors.push(("thing".into(), e));
            }
        }
        match parse_module(tokens, pos) {
            Ok((module, npos)) => {
                pos = npos;
                modules.push(module);
                continue;
            }
            Err(e) => {
                errors.push(("module".into(), e));
            }
        }

        return Err(AstError::AllChildrenFailed {
            parent: "file".into(),
            errors,
        });
    }

    Ok(AstFile {
        package,
        imports,
        things,
        modules,
    })
}

///` package ch.emilycares;`
fn parse_package(tokens: &[PositionToken], pos: usize) -> Result<(AstPackage, usize), AstError> {
    let start = tokens.start(pos)?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Package)?;
    let (name, pos) = parse_name_dot_logical(tokens, pos)?;
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstPackage {
            range: AstRange::from_position_token(start, end),
            annotated,
            name,
        },
        pos,
    ))
}
///`  import java.io.IOException;`
///`  import java.net.Socket;`
fn parse_imports(tokens: &[PositionToken], pos: usize) -> Result<(AstImports, usize), AstError> {
    let mut pos = pos;
    let mut imports = vec![];

    let start = tokens.start(pos)?;
    while let Ok((import, new_pos)) = parse_import(tokens, pos) {
        pos = new_pos;
        imports.push(import);
    }
    let end = tokens.end(pos)?;

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
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Import)?;
    let mut pos = pos;
    let mut is_static = false;
    let mut prefix = false;
    if let Ok(npos) = assert_token(tokens, pos, Token::Static) {
        pos = npos;
        is_static = true;
    }
    let (ident, pos) = parse_name_dot_logical(tokens, pos)?;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Dot)
        && let Ok(npos) = assert_token(tokens, npos, Token::Star)
    {
        pos = npos;
        prefix = true;
    }
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstImport {
            range: AstRange::from_position_token(start, end),
            unit: match (is_static, prefix) {
                (true, true) => AstImportUnit::StaticPrefix(ident),
                (true, false) => match ident.value.rsplit_once('.') {
                    Some((class, method)) => {
                        if method.chars().next().unwrap_or_default().is_lowercase() {
                            AstImportUnit::StaticClassMethod(
                                AstIdentifier {
                                    range: AstRange {
                                        start: ident.range.start,
                                        end: AstPoint {
                                            line: ident.range.start.line,
                                            col: ident.range.end.col - method.len(),
                                        },
                                    },
                                    value: class.into(),
                                },
                                AstIdentifier {
                                    range: AstRange {
                                        start: AstPoint {
                                            line: ident.range.start.line,
                                            col: ident.range.start.col + class.len(),
                                        },
                                        end: ident.range.end,
                                    },
                                    value: method.into(),
                                },
                            )
                        } else {
                            AstImportUnit::StaticClass(ident)
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
pub fn parse_thing(tokens: &[PositionToken], pos: usize) -> Result<(AstThing, usize), AstError> {
    let (annotated, mut pos) = parse_annotated_list(tokens, pos)?;
    let mut avaliability = AstAvailability::empty();
    let mut attributes = AstThingAttributes::empty();
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match t.token {
            Token::Public => avaliability |= AstAvailability::Public,
            Token::Private => avaliability |= AstAvailability::Private,
            Token::Protected => avaliability |= AstAvailability::Protected,
            Token::Static => avaliability |= AstAvailability::Static,
            Token::Final => avaliability |= AstAvailability::Final,
            Token::Abstract => avaliability |= AstAvailability::Abstract,
            Token::StrictFp => avaliability |= AstAvailability::StaticFp,
            Token::Sealed => attributes |= AstThingAttributes::Sealed,
            Token::Non => {
                if let Ok(npos) = assert_token(tokens, pos + 1, Token::Dash)
                    && let Ok(npos) = assert_token(tokens, npos, Token::Sealed)
                {
                    attributes |= AstThingAttributes::NonSealed;
                    pos = npos;
                    continue;
                }
            }
            // Token::At => {
            //     match parse_annotated_list(tokens, pos) {
            //         Ok((an, npos)) => {
            //             pos = npos;
            //             annotated.extend(an);
            //         }
            //         Err(e) => return Err(e),
            //     }
            //     continue;
            // }
            _ => break,
        }
        pos += 1;
    }
    match tokens.get(pos) {
        Some(t) => {
            let pos = pos + 1;
            match t.token {
                Token::Class => parse_class(tokens, pos, avaliability, attributes, annotated),
                Token::Record => parse_record(tokens, pos, avaliability, attributes, annotated),
                Token::Interface => {
                    parse_interface(tokens, pos, avaliability, attributes, annotated)
                }
                Token::Enum => parse_enumeration(tokens, pos, avaliability, attributes, annotated),
                Token::AtInterface => {
                    parse_annotation(tokens, pos, avaliability, attributes, annotated)
                }
                _ => Err(AstError::ExpectedToken(ExpectedToken::from(
                    t,
                    pos,
                    Token::Class,
                ))),
            }
        }
        None => Err(AstError::eof()),
    }
}

fn parse_value(tokens: &[PositionToken], pos: usize) -> Result<(AstValue, usize), AstError> {
    let mut errors = vec![];
    match parse_boolean_literal(tokens, pos) {
        Ok((nuget, pos)) => return Ok((nuget, pos)),
        Err(e) => errors.push(("value boolean".into(), e)),
    }
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
    while assert_token(tokens, pos, Token::At).is_ok() {
        let (a, npos) = parse_annotated(tokens, pos)?;
        out.push(a);
        pos = npos;
    }
    Ok((out, pos))
}
/// `@Overwrite`
/// `@SuppressWarnings({"unchecked", "rawtypes"})`
pub fn parse_annotated(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstAnnotated, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::At)?;
    let (name, pos) = parse_name_dot_logical(tokens, pos)?;
    let mut parameters = AstAnnotatedParameterKind::None;
    let mut pos = pos;
    if let Ok((params, npos)) = parse_annotated_parameters(tokens, pos) {
        parameters = AstAnnotatedParameterKind::Parameter(params);
        pos = npos;
    } else if let Ok((array, npos)) = parse_annotated_array(tokens, pos) {
        parameters = AstAnnotatedParameterKind::Array(array);
        pos = npos;
    }
    let end = tokens.end(pos)?;
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
pub fn parse_lambda(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstLambda, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut pos = pos;
    let parameters;
    'params: {
        let mut errors = vec![];
        match parse_lambda_parameters(tokens, pos) {
            Ok((lparams, npos)) => {
                parameters = lparams;
                pos = npos;
                break 'params;
            }
            Err(e) => {
                errors.push(("lambda parameter".into(), e));
            }
        }
        match parse_name(tokens, pos) {
            Ok((n, npos)) => {
                parameters = AstLambdaParameters {
                    range: n.range,
                    values: vec![AstLambdaParameter {
                        range: n.range,
                        jtype: None,
                        name: n,
                    }],
                };
                pos = npos;
                break 'params;
            }
            Err(e) => {
                errors.push(("lambda name".into(), e));
            }
        }
        return Err(AstError::AllChildrenFailed {
            parent: "lambda parameters".into(),
            errors,
        });
    }
    let mut pos = assert_token(tokens, pos, Token::Arrow)?;
    let mut rhs = AstLambdaRhs::None;
    if let Ok((block, npos)) = parse_block(tokens, pos) {
        pos = npos;
        rhs = AstLambdaRhs::Block(block);
    } else if let Ok((expr, npos)) = parse_expression(tokens, pos, expression_options) {
        pos = npos;
        rhs = AstLambdaRhs::Expr(expr);
    }

    let end = tokens.end(pos)?;
    Ok((
        AstLambda {
            range: AstRange::from_position_token(start, end),
            parameters,
            rhs,
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
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let mut pos = pos;
    let mut values = vec![];

    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParen) {
            pos = npos;
            break;
        }
        let mut jtype = None;
        let start = tokens.start(pos)?;
        let s = pos;
        if let Ok((j, npos)) = parse_jtype(tokens, pos) {
            pos = npos;
            jtype = Some(j);
        }
        let name;
        if let Ok((n, npos)) = parse_name(tokens, pos) {
            pos = npos;
            name = n;
        } else {
            let (n, npos) = parse_name(tokens, s)?;
            name = n;
            jtype = None;
            pos = npos;
        }
        let end = tokens.end(pos)?;
        values.push(AstLambdaParameter {
            range: AstRange::from_position_token(start, end),
            jtype,
            name,
        });

        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
        }
    }
    let end = tokens.end(pos)?;
    Ok((
        AstLambdaParameters {
            range: AstRange::from_position_token(start, end),
            values,
        },
        pos,
    ))
}

/// `{ "", "" }`
fn parse_array(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstValues, usize), AstError> {
    let start = tokens.start(pos)?;
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
        let (value, npos) = parse_expression(tokens, pos, expression_options)?;
        pos = npos;
        values.push(value);
    }
    let end = tokens.end(pos)?;
    Ok((
        AstValues {
            range: AstRange::from_position_token(start, end),
            values,
        },
        pos,
    ))
}
fn parse_array_with_annotated(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstValuesWithAnnotated, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut values = vec![];
    let mut errors = vec![];
    loop {
        errors.clear();
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            break;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
        match parse_expression(tokens, pos, expression_options) {
            Ok((value, npos)) => {
                pos = npos;
                values.push(AstExpresssionOrAnnotated::Expression(value));
                continue;
            }
            Err(e) => errors.push(("expression".into(), e)),
        };
        match parse_annotated(tokens, pos) {
            Ok((an, npos)) => {
                pos = npos;
                values.push(AstExpresssionOrAnnotated::Annotated(an));
                continue;
            }
            Err(e) => errors.push(("annotated".into(), e)),
        };
        return Err(AstError::AllChildrenFailed {
            parent: "array with annotated".into(),
            errors,
        });
    }
    let end = tokens.end(pos)?;
    Ok((
        AstValuesWithAnnotated {
            range: AstRange::from_position_token(start, end),
            values,
        },
        pos,
    ))
}

/// `Ident`
/// `123`
fn parse_value_nuget(tokens: &[PositionToken], pos: usize) -> Result<(AstValue, usize), AstError> {
    let start = tokens.start(pos)?;
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
        Token::HexLiteral(value) => Ok((
            AstValue::Nuget(AstValueNuget::HexLiteral(AstHexLiteral {
                range: AstRange::from_position_token(start, start),
                value: value.clone(),
            })),
            pos + 1,
        )),
        Token::BinaryLiteral(value) => Ok((
            AstValue::Nuget(AstValueNuget::BinaryLiteral(AstBinaryLiteral {
                range: AstRange::from_position_token(start, start),
                value: value.clone(),
            })),
            pos + 1,
        )),
        Token::Number(num) => {
            if let Ok(pos) = assert_token(tokens, pos + 1, Token::Dot) {
                let current = tokens.get(pos).ok_or(AstError::eof())?;
                if let Token::Number(n) = &current.token {
                    let value = format!("{num}.{n}");
                    let pos = pos + 1;
                    let current = tokens.get(pos).ok_or(AstError::eof())?;
                    match &current.token {
                        Token::Identifier(val) if val == "d" => {
                            return Ok((
                                AstValue::Nuget(AstValueNuget::Double(AstDouble {
                                    range: AstRange::from_position_token(start, start),
                                    value,
                                })),
                                pos + 1,
                            ));
                        }
                        Token::Identifier(val) if val == "f" => {
                            return Ok((
                                AstValue::Nuget(AstValueNuget::Float(AstDouble {
                                    range: AstRange::from_position_token(start, start),
                                    value,
                                })),
                                pos + 1,
                            ));
                        }
                        Token::Identifier(val) if val == "l" => {
                            return Ok((
                                AstValue::Nuget(AstValueNuget::Float(AstDouble {
                                    range: AstRange::from_position_token(start, start),
                                    value,
                                })),
                                pos + 1,
                            ));
                        }
                        _ => {
                            return Ok((
                                AstValue::Nuget(AstValueNuget::Float(AstDouble {
                                    range: AstRange::from_position_token(start, start),
                                    value,
                                })),
                                pos,
                            ));
                        }
                    }
                }
            }
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Identifier("l".into())) {
                return Ok((
                    AstValue::Nuget(AstValueNuget::Int(AstInt {
                        range: AstRange::from_position_token(start, start),
                        value: num.clone(),
                    })),
                    npos,
                ));
            }
            Ok((
                AstValue::Nuget(AstValueNuget::Int(AstInt {
                    range: AstRange::from_position_token(start, start),
                    value: num.clone(),
                })),
                pos + 1,
            ))
        }
        Token::StringLiteral(_) => {
            parse_string_literal(tokens, pos).map(|i| (AstValue::Nuget(i.0), i.1))
        }
        Token::CharLiteral(_) => {
            parse_char_literal(tokens, pos).map(|i| (AstValue::Nuget(i.0), i.1))
        }
        Token::True => parse_boolean_literal_input(tokens, pos, true),
        Token::False => parse_boolean_literal_input(tokens, pos, false),
        _ => Err(AstError::InvalidNuget(InvalidToken::from(start, pos))),
    }
}

fn parse_boolean_literal_input(
    tokens: &[PositionToken],
    pos: usize,
    value: bool,
) -> Result<(AstValue, usize), AstError> {
    let start = tokens.start(pos)?;
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
    let start = tokens.start(pos)?;
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
    let start = tokens.start(pos)?;
    match &start.token {
        Token::StringLiteral(str) => {
            let end = tokens.end(pos)?;
            Ok((
                AstValueNuget::StringLiteral(AstIdentifier {
                    range: AstRange::from_position_token(start, end),
                    value: str.clone(),
                }),
                pos + 1,
            ))
        }
        _ => Err(AstError::InvalidString(InvalidToken::from(start, pos))),
    }
}
/// `'\r`
pub fn parse_char_literal(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValueNuget, usize), AstError> {
    let start = tokens.start(pos)?;
    match &start.token {
        Token::CharLiteral(str) => {
            let end = tokens.end(pos)?;
            Ok((
                AstValueNuget::CharLiteral(AstIdentifier {
                    range: AstRange::from_position_token(start, end),
                    value: str.clone(),
                }),
                pos + 1,
            ))
        }
        _ => Err(AstError::InvalidString(InvalidToken::from(start, pos))),
    }
}

fn parse_value_operator_options(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstExpressionOperator, usize), AstError> {
    let start = tokens.start(pos)?;
    match &start.token {
        Token::Plus => {
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Plus) {
                let end = tokens.end(pos)?;
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
                let end = tokens.end(pos)?;
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
                let end = tokens.end(pos)?;
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
                let end = tokens.end(pos)?;
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
        Token::Colon => {
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Colon) {
                let end = tokens.end(pos)?;
                return Ok((
                    AstExpressionOperator::ColonColon(AstRange::from_position_token(start, end)),
                    npos,
                ));
            } else if expression_options != &ExpressionOptions::NoInlineIf {
                return Ok((
                    AstExpressionOperator::Colon(AstRange::from_position_token(start, start)),
                    pos + 1,
                ));
            }
            Err(AstError::InvalidNuget(InvalidToken::from(start, pos)))
        }
        Token::ExclamationMark if expression_options != &ExpressionOptions::NoInlineIf => Ok((
            AstExpressionOperator::ExclemationMark(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Dot => Ok((
            AstExpressionOperator::Dot(AstRange::from_position_token(start, start)),
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
        Token::Equal => Ok((
            AstExpressionOperator::Assign(AstRange::from_position_token(start, start)),
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
        Token::Tilde => Ok((
            AstExpressionOperator::Tilde(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Caret => Ok((
            AstExpressionOperator::Caret(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        _ => Err(AstError::InvalidNuget(InvalidToken::from(start, pos))),
    }
}

fn parse_annotated_array(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValuesWithAnnotated, usize), AstError> {
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (values, pos) = parse_array_with_annotated(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    Ok((values, pos))
}

fn parse_annotated_parameters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstAnnotatedParameter>, usize), AstError> {
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let mut out = vec![];
    let mut errors = vec![];
    let mut pos = pos;
    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParen) {
            pos = npos;
            break;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }

        let start_named = tokens.start(pos)?;
        if let Ok((name, npos)) = parse_name(tokens, pos)
            && let Ok(npos) = assert_token(tokens, npos, Token::Equal)
        {
            match parse_expression(tokens, npos, &ExpressionOptions::None) {
                Ok((expression, npos)) => {
                    pos = npos;
                    let end_named = tokens.end(pos)?;
                    out.push(AstAnnotatedParameter::NamedExpression {
                        range: AstRange::from_position_token(start_named, end_named),
                        name,
                        expression,
                    });
                    continue;
                }
                Err(e) => {
                    errors.push(("named expression".into(), e));
                }
            };
        }
        match parse_expression(tokens, pos, &ExpressionOptions::None) {
            Ok((expression, npos)) => {
                pos = npos;
                out.push(AstAnnotatedParameter::Expression(expression));
                continue;
            }
            Err(e) => {
                errors.push(("exporession".into(), e));
            }
        }
        match parse_annotated(tokens, pos) {
            Ok((ano, npos)) => {
                pos = npos;
                out.push(AstAnnotatedParameter::Annotated(ano));
                continue;
            }
            Err(e) => {
                errors.push(("exporession".into(), e));
            }
        }
        return Err(AstError::AllChildrenFailed {
            parent: "annotated parameters".into(),
            errors,
        });
    }
    Ok((out, pos))
}
fn parse_expression_parameters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstExpression>, usize), AstError> {
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

        let (expression, npos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
        pos = npos;
        out.push(expression);
    }
    Ok((out, pos))
}
fn parse_array_parameters(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(Vec<Vec<AstExpression>>, usize), AstError> {
    let mut out = vec![];
    let mut current = vec![];
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::LeftParenSquare) {
        pos = npos;

        if let Ok((expression, npos)) = parse_expression(tokens, pos, expression_options) {
            pos = npos;
            current.push(expression);
        }
        while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            let (expression, npos) = parse_expression(tokens, pos, expression_options)?;
            pos = npos;
            current.push(expression);
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenSquare) {
            pos = npos;
            out.push(current);
            current = vec![];
        }
    }
    Ok((out, pos))
}

/// `new String()`
pub fn parse_new_class(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstNewClass, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::New)?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let mut pos = pos;
    let mut rhs = AstNewRhs::None;
    let mut errors = vec![];
    match parse_array_parameters(tokens, pos, expression_options) {
        Ok((array_parameters, npos)) => {
            pos = npos;
            if !array_parameters.is_empty() {
                rhs = AstNewRhs::ArrayParameters(array_parameters);
            }
        }
        Err(e) => errors.push(("array_parameters".into(), e)),
    }
    match parse_expression_parameters(tokens, pos) {
        Ok((nrhs, npos)) => {
            pos = npos;
            rhs = AstNewRhs::Parameters(nrhs);
        }
        Err(e) => errors.push(("expression_parameters".into(), e)),
    }
    if jtype.value.is_array() {
        match parse_array(tokens, pos, expression_options) {
            Ok((nrhs, npos)) => {
                pos = npos;
                rhs = AstNewRhs::Array(nrhs);
            }
            Err(e) => errors.push(("array".into(), e)),
        }
    } else {
        match parse_class_block(tokens, pos) {
            Ok((b, npos)) => {
                pos = npos;
                if let AstNewRhs::Parameters(p) = rhs {
                    rhs = AstNewRhs::ParametersAndBlock(p, b);
                } else {
                    rhs = AstNewRhs::Block(b);
                }
            }
            Err(e) => errors.push(("array".into(), e)),
        }
    }
    if matches!(rhs, AstNewRhs::None) {
        return Err(AstError::AllChildrenFailed {
            parent: "new_class".into(),
            errors,
        });
    }

    let end = tokens.end(pos)?;
    Ok((
        AstNewClass {
            range: AstRange::from_position_token(start, end),
            jtype,
            rhs: Box::new(rhs),
        },
        pos,
    ))
}
/// `byte.class`
/// `String.class`
pub fn parse_class_access(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstClassAccess, usize), AstError> {
    let start = tokens.start(pos)?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Dot)?;
    let pos = assert_token(tokens, pos, Token::Class)?;
    let end = tokens.end(pos)?;
    Ok((
        AstClassAccess {
            range: AstRange::from_position_token(start, end),
            jtype,
        },
        pos,
    ))
}
/// Options for expression parsing
#[derive(Debug, PartialEq)]
pub enum ExpressionOptions {
    /// Default expression
    None,
    /// Don't parse 'exp ? expr : expr'
    /// QuestionMark and Colon will not be parsed as operators
    NoInlineIf,
    /// Don't parse labdas
    NoLambda,
}
/// `a.a()`
/// `(byte)'\r`
pub fn parse_expression(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstExpression, usize), AstError> {
    let mut out = vec![];
    let mut pos = pos;
    loop {
        if assert_token(tokens, pos, Token::Semicolon).is_ok() {
            break;
        }
        match parse_expression_inner(tokens, pos, expression_options) {
            Ok((e, npos)) => {
                if e.has_content() {
                    pos = npos;
                    out.push(e);
                } else {
                    break;
                }
            }
            Err(e) => {
                if out.is_empty() {
                    return Err(e);
                }
                break;
            }
        }
    }
    if out.is_empty() {
        let token = tokens.start(pos)?;
        return Err(AstError::EmptyExpression(InvalidToken::from(token, pos)));
    }

    Ok((out, pos))
}
fn parse_expression_inner(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstExpressionKind, usize), AstError> {
    let mut errors = vec![];
    match parse_array(tokens, pos, expression_options) {
        Ok((v, pos)) => return Ok((AstExpressionKind::Array(v), pos)),
        Err(e) => errors.push(("array".into(), e)),
    }
    if expression_options != &ExpressionOptions::NoLambda {
        match parse_lambda(tokens, pos, expression_options) {
            Ok((lambda, pos)) => {
                return Ok((AstExpressionKind::Lambda(lambda), pos));
            }
            Err(e) => errors.push(("lambda".into(), e)),
        }
    }
    match parse_switch(tokens, pos, expression_options) {
        Ok((casted, pos)) => {
            return Ok((AstExpressionKind::InlineSwitch(casted), pos));
        }
        Err(e) => errors.push(("inline switch".into(), e)),
    }
    match parse_casted_expression(tokens, pos) {
        Ok((casted, pos)) => {
            return Ok((AstExpressionKind::Casted(casted), pos));
        }
        Err(e) => errors.push(("casted".into(), e)),
    }
    match parse_new_class(tokens, pos, expression_options) {
        Ok((new, pos)) => return Ok((AstExpressionKind::NewClass(new), pos)),
        Err(e) => errors.push(("new class".into(), e)),
    }
    match parse_class_access(tokens, pos) {
        Ok((a, pos)) => return Ok((AstExpressionKind::ClassAccess(a), pos)),
        Err(e) => errors.push(("class access".into(), e)),
    }
    match parse_jtype_generics(tokens, pos) {
        Ok((a, pos)) => return Ok((AstExpressionKind::Generics(a), pos)),
        Err(e) => errors.push(("type generics".into(), e)),
    }
    match parse_instnceof(tokens, pos) {
        Ok((a, pos)) => return Ok((AstExpressionKind::InstanceOf(a), pos)),
        Err(e) => errors.push(("instanceof".into(), e)),
    }
    match parse_recursive_expression(tokens, pos, expression_options) {
        Ok((recursive, pos)) => {
            return Ok((AstExpressionKind::Recursive(recursive), pos));
        }
        Err(e) => errors.push(("recursive".into(), e)),
    }
    Err(AstError::AllChildrenFailed {
        parent: "expression".into(),
        errors,
    })
}

fn parse_instnceof(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstInstanceOf, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::InstanceOf)?;
    let mut annotated = vec![];
    let mut availability = AstAvailability::empty();
    let mut pos = pos;
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match t.token {
            Token::Final => availability |= AstAvailability::Final,
            Token::At => {
                match parse_annotated_list(tokens, pos) {
                    Ok((an, npos)) => {
                        pos = npos;
                        annotated.extend(an);
                    }
                    Err(e) => return Err(e),
                }
                continue;
            }
            _ => break,
        }
        pos += 1;
    }
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstInstanceOf {
            range: AstRange::from_position_token(start, end),
            annotated,
            availability,
            jtype,
        },
        pos,
    ))
}
fn parse_casted_expression(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstCastedExpression, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (cast, pos) = parse_jtype(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let end = tokens.end(pos)?;
    Ok((
        AstCastedExpression {
            range: AstRange::from_position_token(start, end),
            cast,
        },
        pos,
    ))
}
/// `a.b.c("a".length)`
pub fn parse_recursive_expression(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstRecursiveExpression, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut out = AstRecursiveExpression {
        range: AstRange::from_position_token(start, start),
        ident: None,
        values: None,
        operator: AstExpressionOperator::None,
    };
    let mut pos = pos;
    match start.token {
        Token::Identifier(_) => {
            let (id, npos) = parse_expression_lhs(tokens, pos)?;
            pos = npos;
            out.ident = Some(AstExpressionIdentifier::Identifier(id));
        }
        _ if can_be_ident(&start.token) => {
            let (id, npos) = parse_expression_lhs(tokens, pos)?;
            pos = npos;
            out.ident = Some(AstExpressionIdentifier::Identifier(id));
        }
        Token::LeftParenSquare => {
            pos += 1;
            if let Ok((array_access_expr, npos)) = parse_expression(tokens, pos, expression_options)
            {
                if array_access_expr.is_empty() {
                    out.ident = Some(AstExpressionIdentifier::EmptyArrayAccess);
                } else {
                    out.ident = Some(AstExpressionIdentifier::ArrayAccess(array_access_expr));
                }
                pos = npos;
            } else {
                out.ident = Some(AstExpressionIdentifier::EmptyArrayAccess);
            }
            let npos = assert_token(tokens, pos, Token::RightParenSquare)?;
            pos = npos;
        }
        Token::LeftParen => {
            let values_start = tokens.get(pos).ok_or(AstError::eof())?;
            let (vals, npos) = parse_expression_parameters(tokens, pos)?;
            pos = npos;
            let values_end = tokens.get(pos - 1).ok_or(AstError::eof())?;
            out.values = Some(types::AstValues {
                range: AstRange::from_position_token(values_start, values_end),
                values: vals,
            });
        }
        _ => {
            let mut errors = vec![];
            'others: {
                match parse_value_operator_options(tokens, pos, expression_options) {
                    Ok((op, npos)) => {
                        pos = npos;
                        out.operator = op;
                        break 'others;
                    }
                    Err(e) => errors.push(("operator".into(), e)),
                }
                match parse_value(tokens, pos) {
                    Ok((value, npos)) => {
                        pos = npos;
                        out.ident = Some(AstExpressionIdentifier::Value(value));
                        break 'others;
                    }
                    Err(e) => errors.push(("value".into(), e)),
                }
                return Err(AstError::AllChildrenFailed {
                    parent: "expression".into(),
                    errors,
                });
            }
        }
    }
    let end = tokens.end(pos)?;
    out.range = AstRange::from_position_token(start, end);
    Ok((out, pos))
}

fn parse_expression_lhs(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    match parse_name(tokens, pos) {
        Ok((ident, npos)) => Ok((ident, npos)),
        Err(AstError::InvalidName(e)) => match e.found {
            Token::Class => {
                let start = tokens.start(pos)?;
                Ok((
                    AstIdentifier {
                        range: AstRange::from_position_token(start, start),
                        value: "class".into(),
                    },
                    pos + 1,
                ))
            }
            Token::This => {
                let start = tokens.start(pos)?;
                Ok((
                    AstIdentifier {
                        range: AstRange::from_position_token(start, start),
                        value: "this".into(),
                    },
                    pos + 1,
                ))
            }
            Token::New => {
                let start = tokens.start(pos)?;
                Ok((
                    AstIdentifier {
                        range: AstRange::from_position_token(start, start),
                        value: "new".into(),
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
) -> Result<(Vec<AstBlockVariable>, usize), AstError> {
    parse_block_variable_options(tokens, pos, &BlockEntryOptions::None)
}
fn parse_block_variable_options(
    tokens: &[PositionToken],
    pos: usize,
    block_entry_options: &BlockEntryOptions,
) -> Result<(Vec<AstBlockVariable>, usize), AstError> {
    let (block_variable, pos) = parse_block_variable_no_semicolon(tokens, pos)?;
    let pos = assert_semicolon_options(tokens, pos, block_entry_options)?;
    Ok((block_variable, pos))
}
fn parse_block_variable_no_semicolon(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstBlockVariable>, usize), AstError> {
    let start = tokens.start(pos)?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut pos = pos;
    let mut fin = false;
    if let Ok(npos) = assert_token(tokens, pos, Token::Final) {
        pos = npos;
        fin = true;
    }
    let mut out = vec![];
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (v, pos) = parse_variable_base(tokens, start, &annotated, fin, &jtype, pos)?;
    out.push(v);
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
        let (v, npos) = parse_variable_base(tokens, start, &annotated, fin, &jtype, npos)?;
        pos = npos;
        out.push(v);
    }

    Ok((out, pos))
}

fn parse_variable_base(
    tokens: &[PositionToken],
    start: &PositionToken,
    annotated: &[AstAnnotated],
    fin: bool,
    jtype: &AstJType,
    pos: usize,
) -> Result<(AstBlockVariable, usize), AstError> {
    let mut jtype = jtype.clone();
    let (name, pos) = parse_name(tokens, pos)?;
    let mut pos = parse_array_type_on_name(tokens, pos, &mut jtype);
    let mut value = None;
    if let Ok(npos) = assert_token(tokens, pos, Token::Equal) {
        pos = npos;
        // optional when typing `var = `
        if let Ok((aexpression, npos)) = parse_expression(tokens, npos, &ExpressionOptions::None) {
            pos = npos;
            value = Some(aexpression);
        }
    }
    let end = tokens.end(pos)?;
    Ok((
        AstBlockVariable {
            range: AstRange::from_position_token(start, end),
            fin,
            annotated: annotated.to_owned(),
            jtype,
            name,
            value,
        },
        pos,
    ))
}

fn parse_block_variable_multi_type_no_semicolon(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockVariableMutliType, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut fin = false;
    let (mut annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut pos = pos;
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match t.token {
            Token::Final => fin = true,
            Token::At => {
                let (an, npos) = parse_annotated(tokens, pos)?;
                annotated.push(an);
                pos = npos;
                continue;
            }
            _ => break,
        }
        pos += 1;
    }
    let mut jtypes = vec![];
    loop {
        let (jtype, npos) = parse_jtype(tokens, pos)?;
        jtypes.push(jtype);
        pos = npos;
        if let Ok(npos) = assert_token(tokens, pos, Token::VerticalBar) {
            pos = npos;
        } else {
            break;
        }
    }
    let (name, pos) = parse_name(tokens, pos)?;
    let mut expression = None;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Equal) {
        let (aexpression, npos) = parse_expression(tokens, npos, &ExpressionOptions::None)?;
        pos = npos;
        expression = Some(aexpression);
    }
    let end = tokens.end(pos)?;

    Ok((
        AstBlockVariableMutliType {
            name,
            fin,
            jtypes,
            expression,
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}

/// `return 1;`
pub fn parse_block_return(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockReturn, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Return)?;
    let mut pos = pos;
    let mut expression = AstExpressionOrValue::None;
    if let Ok(npos) = assert_token(tokens, pos, Token::Semicolon) {
        pos = npos;
    } else {
        let (nexpression, npos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
        pos = npos;
        expression = AstExpressionOrValue::Expression(nexpression);
    }
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.end(pos)?;

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
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Yield)?;
    let mut pos = pos;
    let mut expression = AstExpressionOrValue::None;
    if let Ok((nexpression, npos)) = parse_expression(tokens, pos, &ExpressionOptions::None) {
        pos = npos;
        expression = AstExpressionOrValue::Expression(nexpression);
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.end(pos)?;
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
    let start = tokens.start(pos)?;
    let mut pos = assert_token(tokens, pos, Token::Break)?;
    let mut label = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        label = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.end(pos)?;

    Ok((
        AstBlockBreak {
            range: AstRange::from_position_token(start, end),
            label,
        },
        pos,
    ))
}
fn parse_block_assert(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockAssert, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Assert)?;
    let (expression, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.end(pos)?;

    Ok((
        AstBlockAssert {
            range: AstRange::from_position_token(start, end),
            expression,
        },
        pos,
    ))
}
fn parse_block_continue(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockContinue, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut pos = assert_token(tokens, pos, Token::Continue)?;
    let mut label = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        label = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.end(pos)?;

    Ok((
        AstBlockContinue {
            range: AstRange::from_position_token(start, end),
            label,
        },
        pos,
    ))
}
fn parse_block_expression_options(
    tokens: &[PositionToken],
    pos: usize,
    block_entry_options: &BlockEntryOptions,
) -> Result<(AstBlockExpression, usize), AstError> {
    let start = tokens.start(pos)?;
    let (value, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_semicolon_options(tokens, pos, block_entry_options)?;
    let end = tokens.end(pos)?;

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
    block_entry_options: &BlockEntryOptions,
) -> Result<(AstBlockAssign, usize), AstError> {
    let start = tokens.start(pos)?;

    let (key, pos) = parse_recursive_expression(tokens, pos, &ExpressionOptions::None)?;
    let key = vec![AstExpressionKind::Recursive(key)];
    let pos = assert_token(tokens, pos, Token::Equal)?;
    let (expression, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_semicolon_options(tokens, pos, block_entry_options)?;
    let end = tokens.end(pos)?;

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
    let mut avaliability = default_availability;
    let mut type_parameters = None;
    let start = tokens.start(pos)?;
    let (mut annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut pos = pos;
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match t.token {
            Token::Public => avaliability |= AstAvailability::Public,
            Token::Private => avaliability |= AstAvailability::Private,
            Token::Protected => avaliability |= AstAvailability::Protected,
            Token::Synchronized => avaliability |= AstAvailability::Synchronized,
            Token::Static => avaliability |= AstAvailability::Static,
            Token::Final => avaliability |= AstAvailability::Final,
            Token::Abstract => avaliability |= AstAvailability::Abstract,
            Token::Native => avaliability |= AstAvailability::Native,
            Token::At => {
                let (an, npos) = parse_annotated(tokens, pos)?;
                annotated.push(an);
                pos = npos;
                continue;
            }
            _ => break,
        }
        pos += 1;
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
    let end = tokens.end(pos)?;
    Ok((
        AstMethodHeader {
            range: AstRange::from_position_token(start, end),
            avaliability,
            type_parameters,
            name,
            annotated,
            jtype,
            parameters,
            throws,
        },
        pos,
    ))
}

fn parse_variadic(tokens: &[PositionToken], pos: usize) -> Result<usize, AstError> {
    let pos = assert_token(tokens, pos, Token::Dot)?;
    let pos = assert_token(tokens, pos, Token::Dot)?;
    let pos = assert_token(tokens, pos, Token::Dot)?;
    Ok(pos)
}

/// `byte b[]` is modified to be the correct jtype
fn parse_array_type_on_name(tokens: &[PositionToken], pos: usize, jtype: &mut AstJType) -> usize {
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::LeftParenSquare)
        && let Ok(npos) = assert_token(tokens, npos, Token::RightParenSquare)
    {
        pos = npos;
        let orig = jtype.clone();
        *jtype = AstJType {
            range: jtype.range,
            value: AstJTypeKind::Array(Box::new(orig)),
        };
    }
    pos
}
fn parse_constructor_header(
    tokens: &[PositionToken],
    pos: usize,
    default_availability: AstAvailability,
) -> Result<(AstConstructorHeader, usize), AstError> {
    let mut avaliability = default_availability;
    let mut type_parameters = None;
    let start = tokens.start(pos)?;
    let (mut annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut pos = pos;
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match t.token {
            Token::Public => avaliability |= AstAvailability::Public,
            Token::Private => avaliability |= AstAvailability::Private,
            Token::Protected => avaliability |= AstAvailability::Protected,
            Token::Synchronized => avaliability |= AstAvailability::Synchronized,
            Token::Static => avaliability |= AstAvailability::Static,
            Token::Final => avaliability |= AstAvailability::Final,
            Token::At => {
                let (annotated_after, npos) = parse_annotated_list(tokens, pos)?;
                pos = npos;
                annotated.extend(annotated_after);
                continue;
            }
            _ => break,
        }
        pos += 1;
    }

    if let Ok((type_params, npos)) = parse_type_parameters(tokens, pos) {
        type_parameters = Some(type_params);
        pos = npos;
    }

    let (name, pos) = parse_name(tokens, pos)?;
    let (parameters, pos) = parse_constructor_paramerters(tokens, pos)?;
    let mut pos = pos;
    let mut throws = None;
    if let Ok((nthrows, npos)) = parse_throws_declaration(tokens, pos) {
        throws = Some(nthrows);
        pos = npos;
    }
    let end = tokens.end(pos)?;
    Ok((
        AstConstructorHeader {
            range: AstRange::from_position_token(start, end),
            avaliability,
            type_parameters,
            name,
            parameters,
            throws,
            annotated,
        },
        pos,
    ))
}
fn parse_throws_declaration(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstThrowsDeclaration, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Throws)?;
    let (parameters, pos) = parse_type_list(tokens, pos)?;
    let end = tokens.end(pos)?;
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
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Lt)?;
    let mut pos = pos;
    let mut parameters = vec![];
    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::Gt) {
            pos = npos;
            break;
        }

        let start_p = tokens.start(pos)?;
        let (annotated, npos) = parse_annotated_list(tokens, pos)?;
        pos = npos;
        let (name, npos) = parse_name(tokens, pos)?;
        pos = npos;
        let mut supperclass = None;
        if let Ok((s, npos)) = parse_superclass(tokens, pos) {
            supperclass = Some(s);
            pos = npos;
        }
        let end_p = tokens.end(pos)?;
        parameters.push(AstTypeParameter {
            range: AstRange::from_position_token(start_p, end_p),
            annotated,
            name,
            supperclass,
        });
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Gt) {
            pos = npos;
            break;
        }
    }
    let end = tokens.end(pos)?;
    Ok((
        AstTypeParameters {
            range: AstRange::from_position_token(start, end),
            parameters,
        },
        pos,
    ))
}

fn parse_extends(tokens: &[PositionToken], pos: usize) -> Result<(AstExtends, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Extends)?;
    let (parameters, pos) = parse_type_list(tokens, pos)?;
    let end = tokens.end(pos)?;
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
        }
    }
    Ok((parameters, pos))
}

/// { statements; }
pub fn parse_block(tokens: &[PositionToken], pos: usize) -> Result<(AstBlock, usize), AstError> {
    parse_block_brackets(tokens, pos, Token::LeftParenCurly, &Token::RightParenCurly)
}
fn parse_block_brackets(
    tokens: &[PositionToken],
    pos: usize,
    left: Token,
    right: &Token,
) -> Result<(AstBlock, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, left)?;
    let mut pos = pos;
    let mut entries = vec![];
    let mut start_pos;
    loop {
        if let Ok(npos) = assert_token(tokens, pos, right.clone()) {
            pos = npos;
            break;
        }
        start_pos = pos;
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                entries.push(entry);
                pos = npos;
            }
            Err(e) => {
                return Err(e);
            }
        }
        if pos == start_pos {
            eprintln!("No block enty was parsed: {:?}", tokens.get(pos));
            break;
        }
    }
    let end = tokens.end(pos)?;
    Ok((
        AstBlock {
            range: AstRange::from_position_token(start, end),
            entries,
        },
        pos,
    ))
}

/// Options for expression parsing
#[derive(Debug, PartialEq)]
pub enum BlockEntryOptions {
    /// Default expression
    None,
    /// Don't parse `;`
    NoSemicolon,
}
fn parse_block_entry(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockEntry, usize), AstError> {
    parse_block_entry_options(tokens, pos, &BlockEntryOptions::None)
}
fn parse_block_entry_options(
    tokens: &[PositionToken],
    pos: usize,
    block_entry_options: &BlockEntryOptions,
) -> Result<(AstBlockEntry, usize), AstError> {
    let mut errors = vec![];
    match assert_token(tokens, pos, Token::Semicolon) {
        Ok(pos) => {
            let start = tokens.start(pos - 1)?;
            return Ok((
                AstBlockEntry::Semicolon(AstRange::from_position_token(start, start)),
                pos,
            ));
        }
        Err(e) => {
            errors.push(("semicolon".into(), e));
        }
    }
    match parse_inline_block(tokens, pos) {
        Ok((block, pos)) => {
            return Ok((AstBlockEntry::InlineBlock(block), pos));
        }
        Err(e) => {
            errors.push(("block block".into(), e));
        }
    }
    match parse_block_variable_options(tokens, pos, block_entry_options) {
        Ok((vars, pos)) => {
            return Ok((AstBlockEntry::Variable(vars), pos));
        }
        Err(e) => {
            errors.push(("block variable".into(), e));
        }
    }
    match parse_block_return(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Return(nret), pos));
        }
        Err(e) => {
            errors.push(("block return".into(), e));
        }
    }
    match parse_block_yield(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Yield(nret), pos));
        }
        Err(e) => {
            errors.push(("block yield".into(), e));
        }
    }
    match parse_block_break(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Break(nret), pos));
        }
        Err(e) => {
            errors.push(("block break".into(), e));
        }
    }
    match parse_block_assert(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Assert(nret), pos));
        }
        Err(e) => {
            errors.push(("block assert".into(), e));
        }
    }
    match parse_block_continue(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Continue(nret), pos));
        }
        Err(e) => {
            errors.push(("block continue".into(), e));
        }
    }
    match parse_if(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::If(nret), pos));
        }
        Err(e) => {
            errors.push(("block if".into(), e));
        }
    }
    match parse_else_if(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::If(nret), pos));
        }
        Err(e) => {
            errors.push(("block if".into(), e));
        }
    }
    match parse_else(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::If(nret), pos));
        }
        Err(e) => {
            errors.push(("block if".into(), e));
        }
    }
    match parse_while(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::While(nret), pos));
        }
        Err(e) => {
            errors.push(("block while".into(), e));
        }
    }
    match parse_do_while(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::While(nret), pos));
        }
        Err(e) => {
            errors.push(("block do while".into(), e));
        }
    }
    match parse_for(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::For(Box::new(nret)), pos));
        }
        Err(e) => {
            errors.push(("block for".into(), e));
        }
    }
    match parse_for_enhanced(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::ForEnhanced(Box::new(nret)), pos));
        }
        Err(e) => {
            errors.push(("block for enhanced".into(), e));
        }
    }
    match parse_switch(tokens, pos, &ExpressionOptions::None) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Switch(nret), pos));
        }
        Err(e) => {
            errors.push(("block switch".into(), e));
        }
    }
    match parse_switch_case(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::SwitchCase(nret), pos));
        }
        Err(e) => {
            errors.push(("block switch case".into(), e));
        }
    }
    match parse_switch_default(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::SwitchDefault(nret), pos));
        }
        Err(e) => {
            errors.push(("block switch default".into(), e));
        }
    }
    match parse_switch_case_arrow_value(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::SwitchCaseArrowValues(nret), pos));
        }
        Err(e) => {
            errors.push(("block switch case arrow".into(), e));
        }
    }
    match parse_switch_case_arrow_type(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::SwitchCaseArrowType(nret), pos));
        }
        Err(e) => {
            errors.push(("block switch case arrow type".into(), e));
        }
    }
    match parse_switch_case_arrow_default(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::SwitchCaseArrowDefault(nret), pos));
        }
        Err(e) => {
            errors.push(("block switch case arrow".into(), e));
        }
    }
    match parse_try_catch(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::TryCatch(nret), pos));
        }
        Err(e) => {
            errors.push(("block try catch".into(), e));
        }
    }
    match parse_throw(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Throw(nret), pos));
        }
        Err(e) => {
            errors.push(("block throw".into(), e));
        }
    }
    match parse_synchronised_block(tokens, pos) {
        Ok((synchronized_block, pos)) => {
            return Ok((AstBlockEntry::SynchronizedBlock(synchronized_block), pos));
        }
        Err(e) => {
            errors.push(("static block".into(), e));
        }
    }
    match parse_block_assign(tokens, pos, block_entry_options) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Assign(Box::new(nret)), pos));
        }
        Err(e) => {
            errors.push(("block assign".into(), e));
        }
    }
    match parse_thing(tokens, pos) {
        Ok((thing, pos)) => {
            return Ok((AstBlockEntry::Thing(Box::new(thing)), pos));
        }
        Err(e) => {
            errors.push(("class thing".into(), e));
        }
    }
    match parse_block_expression_options(tokens, pos, block_entry_options) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Expression(nret), pos));
        }
        Err(e) => {
            errors.push(("block expression".into(), e));
        }
    }
    Err(AstError::AllChildrenFailed {
        parent: "block".into(),
        errors,
    })
}

fn parse_inline_block(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstInlineBlock, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut label = None;
    let mut pos = pos;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        label = Some(lab);
        pos = npos;
    }
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstInlineBlock {
            range: AstRange::from_position_token(start, end),
            label,
            block,
        },
        pos,
    ))
}
fn parse_block_entry_minimal_options(
    tokens: &[PositionToken],
    pos: usize,
    block_entry_options: &BlockEntryOptions,
) -> Result<(AstBlockEntry, usize), AstError> {
    let mut errors = vec![];
    match parse_block_variable_options(tokens, pos, block_entry_options) {
        Ok((variable, pos)) => {
            return Ok((AstBlockEntry::Variable(variable), pos));
        }
        Err(e) => {
            errors.push(("block variable".into(), e));
        }
    }
    match parse_block_assign(tokens, pos, block_entry_options) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Assign(Box::new(nret)), pos));
        }
        Err(e) => {
            errors.push(("block assign".into(), e));
        }
    }
    match parse_block_expression_options(tokens, pos, block_entry_options) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Expression(nret), pos));
        }
        Err(e) => {
            errors.push(("block expression".into(), e));
        }
    }
    Err(AstError::AllChildrenFailed {
        parent: "block".into(),
        errors,
    })
}

fn parse_while(tokens: &[PositionToken], pos: usize) -> Result<(AstWhile, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut pos = pos;
    let mut label = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        label = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::While)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (control, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let mut pos = assert_token(tokens, pos, Token::RightParen)?;
    let mut content = AstWhileContent::None;
    let mut errors = vec![];
    'while_content: {
        match assert_token(tokens, pos, Token::Semicolon) {
            Ok(npos) => {
                pos = npos;
                break 'while_content;
            }
            Err(e) => errors.push(("semicolon".into(), e)),
        }
        match parse_block(tokens, pos) {
            Ok((block, npos)) => {
                content = AstWhileContent::Block(block);
                pos = npos;
                break 'while_content;
            }
            Err(e) => errors.push(("block".into(), e)),
        }
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                content = AstWhileContent::BlockEntry(Box::new(entry));
                pos = npos;
                break 'while_content;
            }
            Err(e) => errors.push(("recursive expression".into(), e)),
        }
        return Err(AstError::AllChildrenFailed {
            parent: "while".into(),
            errors,
        });
    }
    let end = tokens.end(pos)?;
    Ok((
        AstWhile {
            range: AstRange::from_position_token(start, end),
            control,
            content,
            label,
        },
        pos,
    ))
}
fn parse_do_while(tokens: &[PositionToken], pos: usize) -> Result<(AstWhile, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut pos = pos;
    let mut label = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        label = Some(lab);
        pos = npos;
    }
    let mut pos = assert_token(tokens, pos, Token::Do)?;
    let content;
    let mut errors = vec![];
    'do_while_content: {
        match parse_block(tokens, pos) {
            Ok((block, npos)) => {
                content = AstWhileContent::Block(block);
                pos = npos;
                break 'do_while_content;
            }
            Err(e) => errors.push(("block".into(), e)),
        }
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                content = AstWhileContent::BlockEntry(Box::new(entry));
                pos = npos;
                break 'do_while_content;
            }
            Err(e) => errors.push(("block entry".into(), e)),
        }
        return Err(AstError::AllChildrenFailed {
            parent: "do while".into(),
            errors,
        });
    }

    let pos = assert_token(tokens, pos, Token::While)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (control, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.end(pos)?;
    Ok((
        AstWhile {
            range: AstRange::from_position_token(start, end),
            control,
            content,
            label,
        },
        pos,
    ))
}
/// `for(;;) { ... }`
pub fn parse_for(tokens: &[PositionToken], pos: usize) -> Result<(AstFor, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut pos = pos;
    let mut label = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        label = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::For)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let mut vars = vec![];
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Semicolon) {
        pos = npos;
    } else {
        let (v, npos) = parse_comma_separated_block_entry(tokens, pos)?;
        let npos = assert_token(tokens, npos, Token::Semicolon)?;
        vars = v;
        pos = npos;
    }
    let mut check = vec![];
    if let Ok(npos) = assert_token(tokens, pos, Token::Semicolon) {
        pos = npos;
    } else {
        let (c, npos) = parse_comma_separated_block_entry(tokens, pos)?;
        let npos = assert_token(tokens, npos, Token::Semicolon)?;
        check = c;
        pos = npos;
    }
    let mut changes = vec![];
    if let Ok(npos) = assert_token(tokens, pos, Token::RightParen) {
        pos = npos;
    } else {
        let (c, npos) = parse_comma_separated_block_entry(tokens, pos)?;
        let npos = assert_token(tokens, npos, Token::RightParen)?;
        changes = c;
        pos = npos;
    }
    let mut content = AstForContent::None;
    let mut errors = vec![];
    'for_content: {
        match assert_token(tokens, pos, Token::Semicolon) {
            Ok(npos) => {
                pos = npos;
                break 'for_content;
            }
            Err(e) => errors.push(("semicolon".into(), e)),
        }
        match parse_block(tokens, pos) {
            Ok((block, npos)) => {
                content = AstForContent::Block(block);
                pos = npos;
                break 'for_content;
            }
            Err(e) => errors.push(("block".into(), e)),
        }
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                content = AstForContent::BlockEntry(Box::new(entry));
                pos = npos;
                break 'for_content;
            }
            Err(e) => errors.push(("recursive expression".into(), e)),
        }
        return Err(AstError::AllChildrenFailed {
            parent: "for".into(),
            errors,
        });
    }
    let end = tokens.end(pos)?;
    Ok((
        AstFor {
            range: AstRange::from_position_token(start, end),
            content,
            label,
            vars,
            check,
            changes,
        },
        pos,
    ))
}

fn parse_comma_separated_block_entry(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstBlockEntry>, usize), AstError> {
    let mut vars = vec![];
    let options = BlockEntryOptions::NoSemicolon;
    let (e, pos) = parse_block_entry_minimal_options(tokens, pos, &options)?;
    vars.push(e);
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
        let (e, npos) = parse_block_entry_minimal_options(tokens, npos, &options)?;
        vars.push(e);
        pos = npos;
    }
    Ok((vars, pos))
}

fn parse_switch(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstSwitch, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Switch)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (check, pos) = parse_expression(tokens, pos, expression_options)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.end(pos)?;
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
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Case)?;
    let mut expressions = vec![];
    let (expression, mut pos) =
        parse_default_or_expression(tokens, pos, &ExpressionOptions::NoInlineIf)?;
    expressions.push(expression);
    while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
        let (expression, npos) =
            parse_default_or_expression(tokens, npos, &ExpressionOptions::NoInlineIf)?;
        expressions.push(expression);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::Colon)?;
    let end = tokens.end(pos)?;
    Ok((
        AstSwitchCase {
            range: AstRange::from_position_token(start, end),
            expressions,
        },
        pos,
    ))
}

fn parse_default_or_expression(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstExpressionOrDefault, usize), AstError> {
    if let Ok(pos) = assert_token(tokens, pos, Token::Default) {
        return Ok((AstExpressionOrDefault::Default, pos));
    }
    let (expression, pos) = parse_expression(tokens, pos, expression_options)?;
    Ok((AstExpressionOrDefault::Expression(expression), pos))
}
/// case 1 -> print(1);
pub fn parse_switch_case_arrow_value(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchCaseArrowValues, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut pos = assert_token(tokens, pos, Token::Case)?;
    let mut values = vec![];
    loop {
        let (value, npos) = parse_default_or_expression(tokens, pos, &ExpressionOptions::NoLambda)?;
        values.push(value);
        pos = npos;

        match assert_token(tokens, pos, Token::Comma) {
            Ok(npos) => {
                pos = npos;
            }
            Err(_) => break,
        }
    }
    let pos = assert_token(tokens, pos, Token::Arrow)?;
    let (content, pos) = parse_switch_case_arrow_content(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstSwitchCaseArrowValues {
            range: AstRange::from_position_token(start, end),
            values,
            content: Box::new(content),
        },
        pos,
    ))
}
/// case Long a -> print(a);
pub fn parse_switch_case_arrow_type(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchCaseArrowType, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Case)?;
    let (var, pos) = parse_arrow_var(tokens, pos)?;

    let pos = assert_token(tokens, pos, Token::Arrow)?;
    let (content, pos) = parse_switch_case_arrow_content(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstSwitchCaseArrowType {
            range: AstRange::from_position_token(start, end),
            var,
            content: Box::new(content),
        },
        pos,
    ))
}

fn parse_arrow_var(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchCaseArrowVar, usize), AstError> {
    let start = tokens.start(pos)?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstSwitchCaseArrowVar {
            range: AstRange::from_position_token(start, end),
            jtype,
            name,
        },
        pos,
    ))
}
fn parse_switch_case_arrow_default(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchCaseArrowDefault, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Default)?;
    let pos = assert_token(tokens, pos, Token::Arrow)?;
    let (content, pos) = parse_switch_case_arrow_content(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstSwitchCaseArrowDefault {
            range: AstRange::from_position_token(start, end),
            content: Box::new(content),
        },
        pos,
    ))
}

fn parse_switch_case_arrow_content(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchCaseArrowContent, usize), AstError> {
    let content;
    let mut pos = pos;
    if assert_token(tokens, pos, Token::LeftParenCurly).is_ok() {
        let (block, npos) = parse_block(tokens, pos)?;
        content = AstSwitchCaseArrowContent::Block(block);
        pos = npos;
    } else {
        let (entry, npos) = parse_block_entry(tokens, pos)?;
        content = AstSwitchCaseArrowContent::Entry(Box::new(entry));
        pos = npos;
    }
    Ok((content, pos))
}
fn parse_switch_default(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchDefault, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Default)?;
    let pos = assert_token(tokens, pos, Token::Colon)?;
    let end = tokens.end(pos)?;
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
    let start = tokens.start(pos)?;
    let mut pos = pos;
    let mut label = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        label = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::For)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (var, pos) = parse_block_variable_no_semicolon(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Colon)?;
    let (rhs, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let content;
    let mut errors = vec![];
    let mut pos = pos;
    'for_content: {
        match parse_block(tokens, pos) {
            Ok((block, npos)) => {
                content = AstForContent::Block(block);
                pos = npos;
                break 'for_content;
            }
            Err(e) => errors.push(("block".into(), e)),
        }
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                content = AstForContent::BlockEntry(Box::new(entry));
                pos = npos;
                break 'for_content;
            }
            Err(e) => errors.push(("recursive expression".into(), e)),
        }
        return Err(AstError::AllChildrenFailed {
            parent: "for".into(),
            errors,
        });
    }
    let end = tokens.end(pos)?;
    Ok((
        AstForEnhanced {
            range: AstRange::from_position_token(start, end),
            content,
            label,
            var,
            rhs,
        },
        pos,
    ))
}

fn parse_if(tokens: &[PositionToken], pos: usize) -> Result<(AstIf, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::If)?;
    let start_control = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (control, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let end_control = tokens.end(pos)?;
    let mut pos = pos;
    let content;
    let mut errors = vec![];
    'if_content: {
        match parse_block(tokens, pos) {
            Ok((block, npos)) => {
                content = AstIfContent::Block(block);
                pos = npos;
                break 'if_content;
            }
            Err(e) => errors.push(("block".into(), e)),
        }
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                content = AstIfContent::BlockEntry(Box::new(entry));
                pos = npos;
                break 'if_content;
            }
            Err(e) => errors.push(("recursive expression".into(), e)),
        }
        return Err(AstError::AllChildrenFailed {
            parent: "if".into(),
            errors,
        });
    }
    let end = tokens.end(pos)?;
    Ok((
        AstIf::If {
            range: AstRange::from_position_token(start, end),
            control,
            control_range: AstRange::from_position_token(start_control, end_control),
            content,
        },
        pos,
    ))
}
fn parse_else_if(tokens: &[PositionToken], pos: usize) -> Result<(AstIf, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Else)?;
    let pos = assert_token(tokens, pos, Token::If)?;
    let start_control = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (control, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let end_control = tokens.end(pos)?;
    let mut pos = pos;
    let content;
    let mut errors = vec![];
    'if_content: {
        match parse_block(tokens, pos) {
            Ok((block, npos)) => {
                content = AstIfContent::Block(block);
                pos = npos;
                break 'if_content;
            }
            Err(e) => errors.push(("block".into(), e)),
        }
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                content = AstIfContent::BlockEntry(Box::new(entry));
                pos = npos;
                break 'if_content;
            }
            Err(e) => errors.push(("recursive expression".into(), e)),
        }
        return Err(AstError::AllChildrenFailed {
            parent: "if".into(),
            errors,
        });
    }
    let end = tokens.end(pos)?;
    Ok((
        AstIf::ElseIf {
            range: AstRange::from_position_token(start, end),
            control,
            control_range: AstRange::from_position_token(start_control, end_control),
            content,
        },
        pos,
    ))
}
fn parse_else(tokens: &[PositionToken], pos: usize) -> Result<(AstIf, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Else)?;
    let mut pos = pos;
    let content;
    let mut errors = vec![];
    'if_content: {
        match parse_block(tokens, pos) {
            Ok((block, npos)) => {
                content = AstIfContent::Block(block);
                pos = npos;
                break 'if_content;
            }
            Err(e) => errors.push(("block".into(), e)),
        }
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                content = AstIfContent::BlockEntry(Box::new(entry));
                pos = npos;
                break 'if_content;
            }
            Err(e) => errors.push(("recursive expression".into(), e)),
        }
        return Err(AstError::AllChildrenFailed {
            parent: "if".into(),
            errors,
        });
    }
    let end = tokens.end(pos)?;
    Ok((
        AstIf::Else {
            range: AstRange::from_position_token(start, end),
            content,
        },
        pos,
    ))
}
fn parse_throw(tokens: &[PositionToken], pos: usize) -> Result<(AstThrow, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Throw)?;
    let (expression, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstThrow {
            range: AstRange::from_position_token(start, end),
            expression,
        },
        pos,
    ))
}
fn parse_synchronised_block(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSynchronizedBlock, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Synchronized)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (expression, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstSynchronizedBlock {
            range: AstRange::from_position_token(start, end),
            expression,
            block,
        },
        pos,
    ))
}

fn parse_try_catch(tokens: &[PositionToken], pos: usize) -> Result<(AstTryCatch, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Try)?;
    let mut resources_block = None;
    let mut pos = pos;
    if let Ok((res, npos)) = parse_block_brackets(tokens, pos, Token::LeftParen, &Token::RightParen)
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
        let start = tokens.start(pos)?;
        let npos = assert_token(tokens, pos, Token::LeftParen)?;
        let (variable, npos) = parse_block_variable_multi_type_no_semicolon(tokens, npos)?;
        let npos = assert_token(tokens, npos, Token::RightParen)?;
        let (block, npos) = parse_block(tokens, npos)?;
        pos = npos;
        let end = tokens.end(pos)?;
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
    let end = tokens.end(pos)?;
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
    let start = tokens.start(pos)?;
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
    let end = tokens.end(pos)?;
    Ok((
        AstMethodParamerters {
            range: AstRange::from_position_token(start, end),
            parameters,
        },
        pos,
    ))
}
fn parse_constructor_paramerters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstMethodParamerters, usize), AstError> {
    let start = tokens.start(pos)?;
    let Ok(pos) = assert_token(tokens, pos, Token::LeftParen) else {
        let end = tokens.start(pos)?;
        return Ok((
            AstMethodParamerters {
                range: AstRange::from_position_token(start, end),
                parameters: vec![],
            },
            pos,
        ));
    };
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
    let end = tokens.end(pos)?;
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
    let start = tokens.start(pos)?;
    let mut fin = false;
    let mut variatic = false;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Final) {
        fin = true;
        pos = npos;
    }
    let (mut jtype, pos) = parse_jtype(tokens, pos)?;
    let mut pos = pos;
    if let Ok(npos) = parse_variadic(tokens, pos) {
        variatic = true;
        pos = npos;
    }
    let (name, pos) = parse_name(tokens, pos)?;
    let pos = parse_array_type_on_name(tokens, pos, &mut jtype);
    let end = tokens.end(pos)?;
    Ok((
        AstMethodParamerter {
            range: AstRange::from_position_token(start, end),
            annotated,
            jtype,
            name,
            fin,
            variatic,
        },
        pos,
    ))
}

/// If token can be identifier
pub fn can_be_ident(token: &Token) -> bool {
    matches!(
        token,
        Token::Yield
            | Token::This
            | Token::Permits
            | Token::Super
            | Token::Sealed
            | Token::Record
            | Token::Int
            | Token::Long
            | Token::Short
            | Token::Byte
            | Token::Char
            | Token::Double
            | Token::Float
            | Token::Boolean
            | Token::Var
            | Token::Non
            | Token::Module
            | Token::Exports
            | Token::To
            | Token::Uses
            | Token::New
            | Token::Provides
            | Token::With
            | Token::Requires
            | Token::Transitive
            | Token::Opens
            | Token::Open
    )
}

/// `thing1_`
pub fn parse_name(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    let start = tokens.start(pos)?;
    let value;
    let mut pos = pos;
    match &start.token {
        Token::Identifier(i) => {
            value = i.clone();
            pos += 1;
        }
        _ if can_be_ident(&start.token) => {
            value = start.token.to_string();
            pos += 1;
        }
        _ => {
            return Err(AstError::InvalidName(InvalidToken::from(start, pos)));
        }
    }
    let end = tokens.end(pos)?;
    Ok((
        AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value,
        },
        pos,
    ))
}

/// `thing.1`
pub fn parse_name_dot(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    let start = tokens.start(pos)?;
    let init_pos = pos;
    let mut pos = pos;
    let mut ident = MyString::new();
    loop {
        let Ok(t) = tokens.get(pos).ok_or(AstError::eof()) else {
            break;
        };
        match &t.token {
            Token::Identifier(id) => {
                ident.push_str(id);
                pos += 1;
            }
            Token::Dot => {
                ident.push('.');
                pos += 1;
            }
            _ if can_be_ident(&start.token) => {
                ident.push_str(&start.token.to_string());
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
    if ident.is_empty() {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t, pos)));
    }
    let end = tokens.end(pos)?;
    Ok((
        AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value: ident.clone(),
        },
        pos,
    ))
}
/// `thing.1`
pub fn parse_name_dot_logical(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut pos = pos;
    let mut ident = MyString::new();
    let mut first = true;
    loop {
        let Ok(t) = tokens.get(pos).ok_or(AstError::eof()) else {
            break;
        };
        match &t.token {
            Token::Identifier(id) => {
                if first {
                    first = false;
                    ident.push_str(id);
                    pos += 1;
                } else if tokens.get(pos - 1).ok_or(AstError::eof()).map(|i| &i.token)
                    == Ok(&Token::Dot)
                {
                    ident.push_str(id);
                    pos += 1;
                } else {
                    break;
                }
            }
            _ if can_be_ident(&t.token) => {
                if first {
                    first = false;
                    ident.push_str(&t.token.to_string());
                    pos += 1;
                } else if tokens.get(pos - 1).ok_or(AstError::eof()).map(|i| &i.token)
                    == Ok(&Token::Dot)
                {
                    ident.push_str(&t.token.to_string());
                    pos += 1;
                } else {
                    break;
                }
            }
            Token::Dot => {
                let Some(last) = tokens.get(pos + 1) else {
                    break;
                };
                match last.token {
                    Token::Identifier(_) => {
                        ident.push('.');
                        pos += 1;
                    }
                    _ if can_be_ident(&last.token) => {
                        ident.push('.');
                        pos += 1;
                    }
                    _ => break,
                }
            }
            _ => {
                break;
            }
        }
    }
    if ident.is_empty() {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t, pos)));
    }
    let end = tokens.end(pos)?;
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
    let start = tokens.start(pos)?;
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
    let end = tokens.end(pos)?;
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
    let start = tokens.start(pos)?;
    let mut pos = pos;
    let mut ident = MyString::new();
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
    let end = tokens.end(pos)?;
    Ok((
        AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value: ident,
        },
        pos,
    ))
}

fn parse_superclass(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstSuperClass>, usize), AstError> {
    let Ok(pos) = assert_token(tokens, pos, Token::Extends) else {
        return Ok((vec![], pos));
    };
    let (sp, pos) = parse_supper_class_inner(tokens, pos)?;
    let mut pos = pos;
    let mut out = vec![sp];
    while let Ok(npos) = assert_token(tokens, pos, Token::Ampersand) {
        if let Ok((sp, npos)) = parse_supper_class_inner(tokens, npos) {
            out.push(sp);
            pos = npos;
        }
    }

    Ok((out, pos))
}

fn parse_supper_class_inner(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSuperClass, usize), AstError> {
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let sp = match jtype.value {
        AstJTypeKind::Class(c) | AstJTypeKind::Generic(c, _) => AstSuperClass::Name(c),
        _ => AstSuperClass::None,
    };
    Ok((sp, pos))
}

fn parse_implements(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstJType>, usize), AstError> {
    let Ok(pos) = assert_token(tokens, pos, Token::Implements) else {
        return Ok((vec![], pos));
    };
    let (out, pos) = parse_comma_seperated_jtype(tokens, pos)?;

    Ok((out, pos))
}

fn parse_permits(tokens: &[PositionToken], pos: usize) -> Result<(Vec<AstJType>, usize), AstError> {
    let Ok(pos) = assert_token(tokens, pos, Token::Permits) else {
        return Ok((vec![], pos));
    };
    let (out, pos) = parse_comma_seperated_jtype(tokens, pos)?;

    Ok((out, pos))
}

fn parse_comma_seperated_jtype(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstJType>, usize), AstError> {
    let mut out = vec![];
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    out.push(jtype);
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
        let (jtype, npos) = parse_jtype(tokens, npos)?;
        out.push(jtype);
        pos = npos;
    }
    Ok((out, pos))
}

/// String
/// int
pub fn parse_jtype(tokens: &[PositionToken], pos: usize) -> Result<(AstJType, usize), AstError> {
    let start = tokens.start(pos)?;
    let (mut annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut pos = pos;
    if let Ok((primitive, npos)) = parse_primitive_type(tokens, pos) {
        pos = npos;
        let end = tokens.end(pos)?;
        let mut out = AstJType {
            range: AstRange::from_position_token(start, end),
            value: primitive,
        };

        if let Ok((anno, npos)) = parse_annotated_list(tokens, pos) {
            annotated.extend(anno);
            pos = npos;
        }
        while let Ok(npos) = assert_token(tokens, pos, Token::LeftParenSquare) {
            if let Ok(npos) = assert_token(tokens, npos, Token::RightParenSquare) {
                pos = npos;
                let end = tokens.end(pos)?;
                out = AstJType {
                    range: AstRange::from_position_token(start, end),
                    value: AstJTypeKind::Array(Box::new(out)),
                };
            } else {
                break;
            }
        }
        Ok((out, pos))
    } else if let Ok(current) = tokens.get(pos).ok_or(AstError::eof()) {
        let mut out = AstJType {
            range: AstRange::default(),
            value: AstJTypeKind::Void,
        };
        if let Token::Identifier(ident) = &current.token {
            let ast_identifier = AstIdentifier {
                range: AstRange::from_position_token(current, current),
                value: ident.into(),
            };
            pos += 1;
            if let Ok((gereric_argmuants, npos)) = parse_jtype_generics(tokens, pos) {
                pos = npos;
                out.value = AstJTypeKind::Generic(ast_identifier, gereric_argmuants.jtypes);
            } else {
                out.value = AstJTypeKind::Class(ast_identifier);
            }
            let end = tokens.end(pos)?;
            out.range = AstRange::from_position_token(start, end);

            if let Ok((anno, npos)) = parse_annotated_list(tokens, pos) {
                annotated.extend(anno);
                pos = npos;
            }
            while let Ok(npos) = assert_token(tokens, pos, Token::LeftParenSquare) {
                if let Ok(npos) = assert_token(tokens, npos, Token::RightParenSquare) {
                    pos = npos;
                    let end = tokens.end(pos)?;
                    out = AstJType {
                        range: AstRange::from_position_token(start, end),
                        value: AstJTypeKind::Array(Box::new(out)),
                    };
                } else {
                    break;
                }
            }
            let end = tokens.end(pos)?;
            out.range = AstRange::from_position_token(start, end);

            if let Ok(npos) = assert_token(tokens, pos, Token::Dot)
                && let Ok((inner, npos)) = parse_jtype(tokens, npos)
            {
                out.value = AstJTypeKind::Access {
                    base: Box::new(AstJType {
                        range: AstRange::from_position_token(start, end),
                        value: out.value,
                    }),
                    inner: Box::new(inner),
                };
                pos = npos;
                let end = tokens.end(pos)?;
                out.range = AstRange::from_position_token(start, end);
            }
        }
        if AstJTypeKind::Void == out.value {
            return Err(AstError::InvalidJtype(InvalidToken::from(start, pos)));
        }
        Ok((out, pos))
    } else {
        Err(AstError::InvalidJtype(InvalidToken::from(start, pos)))
    }
}

fn parse_jtype_generics(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstGenerics, usize), AstError> {
    let gstart = tokens.start(pos)?;
    let mut generic_arguments = vec![];
    let pos = assert_token(tokens, pos, Token::Lt)?;
    let mut pos = pos;
    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
        if assert_token(tokens, pos, Token::Gt).is_ok() {
            break;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::QuestionMark) {
            let start = tokens.get(pos).ok_or(AstError::eof())?;
            pos = npos;

            if let Ok(npos) = assert_token(tokens, npos, Token::Implements) {
                pos = npos;
            } else if let Ok(npos) = assert_token(tokens, npos, Token::Extends) {
                pos = npos;
            } else if let Ok(npos) = assert_token(tokens, npos, Token::Super) {
                pos = npos;
            }
            let end = tokens.get(pos).ok_or(AstError::eof())?;
            generic_arguments.push(AstJType {
                range: AstRange::from_position_token(start, end),
                value: AstJTypeKind::Wildcard,
            });
            continue;
        }
        if let Ok((jtype, npos)) = parse_jtype(tokens, pos) {
            pos = npos;
            generic_arguments.push(jtype);
            continue;
        }
        break;
    }
    let pos = assert_token(tokens, pos, Token::Gt)?;
    let gend = tokens.end(pos)?;
    Ok((
        AstGenerics {
            range: AstRange::from_position_token(gstart, gend),
            jtypes: generic_arguments,
        },
        pos,
    ))
}

fn parse_primitive_type(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstJTypeKind, usize), AstError> {
    let current = tokens.get(pos).ok_or(AstError::eof())?;
    match &current.token {
        Token::Int => Ok((AstJTypeKind::Int, pos + 1)),
        Token::Long => Ok((AstJTypeKind::Long, pos + 1)),
        Token::Short => Ok((AstJTypeKind::Short, pos + 1)),
        Token::Byte => Ok((AstJTypeKind::Byte, pos + 1)),
        Token::Char => Ok((AstJTypeKind::Char, pos + 1)),
        Token::Double => Ok((AstJTypeKind::Double, pos + 1)),
        Token::Float => Ok((AstJTypeKind::Float, pos + 1)),
        Token::Boolean => Ok((AstJTypeKind::Boolean, pos + 1)),
        Token::Void => Ok((AstJTypeKind::Void, pos + 1)),
        Token::QuestionMark => Ok((AstJTypeKind::Wildcard, pos + 1)),
        Token::Var => Ok((AstJTypeKind::Var, pos + 1)),
        _ => Err(AstError::InvalidJtype(InvalidToken::from(current, pos))),
    }
}
