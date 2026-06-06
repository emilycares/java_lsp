use ast::types::{AstFile, AstThing, AstTopLevel};
use common::project_kind::ProjectKind;
use config::Configuration;
use lsp_extra::{ToLspRangeError, to_lsp_range};
use lsp_types::{CodeLens, Command};
use my_string::smol_str::SmolStr;
use serde_json::Value;

use crate::{
    command::{COMMAND_CMD, COMMAND_CMD_EDITOR},
    document_link::SRC_TEST,
};

#[derive(Debug)]
pub enum CodeLensError {
    SkipFile,
    Range(ToLspRangeError),
}

pub fn tests(
    ast: &AstFile,
    file: &str,
    project_kind: &ProjectKind,
    config: &Configuration,
    out: &mut Vec<CodeLens>,
) -> Result<(), CodeLensError> {
    match project_kind {
        ProjectKind::Maven { .. } | ProjectKind::Gradle { .. } => {
            if !file.contains(SRC_TEST) {
                return Err(CodeLensError::SkipFile);
            }
        }
        ProjectKind::Unknown => return Err(CodeLensError::SkipFile),
    }

    for t in &ast.top {
        if let AstTopLevel::Thing(t) = t {
            tests_thing(t, out, project_kind, config)?;
        }
    }

    Ok(())
}

fn tests_thing(
    t: &AstThing,
    out: &mut Vec<CodeLens>,
    project_kind: &ProjectKind,
    config: &Configuration,
) -> Result<(), CodeLensError> {
    match t {
        AstThing::Class(ast_class) => {
            let name = ast_class.name.clone();
            let range = to_lsp_range(&name.range).map_err(CodeLensError::Range)?;
            out.push(CodeLens {
                range,
                command: command_test_class(&name.value, project_kind, config),
                data: None,
            });
        }
        AstThing::Record(ast_record) => {
            let name = ast_record.name.clone();
            let range = to_lsp_range(&name.range).map_err(CodeLensError::Range)?;
            out.push(CodeLens {
                range,
                command: command_test_class(&name.value, project_kind, config),
                data: None,
            });
        }
        AstThing::Interface(ast_interface) => {
            let name = ast_interface.name.clone();
            let range = to_lsp_range(&name.range).map_err(CodeLensError::Range)?;
            out.push(CodeLens {
                range,
                command: command_test_class(&name.value, project_kind, config),
                data: None,
            });
        }
        AstThing::Enumeration(ast_enumeration) => {
            let name = ast_enumeration.name.clone();
            let range = to_lsp_range(&name.range).map_err(CodeLensError::Range)?;
            out.push(CodeLens {
                range,
                command: command_test_class(&name.value, project_kind, config),
                data: None,
            });
        }
        AstThing::Annotation(_) => (),
    }
    Ok(())
}

fn command_test_class(
    class_name: &SmolStr,
    project_kind: &ProjectKind,
    config: &Configuration,
) -> Option<Command> {
    let name = format!("Run Test: {}", &class_name);
    let cmd = if config.editor_runs_commands {
        COMMAND_CMD_EDITOR
    } else {
        COMMAND_CMD
    };
    match project_kind {
        ProjectKind::Maven { executable } => Some(Command {
            title: name.clone(),
            command: String::from(cmd),
            arguments: Some(vec![
                Value::String(name),
                Value::String(executable.to_owned()),
                Value::String(String::from("test")),
                Value::String(format!("-Dtest={class_name}")),
            ]),
        }),
        ProjectKind::Gradle { executable, .. } => Some(Command {
            title: name.clone(),
            command: String::from(cmd),
            arguments: Some(vec![
                Value::String(name),
                Value::String(executable.to_owned()),
                Value::String(String::from("test")),
                Value::String(String::from("--tests")),
                Value::String(class_name.to_string()),
            ]),
        }),
        ProjectKind::Unknown => None,
    }
}
