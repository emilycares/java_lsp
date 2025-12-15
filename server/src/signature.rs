use ast::types::AstPoint;
use call_chain::CallItem;
use dashmap::DashMap;
use document::Document;
use lsp_types::{
    Documentation, ParameterInformation, ParameterLabel, SignatureHelp, SignatureInformation,
};
use my_string::MyString;
use parser::dto::{self, Class, ImportUnit};
use variables::{LocalVariable, VariablesError};

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
    class_map: &DashMap<MyString, parser::dto::Class>,
) -> Result<SignatureHelp, SignatureError> {
    let call_chain = call_chain::get_call_chain(&document.ast, point);
    let imports = imports::imports(&document.ast);
    let vars = variables::get_vars(&document.ast, point).map_err(SignatureError::Variables)?;
    get_signature(&call_chain, &imports, &vars, class, class_map)
}
pub fn get_signature(
    call_chain: &[CallItem],
    imports: &[ImportUnit],
    vars: &[LocalVariable],
    class: &Class,
    class_map: &DashMap<MyString, parser::dto::Class>,
) -> Result<SignatureHelp, SignatureError> {
    let args = get_args(call_chain);
    let Some(CallItem::ArgumentList {
        prev,
        range: _,
        active_param,
        filled_params: _,
    }) = args
    else {
        return Err(SignatureError::NotAnArgumentList);
    };
    let Some(active_param) = active_param else {
        return Err(SignatureError::NotAnArgumentList);
    };
    let Some(CallItem::MethodCall {
        name: method_name,
        range: _,
    }) = prev.last()
    else {
        return Err(SignatureError::CouldNotGetMethod);
    };
    let method_name = method_name.as_str();
    // trim last method call
    let prev = &prev[..1];
    let resolve_state = match tyres::resolve_call_chain(prev, vars, imports, class, class_map) {
        Ok(c) => Ok(c),
        Err(e) => Err(SignatureError::Tyres(e)),
    }?;
    let methods: Vec<&dto::Method> = resolve_state
        .class
        .methods
        .iter()
        .filter(|m| m.name == method_name)
        .collect();

    let Some(active_signature) = methods
        .iter()
        .enumerate()
        .find(|(_, m)| m.parameters.len() > *active_param)
    else {
        return Err(SignatureError::CouldNoteGetActiveSignature);
    };
    let active_signature_id = active_signature.0;
    let signatures = methods
        .iter()
        .map(|m| method_to_signature_information(m))
        .collect();

    Ok(SignatureHelp {
        signatures,
        active_signature: TryInto::<u32>::try_into(active_signature_id).ok(),
        active_parameter: TryInto::<u32>::try_into(*active_param).ok(),
    })
}

fn get_args(call_chain: &[CallItem]) -> Option<&CallItem> {
    call_chain.iter().rev().find(|i| match i {
        CallItem::MethodCall { name: _, range: _ }
        | CallItem::FieldAccess { name: _, range: _ }
        | CallItem::Variable { name: _, range: _ }
        | CallItem::Class { name: _, range: _ }
        | CallItem::ClassOrVariable { name: _, range: _ }
        | CallItem::This { range: _ } => false,
        CallItem::ArgumentList {
            prev: _,
            range: _,
            active_param: _,
            filled_params: _,
        } => true,
    })
}

