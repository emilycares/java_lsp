use dashmap::DashMap;
use lsp_types::{
    Documentation, ParameterInformation, ParameterLabel, SignatureHelp, SignatureInformation,
};
use parser::{
    call_chain::{self, CallItem},
    dto::{self, Class, ImportUnit},
};

use crate::{
    document::Document,
    imports::{self},
    tyres, variable,
};

#[derive(Debug, PartialEq)]
pub enum SignatureError {
    Tyres(tyres::TyresError),
    NotAnArgumentList,
    CouldNotGetMethod,
    CouldNoteGetActiveSignature,
    NoCallChain,
}

pub fn signature_driver(
    document: &Document,
    point: &tree_sitter::Point,
    class: &Class,
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Result<SignatureHelp, SignatureError> {
    if let Some(call_chain) =
        call_chain::get_call_chain(&document.tree, document.as_bytes(), &point)
    {
        let imports = imports::imports(document);
        let vars = variable::get_vars(document, &point);
        return get_signature(call_chain, &imports, &vars, class, class_map);
    }
    Err(SignatureError::NoCallChain)
}
pub fn get_signature(
    call_chain: Vec<CallItem>,
    imports: &Vec<ImportUnit>,
    vars: &Vec<variable::LocalVariable>,
    class: &Class,
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Result<SignatureHelp, SignatureError> {
    let args = get_args(&call_chain);
    let Some(CallItem::ArgumentList {
        prev,
        range: _,
        active_param,
        filled_params: _,
    }) = args
    else {
        return Err(SignatureError::NotAnArgumentList);
    };
    let Some(CallItem::MethodCall {
        name: method_name,
        range: _,
    }) = prev.last()
    else {
        return Err(SignatureError::CouldNotGetMethod);
    };
    let class = match tyres::resolve_call_chain(prev, vars, imports, class, class_map) {
        Ok(c) => Ok(c),
        Err(e) => Err(SignatureError::Tyres(e)),
    }?;
    let methods: Vec<&dto::Method> = class
        .methods
        .iter()
        .filter(|m| m.name == *method_name)
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

    return Ok(SignatureHelp {
        signatures,
        active_signature: TryInto::<u32>::try_into(active_signature_id).ok(),
        active_parameter: TryInto::<u32>::try_into(*active_param).ok(),
    });
}

fn get_args(call_chain: &Vec<CallItem>) -> Option<&CallItem> {
    call_chain.iter().rev().find(|i| match i {
        CallItem::MethodCall { name: _, range: _ } => false,
        CallItem::FieldAccess { name: _, range: _ } => false,
        CallItem::Variable { name: _, range: _ } => false,
        CallItem::Class { name: _, range: _ } => false,
        CallItem::ClassOrVariable { name: _, range: _ } => false,
        CallItem::This { range: _ } => false,
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
    use std::path::PathBuf;

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
                imports: vec![],
                name: "String".to_string(),
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "concat".to_string(),
                    parameters: vec![dto::Parameter {
                        name: None,
                        jtype: dto::JType::Class("java.lang.String".to_string()),
                    }],
                    ret: dto::JType::Class("java.lang.String".to_string()),
                    throws: vec![],
                }],
                fields: vec![],
            },
        );
        let class = dto::Class {
            class_path: "".to_string(),
            source: "".to_string(),
            access: vec![dto::Access::Public],
            imports: vec![],
            name: "Test".to_string(),
            methods: vec![],
            fields: vec![],
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
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();

        let out = signature_driver(&doc, &Point::new(5, 29), &class, &class_map);
        assert_eq!(
            out,
            Ok(SignatureHelp {
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
                imports: vec![],
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
                        throws: vec![],
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
                        throws: vec![],
                    },
                ],
                fields: vec![],
            },
        );
        let class = dto::Class {
            class_path: "".to_string(),
            source: "".to_string(),
            access: vec![dto::Access::Public],
            imports: vec![],
            name: "Test".to_string(),
            methods: vec![],
            fields: vec![],
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
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();

        let out = signature_driver(&doc, &Point::new(5, 29), &class, &class_map);
        assert_eq!(
            out,
            Ok(SignatureHelp {
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
                imports: vec![],
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
                        throws: vec![],
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
                        throws: vec![],
                    },
                ],
                fields: vec![],
            },
        );
        let class = dto::Class {
            class_path: "".to_string(),
            source: "".to_string(),
            access: vec![dto::Access::Public],
            imports: vec![],
            name: "Test".to_string(),
            methods: vec![],
            fields: vec![],
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
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();

        let out = signature_driver(&doc, &Point::new(5, 39), &class, &class_map);
        assert_eq!(
            out,
            Ok(SignatureHelp {
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
