#![deny(warnings)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::implicit_hasher)]
mod parent;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ast::types::AstPoint;
use call_chain::CallItem;
use my_string::{
    MyString,
    smol_str::{SmolStrBuilder, format_smolstr},
};
use parser::dto::{Access, Class, Field, ImportUnit, JType, Method};
use variables::LocalVariable;

#[derive(Debug, PartialEq, Clone)]
pub enum TyresError {
    ClassNotFound {
        class_path: MyString,
    },
    NoClassInOps,
    MethodNotFound(MyString),
    FieldNotFound(MyString),
    VariableNotFound(MyString),
    NotImported(MyString),
    CallChainInvalid(Vec<CallItem>),
    CallChainEmpty,
    /// Value needs to be checked, type is var
    CheckValue,
}

#[derive(Debug, Clone)]
pub struct ResolveState {
    pub class: Class,
    pub jtype: JType,
}

#[must_use]
pub fn is_imported_class_name(
    jtype: &str,
    imports: &[ImportUnit],
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
) -> bool {
    is_imported(jtype, imports, class_map).is_some()
}

#[derive(Debug)]
pub enum ImportResult {
    Class(MyString),
    StaticClass(MyString),
}

#[must_use]
pub fn is_imported<'a>(
    jtype: &'a str,
    imports: &'a [ImportUnit],
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
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

            if let Ok(class_map) = class_map.lock()
                && class_map.contains_key(&possible_class_path)
            {
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
            if let Ok(class_map) = class_map.lock()
                && class_map.contains_key(&possible_class_path)
            {
                return Some(ImportResult::StaticClass(possible_class_path));
            }
            None
        }
        ImportUnit::StaticClassMethod(_, _) => None,
    })
}

pub fn resolve(
    class_name: &str,
    imports: &[ImportUnit],
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
) -> Result<ResolveState, TyresError> {
    eprintln!("resolve: {class_name}");

    if class_name.contains('.') {
        let imported_class;
        if let Ok(cm) = class_map.lock()
            && let Some(ic) = cm.get(class_name)
        {
            imported_class = ic.to_owned();
        } else {
            return Err(TyresError::ClassNotFound {
                class_path: class_name.into(),
            });
        }
        return Ok(ResolveState {
            jtype: JType::Class(class_name.into()),
            class: parent::include_parent(imported_class, class_map),
        });
    }

    let mut lang_class_key = SmolStrBuilder::new();
    lang_class_key.push_str("java.lang.");
    lang_class_key.push_str(class_name);
    let lang_class_key = lang_class_key.finish();
    if let Ok(cm) = class_map.lock()
        && let Some(ic) = cm.get(&lang_class_key)
    {
        let lang_class = ic.to_owned();
        drop(cm);
        return Ok(ResolveState {
            jtype: JType::Class(lang_class_key),
            class: parent::include_parent(lang_class, class_map),
        });
    }

    let import_result = is_imported(class_name, imports, class_map);
    match import_result {
        Some(ImportResult::Class(c) | ImportResult::StaticClass(c)) => {
            if let Ok(cm) = class_map.lock()
                && let Some(ic) = cm.get(&c)
            {
                let imported_class = ic.to_owned();
                drop(cm);
                return Ok(ResolveState {
                    jtype: JType::Class(c),
                    class: parent::include_parent(imported_class, class_map),
                });
            }
            Err(TyresError::ClassNotFound { class_path: c })
        }
        None => Err(TyresError::NotImported(class_name.into())),
    }
}

#[must_use]
pub fn resolve_import(
    jtype: &str,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
) -> Vec<String> {
    resolve_class_key(class_map, |class_path| {
        let Some((_, class_name)) = class_path.rsplit_once('.') else {
            return false;
        };
        class_name == jtype
    })
}

pub fn resolve_class_key(
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    infl: impl Fn(&&MyString) -> bool,
) -> Vec<String> {
    if let Ok(cm) = class_map.lock() {
        return cm
            .keys()
            .filter(infl)
            .map(ToString::to_string)
            .collect::<Vec<String>>();
    }
    Vec::new()
}

pub fn resolve_var(
    extend: &LocalVariable,
    imports: &[ImportUnit],
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
) -> Result<ResolveState, TyresError> {
    resolve_jtype(&extend.jtype, imports, class_map)
}