fn method_to_signature_information(method: &dto::Method) -> SignatureInformation {
    let mut label = format!("{}(", method.name);
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
    use std::path::PathBuf;

    use super::signature_driver;
    use ast::types::AstPoint;
    use dashmap::DashMap;
    use document::Document;
    use lsp_types::{
        Documentation, ParameterInformation, ParameterLabel, SignatureHelp, SignatureInformation,
    };
    use my_string::MyString;
    use parser::dto;
    use pretty_assertions::assert_eq;

    #[test]
    fn signarure_base() {
        let class_map: DashMap<MyString, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".into(),
            dto::Class {
                access: dto::Access::Public,
                name: "String".into(),
                methods: vec![dto::Method {
                    access: dto::Access::Public,
                    name: "concat".into(),
                    parameters: vec![dto::Parameter {
                        name: None,
                        jtype: dto::JType::Class("java.lang.String".into()),
                    }],
                    ret: dto::JType::Class("java.lang.String".into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let class = dto::Class {
            access: dto::Access::Public,
            name: "Test".into(),
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
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();

        let out = signature_driver(&doc, &AstPoint::new(5, 29), &class, &class_map).unwrap();
        assert_eq!(
            out,
            SignatureHelp {
                signatures: vec![SignatureInformation {
                    label: "concat(String)".into(),
                    documentation: Some(Documentation::String("String".into())),
                    parameters: Some(vec![ParameterInformation {
                        label: ParameterLabel::Simple("String".into()),
                        documentation: None,
                    }]),
                    active_parameter: None,
                }],
                active_signature: Some(0),
                active_parameter: Some(0),
            },
        );
    }

    #[test]
    fn signature_multi_name() {
        let class_map: DashMap<MyString, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".into(),
            dto::Class {
                access: dto::Access::Public,
                name: "String".into(),
                methods: vec![
                    dto::Method {
                        access: dto::Access::Public,
                        name: "concat".into(),
                        parameters: vec![dto::Parameter {
                            name: None,
                            jtype: dto::JType::Class("java.lang.String".into()),
                        }],
                        ret: dto::JType::Class("java.lang.String".into()),
                        throws: vec![],
                        source: None,
                    },
                    dto::Method {
                        access: dto::Access::Public,
                        name: "concat".into(),
                        parameters: vec![
                            dto::Parameter {
                                name: None,
                                jtype: dto::JType::Class("java.lang.String".into()),
                            },
                            dto::Parameter {
                                name: None,
                                jtype: dto::JType::Class("java.lang.String".into()),
                            },
                        ],
                        ret: dto::JType::Class("java.lang.String".into()),
                        throws: vec![],
                        source: None,
                    },
                ],
                ..Default::default()
            },
        );
        let class = dto::Class {
            access: dto::Access::Public,
            name: "Test".into(),
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
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();

        let out = signature_driver(&doc, &AstPoint::new(5, 29), &class, &class_map).unwrap();
        assert_eq!(
            out,
            SignatureHelp {
                signatures: vec![
                    SignatureInformation {
                        label: "concat(String)".into(),
                        documentation: Some(Documentation::String("String".into())),
                        parameters: Some(vec![ParameterInformation {
                            label: ParameterLabel::Simple("String".into()),
                            documentation: None,
                        }]),
                        active_parameter: None,
                    },
                    SignatureInformation {
                        label: "concat(String, String)".into(),
                        documentation: Some(Documentation::String("String".into())),
                        parameters: Some(vec![
                            ParameterInformation {
                                label: ParameterLabel::Simple("String".into()),
                                documentation: None,
                            },
                            ParameterInformation {
                                label: ParameterLabel::Simple("String".into()),
                                documentation: None,
                            },
                        ],),
                        active_parameter: None,
                    }
                ],
                active_signature: Some(0),
                active_parameter: Some(0),
            },
        );
    }

    #[test]
    fn signature_multi_name_second() {
        let class_map: DashMap<MyString, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".into(),
            dto::Class {
                access: dto::Access::Public,
                name: "String".into(),
                methods: vec![
                    dto::Method {
                        access: dto::Access::Public,
                        name: "concat".into(),
                        parameters: vec![dto::Parameter {
                            name: None,
                            jtype: dto::JType::Class("java.lang.String".into()),
                        }],
                        ret: dto::JType::Class("java.lang.String".into()),
                        throws: vec![],
                        source: None,
                    },
                    dto::Method {
                        access: dto::Access::Public,
                        name: "concat".into(),
                        parameters: vec![
                            dto::Parameter {
                                name: None,
                                jtype: dto::JType::Class("java.lang.String".into()),
                            },
                            dto::Parameter {
                                name: None,
                                jtype: dto::JType::Class("java.lang.String".into()),
                            },
                        ],
                        ret: dto::JType::Class("java.lang.String".into()),
                        throws: vec![],
                        source: None,
                    },
                ],
                ..Default::default()
            },
        );
        let class = dto::Class {
            access: dto::Access::Public,
            name: "Test".into(),
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
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();

        let out = signature_driver(&doc, &AstPoint::new(5, 39), &class, &class_map).unwrap();
        assert_eq!(
            out,
            SignatureHelp {
                signatures: vec![
                    SignatureInformation {
                        label: "concat(String)".into(),
                        documentation: Some(Documentation::String("String".into())),
                        parameters: Some(vec![ParameterInformation {
                            label: ParameterLabel::Simple("String".into()),
                            documentation: None,
                        }]),
                        active_parameter: None,
                    },
                    SignatureInformation {
                        label: "concat(String, String)".into(),
                        documentation: Some(Documentation::String("String".into())),
                        parameters: Some(vec![
                            ParameterInformation {
                                label: ParameterLabel::Simple("String".into()),
                                documentation: None,
                            },
                            ParameterInformation {
                                label: ParameterLabel::Simple("String".into()),
                                documentation: None,
                            },
                        ],),
                        active_parameter: None,
                    }
                ],
                active_signature: Some(1),
                active_parameter: Some(1),
            },
        );
    }
}
