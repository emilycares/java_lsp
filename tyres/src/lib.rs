mod parent;

use std::ops::Deref;

use ast::types::AstPoint;
use call_chain::CallItem;
use dashmap::DashMap;
use parser::dto::{self, Class, ImportUnit, JType};
use smol_str::{SmolStr, SmolStrBuilder};
use variables::LocalVariable;

#[derive(Debug, PartialEq, Clone)]
pub enum TyresError {
    ClassNotFound {
        class_path: SmolStr,
    },
    NoClassInOps,
    MethodNotFound(SmolStr),
    FieldNotFound(SmolStr),
    VariableNotFound(SmolStr),
    NotImported(SmolStr),
    CallChainInvalid(Vec<CallItem>),
    CallChainEmtpy,
    /// Value needs to be checked, type is var
    CheckValue,
}

#[derive(Debug, Clone)]
pub struct ResolveState {
    pub class: Class,
    pub jtype: JType,
}

pub fn is_imported_class_name(
    jtype: &str,
    imports: &[ImportUnit],
    class_map: &DashMap<SmolStr, parser::dto::Class>,
) -> bool {
    is_imported(jtype, imports, class_map).is_some()
}

#[derive(Debug)]
pub enum ImportResult {
    Class(SmolStr),
    StaticClass(SmolStr),
}

