use call_chain::{CallItem, get_call_chain};
use pretty_assertions::assert_eq;
use tree_sitter::{Point, Range};

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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(8, 24));
    assert_eq!(
        out,
        Some(vec![CallItem::ClassOrVariable {
            name: "local".to_string(),
            range: Range {
                start_byte: 125,
                end_byte: 130,
                start_point: Point { row: 8, column: 17 },
                end_point: Point { row: 8, column: 22 }
            }
        }])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 11));
    assert_eq!(
        out,
        Some(vec![CallItem::ClassOrVariable {
            name: "a".to_string(),
            range: Range {
                start_byte: 86,
                end_byte: 87,
                start_point: Point { row: 4, column: 8 },
                end_point: Point { row: 4, column: 9 }
            }
        }])
    );
}

pub const SYMBOL_METHOD: &str = "
package ch.emilycares;

public class Test {

    public void hello() {
        String local = \"\";

        var lo = local.concat(\"hehe\"). 
        return;
    }
}
        ";

#[test]
fn call_chain_method() {
    let (_, tree) = tree_sitter_util::parse(SYMBOL_METHOD).unwrap();

    let out = get_call_chain(&tree, SYMBOL_METHOD.as_bytes(), &Point::new(8, 40));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ClassOrVariable {
                name: "local".to_string(),
                range: Range {
                    start_byte: 117,
                    end_byte: 122,
                    start_point: Point { row: 8, column: 17 },
                    end_point: Point { row: 8, column: 22 },
                }
            },
            CallItem::MethodCall {
                name: "concat".to_string(),
                range: Range {
                    start_byte: 123,
                    end_byte: 129,
                    start_point: Point { row: 8, column: 23 },
                    end_point: Point { row: 8, column: 29 }
                },
            },
            CallItem::FieldAccess {
                name: "return".to_string(),
                range: Range {
                    start_byte: 148,
                    end_byte: 154,
                    start_point: Point { row: 9, column: 8 },
                    end_point: Point { row: 9, column: 14 },
                }
            }
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 19));
    assert_eq!(
        out,
        Some(vec![CallItem::Class {
            name: "String".to_string(),
            range: Range {
                start_byte: 108,
                end_byte: 110,
                start_point: Point { row: 5, column: 15 },
                end_point: Point { row: 5, column: 17 },
            }
        }])
    );
}

