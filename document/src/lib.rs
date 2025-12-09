#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::{fs, path::PathBuf};

use ast::{error::PrintErr, types::AstFile};
use dashmap::{DashMap, mapref::one::RefMut};
use lsp_types::TextDocumentContentChangeEvent;
use my_string::MyString;
use ropey::Rope;

pub struct Document {
    pub text: ropey::Rope,
    pub str_data: String,
    pub ast: AstFile,
    pub path: PathBuf,
    pub class_path: MyString,
}

#[derive(Debug)]
pub enum DocumentError {
    Io(std::io::Error),
    Lexer(ast::lexer::LexerError),
    Ast(ast::error::AstError),
}

impl Document {
    pub fn reload_file_from_disk(&mut self) -> Result<(), DocumentError> {
        let text = fs::read_to_string(&self.path).map_err(DocumentError::Io)?;
        self.text = ropey::Rope::from_str(&text);
        self.str_data = text;
        self.reparse(false)?;
        Ok(())
    }
    pub fn setup_read(path: PathBuf, class_path: MyString) -> Result<Self, DocumentError> {
        let text = fs::read_to_string(&path).map_err(DocumentError::Io)?;
        let rope = ropey::Rope::from_str(&text);
        Self::setup_rope(&text, path, rope, class_path)
    }
    pub fn setup(text: &str, path: PathBuf, class_path: MyString) -> Result<Self, DocumentError> {
        let rope = ropey::Rope::from_str(text);
        Self::setup_rope(text, path, rope, class_path)
    }

    pub fn setup_rope(
        text: &str,
        path: PathBuf,
        rope: Rope,
        class_path: MyString,
    ) -> Result<Self, DocumentError> {
        let tokens = ast::lexer::lex(text).map_err(DocumentError::Lexer)?;
        let ast = ast::parse_file(&tokens);
        ast.print_err(text, &tokens);
        let ast = ast.map_err(DocumentError::Ast)?;
        Ok(Self {
            text: rope,
            str_data: text.to_string(),
            ast,
            path,
            class_path,
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.str_data
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }

    pub fn replace_text(&mut self, text: Rope) -> Result<(), DocumentError> {
        self.text = text;
        self.reparse(true)?;
        Ok(())
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
                    .text
                    .line_to_char(sp.line.try_into().unwrap_or_default())
                    + TryInto::<usize>::try_into(sp.character).unwrap_or_default();
                let end_idx = self
                    .text
                    .line_to_char(ep.line.try_into().unwrap_or_default())
                    + TryInto::<usize>::try_into(ep.character).unwrap_or_default();

                let do_insert = !change.text.is_empty();

                if start_idx < end_idx {
                    self.text.remove(start_idx..end_idx);
                    if do_insert {
                        self.text.insert(start_idx, &change.text);
                    }
                } else {
                    self.text.remove(end_idx..start_idx);
                    if do_insert {
                        self.text.insert(end_idx, &change.text);
                    }
                }

                continue;
            }

            if change.range.is_none() && change.range_length.is_none() {
                self.text = Rope::from_str(&change.text);
            }
        }
        self.reparse(true)?;
        Ok(())
    }
    fn reparse(&mut self, update_str: bool) -> Result<(), DocumentError> {
        if update_str {
            self.str_data = self.text.to_string();
        }
        let tokens = ast::lexer::lex(&self.str_data).map_err(DocumentError::Lexer)?;
        let ast = ast::parse_file(&tokens);
        ast.print_err(&self.str_data, &tokens);
        let ast = ast.map_err(DocumentError::Ast)?;
        self.ast = ast;
        Ok(())
    }
}

pub enum ClassSource<'a, D> {
    Owned(D),
    Ref(RefMut<'a, MyString, Document>),
    Err(DocumentError),
}
#[must_use]
pub fn read_document_or_open_class<'a, 'b>(
    source: &'b str,
    class_path: MyString,
    document_map: &'a DashMap<MyString, Document>,
    uri: &'b str,
) -> ClassSource<'a, Document> {
    document_map.get_mut(uri).map_or_else(
        || match Document::setup_read(PathBuf::from(source), class_path) {
            Ok(doc) => ClassSource::Owned(doc),
            Err(e) => ClassSource::Err(e),
        },
        ClassSource::Ref,
    )
}
