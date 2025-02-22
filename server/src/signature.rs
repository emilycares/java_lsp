use dashmap::DashMap;
use lsp_types::{
    Documentation, ParameterInformation, ParameterLabel, SignatureHelp, SignatureInformation,
};
use parser::{
    call_chain::{self, CallItem},
    dto::{self},
};

use crate::{
    document::Document,
    imports::{self},
    tyres, variable,
};

pub fn signature_driver(
    document: &Document,
    point: &tree_sitter::Point,
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Option<SignatureHelp> {
    if let Some(call_chain) =
        call_chain::get_call_chain(&document.tree, document.as_bytes(), &point)
    {
        let imports = imports::imports(document);
        let vars = variable::get_vars(document, &point);
        return get_signature(call_chain, &imports, &vars, class_map);
    }
    None
}
pub fn get_signature(
    call_chain: Vec<CallItem>,
    imports: &Vec<imports::ImportUnit<'_>>,
    vars: &Vec<variable::LocalVariable>,
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Option<SignatureHelp> {
    let args = get_args(&call_chain);
    let Some(CallItem::ArgumentList {
        prev,
        range: _,
        active_param,
        filled_params: _,
    }) = args
    else {
        return None;
    };
    let Some(CallItem::MethodCall {
        name: method_name,
        range: _,
    }) = prev.last()
    else {
        return None;
    };
    let vars: &Vec<variable::LocalVariable> = &vars;
    let imports: &Vec<imports::ImportUnit<'_>> = &imports;
    if let Some(class) = tyres::resolve_call_chain(&prev, &vars, &imports, class_map) {
        let methods: Vec<&dto::Method> = class
            .methods
            .iter()
            .filter(|m| m.name == *method_name)
            .collect();

        let Some(active_signature) = methods
            .iter()
            .enumerate()
            .filter(|(_, m)| m.parameters.len() > *active_param)
            .next()
        else {
            return None;
        };
        let active_signature_id = active_signature.0;
        let signatures = methods
            .iter()
            .map(|m| method_to_signature_information(m))
            .collect();

        dbg!(&active_param, &active_signature_id);
        return Some(SignatureHelp {
            signatures,
            active_signature: TryInto::<u32>::try_into(active_signature_id).ok(),
            active_parameter: TryInto::<u32>::try_into(*active_param).ok(),
        });
    }
    None
}

fn get_args(call_chain: &Vec<CallItem>) -> Option<&CallItem> {
    call_chain.iter().rev().find(|i| match i {
        CallItem::MethodCall { name: _, range: _ } => false,
        CallItem::FieldAccess { name: _, range: _ } => false,
        CallItem::Variable { name: _, range: _ } => false,
        CallItem::Class { name: _, range: _ } => false,
        CallItem::ArgumentList {
            prev: _,
            range: _,
            active_param: _,
            filled_params: _,
        } => true,
    })
}

fn method_to_signature_information(method: &dto::Method) -> SignatureInformation {
    let parameters: Vec<ParameterInformation> = method
        .parameters
        .iter()
        .map(|p| match &p.name {
            Some(name) => ParameterInformation {
                label: ParameterLabel::Simple(p.jtype.to_string()),
                documentation: Some(Documentation::String(name.clone())),
            },
            None => ParameterInformation {
                label: ParameterLabel::Simple(p.jtype.to_string()),
                documentation: None,
            },
        })
        .collect();
    SignatureInformation {
        label: method.name.clone(),
        documentation: Some(Documentation::String(method.ret.to_string())),
        parameters: Some(parameters),
        active_parameter: None,
    }
}

#[cfg(test)]
pub mod tests {
    use dashmap::DashMap;
    use lsp_types::{
        Documentation, ParameterInformation, ParameterLabel, SignatureHelp, SignatureInformation,
    };
    use parser::dto;
    use pretty_assertions::assert_eq;
    use tree_sitter::Point;

    use crate::document::Document;

    use super::signature_driver;

    #[test]
    fn signarure_base() {
        let class_map: DashMap<String, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".to_string(),
            dto::Class {
                class_path: "".to_string(),
                source: "".to_string(),
                access: vec![dto::Access::Public],
                name: "String".to_string(),
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "concat".to_string(),
                    parameters: vec![dto::Parameter {
                        name: None,
                        jtype: dto::JType::Class("java.lang.String".to_string()),
                    }],
                    ret: dto::JType::Class("java.lang.String".to_string()),
                }],
                fields: vec![],
            },
        );
        let content = "
package ch.emilycares;
public class Test {
    public String hello() {
        String local = \"hey \";
        return local.concat( );
    }
}
";
        let doc = Document::setup(content).unwrap();

        let out = signature_driver(&doc, &Point::new(5, 29), &class_map);
        assert_eq!(
            out,
            Some(SignatureHelp {
                signatures: vec![SignatureInformation {
                    label: "concat".to_string(),
                    documentation: Some(Documentation::String("String".to_string())),
                    parameters: Some(vec![ParameterInformation {
                        label: ParameterLabel::Simple("String".to_string()),
                        documentation: None,
                    }]),
                    active_parameter: None,
                }],
                active_signature: Some(0),
                active_parameter: Some(0),
            },)
        );
    }

    #[test]
    fn signature_multi_name() {
        let class_map: DashMap<String, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".to_string(),
            dto::Class {
                class_path: "".to_string(),
                source: "".to_string(),
                access: vec![dto::Access::Public],
                name: "String".to_string(),
                methods: vec![
                    dto::Method {
                        access: vec![dto::Access::Public],
                        name: "concat".to_string(),
                        parameters: vec![dto::Parameter {
                            name: None,
                            jtype: dto::JType::Class("java.lang.String".to_string()),
                        }],
                        ret: dto::JType::Class("java.lang.String".to_string()),
                    },
                    dto::Method {
                        access: vec![dto::Access::Public],
                        name: "concat".to_string(),
                        parameters: vec![
                            dto::Parameter {
                                name: None,
                                jtype: dto::JType::Class("java.lang.String".to_string()),
                            },
                            dto::Parameter {
                                name: None,
                                jtype: dto::JType::Class("java.lang.String".to_string()),
                            },
                        ],
                        ret: dto::JType::Class("java.lang.String".to_string()),
                    },
                ],
                fields: vec![],
            },
        );
        let content = "
