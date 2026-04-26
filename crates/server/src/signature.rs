use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ast::types::AstPoint;
use call_chain::CallItem;
use document::Document;
use dto::{Class, ImportUnit, Method};
use local_variable::LocalVariable;
use lsp_types::{
    Documentation, ParameterInformation, ParameterLabel, SignatureHelp, SignatureInformation,
};
use my_string::MyString;
use variables::{VariableContext, VariablesError};

#[derive(Debug)]
pub enum SignatureError {
    Tyres(tyres::TyresError),
    NotAnArgumentList,
    CouldNotGetMethod,
    CouldNoteGetActiveSignature,
    NoCallChain,
    Variables(VariablesError),
}

pub fn signature_driver(
    document: &Document,
    point: &AstPoint,
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
) -> Result<SignatureHelp, SignatureError> {
    let call_chain = call_chain::get_call_chain(&document.ast, point);
    let imports = imports::imports(&document.ast);
    let vars = variables::get_vars(
        &document.ast,
        &VariableContext {
            point: Some(*point),
            imports: &imports,
            class,
            class_map: class_map.clone(),
        },
    )
    .map_err(SignatureError::Variables)?;
    get_signature(&call_chain, &imports, &vars, class, class_map)
}
pub fn get_signature(
    call_chain: &[CallItem],
    imports: &[ImportUnit],
    vars: &[LocalVariable],
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
) -> Result<SignatureHelp, SignatureError> {
    let args = get_args(call_chain);
    let Some(CallItem::ArgumentList {
        prev,
        range: _,
        active_param,
        filled_params,
    }) = args
    else {
        return Err(SignatureError::NotAnArgumentList);
    };
    let Some(active_param) = active_param else {
        return Err(SignatureError::NotAnArgumentList);
    };
    let num_params = filled_params.len();
    match &prev.last() {
        Some(CallItem::MethodCall {
            name: method_name,
            args: _,
            range: _,
        }) => signature_help_for_method(
            imports,
            vars,
            class,
            class_map,
            prev,
            *active_param,
            num_params,
            method_name,
        ),
        Some(CallItem::Class { .. } | CallItem::ClassGeneric { .. }) => {
            signature_help_for_constructor(
                imports,
                vars,
                class,
                class_map,
                prev,
                *active_param,
                num_params,
            )
        }

        Some(_) | None => Err(SignatureError::CouldNotGetMethod),
    }
}

#[allow(clippy::too_many_arguments)]
fn signature_help_for_method(
    imports: &[ImportUnit],
    vars: &[LocalVariable],
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    prev: &[CallItem],
    active_param: usize,
    num_params: usize,
    method_name: &str,
) -> Result<SignatureHelp, SignatureError> {
    // trim last method call
    let prev = &prev[..1];
    let resolve_state = match tyres::resolve_call_chain(prev, vars, imports, class, class_map) {
        Ok(c) => Ok(c),
        Err(e) => Err(SignatureError::Tyres(e)),
    }?;
    let methods: Vec<&Method> = resolve_state
        .class
        .methods
        .iter()
        .filter(|i| i.name.as_ref().is_some_and(|i| *i == method_name))
        .collect();

    let Some(active_signature) = methods
        .iter()
        .enumerate()
        .find(|(_, m)| m.parameters.len() >= num_params)
    else {
        return Err(SignatureError::CouldNoteGetActiveSignature);
    };
    let active_signature_id = active_signature.0;
    let signatures = methods
        .iter()
        .map(|m| method_to_signature_information(m, &resolve_state.class.name))
        .collect();

    Ok(SignatureHelp {
        signatures,
        active_signature: TryInto::<u32>::try_into(active_signature_id).ok(),
        active_parameter: TryInto::<u32>::try_into(active_param).ok(),
    })
}
fn signature_help_for_constructor(
    imports: &[ImportUnit],
    vars: &[LocalVariable],
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    prev: &[CallItem],
    active_param: usize,
    num_params: usize,
) -> Result<SignatureHelp, SignatureError> {
    // trim last method call
    let prev = &prev[..1];
    let resolve_state = match tyres::resolve_call_chain(prev, vars, imports, class, class_map) {
        Ok(c) => Ok(c),
        Err(e) => Err(SignatureError::Tyres(e)),
    }?;
    let methods: Vec<&Method> = resolve_state
        .class
        .methods
        .iter()
        .filter(|m| m.name.is_none())
        .collect();

    let Some(active_signature) = methods
        .iter()
        .enumerate()
        .find(|(_, m)| m.parameters.len() >= num_params)
    else {
        return Err(SignatureError::CouldNoteGetActiveSignature);
    };
    let active_signature_id = active_signature.0;
    let signatures = methods
        .iter()
        .map(|m| method_to_signature_information(m, &resolve_state.class.name))
        .collect();

    Ok(SignatureHelp {
        signatures,
        active_signature: TryInto::<u32>::try_into(active_signature_id).ok(),
        active_parameter: TryInto::<u32>::try_into(active_param).ok(),
    })
}

