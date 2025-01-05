use lsp_types::TextDocumentContentChangeEvent;
use ropey::Rope;
use tree_sitter::{Parser, Tree};

pub struct Document {
    pub text: ropey::Rope,
    pub tree: Tree,
    parser: Parser,
}

impl Document {
    pub fn setup(text: &str) -> Option<Self> {
        let rope = ropey::Rope::from_str(text);
        Self::setup_rope(text, rope)
    }

    pub fn setup_rope(text: &str, rope: Rope) -> Option<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_java::LANGUAGE;
        if parser.set_language(&language.into()).is_err() {
            eprintln!("----- Not initialized -----");
            return None;
        }
        let tree = parser.parse(text, None)?;
        Some(Self {
            parser,
            text: rope,
            tree,
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.text.slice(..).as_str().unwrap_or_default().as_bytes()
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
                let start_idx = self.text.line_to_char(sp.line.try_into().unwrap())
                    + TryInto::<usize>::try_into(sp.character).unwrap();
                let end_idx = self.text.line_to_char(ep.line.try_into().unwrap())
                    + TryInto::<usize>::try_into(ep.character).unwrap();

                self.text.remove(start_idx..end_idx);

                self.text.insert(start_idx, &change.text);
                continue;
            }

            if change.range.is_none() && change.range_length.is_none() {
                self.text = Rope::from_str(&change.text);
            }
        }
        self.reparse();
    }
    fn reparse(&mut self) {
        let bytes = self.text.slice(..).as_str().unwrap_or_default().as_bytes();
        // Reusing the previous tree causes issues
        if let Some(ntree) = self.parser.parse(bytes, None) {
            self.tree = ntree;
        }
    }
}