package ch.emilycares;
public class Test {
    public String hello() {
        String local = \"hey \";
        return local.concat( );
    }
}
";
        let doc = Document::setup(content).unwrap();

        let out = signature_driver(&doc, &Point::new(5, 29), &class_map);
        assert_eq!(
            out,
            Some(SignatureHelp {
                signatures: vec![
                    SignatureInformation {
                        label: "concat".to_string(),
                        documentation: Some(Documentation::String("String".to_string())),
                        parameters: Some(vec![ParameterInformation {
                            label: ParameterLabel::Simple("String".to_string()),
                            documentation: None,
                        }]),
                        active_parameter: None,
                    },
                    SignatureInformation {
                        label: "concat".to_string(),
                        documentation: Some(Documentation::String("String".to_string())),
                        parameters: Some(vec![
                            ParameterInformation {
                                label: ParameterLabel::Simple("String".to_string()),
                                documentation: None,
                            },
                            ParameterInformation {
                                label: ParameterLabel::Simple("String".to_string()),
                                documentation: None,
                            },
                        ],),
                        active_parameter: None,
                    }
                ],
                active_signature: Some(0),
                active_parameter: Some(0),
            },)
        );
    }

    #[test]
    fn signature_multi_name_second() {
        let class_map: DashMap<String, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".to_string(),
            dto::Class {
                class_path: "".to_string(),
                source: "".to_string(),
                access: vec![dto::Access::Public],
                name: "String".to_string(),
                methods: vec![
                    dto::Method {
                        access: vec![dto::Access::Public],
                        name: "concat".to_string(),
                        parameters: vec![dto::Parameter {
                            name: None,
                            jtype: dto::JType::Class("java.lang.String".to_string()),
                        }],
                        ret: dto::JType::Class("java.lang.String".to_string()),
                    },
                    dto::Method {
                        access: vec![dto::Access::Public],
                        name: "concat".to_string(),
                        parameters: vec![
                            dto::Parameter {
                                name: None,
                                jtype: dto::JType::Class("java.lang.String".to_string()),
                            },
                            dto::Parameter {
                                name: None,
                                jtype: dto::JType::Class("java.lang.String".to_string()),
                            },
                        ],
                        ret: dto::JType::Class("java.lang.String".to_string()),
                    },
                ],
                fields: vec![],
            },
        );
        let content = r#"
package ch.emilycares;
public class Test {
    public String hello() {
        String local = "hey ";
        return local.concat("", local. );
    }
}
"#;
        let doc = Document::setup(content).unwrap();

        let out = signature_driver(&doc, &Point::new(5, 39), &class_map);
        assert_eq!(
            out,
            Some(SignatureHelp {
                signatures: vec![
                    SignatureInformation {
                        label: "concat".to_string(),
                        documentation: Some(Documentation::String("String".to_string())),
                        parameters: Some(vec![ParameterInformation {
                            label: ParameterLabel::Simple("String".to_string()),
                            documentation: None,
                        }]),
                        active_parameter: None,
                    },
                    SignatureInformation {
                        label: "concat".to_string(),
                        documentation: Some(Documentation::String("String".to_string())),
                        parameters: Some(vec![
                            ParameterInformation {
                                label: ParameterLabel::Simple("String".to_string()),
                                documentation: None,
                            },
                            ParameterInformation {
                                label: ParameterLabel::Simple("String".to_string()),
                                documentation: None,
                            },
                        ],),
                        active_parameter: None,
                    }
                ],
                active_signature: Some(1),
                active_parameter: Some(1),
            },)
        );
    }
}