fn get_args(call_chain: &[CallItem]) -> Option<&CallItem> {
    call_chain.iter().rev().find(|i| match i {
        CallItem::MethodCall { .. }
        | CallItem::FieldAccess { .. }
        | CallItem::Variable { .. }
        | CallItem::Class { .. }
        | CallItem::ClassGeneric { .. }
        | CallItem::ClassOrVariable { .. }
        | CallItem::ArrayAccess { .. }
        | CallItem::This { .. } => false,
        CallItem::ArgumentList {
            prev: _,
            range: _,
            active_param: _,
            filled_params: _,
        } => true,
    })
}

fn method_to_signature_information(method: &Method, class_name: &str) -> SignatureInformation {
    let mut label = method
        .name
        .as_ref()
        .map_or_else(|| format!("{class_name}("), |n| format!("{n}("));
    let mut parameters = Vec::with_capacity(method.parameters.len());
    let mut peekable = method.parameters.iter().peekable();
    while let Some(param) = peekable.next() {
        let jtype = param.jtype.to_string();
        if let Some(name) = &param.name {
            let named = format!("{jtype} {name}");
            label.push_str(&named);
            parameters.push(ParameterInformation {
                label: ParameterLabel::Simple(named),
                documentation: None,
            });
        } else {
            let ty = jtype.clone();
            label.push_str(&ty);
            parameters.push(ParameterInformation {
                label: ParameterLabel::Simple(ty),
                documentation: None,
            });
        }
        if peekable.peek().is_some() {
            label.push_str(", ");
        }
    }
    label.push(')');
    SignatureInformation {
        label,
        documentation: Some(Documentation::String(method.ret.to_string())),
        parameters: Some(parameters),
        active_parameter: None,
    }
}

#[cfg(test)]
pub mod tests {
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use super::signature_driver;
    use ast::types::AstPoint;
    use document::Document;

    use dto::{Access, Class, JType, Method, Parameter};
    use expect_test::expect;
    use my_string::{MyString, smol_str::SmolStr};

