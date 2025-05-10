use std::{fs::read_to_string, path::PathBuf, str::FromStr};

use call_chain::CallItem;
use lsp_types::{GotoDefinitionResponse, Location, Uri};
use parser::dto::{self, ImportUnit};
use position::PositionSymbol;
use tree_sitter::Point;
use tree_sitter_util::lsp::to_lsp_range;
use tyres::TyresError;
use variables::LocalVariable;

use crate::{
    hover::{class_action, ClassActionError},
    Document,
};

#[derive(Debug, PartialEq)]
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
}

pub fn class(
    document: &Document,
    document_uri: &Uri,
    point: &Point,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Result<GotoDefinitionResponse, DefinitionError> {
    let tree = &document.tree;
    let bytes = document.as_bytes();

    match class_action(tree, bytes, point, lo_va, imports, class_map) {
        Ok((class, _range)) => {
            let source_file = get_source_content(&class.source)?;
            let ranges = position::get_class_position(source_file.as_bytes(), &class.name)
                .map_err(DefinitionError::Position)?;
            let uri = class_to_uri(&class)?;
            Ok(go_to_definition_range(uri, ranges))
        }
        Err(ClassActionError::VariableFound { var, range: _ }) => {
            Ok(GotoDefinitionResponse::Scalar(Location {
                uri: document_uri.clone(),
                range: to_lsp_range(var.range),
            }))
        }
        Err(e) => Err(DefinitionError::ClassActon(e)),
    }
}

pub fn call_chain_definition(
    document_uri: Uri,
    point: &Point,
    call_chain: &[CallItem],
    vars: &[LocalVariable],
    imports: &[ImportUnit],
    class: &dto::Class,
    class_map: &dashmap::DashMap<String, dto::Class>,
) -> Result<GotoDefinitionResponse, DefinitionError> {
    let (item, relevat) = call_chain::validate(call_chain, point);

    let resolve_state =
        tyres::resolve_call_chain_to_point(relevat, vars, imports, class, class_map, point)
            .map_err(DefinitionError::Tyres)?;
    match relevat.get(item) {
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

            let content = get_source_content(&source_file)?;
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
            let content = get_source_content(&source_file)?;
            let ranges = position::get_field_positions(content.as_bytes(), name)
                .map_err(DefinitionError::Position)?;
            let uri = source_to_uri(&source_file)?;
            Ok(go_to_definition_range(uri, ranges))
        }
        Some(CallItem::Variable { name, range: _ }) => {
            let Some(range) = vars.iter().find(|n| n.name == *name).map(|v| v.range) else {
                return Err(DefinitionError::LocalVariableNotFound {
                    name: name.to_owned(),
                });
            };
            Ok(GotoDefinitionResponse::Scalar(Location {
                uri: document_uri,
                range: to_lsp_range(range),
            }))
        }
        Some(CallItem::ClassOrVariable { name, range: _ }) => {
            let ranges: Vec<_> = vars
                .iter()
                .filter(|n| n.name == *name)
                .map(|v| PositionSymbol::Range(v.range))
                .collect();

            Ok(go_to_definition_range(document_uri, ranges))
        }
        Some(CallItem::ArgumentList {
            prev: _,
            active_param,
            filled_params,
            range: _,
        }) => {
            if let Some(current_param) = filled_params.get(*active_param) {
                return call_chain_definition(
                    document_uri,
                    point,
                    current_param,
                    vars,
                    imports,
                    class,
                    class_map,
                );
            }
            Err(DefinitionError::ArgumentNotFound)
        }
        Some(a) => unimplemented!("call_chain_definition {:?}", a),
        None => Err(DefinitionError::ValidatedItemDoesNotExists),
    }
}

pub fn get_source_content(source: &String) -> Result<String, DefinitionError> {
    let path = PathBuf::from(source);
    eprintln!("Loading source -> {}", &source);
    if path.exists() {
        if let Ok(sourc_file) = read_to_string(path) {
            return Ok(sourc_file);
        }
    }
    Err(DefinitionError::NoSourceFile {
        file: source.clone(),
    })
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
    use dashmap::DashMap;
    use parser::loader::SourceDestination;
    use pretty_assertions::assert_eq;

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
        let point = Point::new(6, 14);
        let bytes = content.as_bytes();
        let document = Document::setup(
            content,
            PathBuf::from_str("/Test.java").unwrap(),
            "ch.emilycares.Test".to_string(),
        )
        .unwrap();
        let document_uri = Uri::from_str("file:///Test.java").unwrap();
        let class =
            parser::java::load_java_tree(bytes, SourceDestination::None, &document.tree).unwrap();
        let vars = variables::get_vars(&document, &point).unwrap();
        let imports = imports::imports(&document);
        let call_chain = call_chain::get_call_chain(&document.tree, bytes, &point).unwrap();
        let out = call_chain_definition(
            document_uri,
            &point,
            &call_chain,
            &vars,
            &imports,
            &class,
            &get_class_map(),
        );
        assert_eq!(
            out,
            Err(DefinitionError::NoSourceFile {
                file: "/Logger.java".to_string()
            })
        );
    }
    fn get_class_map() -> DashMap<String, dto::Class> {
        let class_map: DashMap<String, dto::Class> = DashMap::new();
        class_map.insert(
            "org.jboss.logging.Logger".to_string(),
            dto::Class {
                source: "/Logger.java".to_string(),
                access: vec![dto::Access::Public],
                name: "Logger".to_string(),
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "info".to_string(),
                    ret: dto::JType::Void,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            "java.lang.String".to_string(),
            dto::Class {
                access: vec![dto::Access::Public],
                name: "String".to_string(),
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "length".to_string(),
                    ret: dto::JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map
    }
}
