use std::collections::HashMap;

use ast::types::{
    AstBlockEntry, AstBlockVariable, AstFile, AstIf, AstIfContent, AstPoint, AstRange,
};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Position, Range, TextEdit, Uri, WorkspaceEdit,
};
use parser::dto::{self, ImportUnit};
use smol_str::SmolStr;
use tyres::TyresError;
use variables::LocalVariable;

pub struct CodeActionContext<'a> {
    pub point: &'a AstPoint,
    pub imports: &'a [ImportUnit],
    pub class_map: &'a dashmap::DashMap<SmolStr, parser::dto::Class>,
    pub class: &'a dto::Class,
    pub vars: &'a [LocalVariable],
    pub current_file: &'a Uri,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum CodeActionError {
    NoCallCain,
    ParseJava(parser::java::ParseJavaError),
    Tyres(TyresError),
}

pub fn replace_with_value_type(
    ast: &AstFile,
    context: &CodeActionContext,
) -> Result<Option<CodeActionOrCommand>, CodeActionError> {
    let mut classvar = None;
    let mut blockvar = None;
    match &ast.thing {
        ast::types::AstThing::Class(ast_class) => {
            let cvars = ast_class
                .block
                .variables
                .iter()
                .find(|i| i.range.is_in_range(context.point));
            if let Some(v) = cvars {
                classvar = Some(v);
            } else {
                let bvars = ast_class
                    .block
                    .methods
                    .iter()
                    .find_map(|i| find_var_block(&i.block, context.point));
                if let Some(v) = bvars {
                    blockvar = Some(v);
                }
            }
        }
        ast::types::AstThing::Interface(_ast_interface) => todo!(),
        ast::types::AstThing::Enumeration(_ast_enumeration) => todo!(),
        ast::types::AstThing::Annotation(_ast_annotation) => todo!(),
    }
    let point;
    let current_type = match (classvar, blockvar) {
        (None, None) => return Ok(None),
        (None, Some(b)) => {
            point = b.range.end;
            &b.jtype
        }
        (Some(c), None) => {
            point = c.range.end;
            &c.jtype
        }
        (Some(_), Some(b)) => {
            point = b.range.end;
            &b.jtype
        }
    };
    // value here
    let call_chain = call_chain::get_call_chain(ast, &point);
    let value_resolve_state = tyres::resolve_call_chain_value(
        &call_chain,
        context.vars,
        context.imports,
        context.class,
        context.class_map,
    )
    .map_err(CodeActionError::Tyres)?;

    if &value_resolve_state.jtype != current_type {
        // Required by lsp types
        #[allow(clippy::mutable_key_type)]
        let mut changes = HashMap::new();
        changes.insert(
            context.current_file.to_owned(),
            vec![TextEdit {
                range: to_lsp_range(&current_type.range),
                new_text: value_resolve_state.class.name.to_string(),
            }],
        );
        let action = CodeActionOrCommand::CodeAction(CodeAction {
            kind: Some(CodeActionKind::QUICKFIX),
            title: format!("Replace variable type with: {}", value_resolve_state.jtype),
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }),
            ..Default::default()
        });
        return Ok(Some(action));
    }

    Ok(None)
}

pub fn to_lsp_range(range: &AstRange) -> Range {
    Range {
        start: Position {
            line: range.start.line as u32,
            character: range.start.col as u32,
        },
        end: Position {
            line: range.end.line as u32,
            character: range.end.col as u32,
        },
    }
}