#[test]
fn call_chain_field() {
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 26));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ClassOrVariable {
                name: "local".to_string(),
                range: Range {
                    start_byte: 113,
                    end_byte: 118,
                    start_point: Point { row: 5, column: 17 },
                    end_point: Point { row: 5, column: 22 },
                }
            },
            CallItem::FieldAccess {
                name: "a".to_string(),
                range: Range {
                    start_byte: 119,
                    end_byte: 120,
                    start_point: Point { row: 5, column: 23 },
                    end_point: Point { row: 5, column: 24 },
                }
            }
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 24));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ClassOrVariable {
                name: "a".to_string(),
                range: Range {
                    start_byte: 106,
                    end_byte: 107,
                    start_point: Point { row: 5, column: 8 },
                    end_point: Point { row: 5, column: 9 },
                }
            },
            CallItem::MethodCall {
                name: "concat".to_string(),
                range: Range {
                    start_byte: 108,
                    end_byte: 114,
                    start_point: Point { row: 5, column: 10 },
                    end_point: Point { row: 5, column: 16 }
                }
            }
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    // the cursor is on the concat method_call
    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 14));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ClassOrVariable {
                name: "a".to_string(),
                range: Range {
                    start_byte: 92,
                    end_byte: 93,
                    start_point: Point { row: 4, column: 8 },
                    end_point: Point { row: 4, column: 9 },
                }
            },
            CallItem::MethodCall {
                name: "concat".to_string(),
                range: Range {
                    start_byte: 94,
                    end_byte: 100,
                    start_point: Point { row: 4, column: 10 },
                    end_point: Point { row: 4, column: 16 }
                }
            },
            CallItem::MethodCall {
                name: "other".to_string(),
                range: Range {
                    start_byte: 105,
                    end_byte: 110,
                    start_point: Point { row: 4, column: 21 },
                    end_point: Point { row: 4, column: 26 }
                }
            }
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 30));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ClassOrVariable {
                name: "local".to_string(),
                range: Range {
                    start_byte: 113,
                    end_byte: 118,
                    start_point: Point { row: 5, column: 17 },
                    end_point: Point { row: 5, column: 22 }
                }
            },
            CallItem::FieldAccess {
                name: "a".to_string(),
                range: Range {
                    start_byte: 119,
                    end_byte: 120,
                    start_point: Point { row: 5, column: 23 },
                    end_point: Point { row: 5, column: 24 }
                },
            },
            CallItem::MethodCall {
                name: "b".to_string(),
                range: Range {
                    start_byte: 121,
                    end_byte: 122,
                    start_point: Point { row: 5, column: 25 },
                    end_point: Point { row: 5, column: 26 }
                }
            },
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 30));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ClassOrVariable {
                name: "local".to_string(),
                range: Range {
                    start_byte: 113,
                    end_byte: 118,
                    start_point: Point { row: 5, column: 17 },
                    end_point: Point { row: 5, column: 22 },
                }
            },
            CallItem::MethodCall {
                name: "a".to_string(),
                range: Range {
                    start_byte: 119,
                    end_byte: 120,
                    start_point: Point { row: 5, column: 23 },
                    end_point: Point { row: 5, column: 24 },
                }
            },
            CallItem::FieldAccess {
                name: "b".to_string(),
                range: Range {
                    start_byte: 123,
                    end_byte: 124,
                    start_point: Point { row: 5, column: 27 },
                    end_point: Point { row: 5, column: 28 },
                }
            }
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 23));
    assert_eq!(
        out,
        Some(vec![CallItem::ClassOrVariable {
            name: "local".to_string(),
            range: Range {
                start_byte: 113,
                end_byte: 118,
                start_point: Point { row: 5, column: 17 },
                end_point: Point { row: 5, column: 22 },
            }
        }])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 28));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ClassOrVariable {
                name: "local".to_string(),
                range: Range {
                    start_byte: 112,
                    end_byte: 117,
                    start_point: Point { row: 5, column: 16 },
                    end_point: Point { row: 5, column: 21 },
                }
            },
            CallItem::MethodCall {
                name: "a".to_string(),
                range: Range {
                    start_byte: 118,
                    end_byte: 119,
                    start_point: Point { row: 5, column: 22 },
                    end_point: Point { row: 5, column: 23 },
                }
            },
            CallItem::FieldAccess {
                name: "c".to_string(),
                range: Range {
                    start_byte: 122,
                    end_byte: 123,
                    start_point: Point { row: 5, column: 26 },
                    end_point: Point { row: 5, column: 27 },
                }
            },
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 28));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ClassOrVariable {
                name: "local".to_string(),
                range: Range {
                    start_byte: 112,
                    end_byte: 117,
                    start_point: Point { row: 5, column: 16 },
                    end_point: Point { row: 5, column: 21 }
                }
            },
            CallItem::FieldAccess {
                name: "a".to_string(),
                range: Range {
                    start_byte: 118,
                    end_byte: 119,
                    start_point: Point { row: 5, column: 22 },
                    end_point: Point { row: 5, column: 23 },
                }
            },
            CallItem::MethodCall {
                name: "c".to_string(),
                range: Range {
                    start_byte: 120,
                    end_byte: 121,
                    start_point: Point { row: 5, column: 24 },
                    end_point: Point { row: 5, column: 25 },
                }
            },
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 20));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ClassOrVariable {
                name: "local".to_string(),
                range: Range {
                    start_byte: 104,
                    end_byte: 109,
                    start_point: Point { row: 5, column: 8 },
                    end_point: Point { row: 5, column: 13 },
                }
            },
            CallItem::FieldAccess {
                name: "a".to_string(),
                range: Range {
                    start_byte: 110,
                    end_byte: 111,
                    start_point: Point { row: 5, column: 14 },
                    end_point: Point { row: 5, column: 15 },
                }
            },
            CallItem::MethodCall {
                name: "c".to_string(),
                range: Range {
                    start_byte: 112,
                    end_byte: 113,
                    start_point: Point { row: 5, column: 16 },
                    end_point: Point { row: 5, column: 17 },
                }
            },
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 16));
    assert_eq!(
        out,
        Some(vec![CallItem::ClassOrVariable {
            name: "String".to_string(),
            range: Range {
                start_byte: 104,
                end_byte: 110,
                start_point: Point { row: 5, column: 8 },
                end_point: Point { row: 5, column: 14 },
            }
        },])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 28));
    assert_eq!(
        out,
        Some(vec![CallItem::ClassOrVariable {
            name: "String".to_string(),
            range: Range {
                start_byte: 116,
                end_byte: 122,
                start_point: Point { row: 5, column: 20 },
                end_point: Point { row: 5, column: 26 },
            }
        },])
    );
}

