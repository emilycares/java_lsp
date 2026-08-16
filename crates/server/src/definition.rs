use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use ast::types::{AstFile, AstPoint};
use call_chain::CallItem;
use document::{Document, DocumentError, read_document_or_open_class};
use dto::{Class, ImportUnit, JType};
use local_variable::LocalVariable;
use lsp_extra::{SourceToUriError, ToLspRangeError, source_to_uri, to_lsp_range};
use lsp_types::{GotoDefinitionResponse, Location, SymbolKind, Uri};
use my_string::NuVec;
use position::PositionSymbol;
use tyres::TyresError;

use crate::hover::{ClassActionError, class_action};

#[derive(Debug)]
#[allow(dead_code)]
pub enum DefinitionError {
    Tyres(TyresError),
    ClassActon(ClassActionError),
    LocalVariableNotFound { name: NuVec },
    ValidatedItemDoesNotExists,
    ArgumentNotFound,
    Document(DocumentError),
    ToLspRange(ToLspRangeError),
    SourceToUri(SourceToUriError),
    FieldNotFound { name: NuVec },
    NotAnArray,
    NoSource,
}
pub struct DefinitionContext<'a> {
    pub document_uri: Uri,
    pub point: &'a AstPoint,
    pub vars: &'a [LocalVariable],
    pub imports: &'a [ImportUnit],
    pub class: &'a Class,
    pub class_map: Arc<RwLock<HashMap<NuVec, Class>>>,
    pub document_map: &'a Arc<RwLock<HashMap<NuVec, Document>>>,
}

pub fn class(
    ast: &AstFile,
    context: &DefinitionContext,
    document_map: &Arc<RwLock<HashMap<NuVec, Document>>>,
) -> Result<GotoDefinitionResponse, DefinitionError> {
    match class_action(
        ast,
        context.point,
        context.vars,
        context.imports,
        &context.class_map,
    ) {
        Ok((class, _range)) => {
            let mut ranges = vec![];
            let uri = class_to_uri(&class)?;
            let Some(source) = class.get_source() else {
                return Err(DefinitionError::NoSource);
            };
            if let Ok(c) = read_document_or_open_class(&source, document_map) {
                position::get_class_position(&c.ast, Some(&class.name), &mut ranges);
            }
            Ok(go_to_definition_range(uri, &ranges)?)
        }
        Err(e) => Err(DefinitionError::ClassActon(e)),
    }
}