pub fn is_imported<'a>(
    jtype: &'a str,
    imports: &'a [ImportUnit],
    class_map: &DashMap<SmolStr, parser::dto::Class>,
) -> Option<ImportResult> {
    if jtype.starts_with("java.lang") {
        return Some(ImportResult::Class(jtype.into()));
    }
    imports.iter().find_map(|i| match i {
        ImportUnit::Class(c) => {
            if ImportUnit::class_path_match_class_name(c, jtype) {
                return Some(ImportResult::Class(c.clone()));
            }
            None
        }
        ImportUnit::StaticClass(c) => {
            if ImportUnit::class_path_match_class_name(c, jtype) {
                return Some(ImportResult::StaticClass(c.clone()));
            }
            None
        }
        ImportUnit::Package(p) | ImportUnit::Prefix(p) => {
            let mut possible_class_path = SmolStrBuilder::new();
            possible_class_path.push_str(p);
            possible_class_path.push('.');
            possible_class_path.push_str(jtype);
            let possible_class_path = possible_class_path.finish();

            if class_map.contains_key(&possible_class_path) {
                return Some(ImportResult::Class(possible_class_path));
            }
            None
        }
        ImportUnit::StaticPrefix(p) => {
            let mut possible_class_path = SmolStrBuilder::new();
            possible_class_path.push_str(p);
            possible_class_path.push('.');
            possible_class_path.push_str(jtype);
            let possible_class_path = possible_class_path.finish();
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
    class_map: &DashMap<SmolStr, parser::dto::Class>,
) -> Result<ResolveState, TyresError> {
    eprintln!("resolve: {jtype}");

    if jtype.contains('.') {
        let Some(imported_class) = class_map.get(jtype) else {
            return Err(TyresError::ClassNotFound {
                class_path: jtype.into(),
            });
        };
        return Ok(ResolveState {
            jtype: JType::Class(jtype.into()),
            class: parent::inclued_parent(imported_class.deref().to_owned(), class_map),
        });
    }

    let mut lang_class_key = SmolStrBuilder::new();
    lang_class_key.push_str("java.lang.");
    lang_class_key.push_str(jtype);
    let lang_class_key = lang_class_key.finish();
    if let Some(lang_class) = class_map.get(&lang_class_key) {
        return Ok(ResolveState {
            jtype: JType::Class(lang_class_key),
            class: parent::inclued_parent(lang_class.deref().to_owned(), class_map),
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
        None => Err(TyresError::NotImported(jtype.into())),
    }
}

pub fn resolve_import(
    jtype: &str,
    class_map: &DashMap<SmolStr, parser::dto::Class>,
) -> Vec<String> {
    resolve_class_key(class_map, |p| p.starts_with(jtype))
}

pub fn resolve_class_key(
    class_map: &DashMap<SmolStr, parser::dto::Class>,
    infl: impl Fn(&&SmolStr) -> bool,
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
    class_map: &DashMap<SmolStr, parser::dto::Class>,
) -> Result<ResolveState, TyresError> {
    resolve_jtype(&extend.jtype, imports, class_map)
}

#[allow(dead_code)]
pub fn resolve_params(
    params: Vec<Vec<CallItem>>,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &DashMap<SmolStr, Class>,
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
    class_map: &DashMap<SmolStr, Class>,
) -> Result<ResolveState, TyresError> {
    if call_chain.is_empty() {
        return Err(TyresError::CallChainEmtpy);
    }
    let mut ops: Vec<ResolveState> = vec![];
    for item in call_chain {
        let op = call_chain_op(item, &ops, lo_va, imports, class, class_map, true);
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
pub fn resolve_call_chain_value(
    call_chain: &[CallItem],
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &DashMap<SmolStr, Class>,
) -> Result<ResolveState, TyresError> {
    if call_chain.is_empty() {
        return Err(TyresError::CallChainEmtpy);
    }
    let mut ops: Vec<ResolveState> = vec![];
    for item in call_chain {
        let op = call_chain_op(item, &ops, lo_va, imports, class, class_map, false);
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
    class_map: &DashMap<SmolStr, Class>,
    point: &AstPoint,
) -> Result<ResolveState, TyresError> {
    if call_chain.is_empty() {
        return Err(TyresError::CallChainEmtpy);
    }
    let mut ops: Vec<ResolveState> = vec![];
    for item in call_chain {
        if item.get_range().is_after_range(point) {
            break;
        }
        let op = call_chain_op(item, &ops, lo_va, imports, class, class_map, true);
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
    class_map: &DashMap<SmolStr, Class>,
    resolve_argument: bool,
) -> Result<ResolveState, TyresError> {
    match item {
        CallItem::MethodCall { name, range: _ } => {
            let Some(ResolveState { class, jtype: _ }) = ops.last() else {
                return Err(TyresError::NoClassInOps);
            };
            if let Some(method) = class.methods.iter().find(|m| m.name == *name) {
                return resolve_jtype(&method.ret, imports, class_map);
            }
            Err(TyresError::MethodNotFound(name.clone()))
        }
        CallItem::FieldAccess { name, range: _ } => {
            let Some(ResolveState { class, jtype: _ }) = ops.last() else {
                return Err(TyresError::NoClassInOps);
            };
            if let Some(method) = class.fields.iter().find(|m| m.name == *name) {
                return resolve_jtype(&method.jtype, imports, class_map);
            }
            Err(TyresError::FieldNotFound(name.clone()))
        }
        CallItem::Variable { name, range: _ } => {
            if let Some(lo) = lo_va.iter().find(|va| va.name == *name) {
                return resolve_var(lo, imports, class_map);
            }
            Err(TyresError::VariableNotFound(name.clone()))
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
            if resolve_argument {
                if let Some(active_param) = active_param
                    && let Some(current_param) = filled_params.get(*active_param)
                    && !current_param.is_empty()
                {
                    return resolve_call_chain(current_param, lo_va, imports, class, class_map);
                }
                return resolve_call_chain(prev, lo_va, imports, class, class_map);
            }
            resolve_call_chain(prev, lo_va, imports, class, class_map)
        }
    }
}

pub fn resolve_jtype(
    jtype: &JType,
    imports: &[ImportUnit],
    class_map: &DashMap<SmolStr, Class>,
) -> Result<ResolveState, TyresError> {
    match jtype {
        JType::Void => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "void".into(),
                ..Default::default()
            },
        }),
        JType::Byte => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "byte".into(),
                ..Default::default()
            },
        }),
        JType::Char => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "char".into(),
                ..Default::default()
            },
        }),
        JType::Double => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "double".into(),
                ..Default::default()
            },
        }),
        JType::Float => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "float".into(),
                ..Default::default()
            },
        }),
        JType::Int => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "int".into(),
                ..Default::default()
            },
        }),
        JType::Long => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "long".into(),
                ..Default::default()
            },
        }),
        JType::Short => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "short".into(),
                ..Default::default()
            },
        }),
        JType::Boolean => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "boolean".into(),
                ..Default::default()
            },
        }),
        JType::Wildcard => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "Wildcard".into(),
                ..Default::default()
            },
        }),
        JType::Array(i) => Ok(ResolveState {
            jtype: jtype.clone(),
            class: Class {
                name: "array".into(),
                methods: vec![dto::Method {
                    name: "clone".into(),
                    ret: JType::Array(i.clone()),
                    ..Default::default()
                }],
                fields: vec![dto::Field {
                    access: vec![],
                    name: "length".into(),
                    jtype: JType::Int,
                    source: None,
                }],
                ..Default::default()
            },
        }),
        JType::Class(c) => resolve(c, imports, class_map),
        JType::Generic(c, _vec) => resolve(c, imports, class_map),
        JType::Parameter(p) => {
            let mut name = SmolStrBuilder::new();
            name.push('<');
            name.push_str(p);
            name.push('>');
            let name = name.finish();
            Ok(ResolveState {
                jtype: jtype.clone(),
                class: Class {
                    name,
                    ..Default::default()
                },
            })
        }
        JType::Var => Err(TyresError::CheckValue),
    }
}
