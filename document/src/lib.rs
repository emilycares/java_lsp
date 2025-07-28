use std::{fs, path::PathBuf};

use ast::types::AstFile;
use dashmap::mapref::one::RefMut;
use lsp_types::TextDocumentContentChangeEvent;
use ropey::Rope;

pub struct Document {
    pub text: ropey::Rope,
    pub str_data: String,
    pub ast: AstFile,
    pub path: PathBuf,
    pub class_path: String,
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
        self.reparse(false);
        Ok(())
    }
    pub fn setup_read(path: PathBuf, class_path: String) -> Result<Self, DocumentError> {
        let text = fs::read_to_string(&path).map_err(DocumentError::Io)?;
        let rope = ropey::Rope::from_str(&text);
        Self::setup_rope(&text, path, rope, class_path)
    }
    pub fn setup(text: &str, path: PathBuf, class_path: String) -> Result<Self, DocumentError> {
        let rope = ropey::Rope::from_str(text);
        Self::setup_rope(text, path, rope, class_path)
    }

    pub fn setup_rope(
        text: &str,
        path: PathBuf,
        rope: Rope,
        class_path: String,
    ) -> Result<Self, DocumentError> {
        let tokens = ast::lexer::lex(text).map_err(DocumentError::Lexer)?;
        let ast = ast::parse_file(&tokens).map_err(DocumentError::Ast)?;
        Ok(Self {
            text: rope,
            str_data: text.to_string(),
            ast,
            path,
            class_path,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.str_data
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }

    pub fn replace_text(&mut self, text: Rope) {
        self.text = text;
        self.reparse(true);
    }

    pub fn apply_text_changes(&mut self, changes: &Vec<TextDocumentContentChangeEvent>) {
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
        self.reparse(true);
    }
    fn reparse(&mut self, update_str: bool) {
        if update_str {
            self.str_data = self.text.to_string();
        }
        if let Ok(tokens) = ast::lexer::lex(&self.str_data).map_err(DocumentError::Lexer) {
            if let Ok(ast) = ast::parse_file(&tokens).map_err(DocumentError::Ast) {
                self.ast = ast;
            }
        }
    }
}

pub enum ClassSource<'a, D> {
    Owned(D),
    Ref(RefMut<'a, String, Document>),
    Err(DocumentError),
}
pub fn read_document_or_open_class<'a, 'b>(
    source: &'b str,
    class_path: String,
    document_map: &'a dashmap::DashMap<String, Document>,
    uri: &'b str,
) -> ClassSource<'a, Document> {
    match document_map.get_mut(uri) {
        Some(d) => ClassSource::Ref(d),
        None => match Document::setup_read(PathBuf::from(source), class_path) {
            Ok(doc) => ClassSource::Owned(doc),
            Err(e) => ClassSource::Err(e),
        },
    }
}
