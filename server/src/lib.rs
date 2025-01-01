mod call_chain;
mod codeaction;
pub mod completion;
mod definition;
mod hover;
mod imports;
mod position;
mod tyres;
mod utils;
mod variable;

use std::collections::HashMap;
use std::str::FromStr;

use common::compile::CompileError;
use common::project_kind::ProjectKind;
use dashmap::{DashMap, DashSet};
use notification::Progress;
use parser::dto::Class;
use ropey::Rope;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tree_sitter::{Parser, Point, Tree};
use utils::to_treesitter_point;

pub async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| {
        let project_kind = common::project_kind::get_project_kind();
        eprintln!("Start java_lsp with project_kind: {:?}", project_kind);
        Backend {
            client,
            error_files: DashSet::new(),
            project_kind,
            document_map: DashMap::new(),
            class_map: DashMap::new(),
        }
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
        let tree = parser.parse(text, None)?;
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
    error_files: DashSet<String>,
    project_kind: ProjectKind,
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

    fn compile(&self, path: &str) -> Vec<CompileError> {
        match self.project_kind {
            ProjectKind::Maven => {
                if let Some(classpath) = maven::compile::generate_classpath() {
                    if let Some(errors) = common::compile::compile_java_file(path, &classpath) {
                        return errors;
                    }
                }
            }
            ProjectKind::Gradle => {
                if let Some(errors) = gradle::compile::compile_java() {
                    return errors;
                }
            }
            ProjectKind::Unknown => eprintln!("Could not find project kind maven or gradle"),
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
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                definition_provider: Some(OneOf::Left(true)),
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
                document_formatting_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..ServerCapabilities::default()
            },
            server_info: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        eprintln!("Init");

        self.client
            .send_notification::<Progress>(ProgressParams {
                token: ProgressToken::String("Load jdk".to_string()),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                    WorkDoneProgressBegin {
                        title: "Loading jdk".to_string(),
                        cancellable: None,
                        message: None,
                        percentage: None,
                    },
                )),
            })
            .await;

        common::jdk::load_classes(&self.class_map);

        self.client
            .send_notification::<Progress>(ProgressParams {
                token: ProgressToken::String("Load jdk".to_string()),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message: None,
                })),
            })
            .await;

        if self.project_kind != ProjectKind::Unknown {
            self.client
                .send_notification::<Progress>(ProgressParams {
                    token: ProgressToken::String(format!(
                        "Load {} dependencies",
                        self.project_kind
                    )),
                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                        WorkDoneProgressBegin {
                            title: format!("Load {} dependencies", self.project_kind),
                            cancellable: None,
                            message: None,
                            percentage: None,
                        },
                    )),
                })
                .await;
            let cm = match self.project_kind {
                ProjectKind::Maven => maven::fetch::fetch_deps(&self.class_map).await,
                ProjectKind::Gradle => gradle::fetch::fetch_deps(&self.class_map).await,
                ProjectKind::Unknown => self.class_map.clone(),
            };
            for pair in cm.into_iter() {
                self.class_map.insert(pair.0, pair.1);
            }

            self.client
                .send_notification::<Progress>(ProgressParams {
                    token: ProgressToken::String(format!(
                        "Load {} dependencies",
                        self.project_kind
                    )),
                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                        WorkDoneProgressEnd { message: None },
                    )),
                })
                .await;
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
        let errors = self.compile(path);
        let mut emap = HashMap::<String, Vec<CompileError>>::new();
        for e in errors {
            self.error_files.insert(e.path.clone());
            if let Some(list) = emap.get_mut(&e.path) {
                list.push(e);
            } else {
                emap.insert(e.path.to_owned(), vec![e]);
            }
        }

        for path in self.error_files.iter() {
            let Some(path) = path.get(..) else {
                continue;
            };
            if let Ok(uri) = Url::parse(&format!("file:/{}", path)) {
                if let Some(errs) = emap.get(path) {
                    let errs: Vec<Diagnostic> = errs
                        .into_iter()
                        .map(|e| {
                            let p = Position::new(e.row as u32 - 1, e.col as u32);
                            Diagnostic::new_simple(Range::new(p, p), e.message.clone())
                        })
                        .collect();
                    self.client.publish_diagnostics(uri, errs, None).await
                } else {
                    self.client.publish_diagnostics(uri, vec![], None).await
                }
            }
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(document) = self.get_document(&uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };
        let point = to_treesitter_point(params.text_document_position_params.position);
        let imports = imports::imports(document.value());

        let class_hover = hover::class(document.value(), &point, &imports, &self.class_map);
        return Ok(class_hover);
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let Some(document) = self.get_document(&uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };
        let text = document.text.to_string();
        let Some(lines) = document.text.lines().len().try_into().ok() else {
            return Ok(None);
        };
        let Some(text) = format::format(text, format::Formatter::Topiary) else {
            return Ok(None);
        };
        Ok(Some(vec![TextEdit::new(
            Range::new(Position::new(0, 0), Position::new(lines, 0)),
            text,
        )]))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let params = params.text_document_position;
        let uri = params.text_document.uri;
        let Some(document) = self.get_document(&uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };
        let mut out = vec![];
        let point = to_treesitter_point(params.position);
        let vars = variable::get_vars(document.value(), &point);

        let imports = imports::imports(document.value());

        let call_chain = completion::complete_call_chain(
            document.value(),
            &point,
            &vars,
            &imports,
            &self.class_map,
        );

        // If there is any extend completion ignore completing vars
        if call_chain.is_empty() {
            out.extend(completion::complete_vars(&vars));
        }

        out.extend(call_chain);

        // TODO: Sort classes by name
        out.extend(completion::classes(
            document.value(),
            &point,
            &imports,
            &self.class_map,
        ));

        Ok(Some(CompletionResponse::Array(out)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let params = params.text_document_position_params;
        let uri = params.text_document.uri;
        let Some(document) = self.get_document(&uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };

        let point = to_treesitter_point(params.position);
        let imports = imports::imports(document.value());

        if let Some(c) = definition::class(document.value(), &point, &imports, &self.class_map) {
            return Ok(Some(c));
        }
        Ok(None)
    }
    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let Some(document) = self.get_document(&params.text_document.uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };
        let current_file = params.text_document.uri;
        let point = to_treesitter_point(params.range.start);
        let bytes = document
            .text
            .slice(..)
            .as_str()
            .unwrap_or_default()
            .as_bytes();

        let imports = imports::imports(document.value());

        if let Some(imps) = codeaction::import_jtype(
            &document.tree,
            bytes,
            point,
            &imports,
            &current_file,
            &self.class_map,
        ) {
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
    Some(text.clone())
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
