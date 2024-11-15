pub mod completion;
mod imports;
mod utils;

use core::panic;
use std::path::Path;
use std::str::FromStr;

use dashmap::DashMap;
use parser::dto::{Class, SourceKind};
use ropey::Rope;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tree_sitter::{Parser, Point, Tree};
use tree_sitter_util::get_node_at_point;
use utils::ttp;

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

impl Document {
    pub fn setup(text: &str) -> Option<Self> {
        let rope = ropey::Rope::from_str(text);
        Self::setup_rope(text, rope)
    }

    pub fn setup_rope(text: &str, rope: Rope) -> Option<Self> {
        let mut parser = Parser::new();
        if parser.set_language(&tree_sitter_java::language()).is_err() {
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
    async fn on_change(&self, params: TextDocumentItem) {
        let rope = ropey::Rope::from_str(&params.text);
        let key = params.uri.to_string();
        if let Some(mut document) = self.document_map.get_mut(&key) {
            let tree = Some(document.tree.clone());
            if let Some(ntree) = document.parser.parse(params.text, tree.as_ref()) {
                document.tree = ntree;
            } else {
                eprintln!("----- Not updated -----");
            }
            document.text = rope;
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

    /// cpl -> class path list
    //fn load_classes(&self, cpl: Vec<&str>) {
    //    let new_classes: Vec<_> = cpl
    //        .iter()
    //        .filter(|cp| !self.class_map.contains_key(**cp))
    //        .filter_map(|p| {
    //            let jdk = format!("./jdk/classes/{}.class", p.replace('.', "/"));
    //            if Path::new(&jdk).exists() {
    //                return match parser::loader::load_class_fs(Path::new(&jdk), p.to_string()) {
    //                    Ok(class) => Some((jdk, class)),
    //                    Err(_) => None,
    //                };
    //            }
    //            let mvn = format!("./target/dependency/{}.class", p.replace('.', "/"));
    //            if Path::new(&mvn).exists() {
    //                return match parser::loader::load_class_fs(Path::new(&mvn), p.to_string()) {
    //                    Ok(class) => Some((mvn, class)),
    //                    Err(_) => None,
    //                };
    //            };
    //
    //            None
    //        })
    //        //.filter_map(|p| match parser::load_class_fs(Path::new(&p)) {
    //        //    Ok(class) => Some((p, class)),
    //        //    Err(_) => None,
    //        //})
    //        .collect();
    //
    //    for (path, class) in new_classes {
    //        self.class_map.insert(path, class);
    //    }
    //}

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
                //code_action_provider: Some(CodeActionProviderCapability::Options(
                //    CodeActionOptions {
                //        code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                //        ..CodeActionOptions::default()
                //    },
                //)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(
                        [' ', '.', '('].iter().map(|i| i.to_string()).collect(),
                    ),
                    ..CompletionOptions::default()
                }),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: None,
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: None,
                        },
                    },
                )),
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
    }

    async fn shutdown(&self) -> Result<()> {
        panic!("Stop");
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri.clone(),
            text: params.text_document.text.clone(),
            version: params.text_document.version,
            language_id: params.text_document.language_id,
        })
        .await;
        //self.client
        //    .publish_diagnostics(
        //        params.text_document.uri.clone(),
        //        Backend::compile(&params.text_document.uri.path()),
        //        Some(params.text_document.version),
        //    )
        //    .await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri.clone(),
            text: std::mem::take(&mut params.content_changes[0].text),
            version: params.text_document.version,
            language_id: "".to_owned(),
        })
        .await;
        //self.client
        //    .publish_diagnostics(
        //        params.text_document.uri.clone(),
        //        Backend::compile(&params.text_document.uri.path()),
        //        Some(params.text_document.version),
        //    )
        //    .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let params = params.text_document_position;
        let uri = params.text_document.uri;
        let Some(document) = self.get_document(&uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };
        let position = params.position;
        let tree = &document.tree;
        let mut out = vec![];

        if let Ok(node) = get_node_at_point(&tree, ttp(position)) {
            let _text = node
                .utf8_text(document.text.to_string().as_bytes())
                .unwrap();
            match node.kind() {
                "type_identifier" => {},
                "identifier" => {
                    out.extend(
                        self.class_map
                            .iter()
                            .filter(|v| {
                                let val = v.value();
                                if val.access.contains(&dto::Access::Private) {
                                    return false;
                                }
                                if val.name.contains("$") {
                                    return false;
                                }
                                true
                            })
                            .take(50)
                            .map(|v| {
                                let val = v.value();
                                CompletionItem::new_simple(val.name.clone(), val.class_path.clone())
                            }),
                    );
                }
                _ => {
                    out.push(CompletionItem::new_simple(
                        "node".to_string(),
                        format!("[{}]{}", node.kind(), text),
                    ));
                }
            }
        }

        if true {
            //dbg!(completion::class_variables(&point, document));
        }

        if false {
            out.extend(self.class_map.iter().map(|v| completion::class(v.value())));
        }

        //out.extend(self.class_map.iter().map(|v| {
        //    let val = v.value();
        //    let methods: Vec<_> = val
        //        .methods
        //        .iter()
        //        .map(|m| {
        //            format!(
        //                "{}({:?})",
        //                m.name,
        //                m.methods
        //                    .iter()
        //                    .map(|p| p.jtype.clone())
        //                    .collect::<Vec<_>>()
        //            )
        //        })
        //        .collect();
        //    CompletionItem::new_simple(val.name.to_string(), methods.join("\n"))
        //}));

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
