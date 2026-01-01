#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::unnecessary_wraps)]
use std::{fs, path::PathBuf};

use ast::{
    error::PrintErr,
    types::{AstFile, AstThing},
};
use dashmap::{DashMap, mapref::one::RefMut};
use lsp_types::{Diagnostic, TextDocumentContentChangeEvent};
use my_string::MyString;
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
}

impl Document {
    pub fn reload_file_from_disk(&mut self) -> Result<(), DocumentError> {
        eprintln!("Reload file from disk: {:?}", self.path.display());
        let text = fs::read_to_string(&self.path).map_err(DocumentError::Io)?;
        self.rope = Rope::from_str(&text);
        self.reparse()?;
        Ok(())
    }
    pub fn setup_read(path: PathBuf) -> Result<Self, DocumentError> {
        eprintln!("Read file from disk: {:?}", path.display());
        let text = fs::read_to_string(&path).map_err(DocumentError::Io)?;
        let rope = Rope::from_str(&text);
        Self::setup_rope(path, rope)
    }
    pub fn setup(text: &str, path: PathBuf) -> Result<Self, DocumentError> {
        let rope = Rope::from_str(text);
        Self::setup_rope(path, rope)
    }

    pub fn setup_rope(path: PathBuf, rope: Rope) -> Result<Self, DocumentError> {
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

        if let Err(DocumentError::Diagnostic(diag)) = o.reparse() {
            return Err(DocumentError::Diagnostic(diag));
        }
        Ok(o)
    }

    pub fn replace_rope(&mut self, text: Rope) -> Result<(), DocumentError> {
        self.rope = text;
        self.reparse()
    }

    pub fn replace_string(&mut self, text: &str) -> Result<(), DocumentError> {
        let rope = Rope::from_str(text);
        self.rope = rope;
        self.reparse()
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
        self.reparse()
    }
    fn reparse(&mut self) -> Result<(), DocumentError> {
        match ast::lexer::lex(&self.rope.to_string()) {
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
pub fn get_class_path(ast: &AstFile) -> Option<String> {
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
        return Some(format!("{}.{}", package.name.value, name));
    }
    None
}

pub enum ClassSource<'a> {
    Owned(Box<Document>),
    Ref(RefMut<'a, MyString, Document>),
}
impl ClassSource<'_> {
    pub fn get_ast(&self) -> Result<&AstFile, DocumentError> {
        match self {
            ClassSource::Owned(document) => Ok(&document.ast),
            ClassSource::Ref(ref_mut) => Ok(&ref_mut.ast),
        }
    }
}
pub fn open_document(
    key: &str,
    content: &str,
    document_map: &DashMap<MyString, Document>,
) -> Result<(), DocumentError> {
    let path = path_without_subclass(key);
    let doc = Document::setup(content, path)?;
    document_map.insert(key.to_owned(), doc);
    Ok(())
}
pub fn read_document_or_open_class<'a>(
    source: &str,
    document_map: &'a DashMap<MyString, Document>,
) -> Result<ClassSource<'a>, DocumentError> {
    document_map.get_mut(source).map_or_else(
        || {
            let path = path_without_subclass(source);
            Document::setup_read(path).map(|doc| {
                document_map.insert(source.to_string(), doc.clone());
                ClassSource::Owned(Box::new(doc))
            })
        },
        |i| Ok(ClassSource::Ref(i)),
    )
}

pub fn get_ast(
    source: &str,
    document_map: &DashMap<MyString, Document>,
) -> Result<AstFile, DocumentError> {
    match read_document_or_open_class(source, document_map)? {
        ClassSource::Owned(d) => Ok(d.ast),
        ClassSource::Ref(d) => Ok(d.ast.clone()),
    }
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