#[test]
fn call_chain_argument() {
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 22));
    assert_eq!(
        out,
        Some(vec![CallItem::ArgumentList {
            prev: vec![
                CallItem::ClassOrVariable {
                    name: "local".to_string(),
                    range: Range {
                        start_byte: 104,
                        end_byte: 109,
                        start_point: Point { row: 5, column: 8 },
                        end_point: Point { row: 5, column: 13 },
                    },
                },
                CallItem::MethodCall {
                    name: "concat".to_string(),
                    range: Range {
                        start_byte: 110,
                        end_byte: 116,
                        start_point: Point { row: 5, column: 14 },
                        end_point: Point { row: 5, column: 20 },
                    },
                },
            ],
            range: Range {
                start_byte: 116,
                end_byte: 119,
                start_point: Point { row: 5, column: 20 },
                end_point: Point { row: 5, column: 23 },
            },
            filled_params: vec![vec![]],
            active_param: 0
        },],)
    );
}

#[test]
fn call_chain_argument_var() {
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 27));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ArgumentList {
                prev: vec![
                    CallItem::ClassOrVariable {
                        name: "local".to_string(),
                        range: Range {
                            start_byte: 104,
                            end_byte: 109,
                            start_point: Point { row: 5, column: 8 },
                            end_point: Point { row: 5, column: 13 },
                        },
                    },
                    CallItem::MethodCall {
                        name: "concat".to_string(),
                        range: Range {
                            start_byte: 110,
                            end_byte: 116,
                            start_point: Point { row: 5, column: 14 },
                            end_point: Point { row: 5, column: 20 },
                        },
                    },
                ],
                range: Range {
                    start_byte: 116,
                    end_byte: 125,
                    start_point: Point { row: 5, column: 20 },
                    end_point: Point { row: 5, column: 29 },
                },
                filled_params: vec![vec![CallItem::ClassOrVariable {
                    name: "local".to_string(),
                    range: Range {
                        start_byte: 117,
                        end_byte: 122,
                        start_point: Point { row: 5, column: 21 },
                        end_point: Point { row: 5, column: 26 }
                    }
                }]],
                active_param: 0
            },
            CallItem::ClassOrVariable {
                name: "local".to_string(),
                range: Range {
                    start_byte: 117,
                    end_byte: 122,
                    start_point: Point { row: 5, column: 21 },
                    end_point: Point { row: 5, column: 26 }
                }
            }
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 27));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ArgumentList {
                prev: vec![
                    CallItem::ClassOrVariable {
                        name: "local".to_string(),
                        range: Range {
                            start_byte: 104,
                            end_byte: 109,
                            start_point: Point { row: 5, column: 8 },
                            end_point: Point { row: 5, column: 13 },
                        },
                    },
                    CallItem::MethodCall {
                        name: "concat".to_string(),
                        range: Range {
                            start_byte: 110,
                            end_byte: 116,
                            start_point: Point { row: 5, column: 14 },
                            end_point: Point { row: 5, column: 20 },
                        },
                    },
                ],
                range: Range {
                    start_byte: 116,
                    end_byte: 124,
                    start_point: Point { row: 5, column: 20 },
                    end_point: Point { row: 5, column: 28 },
                },
                filled_params: vec![vec![CallItem::ClassOrVariable {
                    name: "local".to_string(),
                    range: Range {
                        start_byte: 117,
                        end_byte: 122,
                        start_point: Point { row: 5, column: 21 },
                        end_point: Point { row: 5, column: 26 }
                    }
                }]],
                active_param: 0
            },
            CallItem::ClassOrVariable {
                name: "local".to_string(),
                range: Range {
                    start_byte: 117,
                    end_byte: 122,
                    start_point: Point { row: 5, column: 21 },
                    end_point: Point { row: 5, column: 26 }
                }
            }
        ])
    );
}

