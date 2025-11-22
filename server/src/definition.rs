use std::str::FromStr;

use ast::types::AstPoint;
use call_chain::CallItem;
use document::{Document, DocumentError};
use lsp_types::{GotoDefinitionResponse, Location, Uri};
use my_string::MyString;
use parser::dto::{self, ImportUnit};
use position::PositionSymbol;
use tyres::TyresError;
use variables::LocalVariable;

use crate::{
    codeaction::to_lsp_range,
    hover::{ClassActionError, class_action},
};

#[derive(Debug)]
#[allow(dead_code)]
pub enum DefinitionError {
    Tyres(TyresError),
    ClassActon(ClassActionError),
    NoSourceFile { file: String },
    UriInvalid { uri: String, error: String },
    LocalVariableNotFound { name: String },
    ValidatedItemDoesNotExists,
    NoCallChain,
    Position(position::PosionError),
    ArgumentNotFound,
    Document(DocumentError),
}
pub struct DefinitionContext<'a> {
    pub document_uri: Uri,
    pub point: &'a AstPoint,
    pub vars: &'a [LocalVariable],
    pub imports: &'a [ImportUnit],
    pub class: &'a dto::Class,
    pub class_map: &'a dashmap::DashMap<MyString, dto::Class>,
    pub document_map: &'a dashmap::DashMap<MyString, Document>,
}

pub fn class(
    document: &Document,
    context: &DefinitionContext,
) -> Result<GotoDefinitionResponse, DefinitionError> {
    let ast = &document.ast;

    match class_action(
        ast,
        context.point,
        context.vars,
        context.imports,
        context.class_map,
    ) {
        Ok((class, _range)) => {
            let ranges = position::get_class_position_ast(ast, Some(&class.name))
                .map_err(DefinitionError::Position)?;
            let uri = class_to_uri(&class)?;
            Ok(go_to_definition_range(uri, ranges))
        }
        Err(e) => Err(DefinitionError::ClassActon(e)),
    }
}

