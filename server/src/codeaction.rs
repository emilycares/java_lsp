use std::{collections::HashMap, num::TryFromIntError};

use ast::types::{
    AstBlockEntry, AstBlockVariable, AstFile, AstForContent, AstIf, AstIfContent, AstPoint,
    AstRange, AstThing, AstWhileContent,
};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Position, Range, TextEdit, Uri, WorkspaceEdit,
};
use my_string::MyString;
use parser::dto::{self, ImportUnit};
use tyres::TyresError;
use variables::LocalVariable;

pub struct CodeActionContext<'a> {
    pub point: &'a AstPoint,
    pub imports: &'a [ImportUnit],
    pub class_map: &'a dashmap::DashMap<MyString, parser::dto::Class>,
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
    Int(TryFromIntError),
    ToLspRange(ToLspRangeError),
}

pub fn replace_with_value_type(
    ast: &AstFile,
    context: &CodeActionContext,
) -> Result<Option<CodeActionOrCommand>, CodeActionError> {
    let mut classvar = None;
    let mut blockvar = None;
    for thing in &ast.things {
        match thing {
            AstThing::Class(ast_class) => {
                let cvars = ast_class
                    .block
                    .variables
                    .iter()
                    .find(|i| i.range.is_in_range(context.point));
                if let Some(v) = cvars {
                    classvar = Some(v);
                } else {
                    let bvars = ast_class.block.methods.iter().find_map(|i| {
                        if let Some(block) = &i.block {
                            return find_var_block(block, context.point);
                        }
                        None
                    });

                    if let Some(v) = bvars {
                        blockvar = Some(v);
                    }
                }
            }
            AstThing::Record(ast_record) => {
                let cvars = ast_record
                    .block
                    .variables
                    .iter()
                    .find(|i| i.range.is_in_range(context.point));
                if let Some(v) = cvars {
                    classvar = Some(v);
                } else {
                    let bvars = ast_record.block.methods.iter().find_map(|i| {
                        if let Some(block) = &i.block {
                            return find_var_block(block, context.point);
                        }
                        None
                    });
                    if let Some(v) = bvars {
                        blockvar = Some(v);
                    }
                }
            }
            AstThing::Interface(_ast_interface) => todo!(),
            AstThing::Enumeration(_ast_enumeration) => todo!(),
            AstThing::Annotation(_ast_annotation) => todo!(),
        }
    }
    let point;
    let current_type = match (classvar, blockvar) {
        (None, None) => return Ok(None),
        (None | Some(_), Some(b)) => {
            point = b.range.end;
            &b.jtype
        }
        (Some(c), None) => {
            point = c.range.end;
            &c.jtype
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
        let range = to_lsp_range(&current_type.range).map_err(CodeActionError::ToLspRange)?;
        changes.insert(
            context.current_file.to_owned(),
            vec![TextEdit {
                range,
                new_text: value_resolve_state.class.name.clone(),
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

#[derive(Debug)]
pub enum ToLspRangeError {
    Int(TryFromIntError),
}
pub fn to_lsp_range(range: &AstRange) -> Result<Range, ToLspRangeError> {
    let sl = u32::try_from(range.start.line).map_err(ToLspRangeError::Int)?;
    let sc = u32::try_from(range.start.col).map_err(ToLspRangeError::Int)?;
    let el = u32::try_from(range.end.line).map_err(ToLspRangeError::Int)?;
    let ec = u32::try_from(range.end.col).map_err(ToLspRangeError::Int)?;

    Ok(Range {
        start: Position {
            line: sl,
            character: sc,
        },
        end: Position {
            line: el,
            character: ec,
        },
    })
}

fn find_var_block<'a>(
    block: &'a ast::types::AstBlock,
    point: &'a AstPoint,
) -> Option<&'a AstBlockVariable> {
    block
        .entries
        .iter()
        .find_map(|i| find_var_block_entry(point, i))
}

fn find_var_block_entry<'a>(
    point: &'a AstPoint,
    i: &'a AstBlockEntry,
) -> Option<&'a AstBlockVariable> {
    match i {
        AstBlockEntry::Return(_ast_block_return) => None,
        AstBlockEntry::Assert(_ast_block_return) => None,
        AstBlockEntry::Variable(ast_block_variable) => {
            for v in ast_block_variable {
                if v.range.is_in_range(point) {
                    return Some(v);
                }
            }
            None
        }
        AstBlockEntry::Expression(_ast_block_expression) => None,
        AstBlockEntry::Assign(_ast_block_assign) => None,
        AstBlockEntry::If(ast_if) => match ast_if {
            AstIf::ElseIf {
                range,
                control: _,
                control_range: _,
                content,
            }
            | AstIf::Else { range, content }
            | AstIf::If {
                range,
                control: _,
                control_range: _,
                content,
            } => {
                if range.is_in_range(point) {
                    return match content {
                        AstIfContent::Block(ast_block) => find_var_block(ast_block, point),
                        AstIfContent::BlockEntry(ast_block_entry) => {
                            find_var_block_entry(point, ast_block_entry)
                        }
                    };
                }
                None
            }
        },
        AstBlockEntry::While(ast_while) => {
            if ast_while.range.is_in_range(point) {
                return match &ast_while.content {
                    AstWhileContent::Block(ast_block) => find_var_block(ast_block, point),
                    AstWhileContent::BlockEntry(ast_block_entry) => {
                        find_var_block_entry(point, ast_block_entry)
                    }
                    AstWhileContent::None => None,
                };
            }
            None
        }
        AstBlockEntry::For(ast_for) => {
            if ast_for.range.is_in_range(point) {
                return find_var_for_content(&ast_for.content, point);
            }
            None
        }
        AstBlockEntry::ForEnhanced(ast_for_enhanced) => {
            if ast_for_enhanced.range.is_in_range(point) {
                return find_var_for_content(&ast_for_enhanced.content, point);
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
        AstBlockEntry::SwitchCaseArrowValues(_ast_switch_case_arrow) => None,
        AstBlockEntry::Yield(_ast_block_yield) => None,
        AstBlockEntry::SynchronizedBlock(_ast_synchronized_block) => todo!(),
        AstBlockEntry::SwitchCaseArrowDefault(_ast_switch_case_arrow_default) => todo!(),
        AstBlockEntry::Thing(_ast_thing) => todo!(),
        AstBlockEntry::InlineBlock(ast_block) => find_var_block(&ast_block.block, point),
        AstBlockEntry::Semicolon(_ast_range) => None,
        AstBlockEntry::SwitchCaseArrowType(_ast_switch_case_arrow_type) => todo!(),
    }
}

fn find_var_for_content<'a>(
    content: &'a AstForContent,
    point: &'a AstPoint,
) -> Option<&'a AstBlockVariable> {
    match content {
        AstForContent::Block(ast_block) => find_var_block(ast_block, point),
        AstForContent::BlockEntry(ast_block_entry) => find_var_block_entry(point, ast_block_entry),
        AstForContent::None => None,
    }
}

pub fn import_jtype(
    ast: &AstFile,
    context: &CodeActionContext,
) -> Option<Vec<CodeActionOrCommand>> {
    if let Some(class) = get_class::get_class(ast, context.point)
        && !tyres::is_imported_class_name(&class.name, context.imports, context.class_map)
    {
        let i = tyres::resolve_import(&class.name, context.class_map)
            .iter()
            .map(|a| import_to_code_action(context.current_file, a, ast))
            .collect();
        return Some(i);
    }
    None
}

pub fn get_import_position(ast: &AstFile) -> Result<Position, CodeActionError> {
    if let Some(imports) = &ast.imports {
        // After last import
        let end = imports.range.end;
        let line = u32::try_from(end.line).map_err(CodeActionError::Int)?;
        Ok(Position {
            line: line + 1,
            character: 0,
        })
    } else if let Some(package) = &ast.package {
        // After package
        let end = package.range.end;
        let line = u32::try_from(end.line).map_err(CodeActionError::Int)?;
        Ok(Position {
            line: line + 1,
            character: 0,
        })
    } else {
        // First line
        Ok(Position {
            line: 0,
            character: 0,
        })
    }
}
pub fn import_text_edit(classpath: &str, ast: &AstFile) -> Vec<TextEdit> {
    let pos = get_import_position(ast).map_or(
        Position {
            line: 2,
            character: 0,
        },
        |i| i,
    );

    vec![TextEdit {
        range: Range::new(pos, pos),
        new_text: format!("\nimport {classpath};"),
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
        title: format!("Import {classpath}"),
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
    use my_string::MyString;
    use parser::dto::{self, ImportUnit};
    use pretty_assertions::assert_eq;

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
        let class =
            parser::java::load_java_tree(&doc.ast, parser::SourceDestination::None).unwrap();
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
        let class =
            parser::java::load_java_tree(&doc.ast, parser::SourceDestination::None).unwrap();
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
        let class =
            parser::java::load_java_tree(&doc.ast, parser::SourceDestination::None).unwrap();
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
    fn get_class_map() -> DashMap<MyString, dto::Class> {
        let class_map: DashMap<MyString, dto::Class> = DashMap::new();
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
        class_map.insert(
            "java.io.FileInputStream".into(),
            dto::Class {
                access: dto::Access::Public,
                name: "FileInputStream".into(),
                methods: vec![],
                ..Default::default()
            },
        );
        class_map
    }
}