fn find_var_block<'a>(
    block: &'a ast::types::AstBlock,
    point: &'a AstPoint,
) -> Option<&'a AstBlockVariable> {
    block.entries.iter().find_map(|i| match i {
        AstBlockEntry::Return(_ast_block_return) => None,
        AstBlockEntry::Variable(ast_block_variable) => {
            if ast_block_variable.range.is_in_range(point) {
                return Some(ast_block_variable);
            }
            None
        }
        AstBlockEntry::Expression(_ast_block_expression) => None,
        AstBlockEntry::Assign(_ast_block_assign) => None,
        AstBlockEntry::If(ast_if) => match ast_if {
            AstIf::If {
                range,
                control: _,
                control_range: _,
                content,
                el: _,
            } => {
                if range.is_in_range(point) {
                    return match content {
                        AstIfContent::Block(ast_block) => find_var_block(ast_block, point),
                        AstIfContent::Expression(_ast_expression) => None,
                        AstIfContent::None => None,
                    };
                }
                None
            }
            AstIf::Else { range, content } => {
                if range.is_in_range(point) {
                    return match content {
                        AstIfContent::Block(ast_block) => find_var_block(ast_block, point),
                        AstIfContent::Expression(_ast_expression) => None,
                        AstIfContent::None => None,
                    };
                }
                None
            }
        },
        AstBlockEntry::While(ast_while) => {
            if ast_while.range.is_in_range(point) {
                return find_var_block(&ast_while.block, point);
            }
            None
        }
        AstBlockEntry::For(ast_for) => {
            if ast_for.range.is_in_range(point) {
                return find_var_block(&ast_for.block, point);
            }
            None
        }
        AstBlockEntry::ForEnhanced(ast_for_enhanced) => {
            if ast_for_enhanced.range.is_in_range(point) {
                return find_var_block(&ast_for_enhanced.block, point);
            }
            None
        }
        AstBlockEntry::Break(_ast_block_break) => None,
        AstBlockEntry::Continue(_ast_block_continue) => None,
        AstBlockEntry::Switch(ast_switch) => {
            if ast_switch.range.is_in_range(point) {
                return find_var_block(&ast_switch.block, point);
            }
            None
        }
        AstBlockEntry::SwitchCase(_ast_switch_case) => None,
        AstBlockEntry::SwitchDefault(_ast_switch_default) => None,
        AstBlockEntry::TryCatch(ast_try_catch) => {
            if ast_try_catch.range.is_in_range(point) {
                if ast_try_catch.block.range.is_in_range(point) {
                    return find_var_block(&ast_try_catch.block, point);
                }
                if let Some(b) = &ast_try_catch.resources_block {
                    return find_var_block(b, point);
                }
                if let Some(b) = &ast_try_catch.finally_block {
                    return find_var_block(b, point);
                }
                if let Some(b) = ast_try_catch
                    .cases
                    .iter()
                    .find(|i| i.range.is_in_range(point))
                {
                    return find_var_block(&b.block, point);
                }
            }
            None
        }
        AstBlockEntry::Throw(_ast_throw) => None,
        AstBlockEntry::SwitchCaseArrow(_ast_switch_case_arrow) => None,
        AstBlockEntry::Yield(_ast_block_yield) => None,
    })
}

pub fn import_jtype(
    ast: &AstFile,
    context: &CodeActionContext,
) -> Option<Vec<CodeActionOrCommand>> {
    // if let Ok(n) = tree_sitter_util::get_node_at_point(tree, *context.point) {
    //     if n.kind() == "type_identifier" {
    //         if let Ok(jtype) = n.utf8_text(bytes) {
    if let Some(class) = get_class::get_class(ast, context.point)
        && !tyres::is_imported_class_name(&class.name, context.imports, context.class_map)
    {
        let i = tyres::resolve_import(&class.name, context.class_map)
            .iter()
            .map(|a| import_to_code_action(context.current_file, a, ast))
            .collect();
        return Some(i);
    }
    //         }
    //     }
    // }
    None
}

pub fn get_import_position(ast: &AstFile) -> Option<Position> {
    let end = ast.imports.range.end;
    Some(Position {
        line: (end.line as u32) + 1,
        character: 0,
    })
}
pub fn import_text_edit(classpath: &str, ast: &AstFile) -> Vec<TextEdit> {
    let mut pos = Position {
        line: 2,
        character: 0,
    };
    if let Some(npos) = get_import_position(ast) {
        pos = npos;
    }

    vec![TextEdit {
        range: Range::new(pos, pos),
        new_text: format!("\nimport {};", classpath),
    }]
}