pub fn call_chain_definition(
    call_chain: &[CallItem],
    context: &DefinitionContext,
) -> Result<GotoDefinitionResponse, DefinitionError> {
    let call_chain = call_chain::flatten_argument_lists(call_chain);
    let (item, relevant) = call_chain::validate(&call_chain, context.point);

    let resolve_state = tyres::resolve_call_chain_to_point(
        &relevant,
        context.vars,
        context.imports,
        context.class,
        &context.class_map,
        context.point,
    )
    .map_err(DefinitionError::Tyres)?;
    let Some(source) = resolve_state.class.get_source() else {
        return Err(DefinitionError::NoSource);
    };
    match relevant.get(item) {
        Some(CallItem::This { range: _ }) => {
            let uri = source_to_uri(&source).map_err(DefinitionError::SourceToUri)?;
            let ast = document::get_ast(&source, context.document_map)
                .map_err(DefinitionError::Document)?;
            let mut ranges = Vec::new();
            position::get_class_position(&ast, None, &mut ranges);
            Ok(go_to_definition_range(uri, &ranges)?)
        }
        Some(CallItem::Class { name, range: _ } | CallItem::ClassGeneric { name, .. }) => {
            let uri = source_to_uri(&source).map_err(DefinitionError::SourceToUri)?;
            let ast = document::get_ast(&source, context.document_map)
                .map_err(DefinitionError::Document)?;
            let mut ranges = Vec::new();
            position::get_class_position(&ast, Some(name), &mut ranges);
            Ok(go_to_definition_range(uri, &ranges)?)
        }
        Some(CallItem::MethodCall {
            name,
            args,
            range: _,
        }) => {
            let args_len = args.len();
            let source_file = resolve_state
                .class
                .methods
                .iter()
                .filter(|i| i.name.as_ref().is_some_and(|i| i == name))
                .filter(|i| i.parameters.len() == args_len)
                .find_map(|i| i.source.clone())
                .map_or(source, |m| m);

            let ast = document::get_ast(&source_file, context.document_map)
                .map_err(DefinitionError::Document)?;
            let mut ranges = Vec::new();
            position::get_method_position(&ast, Some(name), Some(args_len), &mut ranges);
            let uri = source_to_uri(&source_file).map_err(DefinitionError::SourceToUri)?;
            Ok(go_to_definition_range(uri, &ranges)?)
        }
        Some(CallItem::FieldAccess { name, range: _ }) => {
            field_definition(context, &resolve_state, name)
        }
        Some(CallItem::Variable { name, range: _ }) => {
            let Some(range) = context
                .vars
                .iter()
                .find(|n| n.name == *name)
                .map(|v| v.range)
            else {
                return Err(DefinitionError::LocalVariableNotFound { name: name.clone() });
            };
            let range = to_lsp_range(&range).map_err(DefinitionError::ToLspRange)?;
            Ok(GotoDefinitionResponse::Scalar(Location {
                uri: context.document_uri.clone(),
                range,
            }))
        }
        Some(CallItem::ClassOrVariable { name, range: _ }) => {
            if let Ok(d) = field_definition(context, &resolve_state, name) {
                return Ok(d);
            }
            let ranges: Vec<_> = context
                .vars
                .iter()
                .filter(|n| n.name == *name)
                .map(|v| PositionSymbol {
                    range: v.range,
                    name: v.name.clone(),
                    kind: SymbolKind::VARIABLE,
                })
                .collect();

            Ok(go_to_definition_range(
                context.document_uri.clone(),
                &ranges,
            )?)
        }
        Some(CallItem::ArgumentList {
            prev: _,
            active_param,
            filled_params,
            range: _,
        }) => {
            if let Some(active_param) = active_param
                && let Some(current_param) = filled_params.get(*active_param)
            {
                return call_chain_definition(current_param, context);
            }
            Err(DefinitionError::ArgumentNotFound)
        }
        Some(CallItem::ArrayAccess { .. }) => {
            if let JType::Array(i) = resolve_state.jtype
                && let Ok(res) = tyres::resolve_jtype(&i, context.imports, &context.class_map)
            {
                let Some(source) = res.class.get_source() else {
                    return Err(DefinitionError::NoSource);
                };
                let ast = document::get_ast(&source, context.document_map)
                    .map_err(DefinitionError::Document)?;
                let mut ranges = Vec::new();
                let name = match *i {
                    JType::Class(name) | JType::Generic(name, _) => Some(name),
                    _ => None,
                };
                position::get_class_position(&ast, name.as_ref(), &mut ranges);
                let uri = source_to_uri(&source).map_err(DefinitionError::SourceToUri)?;
                return go_to_definition_range(uri, &ranges);
            }
            Err(DefinitionError::NotAnArray)
        }
        None => Err(DefinitionError::ValidatedItemDoesNotExists),
    }
}

fn field_definition(
    context: &DefinitionContext<'_>,
    resolve_state: &tyres::ResolveState,
    name: &NuVec,
) -> Result<GotoDefinitionResponse, DefinitionError> {
    if let Some(field) = resolve_state.class.fields.iter().find(|i| &i.name == name) {
        let source = if let Some(s) = field.source.clone() {
            s
        } else {
            let Some(s) = resolve_state.class.get_source() else {
                return Err(DefinitionError::NoSource);
            };
            s
        };

        let ast =
            document::get_ast(&source, context.document_map).map_err(DefinitionError::Document)?;
        let mut ranges = Vec::new();
        position::get_field_position(&ast, Some(name), &mut ranges);
        let uri = source_to_uri(&source).map_err(DefinitionError::SourceToUri)?;
        return go_to_definition_range(uri, &ranges);
    }
    Err(DefinitionError::FieldNotFound { name: name.clone() })
}

pub fn class_to_uri(class: &Class) -> Result<Uri, DefinitionError> {
    let Some(source) = class.get_source() else {
        return Err(DefinitionError::NoSource);
    };
    source_to_uri(&source).map_err(DefinitionError::SourceToUri)
}

