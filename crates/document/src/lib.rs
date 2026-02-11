#![deny(warnings)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::implicit_hasher)]
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use ast::{
    error::PrintErr,
    types::{AstFile, AstThing},
};
use lsp_types::{Diagnostic, TextDocumentContentChangeEvent};
use my_string::{
    MyString,
    smol_str::{ToSmolStr, format_smolstr},
};
use ropey::Rope;

#[derive(Debug, Clone)]
pub struct Document {
    pub rope: Rope,
    pub ast: AstFile,
    pub path: PathBuf,
}

#[derive(Debug)]
pub enum DocumentError {
    Io(std::io::Error),
    Diagnostic(Box<Diagnostic>),
    Locked,
}

impl Document {
    pub fn reload_file_from_disk(&mut self) -> Result<(), DocumentError> {
        eprintln!("Reload file from disk: {:?}", self.path.display());
        let text = fs::read_to_string(&self.path).map_err(DocumentError::Io)?;
        self.rope = Rope::from_str(&text);
        self.reparse(text.as_bytes())?;
        Ok(())
    }
    pub fn setup_read(path: PathBuf) -> Result<Self, DocumentError> {
        eprintln!("Read file from disk: {:?}", path.display());
        let text = fs::read_to_string(&path).map_err(DocumentError::Io)?;
        let rope = Rope::from_str(&text);
        let mut o = Self {
            rope,
            ast: AstFile {
                package: None,
                imports: None,
                things: Vec::new(),
                modules: Vec::new(),
            },
            path,
        };

        o.reparse(text.as_bytes())?;
        Ok(o)
    }
    pub fn setup(text: &str, path: PathBuf) -> Result<Self, DocumentError> {
        let rope = Rope::from_str(text);
        let mut o = Self {
            rope,
            ast: AstFile {
                package: None,
                imports: None,
                things: Vec::new(),
                modules: Vec::new(),
            },
            path,
        };

        o.reparse(text.as_bytes())?;
        Ok(o)
    }

    pub fn apply_text_changes(
        &mut self,
        changes: &[TextDocumentContentChangeEvent],
    ) -> Result<(), DocumentError> {
        for change in changes {
            if let Some(range) = change.range {
                let sp = range.start;
                let ep = range.end;

                // Get the start/end char indices of the line.
                let start_idx = self
                    .rope
                    .line_to_char(sp.line.try_into().unwrap_or_default())
                    + TryInto::<usize>::try_into(sp.character).unwrap_or_default();
                let end_idx = self
                    .rope
                    .line_to_char(ep.line.try_into().unwrap_or_default())
                    + TryInto::<usize>::try_into(ep.character).unwrap_or_default();

                let do_insert = !change.text.is_empty();

                if start_idx < end_idx {
                    self.rope.remove(start_idx..end_idx);
                    if do_insert {
                        self.rope.insert(start_idx, &change.text);
                    }
                } else {
                    self.rope.remove(end_idx..start_idx);
                    if do_insert {
                        self.rope.insert(end_idx, &change.text);
                    }
                }

                continue;
            }

            if change.range.is_none() && change.range_length.is_none() {
                self.rope = Rope::from_str(&change.text);
            }
        }
        self.reparse(self.rope.to_string().as_bytes())
    }
    fn reparse(&mut self, bytes: &[u8]) -> Result<(), DocumentError> {
        match ast::lexer::lex(bytes) {
            Ok(tokens) => {
                let ast = ast::parse_file(&tokens);
                match ast {
                    Ok(ast) => {
                        self.ast = ast;
                    }
                    Err(e) => {
                        e.print_err(&self.rope.to_string(), &tokens);
                        if let Some(diag) = lsp_extra::ast_error_to_diagnostic(&e, &tokens) {
                            return Err(DocumentError::Diagnostic(Box::new(diag)));
                        }
                    }
                }
            }
            Err(e) => {
                return Err(DocumentError::Diagnostic(Box::new(
                    lsp_extra::lexer_error_to_diagnostic(&e),
                )));
            }
        }

        Ok(())
    }
}

#[must_use]
pub fn get_class_path(ast: &AstFile) -> Option<MyString> {
    if let Some(package) = &ast.package
        && let Some(thing) = &ast.things.first()
    {
        let name = match thing {
            AstThing::Class(ast_class) => &ast_class.name.value,
            AstThing::Record(ast_record) => &ast_record.name.value,
            AstThing::Interface(ast_interface) => &ast_interface.name.value,
            AstThing::Enumeration(ast_enumeration) => &ast_enumeration.name.value,
            AstThing::Annotation(ast_annotation) => &ast_annotation.name.value,
        };
        return Some(format_smolstr!("{}.{}", package.name.value, name));
    }
    None
}

pub fn open_document(
    key: &str,
    content: &str,
    document_map: &Arc<Mutex<HashMap<MyString, Document>>>,
) -> Result<(), DocumentError> {
    let path = path_without_subclass(key);
    let doc = Document::setup(content, path)?;

    let Ok(mut dm) = document_map.lock() else {
        return Err(DocumentError::Locked);
    };
    dm.insert(key.to_smolstr(), doc);
    Ok(())
}
pub fn read_document_or_open_class(
    source: &str,
    document_map: &Arc<Mutex<HashMap<MyString, Document>>>,
) -> Result<Document, DocumentError> {
    let Ok(mut dm) = document_map.lock() else {
        return Err(DocumentError::Locked);
    };
    if let Some(document) = dm.get(source) {
        return Ok(document.clone());
    }
    let path = path_without_subclass(source);
    Document::setup_read(path).inspect(|doc| {
        dm.insert(source.to_smolstr(), doc.clone());
    })
}

pub fn get_ast(
    source: &str,
    document_map: &Arc<Mutex<HashMap<MyString, Document>>>,
) -> Result<AstFile, DocumentError> {
    read_document_or_open_class(source, document_map).map(|i| i.ast)
}
fn path_without_subclass(source: &str) -> PathBuf {
    let mut path = PathBuf::from(source);
    {
        if let Some(file_name) = path.file_name()
            && let Some(file_name) = file_name.to_str()
            && file_name.contains('$')
            && let Some((name, extension)) = file_name.split_once('.')
            && let Some((name, _)) = name.split_once('$')
        {
            path.set_file_name(format!("{name}.{extension}"));
        }
    }
    path
}
