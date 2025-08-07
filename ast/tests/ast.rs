use ast::error::PrintErr;
use ast::{
    lexer, parse_block_variable, parse_file, parse_lambda, parse_name, parse_recursive_expression,
    parse_string_literal,
};

#[test]
fn everything() {
    let content = include_str!("../../parser/test/Everything.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn skip_comments() {
    let content = include_str!("../test/FullOffComments.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn locale_variable_table() {
    let content = include_str!("../../parser/test/LocalVariableTable.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn superee() {
    let content = include_str!("../../parser/test/Super.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn constants() {
    let content = include_str!("../../parser/test/Constants.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn super_interface() {
    let content = include_str!("../../parser/test/SuperInterface.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn interface_base() {
    let content = include_str!("../../parser/test/InterfaceBase.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn variants() {
    let content = include_str!("../../parser/test/Variants.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn types() {
    let content = include_str!("../../parser/test/Types.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn annotated() {
    let content = include_str!("../../parser/test/Annotated.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn expression_base() {
    let content = "Logger.getLogger(Test.class)";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_recursive_expression(&tokens, 0);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn expression_array_access() {
    let content = "numbers[0]";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_recursive_expression(&tokens, 0);
    parsed.print_err(content);
    let ast = parsed.unwrap();
    insta::assert_debug_snapshot!(ast);
    assert_eq!(ast.1, tokens.len());
}
#[test]
fn expression_multi_array_access() {
    let content = "numbers[0][0][0]";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_recursive_expression(&tokens, 0);
    parsed.print_err(content);
    let ast = parsed.unwrap();
    insta::assert_debug_snapshot!(ast);
    assert_eq!(ast.1, tokens.len());
}

#[test]
fn equasion_method_call() {
    let content = r#""z" + a.getThing()"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_recursive_expression(&tokens, 0);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn annotation() {
    let content = include_str!("../../parser/test/Annotation.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn more_syntax() {
    let content = include_str!("../../parser/test/Syntax.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn variable_array() {
    let content = r#"String[] cars = {"Volvo", "BMW", "Ford", "Mazda"};"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_block_variable(&tokens, 0);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn multi_line_string() {
    let content = r#"
        """
        Here is a muilti
        line
        string""
        "#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_string_literal(&tokens, 0);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn name() {
    let content = "Logger3m3m3m3";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_name(&tokens, 0);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn lambda() {
    let content = "(n) -> { System.out.println(n); }";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_lambda(&tokens, 0);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn lambda_1() {
    let content = "n -> { }";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_lambda(&tokens, 0);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn lambda_2() {
    let content = "(a, b, c) -> { }";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_lambda(&tokens, 0);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn equal_expr() {
    let content = "a == b ";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_recursive_expression(&tokens, 0);
    parsed.print_err(content);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
