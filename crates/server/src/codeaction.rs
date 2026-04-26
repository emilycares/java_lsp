use std::{
    cmp::Ordering,
    collections::HashMap,
    num::TryFromIntError,
    sync::{Arc, Mutex},
};

use ast::types::{
    AstBlockEntry, AstBlockVariable, AstFile, AstForContent, AstIf, AstIfContent, AstPackage,
    AstPoint, AstRange, AstThing, AstWhileContent,
};
use dto::{Class, ImportUnit};
use local_variable::LocalVariable;
use lsp_extra::{ToLspRangeError, to_lsp_position, to_lsp_range};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Position, Range, TextEdit, Uri, WorkspaceEdit,
};
use my_string::MyString;
use tyres::TyresError;

use crate::{
    document_link::{SRC_MAIN, SRC_TEST},
    hover::jtype_hover_display,
};

pub struct CodeActionContext<'a> {
    pub point: &'a AstPoint,
    pub imports: &'a [ImportUnit],
    pub class_map: Arc<Mutex<HashMap<MyString, Class>>>,
    pub class: &'a Class,
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
            AstThing::Interface(_ast_interface) => (),
            AstThing::Enumeration(_ast_enumeration) => (),
            AstThing::Annotation(_ast_annotation) => (),
        }
    }
    let mut point;
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
    point.col += 1;
    // value here
    let call_chain = call_chain::get_call_chain(ast, &point);
    let value_resolve_state = tyres::resolve_call_chain_value(
        &call_chain,
        context.vars,
        context.imports,
        context.class,
        &context.class_map.clone(),
    )
    .map_err(CodeActionError::Tyres)?;

    if &value_resolve_state.jtype != current_type {
        // Required by lsp types
        #[allow(clippy::mutable_key_type)]
        let mut changes = HashMap::new();
        let range = to_lsp_range(&current_type.range).map_err(CodeActionError::ToLspRange)?;
        let new_text = jtype_hover_display(&value_resolve_state.jtype);
        let title = format!("Replace variable type with: {}", &new_text);
        changes.insert(
            context.current_file.to_owned(),
            vec![TextEdit { range, new_text }],
        );
        let action = CodeActionOrCommand::CodeAction(CodeAction {
            kind: Some(CodeActionKind::QUICKFIX),
            title,
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
        AstBlockEntry::SynchronizedBlock(_ast_synchronized_block) => None,
        AstBlockEntry::SwitchCaseArrowDefault(_ast_switch_case_arrow_default) => None,
        AstBlockEntry::Thing(_ast_thing) => None,
        AstBlockEntry::InlineBlock(ast_block) => find_var_block(&ast_block.block, point),
        AstBlockEntry::Semicolon(_ast_range) => None,
        AstBlockEntry::SwitchCaseArrowType(_ast_switch_case_arrow_type) => None,
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
        && !tyres::is_imported_class_name(&class.name, context.imports, &context.class_map)
    {
        let mut resolve_import: Vec<String> =
            tyres::resolve_import(&class.name, &context.class_map);
        // Prefer java imports
        resolve_import.sort_by(|a, b| {
            let a_j = a.starts_with("java");
            let b_j = b.starts_with("java");
            if a_j && !b_j {
                Ordering::Less
            } else if !a_j && b_j {
                Ordering::Greater
            } else {
                a.cmp(b)
            }
        });

        let i = resolve_import
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
        new_text: format!("import {classpath};\n"),
    }]
}

pub fn generate_class(
    ast: &AstFile,
    current_file: &Uri,
) -> Result<Option<CodeActionOrCommand>, CodeActionError> {
    if !ast.things.is_empty() {
        return Ok(None);
    }
    let path = current_file.path().as_str();
    let name;
    let package;
    if let Some((path, fname)) = path.rsplit_once('/') {
        name = fname.trim_end_matches(".java");
        let is_test = path.contains(SRC_TEST);
        if is_test {
            if let Some((_, p)) = path.split_once(SRC_TEST) {
                package = p.trim_start_matches('/').replace('/', ".");
            } else {
                return Ok(None);
            }
        } else {
            if let Some((_, p)) = path.split_once(SRC_MAIN) {
                package = p.trim_start_matches('/').replace('/', ".");
            } else {
                return Ok(None);
            }
        }
    } else {
        return Ok(None);
    }
    #[allow(clippy::mutable_key_type)]
    let mut changes = HashMap::new();
    if let Some(AstPackage {
        range: AstRange { end, .. },
        ..
    }) = ast.package
    {
        let pos = to_lsp_position(end).map_err(CodeActionError::ToLspRange)?;
        changes.insert(
            current_file.clone(),
            vec![TextEdit {
                range: Range {
                    start: pos,
                    end: pos,
                },
                new_text: format!(
                    "public class {name} {{
                     }}"
                ),
            }],
        );
    } else {
        changes.insert(
            current_file.clone(),
            vec![TextEdit {
                range: Range::default(),
                new_text: format!(
                    "package {package}

public class {name} {{
}}
"
                ),
            }],
        );
    }

    Ok(Some(CodeActionOrCommand::CodeAction(CodeAction {
        title: "Generate Class".to_string(),
        kind: None,
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: None,
        disabled: None,
        data: None,
    })))
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
    use std::{
        collections::HashMap,
        path::PathBuf,
        str::FromStr,
        sync::{Arc, Mutex},
    };

    use ast::types::AstPoint;
    use document::Document;
    use dto::{Access, Class, ImportUnit, JType, Method, SourceDestination};
    use expect_test::expect;
    use lsp_types::Uri;
    use my_string::{MyString, smol_str::SmolStr};
    use pretty_assertions::assert_eq;
    use variables::VariableContext;

    use crate::codeaction::{generate_class, replace_with_value_type};

    use super::CodeActionContext;

    #[test]
    fn replace_type_base() {
        let cont = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        int local = "";
    }
}
        "#;
        let point = AstPoint::new(4, 10);
        let doc = Document::setup(cont, PathBuf::from_str("./").unwrap()).unwrap();
        let imports = vec![];
        let class = parser::java::load_java_tree(&doc.ast, SourceDestination::None);
        let uri = Uri::from_str("file:///a").unwrap();
        let context = CodeActionContext {
            point: &point,
            imports: &imports,
            class_map: get_class_map(),
            class: &class,
            vars: &variables::get_vars(
                &doc.ast,
                &VariableContext {
                    point: Some(point),
                    imports: &imports,
                    class: &class,
                    class_map: get_class_map(),
                },
            )
            .unwrap(),
            current_file: &uri,
        };
        let out = replace_with_value_type(&doc.ast, &context);
        let result = out.unwrap().unwrap();
        match result {
            lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
                assert_eq!(code_action.title, "Replace variable type with: String");
            }
            lsp_types::CodeActionOrCommand::Command(_) => panic!(),
        }
    }

    #[test]
    fn replace_type_with_argument() {
        let cont = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        int local = new FileInputStream(new File(""));
    }
}
        "#;
        let point = AstPoint::new(4, 10);
        let doc = Document::setup(cont, PathBuf::from_str("./").unwrap()).unwrap();
        let imports = vec![
            ImportUnit::Class(SmolStr::new_inline("java.io.FileInputStream")),
            ImportUnit::Class(SmolStr::new_inline("java.io.File")),
        ];
        let class = parser::java::load_java_tree(&doc.ast, SourceDestination::None);
        let uri = Uri::from_str("file:///a").unwrap();
        let context = CodeActionContext {
            point: &point,
            imports: &imports,
            class_map: get_class_map(),
            class: &class,
            vars: &variables::get_vars(
                &doc.ast,
                &VariableContext {
                    point: Some(point),
                    imports: &imports,
                    class: &class,
                    class_map: get_class_map(),
                },
            )
            .unwrap(),
            current_file: &uri,
        };
        let out = replace_with_value_type(&doc.ast, &context);
        let result = out.unwrap().unwrap();
        match result {
            lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
                assert_eq!(
                    code_action.title,
                    "Replace variable type with: FileInputStream"
                );
            }
            lsp_types::CodeActionOrCommand::Command(_) => panic!(),
        }
    }

    #[test]
    fn replace_type_method() {
        let cont = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        String a = "";
        String local = a.length();
    }
}
        "#;
        let point = AstPoint::new(5, 10);
        let doc = Document::setup(cont, PathBuf::from_str("./").unwrap()).unwrap();
        let imports = vec![];
        let class = parser::java::load_java_tree(&doc.ast, SourceDestination::None);
        let uri = Uri::from_str("file:///a").unwrap();
        let context = CodeActionContext {
            point: &point,
            imports: &imports,
            class_map: get_class_map(),
            class: &class,
            vars: &variables::get_vars(
                &doc.ast,
                &VariableContext {
                    point: Some(point),
                    imports: &imports,
                    class: &class,
                    class_map: get_class_map(),
                },
            )
            .unwrap(),
            current_file: &uri,
        };
        let out = replace_with_value_type(&doc.ast, &context);
        let result = out.unwrap().unwrap();
        match result {
            lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
                assert_eq!(code_action.title, "Replace variable type with: int");
            }
            lsp_types::CodeActionOrCommand::Command(_) => panic!(),
        }
    }

    #[test]
    fn generate_class_base() {
        let cont = r"
        ";
        let doc = Document::setup(cont, PathBuf::from_str("./").unwrap()).unwrap();
        let uri = Uri::from_str("file:///C:/src/test/java/my/thing/some/Thing.java").unwrap();
        let out = generate_class(&doc.ast, &uri);
        let result = out.unwrap().unwrap();
        let expected = expect![[r#"
            CodeAction(
                CodeAction {
                    title: "Generate Class",
                    kind: None,
                    diagnostics: None,
                    edit: Some(
                        WorkspaceEdit {
                            changes: Some(
                                {
                                    Uri(
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
                                            path: "/C:/src/test/java/my/thing/some/Thing.java",
                                            query: None,
                                            fragment: None,
                                        },
                                    ): [
                                        TextEdit {
                                            range: Range {
                                                start: Position {
                                                    line: 0,
                                                    character: 0,
                                                },
                                                end: Position {
                                                    line: 0,
                                                    character: 0,
                                                },
                                            },
                                            new_text: "package my.thing.some\n\npublic class Thing {\n}\n",
                                        },
                                    ],
                                },
                            ),
                            document_changes: None,
                            change_annotations: None,
                        },
                    ),
                    command: None,
                    is_preferred: None,
                    disabled: None,
                    data: None,
                },
            )
        "#]];
        expected.assert_debug_eq(&result);
    }
    fn get_class_map() -> Arc<Mutex<HashMap<MyString, Class>>> {
        let mut class_map: HashMap<MyString, Class> = HashMap::new();
        class_map.insert(
            SmolStr::new_inline("java.lang.String"),
            Class {
                access: Access::Public,
                name: SmolStr::new_inline("String"),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some(SmolStr::new_inline("length")),
                    ret: JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map.insert(
            SmolStr::new_inline("java.io.FileInputStream"),
            Class {
                access: Access::Public,
                name: SmolStr::new_inline("FileInputStream"),
                methods: vec![],
                ..Default::default()
            },
        );
        class_map.insert(
            SmolStr::new_inline("java.io.File"),
            Class {
                access: Access::Public,
                name: SmolStr::new_inline("FileInputStream"),
                methods: vec![],
                ..Default::default()
            },
        );
        Arc::new(Mutex::new(class_map))
    }
}
