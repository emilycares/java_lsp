use ast::{
    error::PrintErr,
    types::{AstPoint, AstRange},
};
use call_chain::{CallItem, get_call_chain};
use expect_test::expect;
use my_string::smol_str::SmolStr;

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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(8, 24));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "local",
                range: AstRange {
                    start: AstPoint { 8:17 },
                    end: AstPoint { 8:22 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 11));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "a",
                range: AstRange {
                    start: AstPoint { 4:8 },
                    end: AstPoint { 4:9 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(SYMBOL_METHOD.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(SYMBOL_METHOD, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(8, 40));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "local",
                range: AstRange {
                    start: AstPoint { 8:17 },
                    end: AstPoint { 8:22 },
                },
            },
            MethodCall {
                name: "concat",
                range: AstRange {
                    start: AstPoint { 8:23 },
                    end: AstPoint { 8:29 },
                },
                args: [
                    [
                        Class {
                            name: "String",
                            range: AstRange {
                                start: AstPoint { 8:35 },
                                end: AstPoint { 8:30 },
                            },
                        },
                    ],
                ],
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 19));
    let expected = expect![[r#"
        [
            Class {
                name: "String",
                range: AstRange {
                    start: AstPoint { 5:16 },
                    end: AstPoint { 5:14 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 26));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "local",
                range: AstRange {
                    start: AstPoint { 5:17 },
                    end: AstPoint { 5:22 },
                },
            },
            FieldAccess {
                name: "a",
                range: AstRange {
                    start: AstPoint { 5:23 },
                    end: AstPoint { 5:24 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 24));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "a",
                range: AstRange {
                    start: AstPoint { 5:8 },
                    end: AstPoint { 5:9 },
                },
            },
            MethodCall {
                name: "concat",
                range: AstRange {
                    start: AstPoint { 5:10 },
                    end: AstPoint { 5:16 },
                },
                args: [
                    [
                        Class {
                            name: "String",
                            range: AstRange {
                                start: AstPoint { 5:18 },
                                end: AstPoint { 5:17 },
                            },
                        },
                    ],
                ],
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    // the cursor is on the concat method_call
    let out = get_call_chain(&ast, &AstPoint::new(4, 14));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "a",
                range: AstRange {
                    start: AstPoint { 4:8 },
                    end: AstPoint { 4:9 },
                },
            },
            MethodCall {
                name: "concat",
                range: AstRange {
                    start: AstPoint { 4:10 },
                    end: AstPoint { 4:16 },
                },
                args: [
                    [
                        Class {
                            name: "String",
                            range: AstRange {
                                start: AstPoint { 4:18 },
                                end: AstPoint { 4:17 },
                            },
                        },
                    ],
                ],
            },
            MethodCall {
                name: "other",
                range: AstRange {
                    start: AstPoint { 4:21 },
                    end: AstPoint { 4:26 },
                },
                args: [],
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 30));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "local",
                range: AstRange {
                    start: AstPoint { 5:17 },
                    end: AstPoint { 5:22 },
                },
            },
            FieldAccess {
                name: "a",
                range: AstRange {
                    start: AstPoint { 5:23 },
                    end: AstPoint { 5:24 },
                },
            },
            MethodCall {
                name: "b",
                range: AstRange {
                    start: AstPoint { 5:25 },
                    end: AstPoint { 5:26 },
                },
                args: [],
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
}

#[test]
fn call_chain_method_field() {
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 30));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "local",
                range: AstRange {
                    start: AstPoint { 5:17 },
                    end: AstPoint { 5:22 },
                },
            },
            MethodCall {
                name: "a",
                range: AstRange {
                    start: AstPoint { 5:23 },
                    end: AstPoint { 5:24 },
                },
                args: [],
            },
            FieldAccess {
                name: "b",
                range: AstRange {
                    start: AstPoint { 5:27 },
                    end: AstPoint { 5:28 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 23));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "local",
                range: AstRange {
                    start: AstPoint { 5:17 },
                    end: AstPoint { 5:22 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 28));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "local",
                range: AstRange {
                    start: AstPoint { 5:16 },
                    end: AstPoint { 5:21 },
                },
            },
            MethodCall {
                name: "a",
                range: AstRange {
                    start: AstPoint { 5:22 },
                    end: AstPoint { 5:23 },
                },
                args: [],
            },
            FieldAccess {
                name: "c",
                range: AstRange {
                    start: AstPoint { 5:26 },
                    end: AstPoint { 5:27 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 28));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "local",
                range: AstRange {
                    start: AstPoint { 5:16 },
                    end: AstPoint { 5:21 },
                },
            },
            FieldAccess {
                name: "a",
                range: AstRange {
                    start: AstPoint { 5:22 },
                    end: AstPoint { 5:23 },
                },
            },
            MethodCall {
                name: "c",
                range: AstRange {
                    start: AstPoint { 5:24 },
                    end: AstPoint { 5:25 },
                },
                args: [],
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 20));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "local",
                range: AstRange {
                    start: AstPoint { 5:8 },
                    end: AstPoint { 5:13 },
                },
            },
            FieldAccess {
                name: "a",
                range: AstRange {
                    start: AstPoint { 5:14 },
                    end: AstPoint { 5:15 },
                },
            },
            MethodCall {
                name: "c",
                range: AstRange {
                    start: AstPoint { 5:16 },
                    end: AstPoint { 5:17 },
                },
                args: [],
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 16));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "String",
                range: AstRange {
                    start: AstPoint { 5:8 },
                    end: AstPoint { 5:14 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
}

#[test]
fn call_chain_variable_class() {
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 28));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "String",
                range: AstRange {
                    start: AstPoint { 5:20 },
                    end: AstPoint { 5:26 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 22));
    let expected = expect![[r#"
        [
            ArgumentList {
                prev: [
                    ClassOrVariable {
                        name: "local",
                        range: AstRange {
                            start: AstPoint { 5:8 },
                            end: AstPoint { 5:13 },
                        },
                    },
                    MethodCall {
                        name: "concat",
                        range: AstRange {
                            start: AstPoint { 5:14 },
                            end: AstPoint { 5:20 },
                        },
                        args: [],
                    },
                ],
                active_param: Some(
                    0,
                ),
                filled_params: [
                    [],
                ],
                range: AstRange {
                    start: AstPoint { 5:20 },
                    end: AstPoint { 5:23 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 27));
    let expected = expect![[r#"
        [
            ArgumentList {
                prev: [
                    ClassOrVariable {
                        name: "local",
                        range: AstRange {
                            start: AstPoint { 5:8 },
                            end: AstPoint { 5:13 },
                        },
                    },
                    MethodCall {
                        name: "concat",
                        range: AstRange {
                            start: AstPoint { 5:14 },
                            end: AstPoint { 5:20 },
                        },
                        args: [
                            [
                                ClassOrVariable {
                                    name: "local",
                                    range: AstRange {
                                        start: AstPoint { 5:21 },
                                        end: AstPoint { 5:26 },
                                    },
                                },
                            ],
                        ],
                    },
                ],
                active_param: Some(
                    0,
                ),
                filled_params: [
                    [
                        ClassOrVariable {
                            name: "local",
                            range: AstRange {
                                start: AstPoint { 5:21 },
                                end: AstPoint { 5:26 },
                            },
                        },
                    ],
                ],
                range: AstRange {
                    start: AstPoint { 5:20 },
                    end: AstPoint { 5:29 },
                },
            },
            ClassOrVariable {
                name: "local",
                range: AstRange {
                    start: AstPoint { 5:21 },
                    end: AstPoint { 5:26 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 27));
    let expected = expect![[r#"
        [
            ArgumentList {
                prev: [
                    ClassOrVariable {
                        name: "local",
                        range: AstRange {
                            start: AstPoint { 5:8 },
                            end: AstPoint { 5:13 },
                        },
                    },
                    MethodCall {
                        name: "concat",
                        range: AstRange {
                            start: AstPoint { 5:14 },
                            end: AstPoint { 5:20 },
                        },
                        args: [],
                    },
                ],
                active_param: Some(
                    0,
                ),
                filled_params: [
                    [
                        ClassOrVariable {
                            name: "local",
                            range: AstRange {
                                start: AstPoint { 5:21 },
                                end: AstPoint { 5:26 },
                            },
                        },
                    ],
                ],
                range: AstRange {
                    start: AstPoint { 5:20 },
                    end: AstPoint { 5:28 },
                },
            },
            ClassOrVariable {
                name: "local",
                range: AstRange {
                    start: AstPoint { 5:21 },
                    end: AstPoint { 5:26 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
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
    let expected = expect![[r#"
        [
            ArgumentList {
                prev: [
                    ClassOrVariable {
                        name: "a",
                        range: AstRange {
                            start: AstPoint { 5:8 },
                            end: AstPoint { 5:9 },
                        },
                    },
                    MethodCall {
                        name: "concat",
                        range: AstRange {
                            start: AstPoint { 5:10 },
                            end: AstPoint { 5:16 },
                        },
                        args: [
                            [
                                ClassOrVariable {
                                    name: "b",
                                    range: AstRange {
                                        start: AstPoint { 5:17 },
                                        end: AstPoint { 5:18 },
                                    },
                                },
                            ],
                            [
                                ClassOrVariable {
                                    name: "c",
                                    range: AstRange {
                                        start: AstPoint { 5:20 },
                                        end: AstPoint { 5:21 },
                                    },
                                },
                            ],
                        ],
                    },
                ],
                active_param: Some(
                    1,
                ),
                filled_params: [
                    [
                        ClassOrVariable {
                            name: "b",
                            range: AstRange {
                                start: AstPoint { 5:17 },
                                end: AstPoint { 5:18 },
                            },
                        },
                    ],
                    [
                        ClassOrVariable {
                            name: "c",
                            range: AstRange {
                                start: AstPoint { 5:20 },
                                end: AstPoint { 5:21 },
                            },
                        },
                    ],
                ],
                range: AstRange {
                    start: AstPoint { 5:16 },
                    end: AstPoint { 5:24 },
                },
            },
            ClassOrVariable {
                name: "c",
                range: AstRange {
                    start: AstPoint { 5:20 },
                    end: AstPoint { 5:21 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 19));
    let expected = expect![[r#"
        [
            ArgumentList {
                prev: [
                    ClassOrVariable {
                        name: "a",
                        range: AstRange {
                            start: AstPoint { 5:8 },
                            end: AstPoint { 5:9 },
                        },
                    },
                    MethodCall {
                        name: "concat",
                        range: AstRange {
                            start: AstPoint { 5:10 },
                            end: AstPoint { 5:16 },
                        },
                        args: [
                            [
                                ClassOrVariable {
                                    name: "b",
                                    range: AstRange {
                                        start: AstPoint { 5:17 },
                                        end: AstPoint { 5:18 },
                                    },
                                },
                            ],
                            [
                                ClassOrVariable {
                                    name: "c",
                                    range: AstRange {
                                        start: AstPoint { 5:21 },
                                        end: AstPoint { 5:22 },
                                    },
                                },
                            ],
                        ],
                    },
                ],
                active_param: Some(
                    0,
                ),
                filled_params: [
                    [
                        ClassOrVariable {
                            name: "b",
                            range: AstRange {
                                start: AstPoint { 5:17 },
                                end: AstPoint { 5:18 },
                            },
                        },
                    ],
                    [
                        ClassOrVariable {
                            name: "c",
                            range: AstRange {
                                start: AstPoint { 5:21 },
                                end: AstPoint { 5:22 },
                            },
                        },
                    ],
                ],
                range: AstRange {
                    start: AstPoint { 5:16 },
                    end: AstPoint { 5:23 },
                },
            },
            ClassOrVariable {
                name: "b",
                range: AstRange {
                    start: AstPoint { 5:17 },
                    end: AstPoint { 5:18 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 22));
    let expected = expect![[r#"
        [
            ArgumentList {
                prev: [
                    ClassOrVariable {
                        name: "a",
                        range: AstRange {
                            start: AstPoint { 5:8 },
                            end: AstPoint { 5:9 },
                        },
                    },
                    MethodCall {
                        name: "concat",
                        range: AstRange {
                            start: AstPoint { 5:10 },
                            end: AstPoint { 5:16 },
                        },
                        args: [
                            [
                                ClassOrVariable {
                                    name: "b",
                                    range: AstRange {
                                        start: AstPoint { 5:17 },
                                        end: AstPoint { 5:18 },
                                    },
                                },
                            ],
                            [
                                ClassOrVariable {
                                    name: "c",
                                    range: AstRange {
                                        start: AstPoint { 5:20 },
                                        end: AstPoint { 5:21 },
                                    },
                                },
                            ],
                        ],
                    },
                ],
                active_param: Some(
                    1,
                ),
                filled_params: [
                    [
                        ClassOrVariable {
                            name: "b",
                            range: AstRange {
                                start: AstPoint { 5:17 },
                                end: AstPoint { 5:18 },
                            },
                        },
                    ],
                    [
                        ClassOrVariable {
                            name: "c",
                            range: AstRange {
                                start: AstPoint { 5:20 },
                                end: AstPoint { 5:21 },
                            },
                        },
                    ],
                ],
                range: AstRange {
                    start: AstPoint { 5:16 },
                    end: AstPoint { 5:23 },
                },
            },
            ClassOrVariable {
                name: "c",
                range: AstRange {
                    start: AstPoint { 5:20 },
                    end: AstPoint { 5:21 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 22));
    let expected = expect![[r#"
        [
            ArgumentList {
                prev: [
                    ClassOrVariable {
                        name: "a",
                        range: AstRange {
                            start: AstPoint { 5:8 },
                            end: AstPoint { 5:9 },
                        },
                    },
                    MethodCall {
                        name: "concat",
                        range: AstRange {
                            start: AstPoint { 5:10 },
                            end: AstPoint { 5:16 },
                        },
                        args: [],
                    },
                ],
                active_param: Some(
                    0,
                ),
                filled_params: [
                    [
                        ClassOrVariable {
                            name: "b",
                            range: AstRange {
                                start: AstPoint { 5:17 },
                                end: AstPoint { 5:18 },
                            },
                        },
                        ClassOrVariable {
                            name: "a",
                            range: AstRange {
                                start: AstPoint { 5:19 },
                                end: AstPoint { 5:20 },
                            },
                        },
                    ],
                ],
                range: AstRange {
                    start: AstPoint { 5:16 },
                    end: AstPoint { 5:23 },
                },
            },
            ClassOrVariable {
                name: "b",
                range: AstRange {
                    start: AstPoint { 5:17 },
                    end: AstPoint { 5:18 },
                },
            },
            ClassOrVariable {
                name: "a",
                range: AstRange {
                    start: AstPoint { 5:19 },
                    end: AstPoint { 5:20 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 28));
    let expected = expect![[r#"
        [
            ArgumentList {
                prev: [
                    MethodCall {
                        name: "concat",
                        range: AstRange {
                            start: AstPoint { 4:8 },
                            end: AstPoint { 4:14 },
                        },
                        args: [
                            [
                                ClassOrVariable {
                                    name: "a",
                                    range: AstRange {
                                        start: AstPoint { 4:21 },
                                        end: AstPoint { 4:22 },
                                    },
                                },
                                MethodCall {
                                    name: "getThing",
                                    range: AstRange {
                                        start: AstPoint { 4:23 },
                                        end: AstPoint { 4:31 },
                                    },
                                    args: [],
                                },
                            ],
                        ],
                    },
                ],
                active_param: Some(
                    0,
                ),
                filled_params: [
                    [
                        ClassOrVariable {
                            name: "a",
                            range: AstRange {
                                start: AstPoint { 4:21 },
                                end: AstPoint { 4:22 },
                            },
                        },
                        MethodCall {
                            name: "getThing",
                            range: AstRange {
                                start: AstPoint { 4:23 },
                                end: AstPoint { 4:31 },
                            },
                            args: [],
                        },
                    ],
                ],
                range: AstRange {
                    start: AstPoint { 4:14 },
                    end: AstPoint { 4:34 },
                },
            },
            ClassOrVariable {
                name: "a",
                range: AstRange {
                    start: AstPoint { 4:21 },
                    end: AstPoint { 4:22 },
                },
            },
            MethodCall {
                name: "getThing",
                range: AstRange {
                    start: AstPoint { 4:23 },
                    end: AstPoint { 4:31 },
                },
                args: [],
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 18));
    assert_eq!(
        vec![CallItem::ArgumentList {
            prev: vec![
                CallItem::ClassOrVariable {
                    name: SmolStr::new_inline("a"),
                    range: AstRange {
                        start: AstPoint { line: 4, col: 8 },
                        end: AstPoint { line: 4, col: 9 },
                    },
                },
                CallItem::MethodCall {
                    name: SmolStr::new_inline("concat"),
                    range: AstRange {
                        start: AstPoint { line: 4, col: 10 },
                        end: AstPoint { line: 4, col: 16 },
                    },
                    args: vec![]
                }
            ],
            range: AstRange {
                start: AstPoint { line: 4, col: 16 },
                end: AstPoint { line: 4, col: 19 },
            },
            filled_params: vec![vec![]],
            active_param: Some(0)
        }],
        out,
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 23));
    assert_eq!(
        vec![
            CallItem::ArgumentList {
                prev: vec![
                    CallItem::ClassOrVariable {
                        name: SmolStr::new_inline("a"),
                        range: AstRange {
                            start: AstPoint { line: 5, col: 8 },
                            end: AstPoint { line: 5, col: 9 },
                        },
                    },
                    CallItem::MethodCall {
                        name: SmolStr::new_inline("concat"),
                        range: AstRange {
                            start: AstPoint { line: 5, col: 10 },
                            end: AstPoint { line: 5, col: 16 },
                        },

                        args: vec![vec![
                            CallItem::ClassOrVariable {
                                name: SmolStr::new_inline("b"),
                                range: AstRange {
                                    start: AstPoint { line: 5, col: 17 },
                                    end: AstPoint { line: 5, col: 18 },
                                },
                            },
                            CallItem::MethodCall {
                                name: SmolStr::new_inline("a"),
                                range: AstRange {
                                    start: AstPoint { line: 5, col: 19 },
                                    end: AstPoint { line: 5, col: 20 },
                                },
                                args: vec![],
                            },
                        ],],
                    },
                ],
                range: AstRange {
                    start: AstPoint { line: 5, col: 16 },
                    end: AstPoint { line: 5, col: 24 },
                },
                filled_params: vec![vec![
                    CallItem::ClassOrVariable {
                        name: SmolStr::new_inline("b"),
                        range: AstRange {
                            start: AstPoint { line: 5, col: 17 },
                            end: AstPoint { line: 5, col: 18 }
                        }
                    },
                    CallItem::MethodCall {
                        name: SmolStr::new_inline("a"),
                        range: AstRange {
                            start: AstPoint { line: 5, col: 19 },
                            end: AstPoint { line: 5, col: 20 },
                        },
                        args: vec![]
                    },
                ]],
                active_param: Some(0)
            },
            CallItem::ClassOrVariable {
                name: SmolStr::new_inline("b"),
                range: AstRange {
                    start: AstPoint { line: 5, col: 17 },
                    end: AstPoint { line: 5, col: 18 }
                }
            },
            CallItem::MethodCall {
                name: SmolStr::new_inline("a"),
                range: AstRange {
                    start: AstPoint { line: 5, col: 19 },
                    end: AstPoint { line: 5, col: 20 },
                },
                args: vec![]
            },
        ],
        out,
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 14));
    assert_eq!(
        vec![CallItem::ClassOrVariable {
            name: SmolStr::new_inline("a"),
            range: AstRange {
                start: AstPoint { line: 4, col: 12 },
                end: AstPoint { line: 4, col: 13 },
            }
        }],
        out,
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 19));
    let expected = expect![[r#"
        [
            ClassOrVariable {
                name: "b",
                range: AstRange {
                    start: AstPoint { 4:17 },
                    end: AstPoint { 4:18 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 18));
    assert_eq!(
        vec![CallItem::ClassOrVariable {
            name: SmolStr::new_inline("a"),
            range: AstRange {
                start: AstPoint { line: 4, col: 15 },
                end: AstPoint { line: 4, col: 16 }
            }
        },],
        out,
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 22));
    assert_eq!(
        vec![
            CallItem::ClassOrVariable {
                name: SmolStr::new_inline("a"),
                range: AstRange {
                    start: AstPoint { line: 4, col: 15 },
                    end: AstPoint { line: 4, col: 16 },
                }
            },
            CallItem::MethodCall {
                name: SmolStr::new_inline("b"),
                range: AstRange {
                    start: AstPoint { line: 4, col: 17 },
                    end: AstPoint { line: 4, col: 18 },
                },
                args: vec![]
            }
        ],
        out,
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 22));
    let expected = expect![[r#"
        [
            Class {
                name: "String",
                range: AstRange {
                    start: AstPoint { 4:8 },
                    end: AstPoint { 4:20 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 22));
    let expected = expect![[r#"
        [
            Class {
                name: "String",
                range: AstRange {
                    start: AstPoint { 4:8 },
                    end: AstPoint { 4:20 },
                },
            },
            FieldAccess {
                name: "a",
                range: AstRange {
                    start: AstPoint { 4:21 },
                    end: AstPoint { 4:22 },
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&out);
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 25));
    assert_eq!(
        vec![
            CallItem::Class {
                name: SmolStr::new_inline("String"),
                range: AstRange {
                    start: AstPoint { line: 4, col: 8 },
                    end: AstPoint { line: 4, col: 20 },
                }
            },
            CallItem::MethodCall {
                name: SmolStr::new_inline("a"),
                range: AstRange {
                    start: AstPoint { line: 4, col: 21 },
                    end: AstPoint { line: 4, col: 22 },
                },
                args: vec![]
            }
        ],
        out,
    );
}

#[test]
fn call_chain_field_declaration() {
    let content = "
package ch.emilycares;
public class Test {
    private static Logger LOG = Logger.getLogger(Test.class);
}
";
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(3, 43));
    assert_eq!(
        vec![
            CallItem::ClassOrVariable {
                name: SmolStr::new_inline("Logger"),
                range: AstRange {
                    start: AstPoint { line: 3, col: 32 },
                    end: AstPoint { line: 3, col: 38 },
                }
            },
            CallItem::MethodCall {
                name: SmolStr::new_inline("getLogger"),
                range: AstRange {
                    start: AstPoint { line: 3, col: 39 },
                    end: AstPoint { line: 3, col: 48 },
                },
                args: vec![vec![
                    CallItem::ClassOrVariable {
                        name: SmolStr::new_inline("Test"),
                        range: AstRange {
                            start: AstPoint { line: 3, col: 49 },
                            end: AstPoint { line: 3, col: 53 },
                        },
                    },
                    CallItem::FieldAccess {
                        name: SmolStr::new_inline("class"),
                        range: AstRange {
                            start: AstPoint { line: 3, col: 54 },
                            end: AstPoint { line: 3, col: 59 },
                        },
                    },
                ],]
            }
        ],
        out,
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 27));
    assert_eq!(
        vec![
            CallItem::ClassOrVariable {
                name: SmolStr::new_inline("MediaType"),
                range: AstRange {
                    start: AstPoint { line: 4, col: 14 },
                    end: AstPoint { line: 4, col: 23 },
                }
            },
            CallItem::FieldAccess {
                name: SmolStr::new_inline("TEXT_PLAIN"),
                range: AstRange {
                    start: AstPoint { line: 4, col: 24 },
                    end: AstPoint { line: 4, col: 34 },
                }
            }
        ],
        out,
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens);
    ast.print_err(content, &tokens);
    let ast = ast.unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 27));
    assert_eq!(
        vec![
            CallItem::This {
                range: AstRange {
                    start: AstPoint { line: 4, col: 13 },
                    end: AstPoint { line: 4, col: 17 }
                }
            },
            CallItem::FieldAccess {
                name: SmolStr::new_inline("a"),
                range: AstRange {
                    start: AstPoint { line: 4, col: 18 },
                    end: AstPoint { line: 4, col: 19 }
                }
            },
            CallItem::MethodCall {
                name: SmolStr::new_inline("toString"),
                range: AstRange {
                    start: AstPoint { line: 4, col: 20 },
                    end: AstPoint { line: 4, col: 28 }
                },
                args: vec![]
            }
        ],
        out,
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
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 15));
    assert_eq!(
        vec![
            CallItem::This {
                range: AstRange {
                    start: AstPoint { line: 4, col: 6 },
                    end: AstPoint { line: 4, col: 10 }
                }
            },
            CallItem::FieldAccess {
                name: SmolStr::new_inline("asd"),
                range: AstRange {
                    start: AstPoint { line: 4, col: 11 },
                    end: AstPoint { line: 4, col: 14 }
                }
            },
        ],
        out,
    );
}

#[test]
fn call_chain_constructor_with_argument() {
    let content = r#"
package ch.emilycares;
public class Test {
    public String hello() {
      var i = new FileInputStream(new File(""));
      return "";
    }
}
"#;
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 48));
    assert_eq!(
        vec![CallItem::Class {
            name: SmolStr::new_inline("FileInputStream"),
            range: AstRange {
                start: AstPoint { line: 4, col: 14 },
                end: AstPoint { line: 4, col: 47 },
            },
        },],
        out,
    );
}
#[test]
fn call_chain_in_lambda() {
    let content = r#"
public class Test {
    public Uni<Response> test() {
        return Thing.dothing(t -> {
                    Definition q = new Definition();
                    q.
                });
    }
}
"#;
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 23));
    assert_eq!(
        vec![CallItem::ClassOrVariable {
            name: SmolStr::new_inline("q"),
            range: AstRange {
                start: AstPoint { line: 5, col: 20 },
                end: AstPoint { line: 5, col: 21 },
            },
        },],
        out,
    );
}

#[test]
fn call_chain_var_constructor() {
    let content = r#"
package ch.emilycares;
public class Test {
public static Map<Long, String> m = new HashMap<>( );
}
"#;
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(3, 51));
    assert_eq!(
        vec![CallItem::ArgumentList {
            prev: vec![CallItem::ClassGeneric {
                name: SmolStr::new_inline("HashMap"),
                range: AstRange {
                    start: AstPoint { line: 3, col: 36 },
                    end: AstPoint { line: 3, col: 52 },
                },
                args: Vec::new()
            },],
            active_param: Some(0),
            filled_params: vec![vec![]],
            range: AstRange {
                start: AstPoint { line: 3, col: 48 },
                end: AstPoint { line: 3, col: 52 },
            }
        }],
        out,
    );
}

#[test]
fn call_chain_and_expr() {
    let content = r#"
package ch.emilycares;
public class Test {
public boolean test(String a) {
return a.length > 0
         && Other.thing(a)
         && Some.aaaa(a);
}
}
"#;
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(5, 20));
    assert_eq!(
        vec![
            CallItem::ClassOrVariable {
                name: SmolStr::new_inline("Other"),
                range: AstRange {
                    start: AstPoint { line: 5, col: 12 },
                    end: AstPoint { line: 5, col: 17 },
                },
            },
            CallItem::MethodCall {
                name: SmolStr::new_inline("thing"),
                range: AstRange {
                    start: AstPoint { line: 5, col: 18 },
                    end: AstPoint { line: 5, col: 23 },
                },
                args: vec![]
            }
        ],
        out,
    );
}

#[test]
fn call_chain_and_expr_last() {
    let content = r#"
package ch.emilycares;
public class Test {
public boolean test(String a) {
return a.length > 0
         && Other.thing(a)
         && Some.aaaa(a);
}
}
"#;
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(6, 20));
    assert_eq!(
        vec![
            CallItem::ClassOrVariable {
                name: SmolStr::new_inline("Some"),
                range: AstRange {
                    start: AstPoint { line: 6, col: 12 },
                    end: AstPoint { line: 6, col: 16 },
                },
            },
            CallItem::MethodCall {
                name: SmolStr::new_inline("aaaa"),
                range: AstRange {
                    start: AstPoint { line: 6, col: 17 },
                    end: AstPoint { line: 6, col: 21 },
                },
                args: vec![]
            }
        ],
        out,
    );
}

#[test]
fn call_chain_import_method() {
    let content = r#"
package ch.emilycares;
import static org.junit.jupiter.api.Assertions.assertTrue;
"#;
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(2, 28));
    assert_eq!(
        vec![
            CallItem::Class {
                name: SmolStr::new_inline("Assertions"),
                range: AstRange {
                    start: AstPoint { line: 2, col: 14 },
                    end: AstPoint { line: 2, col: 47 },
                },
            },
            CallItem::MethodCall {
                name: SmolStr::new_inline("assertTrue"),
                range: AstRange {
                    start: AstPoint { line: 2, col: 46 },
                    end: AstPoint { line: 2, col: 57 },
                },
                args: vec![]
            }
        ],
        out,
    );
}

#[test]
fn call_chain_array_access() {
    let content = r#"
package ch.emilycares;
public class Test {
public boolean test(String[] a) {
return a[0]. ;
}
}
"#;
    let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
    let ast = ast::parse_file(&tokens).unwrap();

    let out = get_call_chain(&ast, &AstPoint::new(4, 13));
    assert_eq!(
        vec![
            CallItem::ClassOrVariable {
                name: SmolStr::new_inline("a"),
                range: AstRange {
                    start: AstPoint { line: 4, col: 7 },
                    end: AstPoint { line: 4, col: 8 },
                },
            },
            CallItem::ArrayAccess {
                range: AstRange {
                    start: AstPoint { line: 4, col: 8 },
                    end: AstPoint { line: 4, col: 11 },
                }
            }
        ],
        out,
    );
}