#[test]
fn call_chain_argument_second_var() {
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 23));
    match out.clone().unwrap().first().unwrap() {
        CallItem::ArgumentList {
            prev: _,
            active_param,
            filled_params: _,
            range: _,
        } => assert_eq!(active_param, &1),
        _ => unreachable!(),
    };
    assert_eq!(
        out,
        Some(vec![
            CallItem::ArgumentList {
                prev: vec![
                    CallItem::ClassOrVariable {
                        name: "a".to_string(),
                        range: Range {
                            start_byte: 104,
                            end_byte: 105,
                            start_point: Point { row: 5, column: 8 },
                            end_point: Point { row: 5, column: 9 },
                        },
                    },
                    CallItem::MethodCall {
                        name: "concat".to_string(),
                        range: Range {
                            start_byte: 106,
                            end_byte: 112,
                            start_point: Point { row: 5, column: 10 },
                            end_point: Point { row: 5, column: 16 },
                        },
                    },
                ],
                range: Range {
                    start_byte: 112,
                    end_byte: 120,
                    start_point: Point { row: 5, column: 16 },
                    end_point: Point { row: 5, column: 24 },
                },
                filled_params: vec![
                    vec![CallItem::ClassOrVariable {
                        name: "b".to_string(),
                        range: Range {
                            start_byte: 113,
                            end_byte: 114,
                            start_point: Point { row: 5, column: 17 },
                            end_point: Point { row: 5, column: 18 },
                        },
                    }],
                    vec![CallItem::ClassOrVariable {
                        name: "c".to_string(),
                        range: Range {
                            start_byte: 116,
                            end_byte: 117,
                            start_point: Point { row: 5, column: 20 },
                            end_point: Point { row: 5, column: 21 },
                        },
                    }]
                ],
                active_param: 1
            },
            CallItem::ClassOrVariable {
                name: "c".to_string(),
                range: Range {
                    start_byte: 116,
                    end_byte: 117,
                    start_point: Point { row: 5, column: 20 },
                    end_point: Point { row: 5, column: 21 },
                }
            }
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 19));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ArgumentList {
                prev: vec![
                    CallItem::ClassOrVariable {
                        name: "a".to_string(),
                        range: Range {
                            start_byte: 104,
                            end_byte: 105,
                            start_point: Point { row: 5, column: 8 },
                            end_point: Point { row: 5, column: 9 },
                        },
                    },
                    CallItem::MethodCall {
                        name: "concat".to_string(),
                        range: Range {
                            start_byte: 106,
                            end_byte: 112,
                            start_point: Point { row: 5, column: 10 },
                            end_point: Point { row: 5, column: 16 },
                        },
                    },
                ],
                range: Range {
                    start_byte: 112,
                    end_byte: 119,
                    start_point: Point { row: 5, column: 16 },
                    end_point: Point { row: 5, column: 23 },
                },
                filled_params: vec![
                    vec![CallItem::ClassOrVariable {
                        name: "b".to_string(),
                        range: Range {
                            start_byte: 113,
                            end_byte: 114,
                            start_point: Point { row: 5, column: 17 },
                            end_point: Point { row: 5, column: 18 },
                        }
                    }],
                    vec![CallItem::ClassOrVariable {
                        name: "c".to_string(),
                        range: Range {
                            start_byte: 117,
                            end_byte: 118,
                            start_point: Point { row: 5, column: 21 },
                            end_point: Point { row: 5, column: 22 },
                        }
                    }],
                ],
                active_param: 0
            },
            CallItem::ClassOrVariable {
                name: "b".to_string(),
                range: Range {
                    start_byte: 113,
                    end_byte: 114,
                    start_point: Point { row: 5, column: 17 },
                    end_point: Point { row: 5, column: 18 },
                }
            }
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 22));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ArgumentList {
                prev: vec![
                    CallItem::ClassOrVariable {
                        name: "a".to_string(),
                        range: Range {
                            start_byte: 104,
                            end_byte: 105,
                            start_point: Point { row: 5, column: 8 },
                            end_point: Point { row: 5, column: 9 },
                        },
                    },
                    CallItem::MethodCall {
                        name: "concat".to_string(),
                        range: Range {
                            start_byte: 106,
                            end_byte: 112,
                            start_point: Point { row: 5, column: 10 },
                            end_point: Point { row: 5, column: 16 },
                        },
                    },
                ],
                range: Range {
                    start_byte: 112,
                    end_byte: 119,
                    start_point: Point { row: 5, column: 16 },
                    end_point: Point { row: 5, column: 23 },
                },
                filled_params: vec![
                    vec![CallItem::ClassOrVariable {
                        name: "b".to_string(),
                        range: Range {
                            start_byte: 113,
                            end_byte: 114,
                            start_point: Point { row: 5, column: 17 },
                            end_point: Point { row: 5, column: 18 },
                        }
                    }],
                    vec![CallItem::ClassOrVariable {
                        name: "c".to_string(),
                        range: Range {
                            start_byte: 116,
                            end_byte: 117,
                            start_point: Point { row: 5, column: 20 },
                            end_point: Point { row: 5, column: 21 },
                        }
                    }]
                ],
                active_param: 1
            },
            CallItem::ClassOrVariable {
                name: "c".to_string(),
                range: Range {
                    start_byte: 116,
                    end_byte: 117,
                    start_point: Point { row: 5, column: 20 },
                    end_point: Point { row: 5, column: 21 },
                }
            }
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 22));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ArgumentList {
                prev: vec![
                    CallItem::ClassOrVariable {
                        name: "a".to_string(),
                        range: Range {
                            start_byte: 104,
                            end_byte: 105,
                            start_point: Point { row: 5, column: 8 },
                            end_point: Point { row: 5, column: 9 },
                        },
                    },
                    CallItem::MethodCall {
                        name: "concat".to_string(),
                        range: Range {
                            start_byte: 106,
                            end_byte: 112,
                            start_point: Point { row: 5, column: 10 },
                            end_point: Point { row: 5, column: 16 },
                        },
                    },
                ],
                range: Range {
                    start_byte: 112,
                    end_byte: 119,
                    start_point: Point { row: 5, column: 16 },
                    end_point: Point { row: 5, column: 23 },
                },
                filled_params: vec![vec![
                    CallItem::ClassOrVariable {
                        name: "b".to_string(),
                        range: Range {
                            start_byte: 113,
                            end_byte: 114,
                            start_point: Point { row: 5, column: 17 },
                            end_point: Point { row: 5, column: 18 },
                        }
                    },
                    CallItem::FieldAccess {
                        name: "a".to_string(),
                        range: Range {
                            start_byte: 115,
                            end_byte: 116,
                            start_point: Point { row: 5, column: 19 },
                            end_point: Point { row: 5, column: 20 }
                        },
                    },
                ]],
                active_param: 0
            },
            CallItem::ClassOrVariable {
                name: "b".to_string(),
                range: Range {
                    start_byte: 113,
                    end_byte: 114,
                    start_point: Point { row: 5, column: 17 },
                    end_point: Point { row: 5, column: 18 },
                }
            },
            CallItem::FieldAccess {
                name: "a".to_string(),
                range: Range {
                    start_byte: 115,
                    end_byte: 116,
                    start_point: Point { row: 5, column: 19 },
                    end_point: Point { row: 5, column: 20 }
                },
            },
        ])
    );
}