fn go_to_definition_range(
    uri: Uri,
    ranges: &[PositionSymbol],
) -> Result<GotoDefinitionResponse, DefinitionError> {
    match ranges.len() {
        0 => Ok(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: lsp_types::Range::default(),
        })),
        1 => Ok(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: to_lsp_range(&ranges.first().expect("Length is 1").range)
                .map_err(DefinitionError::ToLspRange)?,
        })),
        2.. => {
            let locations = ranges
                .iter()
                .filter_map(|r| to_lsp_range(&r.range).ok())
                .map(|r| Location {
                    uri: uri.clone(),
                    range: r,
                })
                .collect();
            Ok(GotoDefinitionResponse::Array(locations))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use dto::{Access, JType, Method, SourceDestination};
    use expect_test::expect;
    use variables::VariableContext;

    use crate::backend::get_document_map_key;

    use super::*;

    #[test]
    fn definition_base() {
        let cont = r#"
package ch.emilycares;
import org.jboss.logging.Logger;
public class Test {
    private Logger LOG = Logger.getLogger(GreetingResource.class);
    public String hello() {
        LOG.info("doing hello");
        return "Hello";
    }
}
        "#;
        let point = AstPoint::new(6, 14);
        let document = Document::setup(cont, PathBuf::from_str("/Test.java").unwrap()).unwrap();
        let document_uri = Uri::from_str("file:///Test.java").unwrap();
        let class = parser::java::load_java_tree(&document.ast, SourceDestination::None);
        let imports = imports::imports(&document.ast);
        let vars = variables::get_vars(
            &document.ast,
            &VariableContext {
                point: Some(point),
                imports: &imports,
                class: &class,
                class_map: get_class_map(),
            },
        )
        .unwrap();
        let call_chain = call_chain::get_call_chain(&document.ast, &point);
        let context = DefinitionContext {
            document_uri,
            point: &point,
            vars: &vars,
            imports: &imports,
            class: &class,
            class_map: get_class_map(),
            document_map: &Arc::new(RwLock::new(HashMap::new())),
        };
        let out = call_chain_definition(&call_chain, &context);
        let expected = expect![[r#"
            Err(
                Document(
                    IoNotFound(
                        Some(
                            "Logger",
                        ),
                    ),
                ),
            )
        "#]];
        expected.assert_debug_eq(&out);
    }
    #[test]
    fn definition_stream_map() {
        let cont = r#"
package ch.emilycares;
import java.util.ArrayList;
import java.util.List;
public class Test {
    public String hello() {
        List<String> list = new ArrayList();

        list.stream().map(i -> i);

        return "Hello";
    }
}
        "#;
        let point = AstPoint::new(8, 24);
        let document = Document::setup(cont, PathBuf::from_str("/Test.java").unwrap()).unwrap();
        let document_uri = Uri::from_str("file:///Test.java").unwrap();
        let class = parser::java::load_java_tree(&document.ast, SourceDestination::None);
        let imports = imports::imports(&document.ast);
        let vars = variables::get_vars(
            &document.ast,
            &VariableContext {
                point: Some(point),
                imports: &imports,
                class: &class,
                class_map: get_class_map(),
            },
        )
        .unwrap();
        let call_chain = call_chain::get_call_chain(&document.ast, &point);
        let context = DefinitionContext {
            document_uri,
            point: &point,
            vars: &vars,
            imports: &imports,
            class: &class,
            class_map: get_class_map(),
            document_map: &Arc::new(RwLock::new(HashMap::new())),
        };
        let out = call_chain_definition(&call_chain, &context);
        let expected = expect![[r#"
            Err(
                Document(
                    IoNotFound(
                        Some(
                            "Stream",
                        ),
                    ),
                ),
            )
        "#]];
        expected.assert_debug_eq(&out);
    }
    #[test]
    fn definition_parent_method() {
        let cont = "
package ch.emilycares;
import ch.emilycares.a.ParGreet;
public class Test extends ParGreet {
    public String hello() {
        return greet();
    }
}
        ";
        let point = AstPoint::new(5, 19);
        let document = Document::setup(cont, PathBuf::from_str("/Test.java").unwrap()).unwrap();
        let document_uri = Uri::from_str("file:///Test.java").unwrap();
        let class = parser::java::load_java_tree(&document.ast, SourceDestination::None);
        let imports = imports::imports(&document.ast);
        let vars = variables::get_vars(
            &document.ast,
            &VariableContext {
                point: Some(point),
                imports: &imports,
                class: &class,
                class_map: get_class_map(),
            },
        )
        .unwrap();
        let call_chain = call_chain::get_call_chain(&document.ast, &point);
        let context = DefinitionContext {
            document_uri,
            point: &point,
            vars: &vars,
            imports: &imports,
            class: &class,
            class_map: get_class_map(),
            document_map: &Arc::new(RwLock::new(HashMap::new())),
        };
        let out = call_chain_definition(&call_chain, &context);
        let expected = expect![["
            Err(
                NoSource,
            )
        "]];
        expected.assert_debug_eq(&out);
    }
    #[test]
    fn definition_same_class_method() {
        let cont = "
package ch.emilycares;
public class Test {
    public String hello() {
        return greet();
    }
    private int greet() {
        return 1;
    }
}
        ";
        let point = AstPoint::new(4, 19);
        #[cfg(not(windows))]
        let file = "/Test.java";
        #[cfg(windows)]
        let file = "Test.java";
        let document = Document::setup(cont, PathBuf::from_str(file).unwrap()).unwrap();
        let document_uri = Uri::from_str("file:///Test.java").unwrap();
        let class = parser::java::load_java_tree(
            &document.ast,
            SourceDestination::Here(NuVec::new_static(file.as_bytes())),
        );
        let imports = imports::imports(&document.ast);
        let vars = variables::get_vars(
            &document.ast,
            &VariableContext {
                point: Some(point),
                imports: &imports,
                class: &class,
                class_map: get_class_map(),
            },
        )
        .unwrap();
        let call_chain = call_chain::get_call_chain(&document.ast, &point);
        let context = DefinitionContext {
            document_uri: document_uri.clone(),
            point: &point,
            vars: &vars,
            imports: &imports,
            class: &class,
            class_map: get_class_map(),
            document_map: &Arc::new(RwLock::new(HashMap::new())),
        };
        if let Ok(mut dm) = context.document_map.write() {
            let key = get_document_map_key(&document_uri);
            dm.insert(key, document);
        }
        let out = call_chain_definition(&call_chain, &context);
        let expected = expect![[r#"
            Ok(
                Scalar(
                    Location {
                        uri: Uri(
                            Uri {
                                scheme: Some(
                                    "file",
                                ),
                                authority: Some(
                                    Authority {
                                        userinfo: None,
                                        host: Host {
                                            text: "",
                                            data: RegName(
                                                "",
                                            ),
                                        },
                                        port: None,
                                    },
                                ),
                                path: "/Test.java",
                                query: None,
                                fragment: None,
                            },
                        ),
                        range: Range {
                            start: Position {
                                line: 6,
                                character: 4,
                            },
                            end: Position {
                                line: 8,
                                character: 5,
                            },
                        },
                    },
                ),
            )
        "#]];
        expected.assert_debug_eq(&out);
    }
    fn get_class_map() -> Arc<RwLock<HashMap<NuVec, Class>>> {
        let mut class_map: HashMap<NuVec, Class> = HashMap::new();
        class_map.insert(
            NuVec::new_static(b"ch.emilycares.a.ParGreet"),
            Class {
                methods: vec![Method {
                    name: Some(NuVec::new_static(b"greet")),
                    source: Some(NuVec::new_static(b"greet")),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            NuVec::new_static(b"org.jboss.logging.Logger"),
            Class {
                source: SourceDestination::Here(NuVec::new_static(b"Logger")),
                access: Access::Public,
                name: NuVec::new_static(b"Logger"),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some(NuVec::new_static(b"info")),
                    ret: JType::Void,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            NuVec::new_static(b"java.util.List"),
            Class {
                source: SourceDestination::Here(NuVec::new_static(b"List")),
                access: Access::Public,
                name: NuVec::new_static(b"List"),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some(NuVec::new_static(b"stream")),
                    ret: JType::Class(NuVec::new_static(b"java.util.stream.Stream")),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            NuVec::new_static(b"java.util.stream.Stream"),
            Class {
                source: SourceDestination::Here(NuVec::new_static(b"Stream")),
                access: Access::Public,
                name: NuVec::new_static(b"Stream"),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some(NuVec::new_static(b"map")),
                    ret: JType::Class(NuVec::new_static(b"java.util.stream.Stream")),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            NuVec::new_static(b"java.lang.String"),
            Class {
                source: SourceDestination::Here(NuVec::new_static(b"String")),
                access: Access::Public,
                name: NuVec::new_static(b"String"),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some(NuVec::new_static(b"length")),
                    ret: JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        Arc::new(RwLock::new(class_map))
    }
}
