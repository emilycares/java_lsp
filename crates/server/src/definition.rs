use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ast::types::{AstFile, AstPoint};
use call_chain::CallItem;
use document::{Document, DocumentError, read_document_or_open_class};
use lsp_extra::{SourceToUriError, ToLspRangeError, source_to_uri, to_lsp_range};
use lsp_types::{GotoDefinitionResponse, Location, SymbolKind, Uri};
use my_string::MyString;
use parser::dto::{Class, ImportUnit};
use position::PositionSymbol;
use tyres::TyresError;
use variables::LocalVariable;

use crate::hover::{ClassActionError, class_action};

#[derive(Debug)]
#[allow(dead_code)]
pub enum DefinitionError {
    Tyres(TyresError),
    ClassActon(ClassActionError),
    NoSourceFile { file: String },
    LocalVariableNotFound { name: String },
    ValidatedItemDoesNotExists,
    NoCallChain,
    ArgumentNotFound,
    Document(DocumentError),
    ToLspRange(ToLspRangeError),
    SourceToUri(SourceToUriError),
}
pub struct DefinitionContext<'a> {
    pub document_uri: Uri,
    pub point: &'a AstPoint,
    pub vars: &'a [LocalVariable],
    pub imports: &'a [ImportUnit],
    pub class: &'a Class,
    pub class_map: Arc<Mutex<HashMap<MyString, Class>>>,
    pub document_map: &'a Arc<Mutex<HashMap<MyString, Document>>>,
}

pub fn class(
    ast: &AstFile,
    context: &DefinitionContext,
    document_map: &Arc<Mutex<HashMap<MyString, Document>>>,
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
            if let Ok(c) = read_document_or_open_class(&class.source, document_map) {
                position::get_class_position_ast(&c.ast, Some(&class.name), &mut ranges);
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
    match relevant.get(item) {
        Some(CallItem::This { range: _ }) => {
            let uri =
                source_to_uri(&resolve_state.class.source).map_err(DefinitionError::SourceToUri)?;
            let ast = document::get_ast(&resolve_state.class.source, context.document_map)
                .map_err(DefinitionError::Document)?;
            let mut ranges = Vec::new();
            position::get_class_position_ast(&ast, None, &mut ranges);
            Ok(go_to_definition_range(uri, &ranges)?)
        }
        Some(CallItem::Class { name, range: _ }) => {
            let uri =
                source_to_uri(&resolve_state.class.source).map_err(DefinitionError::SourceToUri)?;
            let ast = document::get_ast(&resolve_state.class.source, context.document_map)
                .map_err(DefinitionError::Document)?;
            let mut ranges = Vec::new();
            position::get_class_position_ast(&ast, Some(name), &mut ranges);
            Ok(go_to_definition_range(uri, &ranges)?)
        }
        Some(CallItem::MethodCall { name, range: _ }) => {
            let source_file = match resolve_state
                .class
                .methods
                .iter()
                .filter(|i| i.name.as_ref().filter(|i| *i == name).is_some())
                .find_map(|i| i.source.clone())
            {
                Some(method_source) => method_source,
                None => resolve_state.class.source,
            };

            let ast = document::get_ast(&source_file, context.document_map)
                .map_err(DefinitionError::Document)?;
            let mut ranges = Vec::new();
            position::get_method_position_ast(&ast, Some(name), &mut ranges);
            let uri = source_to_uri(&source_file).map_err(DefinitionError::SourceToUri)?;
            Ok(go_to_definition_range(uri, &ranges)?)
        }
        Some(CallItem::FieldAccess { name, range: _ }) => {
            let source_file = match resolve_state
                .class
                .fields
                .iter()
                .filter(|i| i.name == *name)
                .find_map(|i| i.source.clone())
            {
                Some(method_source) => method_source,
                None => resolve_state.class.source,
            };
            let ast = document::get_ast(&source_file, context.document_map)
                .map_err(DefinitionError::Document)?;
            let mut ranges = Vec::new();
            position::get_field_position_ast(&ast, Some(name), &mut ranges);
            let uri = source_to_uri(&source_file).map_err(DefinitionError::SourceToUri)?;
            Ok(go_to_definition_range(uri, &ranges)?)
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
        None => Err(DefinitionError::ValidatedItemDoesNotExists),
    }
}

pub fn class_to_uri(class: &Class) -> Result<Uri, DefinitionError> {
    source_to_uri(&class.source).map_err(DefinitionError::SourceToUri)
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

    use parser::dto::{Access, JType, Method};

    use super::*;

    #[test]
    fn definition_base() {
        let content = r#"
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
        let point = AstPoint::new(6, 16);
        let document = Document::setup(content, PathBuf::from_str("/Test.java").unwrap()).unwrap();
        let document_uri = Uri::from_str("file:///Test.java").unwrap();
        let class = parser::java::load_java_tree(&document.ast, parser::SourceDestination::None);
        let vars = variables::get_vars(&document.ast, &point).unwrap();
        let imports = imports::imports(&document.ast);
        let call_chain = call_chain::get_call_chain(&document.ast, &point);
        let context = DefinitionContext {
            document_uri,
            point: &point,
            vars: &vars,
            imports: &imports,
            class: &class,
            class_map: get_class_map(),
            document_map: &Arc::new(Mutex::new(HashMap::new())),
        };
        let out = call_chain_definition(&call_chain, &context);
        assert!(out.is_err());
    }
    #[test]
    fn definition_stream_map() {
        let content = r#"
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
        let document = Document::setup(content, PathBuf::from_str("/Test.java").unwrap()).unwrap();
        let document_uri = Uri::from_str("file:///Test.java").unwrap();
        let class = parser::java::load_java_tree(&document.ast, parser::SourceDestination::None);
        let vars = variables::get_vars(&document.ast, &point).unwrap();
        let imports = imports::imports(&document.ast);
        let call_chain = call_chain::get_call_chain(&document.ast, &point);
        let context = DefinitionContext {
            document_uri,
            point: &point,
            vars: &vars,
            imports: &imports,
            class: &class,
            class_map: get_class_map(),
            document_map: &Arc::new(Mutex::new(HashMap::new())),
        };
        let out = call_chain_definition(&call_chain, &context);
        assert!(out.is_err());
    }
    fn get_class_map() -> Arc<Mutex<HashMap<MyString, Class>>> {
        let mut class_map: HashMap<MyString, Class> = HashMap::new();
        class_map.insert(
            "org.jboss.logging.Logger".into(),
            Class {
                source: "/Logger.java".into(),
                access: Access::Public,
                name: "Logger".into(),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some("info".into()),
                    ret: JType::Void,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            "java.util.List".into(),
            Class {
                source: "/List.java".into(),
                access: Access::Public,
                name: "List".into(),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some("stream".into()),
                    ret: JType::Class("java.util.stream.Stream".into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            "java.util.stream.Stream".into(),
            Class {
                source: "/Stream.java".into(),
                access: Access::Public,
                name: "Stream".into(),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some("map".into()),
                    ret: JType::Class("java.util.stream.Stream".into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            "java.lang.String".into(),
            Class {
                access: Access::Public,
                name: "String".into(),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some("length".into()),
                    ret: JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        Arc::new(Mutex::new(class_map))
    }
}