#[test]
fn call_chain_argument_calc() {
    let content = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        concat("a" + a.getThing());
        return;
    }
}
"#;
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 28));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ArgumentList {
                prev: vec![CallItem::MethodCall {
                    name: "concat".to_string(),
                    range: Range {
                        start_byte: 78,
                        end_byte: 84,
                        start_point: Point { row: 4, column: 8 },
                        end_point: Point { row: 4, column: 14 }
                    },
                },],
                filled_params: vec![vec![
                    CallItem::ClassOrVariable {
                        name: "a".to_string(),
                        range: Range {
                            start_byte: 91,
                            end_byte: 92,
                            start_point: Point { row: 4, column: 21 },
                            end_point: Point { row: 4, column: 22 },
                        }
                    },
                    CallItem::MethodCall {
                        name: "getThing".to_string(),
                        range: Range {
                            start_byte: 93,
                            end_byte: 101,
                            start_point: Point { row: 4, column: 23 },
                            end_point: Point { row: 4, column: 31 }
                        },
                    },
                ]],
                active_param: 0,
                range: Range {
                    start_byte: 84,
                    end_byte: 104,
                    start_point: Point { row: 4, column: 14 },
                    end_point: Point { row: 4, column: 34 },
                },
            },
            CallItem::ClassOrVariable {
                name: "a".to_string(),
                range: Range {
                    start_byte: 91,
                    end_byte: 92,
                    start_point: Point { row: 4, column: 21 },
                    end_point: Point { row: 4, column: 22 },
                }
            },
            CallItem::MethodCall {
                name: "getThing".to_string(),
                range: Range {
                    start_byte: 93,
                    end_byte: 101,
                    start_point: Point { row: 4, column: 23 },
                    end_point: Point { row: 4, column: 31 }
                },
            },
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 18));
    assert_eq!(
        out,
        Some(vec![CallItem::ArgumentList {
            prev: vec![
                CallItem::ClassOrVariable {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 78,
                        end_byte: 79,
                        start_point: Point { row: 4, column: 8 },
                        end_point: Point { row: 4, column: 9 },
                    },
                },
                CallItem::MethodCall {
                    name: "concat".to_string(),
                    range: Range {
                        start_byte: 80,
                        end_byte: 86,
                        start_point: Point { row: 4, column: 10 },
                        end_point: Point { row: 4, column: 16 },
                    },
                }
            ],
            range: Range {
                start_byte: 86,
                end_byte: 89,
                start_point: Point { row: 4, column: 16 },
                end_point: Point { row: 4, column: 19 },
            },
            filled_params: vec![vec![]],
            active_param: 0
        }])
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 23));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ArgumentList {
                prev: vec![
                    CallItem::ClassOrVariable {
                        name: "a".to_string(),
                        range: Range {
                            start_byte: 104,
                            end_byte: 105,
                            start_point: Point { row: 5, column: 8 },
                            end_point: Point { row: 5, column: 9 },
                        },
                    },
                    CallItem::MethodCall {
                        name: "concat".to_string(),
                        range: Range {
                            start_byte: 106,
                            end_byte: 112,
                            start_point: Point { row: 5, column: 10 },
                            end_point: Point { row: 5, column: 16 },
                        },
                    },
                ],
                range: Range {
                    start_byte: 112,
                    end_byte: 120,
                    start_point: Point { row: 5, column: 16 },
                    end_point: Point { row: 5, column: 24 },
                },
                filled_params: vec![vec![
                    CallItem::ClassOrVariable {
                        name: "b".to_string(),
                        range: Range {
                            start_byte: 113,
                            end_byte: 114,
                            start_point: Point { row: 5, column: 17 },
                            end_point: Point { row: 5, column: 18 }
                        }
                    },
                    CallItem::MethodCall {
                        name: "a".to_string(),
                        range: Range {
                            start_byte: 115,
                            end_byte: 116,
                            start_point: Point { row: 5, column: 19 },
                            end_point: Point { row: 5, column: 20 },
                        }
                    },
                ]],
                active_param: 0
            },
            CallItem::ClassOrVariable {
                name: "b".to_string(),
                range: Range {
                    start_byte: 113,
                    end_byte: 114,
                    start_point: Point { row: 5, column: 17 },
                    end_point: Point { row: 5, column: 18 }
                }
            },
            CallItem::MethodCall {
                name: "a".to_string(),
                range: Range {
                    start_byte: 115,
                    end_byte: 116,
                    start_point: Point { row: 5, column: 19 },
                    end_point: Point { row: 5, column: 20 },
                }
            },
        ])
    );
}

