mod parent;

use std::ops::Deref;

use call_chain::CallItem;
use dashmap::DashMap;
use parser::dto::{self, Class, ImportUnit, JType};
use tree_sitter::Point;
use tree_sitter_util::is_point_in_range;
use variables::LocalVariable;

#[derive(Debug, PartialEq, Clone)]
pub enum TyresError {
    ClassNotFound { class_path: String },
    NoClassInOps,
    MethodNotFound(String),
    FieldNotFound(String),
    VariableNotFound(String),
    NotImported(String),
    CallChainInvalid(Vec<CallItem>),
    CallChainEmtpy,
}

#[derive(Debug, Clone)]
pub struct ResolveState {
    pub class: Class,
    pub jtype: JType,
}

pub fn is_imported_class_name(
    jtype: &str,
    imports: &[ImportUnit],
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> bool {
    is_imported(jtype, imports, class_map).is_some()
}

#[derive(Debug)]
pub enum ImportResult {
    Class(String),
    StaticClass(String),
}

pub fn is_imported<'a>(
    jtype: &'a str,
    imports: &'a [ImportUnit],
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Option<ImportResult> {
    if jtype.starts_with("java.lang") {
        return Some(ImportResult::Class(jtype.to_string()));
    }
    imports.iter().find_map(|i| match i {
        ImportUnit::Class(c) => {
            if ImportUnit::class_path_match_class_name(c, jtype) {
                return Some(ImportResult::Class(c.to_string()));
            }
            None
        }
        ImportUnit::StaticClass(c) => {
            if ImportUnit::class_path_match_class_name(c, jtype) {
                return Some(ImportResult::StaticClass(c.to_string()));
            }
            None
        }
        ImportUnit::Package(p) | ImportUnit::Prefix(p) => {
            let possible_class_path = format!("{}.{}", p, jtype);
            if class_map.contains_key(&possible_class_path) {
                return Some(ImportResult::Class(possible_class_path));
            }
            None
        }
        ImportUnit::StaticPrefix(p) => {
            let possible_class_path = format!("{}.{}", p, jtype);
            if class_map.contains_key(&possible_class_path) {
                return Some(ImportResult::StaticClass(possible_class_path));
            }
            None
        }
        ImportUnit::StaticClassMethod(_, _) => None,
    })
}

pub fn resolve(
    jtype: &str,
    imports: &[ImportUnit],
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Result<ResolveState, TyresError> {
    eprintln!("resolve: {}", jtype);
    let lang_class_key = format!("java.lang.{}", jtype);
    if let Some(lang_class) = class_map.get(lang_class_key.as_str()) {
        return Ok(ResolveState {
            jtype: JType::Class(lang_class_key),
            class: parent::inclued_parent(lang_class.deref().to_owned(), class_map),
        });
    }

    if jtype.contains('.') {
        let Some(imported_class) = class_map.get(jtype) else {
            return Err(TyresError::ClassNotFound {
                class_path: jtype.to_string(),
            });
        };
        return Ok(ResolveState {
            jtype: JType::Class(jtype.to_string()),
            class: parent::inclued_parent(imported_class.deref().to_owned(), class_map),
        });
    }

    let import_result = is_imported(jtype, imports, class_map);
    match import_result {
        Some(ImportResult::Class(c)) | Some(ImportResult::StaticClass(c)) => {
            let Some(imported_class) = class_map.get(&c) else {
                return Err(TyresError::ClassNotFound { class_path: c });
            };
            Ok(ResolveState {
                jtype: JType::Class(c),
                class: parent::inclued_parent(imported_class.deref().to_owned(), class_map),
            })
        }
        None => Err(TyresError::NotImported(jtype.to_string())),
    }
}

pub fn resolve_import(
    jtype: &str,
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Vec<String> {
    resolve_class_key(class_map, |p| p.starts_with(jtype))
}

pub fn resolve_class_key(
    class_map: &DashMap<std::string::String, parser::dto::Class>,
    infl: impl Fn(&&std::string::String) -> bool,
) -> Vec<String> {
    class_map
        .clone()
        .into_read_only()
        .keys()
        .filter(infl)
        .map(|a| a.to_string())
        .collect::<Vec<String>>()
}

pub fn resolve_var(
    extend: &LocalVariable,
    imports: &[ImportUnit],
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Result<ResolveState, TyresError> {
    resolve_jtype(&extend.jtype, imports, class_map)
}

#[allow(dead_code)]
pub fn resolve_params(
    params: Vec<Vec<CallItem>>,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &DashMap<String, Class>,
) -> Vec<Result<ResolveState, TyresError>> {
    params
        .iter()
        .map(|c| resolve_call_chain(c, lo_va, imports, class, class_map))
        .collect()
}

pub fn resolve_call_chain(
    call_chain: &[CallItem],
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &DashMap<String, Class>,
) -> Result<ResolveState, TyresError> {
    if call_chain.is_empty() {
        return Err(TyresError::CallChainEmtpy);
    }
    let mut ops: Vec<ResolveState> = vec![];
    for item in call_chain {
        let op = call_chain_op(item, &ops, lo_va, imports, class, class_map);
        if let Ok(op) = op {
            ops.push(op);
        }
    }
    match ops.last() {
        Some(last) => Ok(last.clone()),
        None => Err(TyresError::CallChainInvalid(
            call_chain.iter().map(Clone::clone).collect(),
        )),
    }
}
pub fn resolve_call_chain_to_point(
    call_chain: &[CallItem],
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &DashMap<String, Class>,
    point: &Point,
) -> Result<ResolveState, TyresError> {
    if call_chain.is_empty() {
        return Err(TyresError::CallChainEmtpy);
    }
    let mut ops: Vec<ResolveState> = vec![];
    for item in call_chain {
        if is_point_in_range(point, item.get_range()) {
            break;
        }
        let op = call_chain_op(item, &ops, lo_va, imports, class, class_map);
        if let Ok(op) = op {
            ops.push(op);
        }
    }
    match ops.last() {
        Some(last) => Ok(last.clone()),
        None => Err(TyresError::CallChainInvalid(
            call_chain.iter().map(Clone::clone).collect(),
        )),
    }
}

fn call_chain_op(
    item: &CallItem,
    ops: &[ResolveState],
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &DashMap<String, Class>,
) -> Result<ResolveState, TyresError> {
    match item {
        CallItem::MethodCall { name, range: _ } => {
            let Some(ResolveState { class, jtype: _ }) = ops.last() else {
                return Err(TyresError::NoClassInOps);
            };
            if let Some(method) = class.methods.iter().find(|m| m.name == *name) {
                return resolve_jtype(&method.ret, imports, class_map);
            }
            Err(TyresError::MethodNotFound(name.to_string()))
        }
        CallItem::FieldAccess { name, range: _ } => {
            let Some(ResolveState { class, jtype: _ }) = ops.last() else {
                return Err(TyresError::NoClassInOps);
            };
            if let Some(method) = class.fields.iter().find(|m| m.name == *name) {
                return resolve_jtype(&method.jtype, imports, class_map);
            }
            Err(TyresError::FieldNotFound(name.to_string()))
        }
        CallItem::Variable { name, range: _ } => {
            if let Some(lo) = lo_va.iter().find(|va| va.name == *name) {
                return resolve_var(lo, imports, class_map);
            }
            Err(TyresError::VariableNotFound(name.to_string()))
        }
        CallItem::This { range: _ } => Ok(ResolveState {
            class: class.clone(),
            jtype: JType::Class(class.class_path.clone()),
        }),
        CallItem::Class { name, range: _ } => resolve(name, imports, class_map),
        CallItem::ClassOrVariable { name, range: _ } => {
            if let Some(lo) = lo_va.iter().find(|va| va.name == *name) {
                return resolve_var(lo, imports, class_map);
            }
            resolve(name, imports, class_map)
        }
        CallItem::ArgumentList {
            prev,
            range: _,
            active_param,
            filled_params,
        } => {
            if let Some(active_param) = active_param {
                if let Some(current_param) = filled_params.get(*active_param) {
                    if !current_param.is_empty() {
                        return resolve_call_chain(current_param, lo_va, imports, class, class_map);
                    }
                }
            }
            resolve_call_chain(prev, lo_va, imports, class, class_map)
        }
    }
}

pub fn resolve_jtype(
    jtype: &JType,
    imports: &[ImportUnit],
    class_map: &DashMap<String, Class>,
) -> Result<ResolveState, TyresError> {
    match jtype {
        JType::Void => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "void".to_string(),
                ..Default::default()
            },
        }),
        JType::Byte => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "byte".to_string(),
                ..Default::default()
            },
        }),
        JType::Char => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "char".to_string(),
                ..Default::default()
            },
        }),
        JType::Double => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "double".to_string(),
                ..Default::default()
            },
        }),
        JType::Float => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "float".to_string(),
                ..Default::default()
            },
        }),
        JType::Int => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "int".to_string(),
                ..Default::default()
            },
        }),
        JType::Long => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "long".to_string(),
                ..Default::default()
            },
        }),
        JType::Short => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "short".to_string(),
                ..Default::default()
            },
        }),
        JType::Boolean => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "boolean".to_string(),
                ..Default::default()
            },
        }),
        JType::Wildcard => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "Wildcard".to_string(),
                ..Default::default()
            },
        }),
        JType::Array(i) => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "array".to_string(),
                methods: vec![dto::Method {
                    name: "clone".to_string(),
                    ret: JType::Array(i.clone()),
                    ..Default::default()
                }],
                fields: vec![dto::Field {
                    access: vec![],
                    name: "length".to_string(),
                    jtype: JType::Int,
                    source: None,
                }],
                ..Default::default()
            },
        }),
        JType::Class(c) => resolve(c, imports, class_map),
        JType::Generic(c, _vec) => resolve(c, imports, class_map),
        JType::Parameter(p) => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: format!("<{}>", p),
                ..Default::default()
            },
        }),
    }
}