#[allow(unused)]
pub fn import_to_code_action(
    current_file: &Uri,
    classpath: &str,
    ast: &AstFile,
) -> CodeActionOrCommand {
    // Required by lsp types
    #[allow(clippy::mutable_key_type)]
    let mut changes = HashMap::new();
    changes.insert(current_file.to_owned(), import_text_edit(classpath, ast));
    CodeActionOrCommand::CodeAction(CodeAction {
        kind: Some(CodeActionKind::QUICKFIX),
        title: format!("Import {}", classpath),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }),
        ..Default::default()
    })
}
#[cfg(test)]
pub mod tests {
    use std::{path::PathBuf, str::FromStr};

    use ast::types::AstPoint;
    use dashmap::DashMap;
    use document::Document;
    use lsp_types::Uri;
    use parser::dto::{self, ImportUnit};
    use pretty_assertions::assert_eq;
    use smol_str::SmolStr;

    use crate::codeaction::replace_with_value_type;

    use super::CodeActionContext;

    #[test]
    fn replace_type_base() {
        let content = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        int local = "";
    }
}
        "#;
        let point = AstPoint::new(4, 10);
        let doc = Document::setup(
            content,
            PathBuf::from_str("./").unwrap(),
            "ch.emilycares.Test".into(),
        )
        .unwrap();
        let imports = vec![];
        let class = parser::java::load_java_tree(&doc.ast, parser::loader::SourceDestination::None)
            .unwrap();
        let uri = Uri::from_str("file:///a").unwrap();
        let context = CodeActionContext {
            point: &point,
            imports: &imports,
            class_map: &get_class_map(),
            class: &class,
            vars: &variables::get_vars(&doc.ast, &point).unwrap(),
            current_file: &uri,
        };
        let out = replace_with_value_type(&doc.ast, &context);
        let result = out.unwrap().unwrap();
        match result {
            lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
                assert_eq!(code_action.title, "Replace variable type with: String");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn replace_type_with_argument() {
        let content = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        int local = new FileInputStream(new File(""));
    }
}
        "#;
        let point = AstPoint::new(4, 10);
        let doc = Document::setup(
            content,
            PathBuf::from_str("./").unwrap(),
            "ch.emilycares.Test".into(),
        )
        .unwrap();
        let imports = vec![ImportUnit::Class("java.io.FileInputStream".into())];
        let class = parser::java::load_java_tree(&doc.ast, parser::loader::SourceDestination::None)
            .unwrap();
        let uri = Uri::from_str("file:///a").unwrap();
        let context = CodeActionContext {
            point: &point,
            imports: &imports,
            class_map: &get_class_map(),
            class: &class,
            vars: &variables::get_vars(&doc.ast, &point).unwrap(),
            current_file: &uri,
        };
        let out = replace_with_value_type(&doc.ast, &context);
        let result = out.unwrap().unwrap();
        match result {
            lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
                assert_eq!(
                    code_action.title,
                    "Replace variable type with: java.io.FileInputStream"
                );
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn replace_type_method() {
        let content = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        String a = "";
        String local = a.length();
    }
}
        "#;
        let point = AstPoint::new(5, 10);
        let doc = Document::setup(
            content,
            PathBuf::from_str("./").unwrap(),
            "ch.emilycares.Test".into(),
        )
        .unwrap();
        let imports = vec![];
        let class = parser::java::load_java_tree(&doc.ast, parser::loader::SourceDestination::None)
            .unwrap();
        let uri = Uri::from_str("file:///a").unwrap();
        let context = CodeActionContext {
            point: &point,
            imports: &imports,
            class_map: &get_class_map(),
            class: &class,
            vars: &variables::get_vars(&doc.ast, &point).unwrap(),
            current_file: &uri,
        };
        let out = replace_with_value_type(&doc.ast, &context);
        let result = out.unwrap().unwrap();
        match result {
            lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
                assert_eq!(code_action.title, "Replace variable type with: int");
            }
            _ => unreachable!(),
        }
    }
    fn get_class_map() -> DashMap<SmolStr, dto::Class> {
        let class_map: DashMap<SmolStr, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".into(),
            dto::Class {
                access: vec![dto::Access::Public],
                name: "String".into(),
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "length".into(),
                    ret: dto::JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            "java.io.FileInputStream".into(),
            dto::Class {
                access: vec![dto::Access::Public],
                name: "FileInputStream".into(),
                methods: vec![],
                ..Default::default()
            },
        );
        class_map
    }
}
