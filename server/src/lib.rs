mod codeaction;
pub mod completion;
mod definition;
pub mod document;
mod hover;
mod imports;
mod position;
pub mod signature;
mod tyres;
mod utils;
mod variable;

use std::path::PathBuf;
use std::str::FromStr;
use std::{collections::HashMap, fs::read_to_string};

use common::compile::CompileError;
use common::project_kind::ProjectKind;
use dashmap::{DashMap, DashSet};
use document::Document;
use lsp_types::request::SignatureHelpRequest;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidOpenTextDocument, DidSaveTextDocument, Notification, Progress,
        PublishDiagnostics,
    },
    request::{
        CodeActionRequest, Completion, DocumentSymbolRequest, Formatting, GotoDefinition,
        HoverRequest, Request, WorkspaceSymbolRequest,
    },
    CodeActionKind, CodeActionOptions, CodeActionParams, CodeActionProviderCapability,
    CodeActionResponse, CompletionOptions, CompletionParams, CompletionResponse, Diagnostic,
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    DocumentFormattingParams, DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverParams, HoverProviderCapability, InitializeParams,
    InitializedParams, OneOf, Position, ProgressParams, ProgressParamsValue, ProgressToken,
    PublishDiagnosticsParams, Range, ServerCapabilities, SignatureHelp, SignatureHelpOptions,
    SignatureHelpParams, TextDocumentContentChangeEvent, TextDocumentItem,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Uri, WorkDoneProgress,
    WorkDoneProgressBegin, WorkDoneProgressEnd, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use lsp_types::{SymbolInformation, WorkDoneProgressOptions};
use parser::dto::Class;
use utils::to_treesitter_point;

use lsp_server::{Connection, Message, Response};

pub async fn main() -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();
    let project_kind = common::project_kind::get_project_kind();
    eprintln!("Start java_lsp with project_kind: {:?}", project_kind);
    let backend = Backend {
        connection: &connection,
        error_files: DashSet::new(),
        project_kind,
        document_map: DashMap::new(),
        class_map: DashMap::new(),
    };

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL,
        )),
        definition_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
            ..CodeActionOptions::default()
        })),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some([' ', '.', '('].iter().map(|i| i.to_string()).collect()),
            ..CompletionOptions::default()
        }),
        document_symbol_provider: Some(OneOf::Left(true)),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".to_owned(), ",".to_owned(), "<".to_owned()]),
            retrigger_characters: None,
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
        }),
        ..ServerCapabilities::default()
    })
    .unwrap_or_default();
    let initialization_params = match connection.initialize(server_capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };
    main_loop(backend, initialization_params).await?;
    io_threads.join()?;

    // Shut down gracefully.
    eprintln!("shutting down server");
    Ok(())
}

