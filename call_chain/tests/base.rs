use ast::{
    error::PrintErr,
    types::{AstPoint, AstRange},
};
use call_chain::{CallItem, get_call_chain};
use pretty_assertions::assert_eq;

#[test]
fn call_chain_base() {
    let content = "
package ch.emilycares;

public class Test {

    public void hello(String a) {
        String local = \"\";

        var lo = local. 
        return;
    }
}
        ";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(8, 24));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_variable() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello(String a) {
        a.  
        return;
    }
}
        ";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 11));
    insta::assert_debug_snapshot!(out);
}

pub const SYMBOL_METHOD: &str = r#"
package ch.emilycares;

public class Test {

    public void hello() {
        String local = "";

        var lo = local.concat("hehe"). 
        return;
    }
}
        "#;

#[test]
fn call_chain_method_a() {
    let tokens = ast::lexer::lex(SYMBOL_METHOD).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(SYMBOL_METHOD, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(8, 40));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_string() {
    let content = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        String a = "";
        return "".  ;
    }
}
"#;
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 19));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_field_base() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var lo = local.a.
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 26));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_method_base() {
    let content = "
package ch.emilycares;
public class GreetingResource {
    String a;
    public String hello() {
        a.concat(\"\"). 
        return \"huh\";
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 24));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_method_info() {
    let content = "
package ch.emilycares;
public class GreetingResource {
    public String hello() {
        a.concat(\"\").other();
        return \"huh\";
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    // the cursor is on the concat method_call
    let out = get_call_chain(&ast, &AstPoint::new(4, 14));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_field_method() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var lo = local.a.b(). ;
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 30));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_menthod_field() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var lo = local.a().b.
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 30));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_semicolon_simple() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var lo = local. ;
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 23));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_semicolon_field() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        int c = local.a().c.;
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 28));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_semicolon_method() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        int c = local.a.c().;
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 28));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_statement() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        local.a.c().;
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 20));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_class() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        String. 
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 16));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_varible_class() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var local = String. 
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 28));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_argument_base() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        local.concat( )
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 22));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_argument_var_base() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        local.concat(local. );
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 27));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_argument_var_no_dot() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        local.concat(local );
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 27));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_argument_second_var_base() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        a.concat(b, c. );
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 23));
    match out.clone().first().unwrap() {
        CallItem::ArgumentList {
            prev: _,
            active_param,
            filled_params: _,
            range: _,
        } => assert_eq!(active_param, &Some(1)),
        _ => unreachable!(),
    };
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_argument_active_param_not_last() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        a.concat(b , c);
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 19));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_argument_second_var_no_dot() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        a.concat(b, c );
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 22));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_argument_field() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        a.concat(b.a  );
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 22));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_argument_calc() {
    let content = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        concat("z" + a.getThing());
        return;
    }
}
"#;
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 28));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_arguments() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        a.concat( );
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 18));
    assert_eq!(
        out,
        vec![CallItem::ArgumentList {
            prev: vec![
                CallItem::ClassOrVariable {
                    name: "a".into(),
                    range: AstRange {
                        start: AstPoint { line: 4, col: 8 },
                        end: AstPoint { line: 4, col: 9 },
                    },
                },
                CallItem::MethodCall {
                    name: "concat".into(),
                    range: AstRange {
                        start: AstPoint { line: 4, col: 10 },
                        end: AstPoint { line: 4, col: 16 },
                    },
                }
            ],
            range: AstRange {
                start: AstPoint { line: 4, col: 16 },
                end: AstPoint { line: 4, col: 19 },
            },
            filled_params: vec![vec![]],
            active_param: Some(0)
        }]
    );
}

#[test]
fn call_chain_argument_method() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        a.concat(b.a() );
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 23));
    assert_eq!(
        out,
        vec![
            CallItem::ArgumentList {
                prev: vec![
                    CallItem::ClassOrVariable {
                        name: "a".into(),
                        range: AstRange {
                            start: AstPoint { line: 5, col: 8 },
                            end: AstPoint { line: 5, col: 9 },
                        },
                    },
                    CallItem::MethodCall {
                        name: "concat".into(),
                        range: AstRange {
                            start: AstPoint { line: 5, col: 10 },
                            end: AstPoint { line: 5, col: 16 },
                        },
                    },
                ],
                range: AstRange {
                    start: AstPoint { line: 5, col: 16 },
                    end: AstPoint { line: 5, col: 24 },
                },
                filled_params: vec![vec![
                    CallItem::ClassOrVariable {
                        name: "b".into(),
                        range: AstRange {
                            start: AstPoint { line: 5, col: 17 },
                            end: AstPoint { line: 5, col: 18 }
                        }
                    },
                    CallItem::MethodCall {
                        name: "a".into(),
                        range: AstRange {
                            start: AstPoint { line: 5, col: 19 },
                            end: AstPoint { line: 5, col: 20 },
                        }
                    },
                ]],
                active_param: Some(0)
            },
            CallItem::ClassOrVariable {
                name: "b".into(),
                range: AstRange {
                    start: AstPoint { line: 5, col: 17 },
                    end: AstPoint { line: 5, col: 18 }
                }
            },
            CallItem::MethodCall {
                name: "a".into(),
                range: AstRange {
                    start: AstPoint { line: 5, col: 19 },
                    end: AstPoint { line: 5, col: 20 },
                }
            },
        ]
    );
}

