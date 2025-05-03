mod codeaction;
pub mod completion;
mod definition;
mod hover;
pub mod references;
pub mod signature;

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::{collections::HashMap, fs::read_to_string};

use call_chain::get_call_chain;
use codeaction::CodeActionContext;
use common::project_kind::ProjectKind;
use common::TaskProgress;
use compile::CompileError;
use dashmap::{DashMap, DashSet};
use document::Document;
use hover::{class_action, ClassActionError};
use lsp_types::request::{References, SignatureHelpRequest};
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
    GotoDefinitionResponse, Hover, HoverParams, HoverProviderCapability, InitializeParams, OneOf,
    Position, ProgressParams, ProgressParamsValue, ProgressToken, PublishDiagnosticsParams, Range,
    ServerCapabilities, SignatureHelp, SignatureHelpOptions, SignatureHelpParams,
    TextDocumentContentChangeEvent, TextDocumentItem, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextEdit, Uri, WorkDoneProgressBegin, WorkDoneProgressEnd,
    WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use lsp_types::{
    ClientCapabilities, Location, ReferenceParams, SymbolInformation, WorkDoneProgress,
    WorkDoneProgressReport,
};
use parking_lot::Mutex;
use parser::dto::Class;
use position::PositionSymbol;
use references::{ReferenceUnit, ReferencesContext};

use lsp_server::{Connection, Message, Response};
use tree_sitter_util::lsp::to_treesitter_point;

pub async fn main() -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();
    let project_kind = common::project_kind::get_project_kind();
    eprintln!("Start java_lsp with project_kind: {:?}", project_kind);
    let backend = Backend {
        connection: Arc::new(connection),
        error_files: DashSet::new(),
        project_kind,
        document_map: DashMap::new(),
        class_map: DashMap::new(),
        reference_map: DashMap::new(),
        client_capabilities: Arc::new(None),
    };

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL,
        )),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
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
            ..Default::default()
        }),
        ..ServerCapabilities::default()
    })
    .unwrap_or_default();
    let initialization_params = match backend.connection.initialize(server_capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };
    let params: InitializeParams =
        serde_json::from_value(initialization_params.clone()).unwrap_or_default();
    main_loop(backend, params).await?;
    io_threads.join()?;

    // Shut down gracefully.
    eprintln!("shutting down server");
    Ok(())
}

