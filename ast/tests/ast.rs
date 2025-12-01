use ast::class::{parse_class_block, parse_class_method, parse_class_variable};
use ast::error::PrintErr;
use ast::{
    ExpressionOptions, lexer, parse_annotated, parse_block, parse_block_return,
    parse_block_variable, parse_expression, parse_file, parse_for, parse_jtype, parse_lambda,
    parse_name, parse_name_dot_logical, parse_new_class, parse_string_literal,
    parse_switch_case_arrow_value,
};

#[test]
fn everything() {
    let content = include_str!("../../parser/test/Everything.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn skip_comments() {
    let content = include_str!("../test/FullOffComments.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn locale_variable_table() {
    let content = include_str!("../../parser/test/LocalVariableTable.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn superee() {
    let content = include_str!("../../parser/test/Super.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn constants() {
    let content = include_str!("../../parser/test/Constants.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn super_interface() {
    let content = include_str!("../../parser/test/SuperInterface.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn interface_base() {
    let content = include_str!("../../parser/test/InterfaceBase.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn variants() {
    let content = include_str!("../../parser/test/Variants.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn types() {
    let content = include_str!("../../parser/test/Types.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn annotated() {
    let content = include_str!("../../parser/test/Annotated.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn expression_base() {
    let content = "Logger.getLogger(Test.class)";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_expression(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn expression_array_access() {
    let content = "numbers[0]";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_expression(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    let ast = parsed.unwrap();
    insta::assert_debug_snapshot!(ast);
    assert_eq!(ast.1, tokens.len());
}
#[test]
fn expression_multi_array_access() {
    let content = "numbers[0][0][0]";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_expression(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    let ast = parsed.unwrap();
    insta::assert_debug_snapshot!(ast);
    assert_eq!(ast.1, tokens.len());
}

#[test]
fn equasion_method_call() {
    let content = r#""z" + a.getThing()"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_expression(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn annotation() {
    let content = include_str!("../../parser/test/Annotation.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn more_syntax() {
    let content = include_str!("../../parser/test/Syntax.java");
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_file(&tokens);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn variable_array() {
    let content = r#"String[] cars = {"Volvo", "BMW", "Ford", "Mazda"};"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_block_variable(&tokens, 0);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn variable_var_no_value() {
    let content = r#"{var a = }"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_block(&tokens, 0);
    parsed.print_err(content, &tokens);
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
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn name() {
    let content = "Logger3m3m3m3";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_name(&tokens, 0);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn lambda() {
    let content = "(n) -> { System.out.println(n); }";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_lambda(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn lambda_1() {
    let content = "n -> { }";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_lambda(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn lambda_2() {
    let content = "(a, b, c) -> { }";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_lambda(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn lambda_value() {
    let content = "n -> true";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_lambda(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn lambda_expr() {
    let content = "n -> v.toString()";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_lambda(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn lambda_in_expression() {
    let content = "numbers.forEach( (n) -> { System.out.println(n); } )";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_expression(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn equal_expr() {
    let content = "a == b ";
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_expression(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

// new String()
#[test]
fn new_string() {
    let content = r#"
     return new String();   
    "#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_block_return(&tokens, 0);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}

#[test]
fn new_array() {
    let content = r#"
     return new Object[][] {
            { "NumberPatterns",
                new String[] {
                   "",
                   ""
                }
            },
        };   
    "#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_block_return(&tokens, 0);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn long_expr() {
    let content = r#"IAFactory.getInstance().getIA("localhost", 1344, SERVICE).support(true)"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_expression(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn cast() {
    let content = r#"new byte[] {(byte)'a'}"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_new_class(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn double_plus() {
    let content = r#"values[i++]"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_expression(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    insta::assert_debug_snapshot!(parsed.unwrap());
}
#[test]
fn method_no_body() {
    let content = r#"protected abstract T create(Object key);"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_class_method(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn return_casted_newclass() {
    let content = r#"return (Entry<T>[]) new Entry<?>[length];"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_block_return(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    insta::assert_debug_snapshot!(parsed);
    assert_eq!(tokens.len(), parsed.1);
}

#[test]
fn jtype_geneic_array() {
    let content = r#"Entry<T>[]"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_jtype(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn class_var_hashmap_genirics() {
    let content = r#"private HashMap<String, List<PropertyDescriptor>> pdStore = new HashMap<>();"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_class_variable(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn name_dot() {
    let content = r#"Thing1.other."#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_name_dot_logical(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len() - 1, parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn labled_emty_for() {
    let content = r#" l: for (;;) {} "#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_for(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn new_casted_parameter() {
    let content = r#" new HandleTable(10, (float) 3.00) "#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_new_class(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn class_colon_colon_new() {
    let content = r#"Class<?>[]::new"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_expression(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn parmeter_class_function_pass() {
    let content = r#"toArray(Class<?>[]::new)"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_expression(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn multiline_string_arg() {
    let content = r#"{
                        System.err.println("""
                                          The VFORK launch mechanism has been deprecated for being dangerous.
                                          It will be removed in a future java version. Either remove the
                                          jdk.lang.Process.launchMechanism property (preferred) or use FORK mode
                                          instead (-Djdk.lang.Process.launchMechanism=FORK).
                                          """);
                                          }
    "#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_block(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn class_block_annotation() {
    let content = r#"{
    @Target(ElementType.METHOD)
    @Retention(RetentionPolicy.RUNTIME)
    @interface Compiled {
    }
    }
    "#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_class_block(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn lambda_with_types() {
    let content = r#"(T t, U u) -> after.apply(apply(t, u))"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_lambda(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn lambda_with_no_parameters() {
    let content = r#"() -> a"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_lambda(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn jtype_access() {
    let content = r#"Something.Inner"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_jtype(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn jtype_package() {
    let content = r#"javax.crypto.interfaces.DHPrivateKey"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_jtype(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn jtype_int() {
    let content = r#"int"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_jtype(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn jtype_int_array() {
    let content = r#"int[]"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_jtype(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn jtype_class_generic() {
    let content = r#"HashMap<String, Integer>"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_jtype(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn jtype_class_generic_generic() {
    let content = r#"Predicate<Class<?>>"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_jtype(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn casted_calculation() {
    let content = r#"case MILLI_OF_DAY -> (int) (toNanoOfDay() / 1000_000);"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_switch_case_arrow_value(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn name_dot_logical() {
    let content = r#"@Overwrite Other."#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_annotated(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn annotated_array() {
    let content = r#"@SuppressWarnings({"unchecked", "rawtypes"})"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_annotated(&tokens, 0);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
#[test]
fn colon_colon_new() {
    let content = r#"byte[]"#;
    let tokens = lexer::lex(content).unwrap();
    let parsed = parse_expression(&tokens, 0, &ExpressionOptions::None);
    parsed.print_err(content, &tokens);
    let parsed = parsed.unwrap();
    assert_eq!(tokens.len(), parsed.1);
    insta::assert_debug_snapshot!(parsed);
}