#[test]
fn call_chain_if() {
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 14));
    assert_eq!(
        out,
        Some(vec![CallItem::ClassOrVariable {
            name: "a".to_string(),
            range: Range {
                start_byte: 82,
                end_byte: 83,
                start_point: Point { row: 4, column: 12 },
                end_point: Point { row: 4, column: 13 },
            }
        }])
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 19));
    assert_eq!(
        out,
        Some(vec![CallItem::ClassOrVariable {
            name: "b".to_string(),
            range: Range {
                start_byte: 87,
                end_byte: 88,
                start_point: Point { row: 4, column: 17 },
                end_point: Point { row: 4, column: 18 },
            }
        },])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 18));
    assert_eq!(
        out,
        Some(vec![CallItem::ClassOrVariable {
            name: "a".to_string(),
            range: Range {
                start_byte: 85,
                end_byte: 86,
                start_point: Point { row: 4, column: 15 },
                end_point: Point { row: 4, column: 16 }
            }
        },])
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 22));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ClassOrVariable {
                name: "a".to_string(),
                range: Range {
                    start_byte: 85,
                    end_byte: 86,
                    start_point: Point { row: 4, column: 15 },
                    end_point: Point { row: 4, column: 16 },
                }
            },
            CallItem::MethodCall {
                name: "b".to_string(),
                range: Range {
                    start_byte: 87,
                    end_byte: 88,
                    start_point: Point { row: 4, column: 17 },
                    end_point: Point { row: 4, column: 18 },
                }
            }
        ])
    );
}