async fn main_loop(
    mut backend: Backend,
    params: InitializeParams,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    backend.initialized(params.capabilities).await;
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
                    References::METHOD => {
                        if let Ok(params) = serde_json::from_value::<ReferenceParams>(req.params) {
                            let result = backend.referneces(params).await;
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

struct Backend {
    error_files: DashSet<String>,
    project_kind: ProjectKind,
    document_map: DashMap<String, Document>,
    class_map: DashMap<String, Class>,
    reference_map: DashMap<String, Vec<ReferenceUnit>>,
    client_capabilities: Arc<Option<ClientCapabilities>>,
    connection: Arc<Connection>,
}
impl Backend {
    fn on_change(&self, uri: String, changes: Vec<TextDocumentContentChangeEvent>) {
        let Some(mut document) = self.document_map.get_mut(&uri) else {
            return;
        };
        document.apply_text_changes(&changes);
    }

    fn on_open(&self, params: TextDocumentItem) {
        let ipath = params.uri.path().as_str();
        let path = PathBuf::from(ipath);
        let rope = ropey::Rope::from_str(&params.text);
        let key = params.uri.to_string();
        if let Some(mut document) = self.document_map.get_mut(&key) {
            document.replace_text(rope);
        } else {
            match parser::java::load_java(
                params.text.as_bytes(),
                parser::loader::SourceDestination::None,
            ) {
                Ok(class) => {
                    match Document::setup_rope(&params.text, path, rope, class.class_path) {
                        Ok(doc) => {
                            self.document_map.insert(key, doc);
                        }
                        Err(e) => {
                            eprintln!("Failed to setup document: {:?}", e);
                        }
                    }
                }
                Err(e) => eprintln!("Got error parsing document {}, {:?}", ipath, e),
            }
        }
    }

    fn send_dianostic(con: Arc<Connection>, uri: Uri, diagnostics: Vec<Diagnostic>) {
        if let Ok(params) = serde_json::to_value(PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        }) {
            let _ = con
                .sender
                .send(Message::Notification(lsp_server::Notification {
                    method: PublishDiagnostics::METHOD.to_string(),
                    params,
                }));
        }
    }

    fn progress_start(&self, task: String) {
        eprintln!("Start progress on: {}", task);
        if let Ok(params) = serde_json::to_value(ProgressParams {
            token: ProgressToken::String(task.clone()),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(WorkDoneProgressBegin {
                title: task,
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
    fn global_error(_con: Arc<Connection>, message: String) {
        eprintln!("Error: {}", message);
    }
    fn progress_update_persentage_a(
        con: Arc<Connection>,
        task: Arc<String>,
        message: String,
        percentage: Option<u32>,
    ) {
        Backend::progress_update_persentage(con, Arc::unwrap_or_clone(task), message, percentage);
    }

    fn progress_update_persentage(
        con: Arc<Connection>,
        task: String,
        message: String,
        percentage: Option<u32>,
    ) {
        eprintln!(
            "Report progress on: {} {:?} status: {}",
            task, percentage, message
        );
        if let Ok(params) = serde_json::to_value(ProgressParams {
            token: ProgressToken::String(task),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                WorkDoneProgressReport {
                    cancellable: None,
                    message: Some(message),
                    percentage,
                },
            )),
        }) {
            let _ = con
                .sender
                .send(Message::Notification(lsp_server::Notification {
                    method: Progress::METHOD.to_string(),
                    params,
                }));
        }
    }

    fn progress_end(&self, task: String) {
        eprintln!("End progress on: {}", task);
        if let Ok(params) = serde_json::to_value(ProgressParams {
            token: ProgressToken::String(task),
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
                    if let Some(errors) = compile::compile_java_file(path, &classpath) {
                        return errors;
                    }
                }
            }
            ProjectKind::Gradle {
                path_build_gradle: _,
            } => {
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
            if let Ok(uri) = Uri::from_str(&format!("file:///{}", path)) {
                if let Some(errs) = emap.get(path) {
                    let errs: Vec<Diagnostic> = errs
                        .iter()
                        .map(|e| {
                            let p = Position::new(e.row as u32 - 1, e.col as u32);
                            Diagnostic::new_simple(Range::new(p, p), e.message.clone())
                        })
                        .collect();
                    Backend::send_dianostic(self.connection.clone(), uri, errs);
                } else {
                    Backend::send_dianostic(self.connection.clone(), uri, vec![]);
                }
            }
        }
    }

    async fn initialized(&mut self, client_capabilities: ClientCapabilities) {
        self.progress_start("Init".to_string());
        self.client_capabilities = Arc::new(Some(client_capabilities));
        let task = "Load jdk".to_string();

        self.progress_start(task.clone());
        let _ = jdk::load_classes(&self.class_map).await;
        self.progress_end(task.clone());

        if self.project_kind != ProjectKind::Unknown {
            let task = format!("Load {} dependencies", self.project_kind);
            self.progress_start(task.clone());
            let (sender, reciever) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
                persentage: 0,
                error: false,
                message: "...".to_string(),
            });
            let reciever = Arc::new(Mutex::new(reciever));
            let project_kind = self.project_kind.clone();
            let class_map = Arc::new(self.class_map.clone());
            let con = self.connection.clone();
            let atask = Arc::new(task.clone());

            tokio::select! {
                _ = read_forward(reciever, con, atask)  => {},
                cm = fetch_deps(sender, project_kind, class_map) => {
                    if let Some(cm) = cm {
                        for pair in cm.into_iter() {
                            self.class_map.insert(pair.0, pair.1);
                        }
                    }
                }
            }
            self.progress_end(task);
        }

        let task = "Load project files".to_string();
        self.progress_start(task.clone());
        let project_classes = match self.project_kind {
            ProjectKind::Maven => maven::project::load_project_folders(),
            ProjectKind::Gradle {
                path_build_gradle: _,
            } => gradle::project::load_project_folders(),
            ProjectKind::Unknown => vec![],
        };
        Backend::progress_update_persentage(
            self.connection.clone(),
            task.clone(),
            "Initializing reference map".to_string(),
            None,
        );
        match references::init_refernece_map(&project_classes, &self.class_map, &self.reference_map)
            .await
        {
            Ok(_) => (),
            Err(e) => eprintln!("Got reference error: {:?}", e),
        }
        Backend::progress_update_persentage(
            self.connection.clone(),
            task.clone(),
            "Populating class map".to_string(),
            None,
        );
        for class in project_classes {
            self.class_map.insert(class.class_path.clone(), class);
        }
        self.progress_end(task);

        self.progress_end("Init".to_string());
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

        let Some(mut document) = self.document_map.get_mut(params.text_document.uri.as_str())
        else {
            eprintln!("no doc found");
            return;
        };
        match parser::update_project_java_file(PathBuf::from(path.as_str()), document.as_bytes()) {
            Ok(class) => {
                document.class_path = class.class_path.clone();
                let class_path = class.class_path.clone();
                match references::reference_update_class(
                    &class,
                    &self.class_map,
                    &self.reference_map,
                ) {
                    Ok(_) => {}
                    Err(e) => eprintln!("Got reference error: {:?}", e),
                };
                self.class_map.insert(class_path, class);
            }
            Err(e) => eprintln!("Save file parse error {:?}", e),
        }
    }

    async fn hover(&self, params: HoverParams) -> Option<Hover> {
        let uri = params.text_document_position_params.text_document.uri;
        let document = self.document_map.get_mut(uri.as_str())?;
        let point = to_treesitter_point(params.text_document_position_params.position);
        let imports = imports::imports(document.value());
        let vars = variables::get_vars(document.value(), &point);

        match hover::base(document.value(), &point, &vars, &imports, &self.class_map) {
            Ok(hover) => Some(hover),
            Err(e) => {
                eprintln!("Error while hover: {e:?}");
                None
            }
        }
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Option<Vec<TextEdit>> {
        let uri = params.text_document.uri;
        let Some(mut document) = self.document_map.get_mut(uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let lines = document.text.lines().len().try_into().ok()?;
        if let Err(e) = format::format(document.path.clone()) {
            eprintln!("Formatter error: {:?}", e);
            return None;
        }
        if let Err(e) = document.reload_file_from_disk() {
            eprintln!("Formatter unable to reload from disk: {:?}", e);
            return None;
        }

        // self.connection.
        Some(vec![TextEdit::new(
            Range::new(Position::new(0, 0), Position::new(lines, 0)),
            document.str_data.clone(),
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
        let vars = variables::get_vars(document.value(), &point);

        let imports = imports::imports(document.value());
        let Some(class) = &self.class_map.get(&document.class_path) else {
            eprintln!("Could not find class {}", document.class_path);
            return None;
        };

        let mut do_rest = true;
        match completion::complete_call_chain(
            document.value(),
            &point,
            &vars,
            &imports,
            class,
            &self.class_map,
        ) {
            Ok(call_chain) => {
                if !call_chain.is_empty() {
                    do_rest = false;
                }
                out.extend(call_chain);
            }
            Err(e) => {
                eprintln!("Error while completion: {e:?}");
            }
        }

        // If there is any extend completion ignore completing vars
        if do_rest {
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
        let vars = variables::get_vars(document.value(), &point);

        match definition::class(
            document.value(),
            &uri,
            &point,
            &vars,
            &imports,
            &self.class_map,
        ) {
            Ok(definition) => return Some(definition),
            Err(e) => {
                eprintln!("Error while class definition: {e:?}");
            }
        }
        let Some(class) = &self.class_map.get(&document.class_path) else {
            eprintln!("Could not find class {}", document.class_path);
            return None;
        };
        let Some(call_chain) = get_call_chain(&document.tree, document.as_bytes(), &point) else {
            eprintln!("Defintion could not get callchain");
            return None;
        };
        match definition::call_chain_definition(
            uri,
            &point,
            &call_chain,
            &vars,
            &imports,
            class,
            &self.class_map,
        ) {
            Ok(definition) => return Some(definition),
            Err(e) => {
                eprintln!("Error while definition: {e:?}");
            }
        }
        None
    }

    async fn referneces(&self, params: ReferenceParams) -> Option<Vec<Location>> {
        let params = params.text_document_position;
        let uri = params.text_document.uri;
        let Some(document) = self.document_map.get_mut(uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let point = to_treesitter_point(params.position);
        let imports = imports::imports(document.value());
        let vars = variables::get_vars(document.value(), &point);
        match class_action(
            &document.tree,
            document.as_bytes(),
            &point,
            &vars,
            &imports,
            &self.class_map,
        ) {
            Ok((class, _range)) => {
                if let Some(value) =
                    references::class_path(&class.class_path, &self.reference_map, &self.class_map)
                {
                    return Some(value);
                }
            }
            Err(ClassActionError::VariableFound { var: _, range: _ }) => {}
            Err(e) => eprintln!("Got refrence class error: {:?}", e),
        }
        let Some(call_chain) = get_call_chain(&document.tree, document.as_bytes(), &point) else {
            eprintln!("Defintion could not get callchain");
            return None;
        };
        let Some(class) = &self.class_map.get(&document.class_path) else {
            eprintln!("Could not find class {}", document.class_path);
            return None;
        };
        let context = ReferencesContext {
            point: &point,
            imports: &imports,
            class_map: &self.class_map,
            class,
            vars: &vars,
        };

        match references::call_chain_references(
            &call_chain,
            &context,
            &self.reference_map,
            &self.document_map,
        ) {
            Ok(refs) => Some(refs),
            Err(e) => {
                eprintln!("Got refrence call_chain error: {:?}", e);
                None
            }
        }
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

        let Some(class) = &self.class_map.get(&document.class_path) else {
            eprintln!("Could not find class {}", document.class_path);
            return None;
        };
        let vars = variables::get_vars(document.value(), &point);

        let context = CodeActionContext {
            point: &point,
            imports: &imports,
            class_map: &self.class_map,
            class,
            vars: &vars,
            current_file: &current_file,
        };
        if let Some(imps) = codeaction::import_jtype(&document.tree, bytes, &context) {
            return Some(imps);
        }

        match codeaction::replace_with_value_type(&document.tree, bytes, &context) {
            Ok(None) => (),
            Ok(Some(e)) => return Some(vec![e]),
            Err(e) => {
                eprintln!("Got error code_action replace with value: {:?}", e);
            }
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

        let symbols = position::get_symbols(document.as_bytes());
        match symbols {
            Ok(symbols) => {
                let symbols = position::symbols_to_document_symbols(symbols, uri);
                Some(DocumentSymbolResponse::Flat(symbols))
            }
            Err(e) => {
                eprintln!("Error while document symbol: {e:?}");
                None
            }
        }
    }

    async fn workspace_document_symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Option<WorkspaceSymbolResponse> {
        let files = match self.project_kind {
            ProjectKind::Maven => maven::project::get_paths(),
            ProjectKind::Gradle {
                path_build_gradle: _,
            } => gradle::project::get_paths(),
            ProjectKind::Unknown => vec![],
        };

        let symbols: Vec<SymbolInformation> = files
            .into_iter()
            .filter_map(|i| Some((i.clone(), read_to_string(i).ok()?)))
            .map(|(path, src)| (path, position::get_symbols(src.as_bytes())))
            .map(|(path, symbols)| {
                (
                    path,
                    match symbols {
                        Ok(s) => s
                            .into_iter()
                            .filter(|i| match i {
                                PositionSymbol::Range(_range) => false,
                                PositionSymbol::Symbol {
                                    range: _,
                                    name,
                                    kind: _,
                                } => name.contains(&params.query),
                            })
                            .collect(),
                        Err(e) => {
                            eprintln!("Errors with workspace document symbol: {:?}", e);
                            vec![]
                        }
                    },
                )
            })
            .filter_map(|(path, symbols)| {
                let uri = Uri::from_str(&format!("file:///{}", path)).ok()?;
                Some(position::symbols_to_document_symbols(symbols, uri))
            })
            .flatten()
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
        let Some(class) = &self.class_map.get(&document.class_path) else {
            eprintln!("Could not find class {}", document.class_path);
            return None;
        };

        match signature::signature_driver(&document, &point, class, &self.class_map) {
            Ok(hover) => Some(hover),
            Err(e) => {
                eprintln!("Error while hover: {e:?}");
                None
            }
        }
    }
}

async fn read_forward(
    rx: Arc<Mutex<tokio::sync::watch::Receiver<TaskProgress>>>,
    con: Arc<Connection>,
    task: Arc<String>,
) {
    let con = con.clone();
    let task = task.clone();
    tokio::spawn(async move {
        let ex = &mut rx.lock();

        loop {
            let i = ex.borrow_and_update();
            if !i.has_changed() {
                continue;
            }
            let task = task.clone();
            let con = con.clone();
            match i.error {
                true => Backend::global_error(con, i.message.clone()),
                false => Backend::progress_update_persentage_a(
                    con,
                    task,
                    i.message.clone(),
                    Some(i.persentage.try_into().unwrap()),
                ),
            };
        }
    })
    .await
    .expect("asdf")
}

async fn fetch_deps(
    sender: tokio::sync::watch::Sender<TaskProgress>,
    project_kind: ProjectKind,
    class_map: Arc<dashmap::DashMap<std::string::String, parser::dto::Class>>,
) -> Option<DashMap<String, Class>> {
    tokio::spawn(async move {
        match project_kind {
            ProjectKind::Maven => match maven::fetch::fetch_deps(&class_map, sender).await {
                Ok(o) => Some(o),
                Err(e) => {
                    eprintln!("Got error while loading maven project: {e:?}");
                    None
                }
            },
            ProjectKind::Gradle {
                path_build_gradle: path,
            } => match gradle::fetch::fetch_deps(&class_map, path, sender).await {
                Ok(o) => Some(o),
                Err(e) => {
                    eprintln!("Got error while loading gradle project: {e:?}");
                    None
                }
            },
            ProjectKind::Unknown => None,
        }
    })
    .await
    .expect("asdf")
}