#[test]
fn call_chain_if_base() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        if (a ) {
        }
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 14));
    assert_eq!(
        out,
        vec![CallItem::ClassOrVariable {
            name: "a".into(),
            range: AstRange {
                start: AstPoint { line: 4, col: 12 },
                end: AstPoint { line: 4, col: 13 },
            }
        }]
    );
}

#[test]
fn call_chain_if_condition() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        if (a == b ) {
        }
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 19));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_return() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        return a. ;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 18));
    assert_eq!(
        out,
        vec![CallItem::ClassOrVariable {
            name: "a".into(),
            range: AstRange {
                start: AstPoint { line: 4, col: 15 },
                end: AstPoint { line: 4, col: 16 }
            }
        },]
    );
}

#[test]
fn call_chain_return_method_call() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        return a.b(). ;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 22));
    assert_eq!(
        out,
        vec![
            CallItem::ClassOrVariable {
                name: "a".into(),
                range: AstRange {
                    start: AstPoint { line: 4, col: 15 },
                    end: AstPoint { line: 4, col: 16 },
                }
            },
            CallItem::MethodCall {
                name: "b".into(),
                range: AstRange {
                    start: AstPoint { line: 4, col: 17 },
                    end: AstPoint { line: 4, col: 18 },
                }
            }
        ]
    );
}

#[test]
fn call_chain_new_instance_base() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        new String()
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 22));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_new_instance_field() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        new String().a.
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 22));
    insta::assert_debug_snapshot!(out);
}

#[test]
fn call_chain_new_instance_method() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        new String().a(). ;
        return;
    }
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 25));
    assert_eq!(
        out,
        vec![
            CallItem::Class {
                name: "String".into(),
                range: AstRange {
                    start: AstPoint { line: 4, col: 8 },
                    end: AstPoint { line: 4, col: 20 },
                }
            },
            CallItem::MethodCall {
                name: "a".into(),
                range: AstRange {
                    start: AstPoint { line: 4, col: 21 },
                    end: AstPoint { line: 4, col: 22 },
                }
            }
        ]
    );
}

#[test]
fn call_chain_field_declartion() {
    let content = "
package ch.emilycares;
public class Test {
    private static Logger LOG = Logger.getLogger(Test.class);
}
";
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(3, 43));
    assert_eq!(
        out,
        vec![
            CallItem::ClassOrVariable {
                name: "Logger".into(),
                range: AstRange {
                    start: AstPoint { line: 3, col: 32 },
                    end: AstPoint { line: 3, col: 38 },
                }
            },
            CallItem::MethodCall {
                name: "getLogger".into(),
                range: AstRange {
                    start: AstPoint { line: 3, col: 39 },
                    end: AstPoint { line: 3, col: 48 },
                }
            }
        ]
    );
}

#[test]
fn call_chain_annotation_parameter() {
    let content = r#"
package ch.emilycares;
import jakarta.ws.rs.Produces;
public class Test {
    @Produces(MediaType.TEXT_PLAIN)
    public String hello() {
      return "a";
    }
}
"#;
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 27));
    assert_eq!(
        out,
        vec![
            CallItem::ClassOrVariable {
                name: "MediaType".into(),
                range: AstRange {
                    start: AstPoint { line: 4, col: 14 },
                    end: AstPoint { line: 4, col: 23 },
                }
            },
            CallItem::FieldAccess {
                name: "TEXT_PLAIN".into(),
                range: AstRange {
                    start: AstPoint { line: 4, col: 24 },
                    end: AstPoint { line: 4, col: 34 },
                }
            }
        ]
    );
}

#[test]
fn call_chain_this() {
    let content = r#"
package ch.emilycares;
public class Test {
    public String hello() {
      return this.a.toString();
    }
}
"#;
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 27));
    assert_eq!(
        out,
        vec![
            CallItem::This {
                range: AstRange {
                    start: AstPoint { line: 4, col: 13 },
                    end: AstPoint { line: 4, col: 17 }
                }
            },
            CallItem::FieldAccess {
                name: "a".into(),
                range: AstRange {
                    start: AstPoint { line: 4, col: 18 },
                    end: AstPoint { line: 4, col: 19 }
                }
            },
            CallItem::MethodCall {
                name: "toString".into(),
                range: AstRange {
                    start: AstPoint { line: 4, col: 20 },
                    end: AstPoint { line: 4, col: 28 }
                }
            }
        ]
    );
}

#[test]
fn call_chain_this_set() {
    let content = r#"
package ch.emilycares;
public class Test {
    public String hello() {
      this.asd = "a";
      return "";
    }
}
"#;
    let tokens = ast::lexer::lex(content).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 15));
    assert_eq!(
        out,
        vec![
            CallItem::This {
                range: AstRange {
                    start: AstPoint { line: 4, col: 6 },
                    end: AstPoint { line: 4, col: 10 }
                }
            },
            CallItem::FieldAccess {
                name: "asd".into(),
                range: AstRange {
                    start: AstPoint { line: 4, col: 11 },
                    end: AstPoint { line: 4, col: 14 }
                }
            },
        ]
    );
}