#[test]
fn call_chain_new_instance() {
    let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        new String()
        return;
    }
}
";
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 22));
    assert_eq!(
        out,
        Some(vec![CallItem::Class {
            name: "String".to_string(),
            range: Range {
                start_byte: 82,
                end_byte: 88,
                start_point: Point { row: 4, column: 12 },
                end_point: Point { row: 4, column: 18 }
            }
        }])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 22));
    assert_eq!(
        out,
        Some(vec![
            CallItem::Class {
                name: "String".to_string(),
                range: Range {
                    start_byte: 82,
                    end_byte: 88,
                    start_point: Point { row: 4, column: 12 },
                    end_point: Point { row: 4, column: 18 },
                }
            },
            CallItem::FieldAccess {
                name: "a".to_string(),
                range: Range {
                    start_byte: 91,
                    end_byte: 92,
                    start_point: Point { row: 4, column: 21 },
                    end_point: Point { row: 4, column: 22 },
                }
            }
        ])
    );
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 25));
    assert_eq!(
        out,
        Some(vec![
            CallItem::Class {
                name: "String".to_string(),
                range: Range {
                    start_byte: 82,
                    end_byte: 88,
                    start_point: Point { row: 4, column: 12 },
                    end_point: Point { row: 4, column: 18 },
                }
            },
            CallItem::MethodCall {
                name: "a".to_string(),
                range: Range {
                    start_byte: 91,
                    end_byte: 92,
                    start_point: Point { row: 4, column: 21 },
                    end_point: Point { row: 4, column: 22 },
                }
            }
        ])
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(3, 43));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ClassOrVariable {
                name: "Logger".to_string(),
                range: Range {
                    start_byte: 76,
                    end_byte: 82,
                    start_point: Point { row: 3, column: 32 },
                    end_point: Point { row: 3, column: 38 },
                }
            },
            CallItem::MethodCall {
                name: "getLogger".to_string(),
                range: Range {
                    start_byte: 83,
                    end_byte: 92,
                    start_point: Point { row: 3, column: 39 },
                    end_point: Point { row: 3, column: 48 },
                }
            }
        ])
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 27));
    assert_eq!(
        out,
        Some(vec![
            CallItem::ClassOrVariable {
                name: "MediaType".to_string(),
                range: Range {
                    start_byte: 89,
                    end_byte: 98,
                    start_point: Point { row: 4, column: 14 },
                    end_point: Point { row: 4, column: 23 },
                }
            },
            CallItem::FieldAccess {
                name: "TEXT_PLAIN".to_string(),
                range: Range {
                    start_byte: 99,
                    end_byte: 109,
                    start_point: Point { row: 4, column: 24 },
                    end_point: Point { row: 4, column: 34 },
                }
            }
        ])
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 27));
    assert_eq!(
        out,
        Some(vec![
            CallItem::This {
                range: Range {
                    start_byte: 85,
                    end_byte: 89,
                    start_point: Point { row: 4, column: 13 },
                    end_point: Point { row: 4, column: 17 }
                }
            },
            CallItem::FieldAccess {
                name: "a".to_string(),
                range: Range {
                    start_byte: 90,
                    end_byte: 91,
                    start_point: Point { row: 4, column: 18 },
                    end_point: Point { row: 4, column: 19 }
                }
            },
            CallItem::MethodCall {
                name: "toString".to_string(),
                range: Range {
                    start_byte: 92,
                    end_byte: 100,
                    start_point: Point { row: 4, column: 20 },
                    end_point: Point { row: 4, column: 28 }
                }
            }
        ])
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
    let (_, tree) = tree_sitter_util::parse(content).unwrap();

    let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 13));
    assert_eq!(
        out,
        Some(vec![
            CallItem::This {
                range: Range {
                    start_byte: 78,
                    end_byte: 82,
                    start_point: Point { row: 4, column: 6 },
                    end_point: Point { row: 4, column: 10 }
                }
            },
            CallItem::FieldAccess {
                name: "asd".to_string(),
                range: Range {
                    start_byte: 83,
                    end_byte: 86,
                    start_point: Point { row: 4, column: 11 },
                    end_point: Point { row: 4, column: 14 }
                }
            },
        ])
    );
}