pub fn call_chain_definition(
    call_chain: &[CallItem],
    context: &DefinitionContext,
) -> Result<GotoDefinitionResponse, DefinitionError> {
    let call_chain = call_chain::flatten_argument_lists(call_chain);
    let (item, relevat) = call_chain::validate(&call_chain, context.point);

    let resolve_state = tyres::resolve_call_chain_to_point(
        &relevat,
        context.vars,
        context.imports,
        context.class,
        context.class_map,
        context.point,
    )
    .map_err(DefinitionError::Tyres)?;
    match relevat.get(item) {
        Some(CallItem::This { range: _ }) => {
            let uri = source_to_uri(&resolve_state.class.source)?;
            let content = get_source_content(&resolve_state.class.source, context.document_map)?;
            let ranges = position::get_class_position_str(&content, None)
                .map_err(DefinitionError::Position)?;
            Ok(go_to_definition_range(uri, ranges))
        }
        Some(CallItem::Class { name, range: _ }) => {
            let uri = source_to_uri(&resolve_state.class.source)?;
            let content = get_source_content(&resolve_state.class.source, context.document_map)?;
            let ranges = position::get_class_position_str(&content, Some(name))
                .map_err(DefinitionError::Position)?;
            Ok(go_to_definition_range(uri, ranges))
        }
        Some(CallItem::MethodCall { name, range: _ }) => {
            let source_file = match resolve_state
                .class
                .methods
                .iter()
                .filter(|i| i.name == *name)
                .find_map(|i| i.source.clone())
            {
                Some(method_source) => method_source,
                None => resolve_state.class.source,
            };

            let content = get_source_content(&source_file, context.document_map)?;
            let ranges = position::get_method_positions(content.as_bytes(), name)
                .map_err(DefinitionError::Position)?;
            let uri = source_to_uri(&source_file)?;
            Ok(go_to_definition_range(uri, ranges))
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
            let content = get_source_content(&source_file, context.document_map)?;
            let ranges = position::get_field_positions(content.as_bytes(), name)
                .map_err(DefinitionError::Position)?;
            let uri = source_to_uri(&source_file)?;
            Ok(go_to_definition_range(uri, ranges))
        }
        Some(CallItem::Variable { name, range: _ }) => {
            let Some(range) = context
                .vars
                .iter()
                .find(|n| n.name == *name)
                .map(|v| v.range)
            else {
                return Err(DefinitionError::LocalVariableNotFound {
                    name: name.to_string(),
                });
            };
            Ok(GotoDefinitionResponse::Scalar(Location {
                uri: context.document_uri.clone(),
                range: to_lsp_range(&range),
            }))
        }
        Some(CallItem::ClassOrVariable { name, range: _ }) => {
            let ranges: Vec<_> = context
                .vars
                .iter()
                .filter(|n| n.name == *name)
                .map(|v| PositionSymbol::Range(v.range))
                .collect();

            Ok(go_to_definition_range(context.document_uri.clone(), ranges))
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
        Some(CallItem::Package { range: _, name: _ }) => todo!(),
        None => Err(DefinitionError::ValidatedItemDoesNotExists),
    }
}

pub fn get_source_content(
    source: &str,
    document_map: &dashmap::DashMap<MyString, Document>,
) -> Result<String, DefinitionError> {
    let uri = source_to_uri(source)?;
    match document::read_document_or_open_class(source, "".into(), document_map, uri.as_str()) {
        document::ClassSource::Owned(d) => Ok(d.str_data),

        document::ClassSource::Ref(d) => Ok(d.str_data.clone()),
        document::ClassSource::Err(e) => Err(DefinitionError::Document(e)),
    }
}

pub fn class_to_uri(class: &dto::Class) -> Result<Uri, DefinitionError> {
    source_to_uri(&class.source)
}
pub fn source_to_uri(source: &str) -> Result<Uri, DefinitionError> {
    let str_uri = format!("file:///{}", source.replace("\\", "/"));
    let uri = Uri::from_str(&str_uri);
    match uri {
        Ok(uri) => Ok(uri),
        Err(e) => Err(DefinitionError::UriInvalid {
            uri: str_uri,
            error: format!("{e:?}"),
        }),
    }
}

fn go_to_definition_range(uri: Uri, ranges: Vec<PositionSymbol>) -> GotoDefinitionResponse {
    match ranges.len() {
        0 => GotoDefinitionResponse::Scalar(Location {
            uri,
            range: lsp_types::Range::default(),
        }),
        1 => GotoDefinitionResponse::Scalar(Location {
            uri,
            range: to_lsp_range(ranges.first().expect("Length is 1").get_range()),
        }),
        2.. => {
            let locations = ranges
                .iter()
                .map(|r| to_lsp_range(r.get_range()))
                .map(|r| Location {
                    uri: uri.clone(),
                    range: r,
                })
                .collect();
            GotoDefinitionResponse::Array(locations)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use dashmap::DashMap;

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
        let document = Document::setup(
            content,
            PathBuf::from_str("/Test.java").unwrap(),
            "ch.emilycares.Test".into(),
        )
        .unwrap();
        let document_uri = Uri::from_str("file:///Test.java").unwrap();
        let class =
            parser::java::load_java_tree(&document.ast, parser::SourceDestination::None).unwrap();
        let vars = variables::get_vars(&document.ast, &point).unwrap();
        let imports = imports::imports(&document);
        let call_chain = call_chain::get_call_chain(&document.ast, &point);
        let context = DefinitionContext {
            document_uri,
            point: &point,
            vars: &vars,
            imports: &imports,
            class: &class,
            class_map: &get_class_map(),
            document_map: &DashMap::new(),
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
        let document = Document::setup(
            content,
            PathBuf::from_str("/Test.java").unwrap(),
            "ch.emilycares.Test".into(),
        )
        .unwrap();
        let document_uri = Uri::from_str("file:///Test.java").unwrap();
        let class =
            parser::java::load_java_tree(&document.ast, parser::SourceDestination::None).unwrap();
        let vars = variables::get_vars(&document.ast, &point).unwrap();
        let imports = imports::imports(&document);
        let call_chain = call_chain::get_call_chain(&document.ast, &point);
        let context = DefinitionContext {
            document_uri,
            point: &point,
            vars: &vars,
            imports: &imports,
            class: &class,
            class_map: &get_class_map(),
            document_map: &DashMap::new(),
        };
        let out = call_chain_definition(&call_chain, &context);
        assert!(out.is_err());
    }
    fn get_class_map() -> DashMap<MyString, dto::Class> {
        let class_map: DashMap<MyString, dto::Class> = DashMap::new();
        class_map.insert(
            "org.jboss.logging.Logger".into(),
            dto::Class {
                source: "/Logger.java".into(),
                access: dto::Access::Public,
                name: "Logger".into(),
                methods: vec![dto::Method {
                    access: dto::Access::Public,
                    name: "info".into(),
                    ret: dto::JType::Void,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            "java.util.List".into(),
            dto::Class {
                source: "/List.java".into(),
                access: dto::Access::Public,
                name: "List".into(),
                methods: vec![dto::Method {
                    access: dto::Access::Public,
                    name: "stream".into(),
                    ret: dto::JType::Class("java.util.stream.Stream".into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            "java.util.stream.Stream".into(),
            dto::Class {
                source: "/Stream.java".into(),
                access: dto::Access::Public,
                name: "Stream".into(),
                methods: vec![dto::Method {
                    access: dto::Access::Public,
                    name: "map".into(),
                    ret: dto::JType::Class("java.util.stream.Stream".into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            "java.lang.String".into(),
            dto::Class {
                access: dto::Access::Public,
                name: "String".into(),
                methods: vec![dto::Method {
                    access: dto::Access::Public,
                    name: "length".into(),
                    ret: dto::JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map
    }
}