#[must_use]
pub fn resolve_params(
    params: &[Vec<CallItem>],
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
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
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
) -> Result<ResolveState, TyresError> {
    if call_chain.is_empty() {
        return Err(TyresError::CallChainEmpty);
    }
    let mut ops: Vec<ResolveState> = vec![ResolveState {
        class: class.clone(),
        jtype: JType::Class(class.class_path.clone()),
    }];
    let mut cc = call_chain.iter().peekable();
    while let Some(item) = cc.next() {
        let op = if cc.peek().is_some() {
            call_chain_op(item, &ops, lo_va, imports, class, class_map, true, false)?
        } else {
            call_chain_op_self(item, &ops, lo_va, imports, class, class_map, true)?
        };

        ops.push(op);
    }
    ops.last().map_or_else(
        || Err(TyresError::CallChainInvalid(call_chain.to_vec())),
        |last| Ok(last.clone()),
    )
}
pub fn resolve_call_chain_value(
    call_chain: &[CallItem],
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
) -> Result<ResolveState, TyresError> {
    if call_chain.is_empty() {
        return Err(TyresError::CallChainEmpty);
    }
    let mut ops: Vec<ResolveState> = vec![ResolveState {
        class: class.clone(),
        jtype: JType::Class(class.class_path.clone()),
    }];
    for item in call_chain {
        let op = call_chain_op(item, &ops, lo_va, imports, class, class_map, false, true)?;
        ops.push(op);
    }
    ops.last().map_or_else(
        || Err(TyresError::CallChainInvalid(call_chain.to_vec())),
        |last| Ok(last.clone()),
    )
}
pub fn resolve_call_chain_to_point(
    call_chain: &[CallItem],
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    point: &AstPoint,
) -> Result<ResolveState, TyresError> {
    if call_chain.is_empty() {
        return Err(TyresError::CallChainEmpty);
    }
    let mut ops: Vec<ResolveState> = vec![ResolveState {
        class: class.clone(),
        jtype: JType::Class(class.class_path.clone()),
    }];
    let mut cc = call_chain.iter().peekable();
    while let Some(item) = cc.next() {
        if item.get_range().is_after_range(point) {
            break;
        }
        let op = if cc.peek().is_some() {
            call_chain_op(item, &ops, lo_va, imports, class, class_map, true, false)
        } else {
            call_chain_op_self(item, &ops, lo_va, imports, class, class_map, true)
        };

        if let Ok(op) = op {
            ops.push(op);
        }
    }
    ops.last().map_or_else(
        || Err(TyresError::CallChainInvalid(call_chain.to_vec())),
        |last| Ok(last.clone()),
    )
}

#[allow(clippy::too_many_arguments)]
fn call_chain_op(
    item: &CallItem,
    ops: &[ResolveState],
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    resolve_argument: bool,
    return_value: bool,
) -> Result<ResolveState, TyresError> {
    match item {
        CallItem::MethodCall { name, range: _ } => {
            let Some(ResolveState { class, jtype: _ }) = ops.last() else {
                return Err(TyresError::NoClassInOps);
            };
            if let Some(method) = class.methods.iter().find(|m| m.name == Some(name.clone())) {
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
            if return_value {
                resolve_call_chain_value(prev, lo_va, imports, class, class_map)
            } else {
                resolve_call_chain(prev, lo_va, imports, class, class_map)
            }
        }
    }
}
fn call_chain_op_self(
    item: &CallItem,
    ops: &[ResolveState],
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    resolve_argument: bool,
) -> Result<ResolveState, TyresError> {
    match item {
        CallItem::MethodCall { name, range: _ } => {
            let Some(last) = ops.last() else {
                return Err(TyresError::NoClassInOps);
            };
            if last
                .class
                .methods
                .iter()
                .any(|m| m.name == Some(name.clone()))
            {
                return Ok(last.clone());
            }
            Err(TyresError::MethodNotFound(name.clone()))
        }
        CallItem::FieldAccess { name, range: _ } => {
            let Some(last) = ops.last() else {
                return Err(TyresError::NoClassInOps);
            };
            if last.class.fields.iter().any(|m| m.name == *name) {
                return Ok(last.clone());
            }
            Err(TyresError::FieldNotFound(name.clone()))
        }
        CallItem::Variable { name, range: _ } => {
            let Some(last) = ops.last() else {
                return Err(TyresError::NoClassInOps);
            };
            if lo_va.iter().any(|va| va.name == *name) {
                return Ok(last.clone());
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
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
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
                methods: vec![Method {
                    name: Some("clone".into()),
                    ret: JType::Array(i.clone()),
                    ..Default::default()
                }],
                fields: vec![Field {
                    access: Access::empty(),
                    name: "length".into(),
                    jtype: JType::Int,
                    source: None,
                }],
                ..Default::default()
            },
        }),
        JType::Class(c) | JType::Generic(c, _) => resolve(c, imports, class_map),
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
        JType::Access { base, inner } => {
            let query = format_smolstr!("{}${}", &base, &inner);
            eprintln!("Resolve JType::Access: {query}");
            if let Ok(cm) = class_map.lock()
                && let Some(out) = cm.get(&query)
            {
                Ok(ResolveState {
                    class: out.to_owned(),
                    jtype: JType::Access {
                        base: base.clone(),
                        inner: inner.clone(),
                    },
                })
            } else {
                Err(TyresError::ClassNotFound { class_path: query })
            }
        }
    }
}