async fn main_loop(
    backend: Backend<'_>,
    params: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap_or_default();
    backend.initialized(InitializedParams {}).await;
    for msg in &backend.connection.receiver {
        match msg {
            Message::Request(req) => {
                if backend.connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                match req.method.as_str() {
                    HoverRequest::METHOD => {
                        if let Ok(params) = serde_json::from_value::<HoverParams>(req.params) {
                            let result = backend.hover(params).await;
                            let _ = backend.connection.sender.send(Message::Response(Response {
                                id: req.id,
                                result: serde_json::to_value(result).ok(),
                                error: None,
                            }));
                        }
                    }
                    Formatting::METHOD => {
                        if let Ok(params) =
                            serde_json::from_value::<DocumentFormattingParams>(req.params)
                        {
                            let result = backend.formatting(params).await;
                            let _ = backend.connection.sender.send(Message::Response(Response {
                                id: req.id,
                                result: serde_json::to_value(result).ok(),
                                error: None,
                            }));
                        }
                    }
                    GotoDefinition::METHOD => {
                        if let Ok(params) =
                            serde_json::from_value::<GotoDefinitionParams>(req.params)
                        {
                            let result = backend.goto_definition(params).await;
                            let _ = backend.connection.sender.send(Message::Response(Response {
                                id: req.id,
                                result: serde_json::to_value(result).ok(),
                                error: None,
                            }));
                        }
                    }
                    Completion::METHOD => {
                        if let Ok(params) = serde_json::from_value::<CompletionParams>(req.params) {
                            let result = backend.completion(params).await;
                            let _ = backend.connection.sender.send(Message::Response(Response {
                                id: req.id,
                                result: serde_json::to_value(result).ok(),
                                error: None,
                            }));
                        }
                    }
                    CodeActionRequest::METHOD => {
                        if let Ok(params) = serde_json::from_value::<CodeActionParams>(req.params) {
                            let result = backend.code_action(params).await;
                            let _ = backend.connection.sender.send(Message::Response(Response {
                                id: req.id,
                                result: serde_json::to_value(result).ok(),
                                error: None,
                            }));
                        }
                    }
                    DocumentSymbolRequest::METHOD => {
                        if let Ok(params) =
                            serde_json::from_value::<DocumentSymbolParams>(req.params)
                        {
                            let result = backend.document_symbol(params).await;
                            let _ = backend.connection.sender.send(Message::Response(Response {
                                id: req.id,
                                result: serde_json::to_value(result).ok(),
                                error: None,
                            }));
                        }
                    }
                    WorkspaceSymbolRequest::METHOD => {
                        if let Ok(params) =
                            serde_json::from_value::<WorkspaceSymbolParams>(req.params)
                        {
                            let result = backend.workspace_document_symbol(params).await;
                            let _ = backend.connection.sender.send(Message::Response(Response {
                                id: req.id,
                                result: serde_json::to_value(result).ok(),
                                error: None,
                            }));
                        }
                    }
                    SignatureHelpRequest::METHOD => {
                        if let Ok(params) =
                            serde_json::from_value::<SignatureHelpParams>(req.params)
                        {
                            let result = backend.signature_help(params).await;
                            let _ = backend.connection.sender.send(Message::Response(Response {
                                id: req.id,
                                result: serde_json::to_value(result).ok(),
                                error: None,
                            }));
                        }
                    }
                    _ => {}
                }
            }
            Message::Response(resp) => {
                eprintln!("got response: {resp:?}");
            }
            Message::Notification(not) => {
                match not.method.as_str() {
                    DidOpenTextDocument::METHOD => {
                        if let Ok(params) =
                            serde_json::from_value::<DidOpenTextDocumentParams>(not.params)
                        {
                            backend.did_open(params);
                        }
                    }
                    DidChangeTextDocument::METHOD => {
                        if let Ok(params) =
                            serde_json::from_value::<DidChangeTextDocumentParams>(not.params)
                        {
                            backend.did_change(params);
                        }
                    }
                    DidSaveTextDocument::METHOD => {
                        if let Ok(params) =
                            serde_json::from_value::<DidSaveTextDocumentParams>(not.params)
                        {
                            backend.did_save(params).await;
                        }
                    }
                    _ => {}
                };
            }
        }
    }
    Ok(())
}

