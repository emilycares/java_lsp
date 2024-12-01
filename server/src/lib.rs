pub mod completion;
mod imports;
mod tyres;
mod utils;
mod variable;
mod codeaction;

use core::panic;
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
use utils::ttp;

pub async fn main() {
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
        let Some(tree) = parser.parse(text, None) else {
            return None;
        };
        Some(Self {
            parser,
            text: rope,
            tree,
        })
    }
}

struct Backend {
    #[allow(dead_code)]
    client: Client,
    document_map: DashMap<String, Document>,
    class_map: DashMap<String, Class>,
}
impl Backend {
    async fn on_change(&self, uri: String, changes: Vec<TextDocumentContentChangeEvent>) {
        let Some(mut document) = self.document_map.get_mut(&uri) else {
            return;
        };
        let mut text = document.text.clone();
        let ntext = apply_text_changes(changes, &mut text);
        if let Some(n) = ntext {
            text = n;
        }

        let bytes = text.slice(..).as_str().unwrap_or_default().as_bytes();
        // Reusing the previous tree causes issues
        if let Some(ntree) = document.parser.parse(bytes, None) {
            document.tree = ntree;
        } else {
            eprintln!("----- Not updated -----");
        }
        document.text = text;
    }

    async fn on_open(&self, params: TextDocumentItem) {
        let rope = ropey::Rope::from_str(&params.text);
        let key = params.uri.to_string();
        if let Some(mut document) = self.document_map.get_mut(&key) {
            let tree = Some(document.tree.clone());
            document.text = rope;
            if let Some(ntree) = document.parser.parse(params.text, tree.as_ref()) {
                document.tree = ntree;
            } else {
                eprintln!("----- Not updated -----");
            }
        } else {
            self.document_map
                .insert(key, Document::setup_rope(&params.text, rope).unwrap());
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
        self.on_open(TextDocumentItem {
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

    #[allow(dead_code)]
    fn compile(path: &str) -> Vec<Diagnostic> {
        if let Some(classpath) = maven::compile::generate_classpath() {
            if let Some(errors) = maven::compile::compile_java_file(path, &classpath) {
                return errors
                    .into_iter()
                    .map(|e| {
                        let p = Position::new(e.row as u32 - 1, e.col as u32);
                        Diagnostic::new_simple(Range::new(p, p), e.message)
                    })
                    .collect();
            }
        }
        vec![]
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
                //definition_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                        ..CodeActionOptions::default()
                    },
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(
                        [' ', '.', '('].iter().map(|i| i.to_string()).collect(),
                    ),
                    ..CompletionOptions::default()
                }),
                diagnostic_provider: None,
                ..ServerCapabilities::default()
            },
            server_info: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        eprintln!("Init");

        let path = Path::new(".jdk.cfc");
        if path.exists() {
            if let Ok(classes) = parser::loader::load_class_folder("jdk") {
                for class in classes.classes {
                    self.class_map.insert(class.class_path.clone(), class);
                }
            }
        } else {
            // nix run github:nix-community/nix-index#nix-locate -- jmods/java.base.jmod
            // jmod extract openjdk-22.0.2_windows-x64_bin/jdk-22.0.2/jmods/java.base.jmod
            // mvn dependency:unpack
            let classes = parser::loader::load_classes("./jdk/classes/");
            parser::loader::save_class_folder("jdk", &classes).unwrap();
            for class in classes.classes {
                self.class_map.insert(class.class_path.clone(), class);
            }
        }

        let path = Path::new(".maven.cfc");
        if path.exists() {
            if let Ok(classes) = parser::loader::load_class_folder("maven") {
                for class in classes.classes {
                    self.class_map.insert(class.class_path.clone(), class);
                }
            }
        } else {
            let classes = parser::loader::load_classes("./target/dependency/");
            parser::loader::save_class_folder("maven", &classes).unwrap();
            for class in classes.classes {
                self.class_map.insert(class.class_path.clone(), class);
            }
        }

        eprintln!("Init done");
    }

    async fn shutdown(&self) -> Result<()> {
        panic!("Stop");
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_open(TextDocumentItem {
            uri: params.text_document.uri.clone(),
            text: params.text_document.text.clone(),
            version: params.text_document.version,
            language_id: params.text_document.language_id,
        })
        .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.on_change(params.text_document.uri.to_string(), params.content_changes)
            .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        #[allow(unused_mut)]
        let mut path = params.text_document.uri.path();
        // The path on windows should not look like this: /C:/asdas remove the leading slash
        #[cfg(target_os = "windows")]
        {
            path = &path[1..];
        }
        self.client
            .publish_diagnostics(
                params.text_document.uri.clone(),
                Backend::compile(path),
                None,
            )
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let params = params.text_document_position;
        let uri = params.text_document.uri;
        let Some(document) = self.get_document(&uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };
        let mut out = vec![];
        let vars = variable::get_vars(document.value(), &ttp(params.position));

        let imports = imports::imports(document.value());

        out.extend(completion::extend_completion(
            document.value(),
            &ttp(params.position),
            &vars,
            &imports,
            &self.class_map,
        ));

        out.extend(completion::complete_vars(&vars));

        //out.extend(
        //    self.class_map
        //        .iter()
        //        .filter(|i| i.access.contains(&parser::dto::Access::Public))
        //        .map(|v| completion::class_describe(v.value())),
        //);

        Ok(Some(CompletionResponse::Array(out)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let params = params.text_document_position_params;
        let uri = params.text_document.uri;
        let _position = params.position;
        let Some(_document) = self.get_document(&uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };

        Ok(None)
    }
    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let Some(document) = self.get_document(&params.text_document.uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };
        let current_file = params.text_document.uri;
        let point = ttp(params.range.start);
        let bytes = document
            .text
            .slice(..)
            .as_str()
            .unwrap_or_default()
            .as_bytes();

        let imports = imports::imports(document.value());

        if let Some(imps) = codeaction::import_jtype(&document.tree, bytes, point, &imports, &current_file, &self.class_map) {
            return Ok(Some(imps));
        }

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

fn apply_text_changes(
    changes: Vec<TextDocumentContentChangeEvent>,
    text: &mut Rope,
) -> Option<Rope> {
    for change in changes {
        if let Some(range) = change.range {
            let sp = range.start;
            let ep = range.end;

            // Get the start/end char indices of the line.
            let start_idx = text.line_to_char(sp.line.try_into().unwrap())
                + TryInto::<usize>::try_into(sp.character).unwrap();
            let end_idx = text.line_to_char(ep.line.try_into().unwrap())
                + TryInto::<usize>::try_into(ep.character).unwrap();

            text.remove(start_idx..end_idx);

            text.insert(start_idx, &change.text);
            continue;
        }

        if change.range.is_none() && change.range_length.is_none() {
            return Some(Rope::from_str(&change.text));
        }
    }
    None
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
