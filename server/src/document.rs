use std::{fs, path::PathBuf};

use lsp_types::TextDocumentContentChangeEvent;
use ropey::Rope;
use tree_sitter::{Parser, Tree};

pub struct Document {
    pub text: ropey::Rope,
    pub str_data: String,
    pub tree: Tree,
    pub path: PathBuf,
    parser: Parser,
}

impl Document {
    pub fn setup_read(path: PathBuf) -> Option<Self> {
        let text = fs::read_to_string(&path).ok()?;
        let rope = ropey::Rope::from_str(&text);
        Self::setup_rope(&text, path, rope)
    }
    pub fn setup(text: &str, path: PathBuf) -> Option<Self> {
        let rope = ropey::Rope::from_str(text);
        Self::setup_rope(text, path, rope)
    }

    pub fn setup_rope(text: &str, path: PathBuf, rope: Rope) -> Option<Self> {
        let Some((parser, tree)) = tree_sitter_util::parse(text) else {
            return None;
        };
        Some(Self {
            parser,
            text: rope,
            str_data: text.to_string(),
            tree,
            path,
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
        self.reparse();
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
        self.reparse();
    }
    fn reparse(&mut self) {
        self.str_data = self.text.to_string();
        let bytes = self.str_data.as_bytes();
        // Reusing the previous tree causes issues
        if let Some(ntree) = self.parser.parse(bytes, None) {
            self.tree = ntree;
        }
    }
}