struct Backend<'a> {
    error_files: DashSet<String>,
    project_kind: ProjectKind,
    document_map: DashMap<String, Document>,
    class_map: DashMap<String, Class>,
    connection: &'a Connection,
}
impl Backend<'_> {
    fn on_change(&self, uri: String, changes: Vec<TextDocumentContentChangeEvent>) {
        let Some(mut document) = self.document_map.get_mut(&uri) else {
            return;
        };
        document.apply_text_changes(&changes);
    }

    fn on_open(&self, params: TextDocumentItem) {
        let path = params.uri.path().as_str();
        let path = PathBuf::from(path);
        let rope = ropey::Rope::from_str(&params.text);
        let key = params.uri.to_string();
        if let Some(mut document) = self.document_map.get_mut(&key) {
            document.replace_text(rope);
        } else {
            if let Some(doc) = Document::setup_rope(&params.text, path, rope) {
                self.document_map.insert(key, doc);
            }
        }
    }

    fn send_dianostic(&self, uri: Uri, diagnostics: Vec<Diagnostic>) {
        if let Ok(params) = serde_json::to_value(PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        }) {
            let _ = self
                .connection
                .sender
                .send(Message::Notification(lsp_server::Notification {
                    method: PublishDiagnostics::METHOD.to_string(),
                    params,
                }));
        }
    }

    fn progress_start(&self, task: &str) {
        eprintln!("Start progress on: {}", task);
        if let Ok(params) = serde_json::to_value(ProgressParams {
            token: ProgressToken::String(task.to_string()),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(WorkDoneProgressBegin {
                title: task.to_string(),
                cancellable: None,
                message: None,
                percentage: None,
            })),
        }) {
            let _ = self
                .connection
                .sender
                .send(Message::Notification(lsp_server::Notification {
                    method: Progress::METHOD.to_string(),
                    params,
                }));
        }
    }

    fn progress_end(&self, task: &str) {
        eprintln!("End progress on: {}", task);
        if let Ok(params) = serde_json::to_value(ProgressParams {
            token: ProgressToken::String(task.to_string()),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                message: None,
            })),
        }) {
            let _ = self
                .connection
                .sender
                .send(Message::Notification(lsp_server::Notification {
                    method: Progress::METHOD.to_string(),
                    params,
                }));
        }
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
    async fn publish_compile_errors(&self, errors: Vec<CompileError>) {
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
            if let Ok(uri) = Uri::from_str(&format!("file://{}", path)) {
                if let Some(errs) = emap.get(path) {
                    let errs: Vec<Diagnostic> = errs
                        .iter()
                        .map(|e| {
                            let p = Position::new(e.row as u32 - 1, e.col as u32);
                            Diagnostic::new_simple(Range::new(p, p), e.message.clone())
                        })
                        .collect();
                    self.send_dianostic(uri, errs);
                } else {
                    self.send_dianostic(uri, vec![]);
                }
            }
        }
    }

    async fn initialized(&self, _: InitializedParams) {
        eprintln!("Init");

        self.progress_start("Load jdk");
        common::jdk::load_classes(&self.class_map);
        self.progress_end("Load jdk");

        if self.project_kind != ProjectKind::Unknown {
            let prog_lable = format!("Load {} dependencies", self.project_kind);
            self.progress_start(&prog_lable);
            let cm = match self.project_kind {
                ProjectKind::Maven => maven::fetch::fetch_deps(&self.class_map).await,
                ProjectKind::Gradle => gradle::fetch::fetch_deps(&self.class_map).await,
                ProjectKind::Unknown => None,
            };
            if let Some(cm) = cm {
                for pair in cm.into_iter() {
                    self.class_map.insert(pair.0, pair.1);
                }
            }

            self.progress_end(&prog_lable);
        }

        self.progress_start("Load project files");
        let project_classes = match self.project_kind {
            ProjectKind::Maven => maven::project::load_project_folders(),
            ProjectKind::Gradle => gradle::project::load_project_folders(),
            ProjectKind::Unknown => vec![],
        };
        for class in project_classes {
            eprintln!("Found local class: {}", class.source);
            self.class_map.insert(class.class_path.clone(), class);
        }
        self.progress_end("Load project files");

        eprintln!("Init done");
    }

    fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_open(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
            language_id: params.text_document.language_id,
        });
    }

    fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.on_change(params.text_document.uri.to_string(), params.content_changes);
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        #[allow(unused_mut)]
        let mut path = params.text_document.uri.path();
        // The path on windows should not look like this: /C:/asdas remove the leading slash

        let errors = self.compile(path.as_str());
        self.publish_compile_errors(errors).await;

        let Some(document) = self.document_map.get_mut(params.text_document.uri.as_str()) else {
            eprintln!("no doc found");
            return;
        };
        if let Some(class) =
            parser::update_project_java_file(PathBuf::from(path.as_str()), document.as_bytes())
        {
            self.class_map.insert(class.class_path.clone(), class);
        }
    }

    async fn hover(&self, params: HoverParams) -> Option<Hover> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(document) = self.document_map.get_mut(uri.as_str()) else {
            return None;
        };
        let point = to_treesitter_point(params.text_document_position_params.position);
        let imports = imports::imports(document.value());
        let vars = variable::get_vars(document.value(), &point);

        hover::base(document.value(), &point, &vars, &imports, &self.class_map)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Option<Vec<TextEdit>> {
        let uri = params.text_document.uri;
        let Some(document) = self.document_map.get_mut(uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let Some(lines) = document.text.lines().len().try_into().ok() else {
            return None;
        };
        let Some(text) = format::format(document.text.to_string(), document.path.clone()) else {
            return None;
        };

        // self.connection.
        Some(vec![TextEdit::new(
            Range::new(Position::new(0, 0), Position::new(lines, 0)),
            text,
        )])
    }

    async fn completion(&self, params: CompletionParams) -> Option<CompletionResponse> {
        let params = params.text_document_position;
        let uri = params.text_document.uri;
        let Some(document) = self.document_map.get_mut(uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
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
            eprintln!("Call chain is emtpy");
            out.extend(completion::static_methods(&imports, &self.class_map));
            out.extend(completion::complete_vars(&vars));
            out.extend(completion::classes(
                document.value(),
                &point,
                &imports,
                &self.class_map,
            ));
        }

        out.extend(call_chain);

        Some(CompletionResponse::Array(out))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Option<GotoDefinitionResponse> {
        let params = params.text_document_position_params;
        let uri = params.text_document.uri;
        let Some(document) = self.document_map.get_mut(uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
        };

        let point = to_treesitter_point(params.position);
        let imports = imports::imports(document.value());

        if let Some(c) = definition::class(document.value(), uri, &point, &imports, &self.class_map)
        {
            return Some(c);
        }
        None
    }
    async fn code_action(&self, params: CodeActionParams) -> Option<CodeActionResponse> {
        let Some(document) = self.document_map.get_mut(params.text_document.uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let current_file = params.text_document.uri;
        let point = to_treesitter_point(params.range.start);
        let bytes = document.as_bytes();

        let imports = imports::imports(document.value());

        if let Some(imps) = codeaction::import_jtype(
            &document.tree,
            bytes,
            point,
            &imports,
            &current_file,
            &self.class_map,
        ) {
            return Some(imps);
        }

        None
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Option<DocumentSymbolResponse> {
        let Some(document) = self.document_map.get_mut(params.text_document.uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let uri = params.text_document.uri;

        let symbols = position::get_symbols(document.as_str());
        let symbols = position::symbols_to_document_symbols(symbols, uri);
        Some(DocumentSymbolResponse::Flat(symbols))
    }

    async fn workspace_document_symbol(
        &self,
        _params: WorkspaceSymbolParams,
    ) -> Option<WorkspaceSymbolResponse> {
        let files = match self.project_kind {
            ProjectKind::Maven => maven::project::get_paths(),
            ProjectKind::Gradle => gradle::project::get_paths(),
            ProjectKind::Unknown => vec![],
        };

        let symbols: Vec<SymbolInformation> = files
            .into_iter()
            .filter_map(|i| Some((i.clone(), read_to_string(i).ok()?)))
            .map(|(path, src)| (path, position::get_symbols(src.as_str())))
            .filter_map(|(path, symbols)| {
                let uri = Uri::from_str(&format!("file://{}", path)).ok()?;
                Some(position::symbols_to_document_symbols(symbols, uri))
            })
            .flat_map(|i| i)
            .collect();
        Some(WorkspaceSymbolResponse::Flat(symbols))
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Option<SignatureHelp> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(document) = self.document_map.get_mut(uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let point = to_treesitter_point(params.text_document_position_params.position);

        signature::signature_driver(&document, &point, &self.class_map)
    }
}
