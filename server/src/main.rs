mod imports;

use std::path::Path;
use std::str::FromStr;

use dashmap::DashMap;
use parser::dto::Class;
use ropey::Rope;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tree_sitter::{Parser, Point, Tree};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        document_map: DashMap::new(),
        class_map: DashMap::new(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

pub struct Document {
    text: ropey::Rope,
    tree: Tree,
    parser: Parser,
}

struct Backend {
    client: Client,
    document_map: DashMap<String, Document>,
    class_map: DashMap<String, Class>,
}
impl Backend {
    async fn on_change(&self, params: TextDocumentItem) {
        let rope = ropey::Rope::from_str(&params.text);
        let key = params.uri.to_string();
        if let Some(mut document) = self.document_map.get_mut(&key) {
            let tree = Some(document.tree.clone());
            if let Some(ntree) = document.parser.parse(params.text, tree.as_ref()) {
                document.tree = ntree;
            }
            document.text = rope;
        } else {
            let mut parser = Parser::new();
            if parser.set_language(tree_sitter_java::language()).is_err() {
                return;
            }
            let Some(tree) = parser.parse(params.text, None) else {
                return;
            };
            self.document_map.insert(
                key,
                Document {
                    text: rope,
                    tree,
                    parser,
                },
            );
        }
    }

    fn _get_opened_document(
        &self,
        uri: &Url,
    ) -> Option<dashmap::mapref::one::Ref<'_, std::string::String, Document>> {
        // when file is open
        if let Some(document) = self.document_map.get(uri.as_str()) {
            return Some(document);
        };
        None
    }

    async fn get_document(
        &self,
        uri: &Url,
    ) -> Option<dashmap::mapref::one::Ref<'_, std::string::String, Document>> {
        // when file is open
        if let Some(document) = self._get_opened_document(uri) {
            return Some(document);
        };

        let Ok(text) = std::fs::read_to_string(uri.path()) else {
            eprintln!("Unable to open file and it is also not available on the client");
            return None;
        };

        // The file was no opened yet on the client so we have to open it.
        self.on_change(TextDocumentItem {
            uri: uri.clone(),
            text,
            version: 1,
            language_id: "".to_owned(),
        })
        .await;

        // The file should now be loaded
        if let Some(document) = self._get_opened_document(uri) {
            return Some(document);
        };
        None
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                        ..CodeActionOptions::default()
                    },
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some([' ', '.'].iter().map(|i| i.to_string()).collect()),
                    ..CompletionOptions::default()
                }),
                ..ServerCapabilities::default()
            },
            server_info: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let cpl = imports::get_classes_to_load(&params.text_document.text);
        let new_classes: Vec<_> = cpl
            .iter()
            .filter(|cp| !self.class_map.contains_key(**cp))
            .map(|p| format!("./target/dependency/{}.class", p.replace('.', "/")))
            .filter(|p| Path::new(p).exists())
            .map(|p| match parser::load_class_fs(Path::new(&p)) {
                Ok(class) => Some((p, class)),
                Err(_) => None,
            })
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .collect();
        for (path, class) in new_classes {
            dbg!(&path);
            dbg!(&class);
            self.class_map.insert(path, class);
        }

        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
            language_id: params.text_document.language_id,
        })
        .await
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: std::mem::take(&mut params.content_changes[0].text),
            version: params.text_document.version,
            language_id: "".to_owned(),
        })
        .await
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let params = params.text_document_position;
        let uri = params.text_document.uri;
        let position = params.position;
        let Some(document) = self.get_document(&uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };
        let mut out = vec![];
        out.extend(self.class_map.iter().map(|v| {
            let val = v.value();
            let methods: Vec<_> = val.methods.iter().map(|m| m.name.to_string()).collect();
            CompletionItem::new_simple(val.name.to_string(), methods.join("\n"))
        }));

        eprintln!("{:?}", out);

        Ok(Some(CompletionResponse::Array(out)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let params = params.text_document_position_params;
        let uri = params.text_document.uri;
        let position = params.position;
        let Some(document) = self.get_document(&uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };

        Ok(None)
    }
    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let Some(_document) = self.get_document(&params.text_document.uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };
        let point = params.range.start;
        let point = Point {
            row: point.line.try_into().unwrap_or_default(),
            column: point.character.try_into().unwrap_or_default(),
        };
        let _arguments = Some(vec![
            Value::String(params.text_document.uri.to_string()),
            Value::Number(point.row.into()),
            Value::Number(point.column.into()),
        ]);
        Ok(None)
    }
    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        let (_point, uri) = parser_command_args(params.clone());
        let Some(url) = uri else {
            return Ok(None);
        };
        let Some(_document) = self.get_document(&url).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };

        Ok(None)
    }
}

pub fn parser_command_args(params: ExecuteCommandParams) -> (Point, Option<Url>) {
    let mut uri: String = String::new();
    let mut row: usize = 0;
    let mut column: usize = 0;
    for (i, arguments) in params.arguments.into_iter().enumerate() {
        match arguments {
            Value::String(string) => {
                if i == 0 {
                    uri = string;
                }
            }
            Value::Number(number) => {
                if i == 1 {
                    row = number
                        .as_u64()
                        .unwrap_or_default()
                        .try_into()
                        .unwrap_or_default();
                }
                if i == 2 {
                    column = number
                        .as_u64()
                        .unwrap_or_default()
                        .try_into()
                        .unwrap_or_default();
                }
            }
            Value::Null => (),
            Value::Bool(_) => (),
            Value::Array(_) => (),
            Value::Object(_) => (),
        }
    }
    let point = Point { row, column };
    let url = Url::from_str(&uri).ok();
    (point, url)
}