    #[test]
    fn signarure_base() {
        let mut class_map: HashMap<MyString, Class> = HashMap::new();
        class_map.insert(
            SmolStr::new_inline("java.lang.String"),
            Class {
                access: Access::Public,
                name: SmolStr::new_inline("String"),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some(SmolStr::new_inline("concat")),
                    parameters: vec![Parameter {
                        name: None,
                        jtype: JType::Class(SmolStr::new_inline("java.lang.String")),
                    }],
                    ret: JType::Class(SmolStr::new_inline("java.lang.String")),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let class_map = Arc::new(Mutex::new(class_map));
        let class = Class {
            access: Access::Public,
            name: SmolStr::new_inline("Test"),
            ..Default::default()
        };
        let content = "
package ch.emilycares;
public class Test {
    public String hello() {
        String local = \"hey \";
        return local.concat( );
    }
}
";
        let doc = Document::setup(content, PathBuf::new()).unwrap();

        let out = signature_driver(&doc, &AstPoint::new(5, 29), &class, &class_map).unwrap();
        let expected = expect![[r#"
            SignatureHelp {
                signatures: [
                    SignatureInformation {
                        label: "concat(String)",
                        documentation: Some(
                            String(
                                "String",
                            ),
                        ),
                        parameters: Some(
                            [
                                ParameterInformation {
                                    label: Simple(
                                        "String",
                                    ),
                                    documentation: None,
                                },
                            ],
                        ),
                        active_parameter: None,
                    },
                ],
                active_signature: Some(
                    0,
                ),
                active_parameter: Some(
                    0,
                ),
            }
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn signature_multi_name() {
        let mut class_map: HashMap<MyString, Class> = HashMap::new();
        class_map.insert(
            SmolStr::new_inline("java.lang.String"),
            Class {
                access: Access::Public,
                name: SmolStr::new_inline("String"),
                methods: vec![
                    Method {
                        access: Access::Public,
                        name: Some(SmolStr::new_inline("concat")),
                        parameters: vec![Parameter {
                            name: None,
                            jtype: JType::Class(SmolStr::new_inline("java.lang.String")),
                        }],
                        ret: JType::Class(SmolStr::new_inline("java.lang.String")),
                        throws: vec![],
                        source: None,
                    },
                    Method {
                        access: Access::Public,
                        name: Some(SmolStr::new_inline("concat")),
                        parameters: vec![
                            Parameter {
                                name: None,
                                jtype: JType::Class(SmolStr::new_inline("java.lang.String")),
                            },
                            Parameter {
                                name: None,
                                jtype: JType::Class(SmolStr::new_inline("java.lang.String")),
                            },
                        ],
                        ret: JType::Class(SmolStr::new_inline("java.lang.String")),
                        throws: vec![],
                        source: None,
                    },
                ],
                ..Default::default()
            },
        );
        let class_map = Arc::new(Mutex::new(class_map));
        let class = Class {
            access: Access::Public,
            name: SmolStr::new_inline("Test"),
            ..Default::default()
        };
        let content = "
package ch.emilycares;
public class Test {
    public String hello() {
        String local = \"hey \";
        return local.concat( );
    }
}
";
        let doc = Document::setup(content, PathBuf::new()).unwrap();

        let out = signature_driver(&doc, &AstPoint::new(5, 29), &class, &class_map).unwrap();
        let expected = expect![[r#"
            SignatureHelp {
                signatures: [
                    SignatureInformation {
                        label: "concat(String)",
                        documentation: Some(
                            String(
                                "String",
                            ),
                        ),
                        parameters: Some(
                            [
                                ParameterInformation {
                                    label: Simple(
                                        "String",
                                    ),
                                    documentation: None,
                                },
                            ],
                        ),
                        active_parameter: None,
                    },
                    SignatureInformation {
                        label: "concat(String, String)",
                        documentation: Some(
                            String(
                                "String",
                            ),
                        ),
                        parameters: Some(
                            [
                                ParameterInformation {
                                    label: Simple(
                                        "String",
                                    ),
                                    documentation: None,
                                },
                                ParameterInformation {
                                    label: Simple(
                                        "String",
                                    ),
                                    documentation: None,
                                },
                            ],
                        ),
                        active_parameter: None,
                    },
                ],
                active_signature: Some(
                    0,
                ),
                active_parameter: Some(
                    0,
                ),
            }
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn signature_multi_name_second() {
        let mut class_map: HashMap<MyString, Class> = HashMap::new();
        class_map.insert(
            SmolStr::new_inline("java.lang.String"),
            Class {
                access: Access::Public,
                name: SmolStr::new_inline("String"),
                methods: vec![
                    Method {
                        access: Access::Public,
                        name: Some(SmolStr::new_inline("concat")),
                        parameters: vec![Parameter {
                            name: None,
                            jtype: JType::Class(SmolStr::new_inline("java.lang.String")),
                        }],
                        ret: JType::Class(SmolStr::new_inline("java.lang.String")),
                        throws: vec![],
                        source: None,
                    },
                    Method {
                        access: Access::Public,
                        name: Some(SmolStr::new_inline("concat")),
                        parameters: vec![
                            Parameter {
                                name: None,
                                jtype: JType::Class(SmolStr::new_inline("java.lang.String")),
                            },
                            Parameter {
                                name: None,
                                jtype: JType::Class(SmolStr::new_inline("java.lang.String")),
                            },
                        ],
                        ret: JType::Class(SmolStr::new_inline("java.lang.String")),
                        throws: vec![],
                        source: None,
                    },
                ],
                ..Default::default()
            },
        );
        let class_map = Arc::new(Mutex::new(class_map));
        let class = Class {
            access: Access::Public,
            name: SmolStr::new_inline("Test"),
            ..Default::default()
        };
        let content = r#"
package ch.emilycares;
public class Test {
    public String hello() {
        String local = "hey ";
        return local.concat("", local. );
    }
}
"#;
        let doc = Document::setup(content, PathBuf::new()).unwrap();

        let out = signature_driver(&doc, &AstPoint::new(5, 39), &class, &class_map).unwrap();
        let expected = expect![[r#"
            SignatureHelp {
                signatures: [
                    SignatureInformation {
                        label: "concat(String)",
                        documentation: Some(
                            String(
                                "String",
                            ),
                        ),
                        parameters: Some(
                            [
                                ParameterInformation {
                                    label: Simple(
                                        "String",
                                    ),
                                    documentation: None,
                                },
                            ],
                        ),
                        active_parameter: None,
                    },
                    SignatureInformation {
                        label: "concat(String, String)",
                        documentation: Some(
                            String(
                                "String",
                            ),
                        ),
                        parameters: Some(
                            [
                                ParameterInformation {
                                    label: Simple(
                                        "String",
                                    ),
                                    documentation: None,
                                },
                                ParameterInformation {
                                    label: Simple(
                                        "String",
                                    ),
                                    documentation: None,
                                },
                            ],
                        ),
                        active_parameter: None,
                    },
                ],
                active_signature: Some(
                    1,
                ),
                active_parameter: Some(
                    1,
                ),
            }
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn signature_field_constructor() {
        let mut class_map: HashMap<MyString, Class> = HashMap::new();
        class_map.insert(
            SmolStr::new_inline("java.util.HashMap"),
            Class {
                access: Access::Public,
                name: SmolStr::new_inline("HashMap"),
                methods: vec![Method {
                    access: Access::Public,
                    name: None,
                    parameters: vec![Parameter {
                        name: None,
                        jtype: JType::Class(SmolStr::new_inline("java.lang.String")),
                    }],
                    ret: JType::Class(SmolStr::new_inline("java.lang.String")),
                    throws: vec![],
                    source: None,
                }],
                ..Default::default()
            },
        );
        let class_map = Arc::new(Mutex::new(class_map));
        let class = Class {
            access: Access::Public,
            name: SmolStr::new_inline("Test"),
            ..Default::default()
        };
        let content = r"
package ch.emilycares;
import java.util.HashMap;
public class Test {
public static Map<Long, String> m = new HashMap<>( );
}
";
        let doc = Document::setup(content, PathBuf::new()).unwrap();

        let out = signature_driver(&doc, &AstPoint::new(4, 51), &class, &class_map).unwrap();
        let expected = expect![[r#"
            SignatureHelp {
                signatures: [
                    SignatureInformation {
                        label: "HashMap(String)",
                        documentation: Some(
                            String(
                                "String",
                            ),
                        ),
                        parameters: Some(
                            [
                                ParameterInformation {
                                    label: Simple(
                                        "String",
                                    ),
                                    documentation: None,
                                },
                            ],
                        ),
                        active_parameter: None,
                    },
                ],
                active_signature: Some(
                    0,
                ),
                active_parameter: Some(
                    0,
                ),
            }
        "#]];
        expected.assert_debug_eq(&out);
    }
}
