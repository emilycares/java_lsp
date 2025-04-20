mod codeaction;
pub mod completion;
mod definition;
pub mod document;
mod hover;
mod imports;
mod position;
pub mod references;
pub mod signature;
mod tyres;
mod utils;
mod variable;

use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use std::{collections::HashMap, fs::read_to_string};

use common::compile::CompileError;
use common::project_kind::ProjectKind;
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
    TextDocumentSyncKind, TextEdit, Uri, WorkDoneProgress, WorkDoneProgressBegin,
    WorkDoneProgressEnd, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use lsp_types::{
    ClientCapabilities, Location, ReferenceParams, SymbolInformation, WorkDoneProgressOptions,
    WorkDoneProgressReport,
};
use parser::call_chain::get_call_chain;
use parser::dto::Class;
use position::PositionSymbol;
use references::ReferenceUnit;
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
        reference_map: DashMap::new(),
        client_capabilities: Rc::new(None),
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
    mut backend: Backend<'_>,
    params: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let params: InitializeParams = serde_json::from_value(params).unwrap_or_default();
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

struct Backend<'a> {
    error_files: DashSet<String>,
    project_kind: ProjectKind,
    document_map: DashMap<String, Document>,
    class_map: DashMap<String, Class>,
    reference_map: DashMap<String, Vec<ReferenceUnit>>,
    client_capabilities: Rc<Option<ClientCapabilities>>,
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
        let ipath = params.uri.path().as_str();
        let path = PathBuf::from(ipath);
        let rope = ropey::Rope::from_str(&params.text);
        let key = params.uri.to_string();
        if let Some(mut document) = self.document_map.get_mut(&key) {
            document.replace_text(rope);
        } else {
            match parser::java::load_java(
                &params.text.as_bytes(),
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

    fn progress_update(&self, task: &str, message: &str) {
        eprintln!("Report progress on: {} status: {}", task, message);
        if let Ok(params) = serde_json::to_value(ProgressParams {
            token: ProgressToken::String(task.to_string()),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                WorkDoneProgressReport {
                    cancellable: None,
                    message: Some(message.to_string()),
                    percentage: None,
                },
            )),
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
            if let Ok(uri) = Uri::from_str(&format!("file:///{}", path)) {
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

    async fn initialized(&mut self, client_capabilities: ClientCapabilities) {
        eprintln!("Init");
        self.client_capabilities = Rc::new(Some(client_capabilities));
        let task = "Load jdk";

        self.progress_start(task);
        let _ = common::jdk::load_classes(&self.class_map).await;
        self.progress_end(task);

        if self.project_kind != ProjectKind::Unknown {
            let prog_lable = format!("Load {} dependencies", self.project_kind);
            self.progress_start(&prog_lable);
            let cm = match self.project_kind {
                ProjectKind::Maven => match maven::fetch::fetch_deps(&self.class_map).await {
                    Ok(o) => Some(o),
                    Err(maven::fetch::MavenFetchError::NoWorkToDo) => None,
                    Err(e) => {
                        eprintln!("Got error while loading maven project: {e:?}");
                        None
                    }
                },
                ProjectKind::Gradle => match gradle::fetch::fetch_deps(&self.class_map).await {
                    Ok(o) => Some(o),
                    Err(gradle::fetch::GradleFetchError::NoWorkToDo) => None,
                    Err(e) => {
                        eprintln!("Got error while loading gradle project: {e:?}");
                        None
                    }
                },
                ProjectKind::Unknown => None,
            };
            if let Some(cm) = cm {
                for pair in cm.into_iter() {
                    self.class_map.insert(pair.0, pair.1);
                }
            }

            self.progress_end(&prog_lable);
        }

        let task = "Load project files";
        self.progress_start(task);
        let project_classes = match self.project_kind {
            ProjectKind::Maven => maven::project::load_project_folders(),
            ProjectKind::Gradle => gradle::project::load_project_folders(),
            ProjectKind::Unknown => vec![],
        };
        self.progress_update(task, "Initializing reference map");
        match references::init_refernece_map(&project_classes, &self.class_map, &self.reference_map)
        {
            Ok(_) => (),
            Err(e) => eprintln!("Got reference error: {:?}", e),
        }
        self.progress_update(task, "Populating class map");
        for class in project_classes {
            self.class_map.insert(class.class_path.clone(), class);
        }
        self.progress_end(task);

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
        let Some(document) = self.document_map.get_mut(uri.as_str()) else {
            return None;
        };
        let point = to_treesitter_point(params.text_document_position_params.position);
        let imports = imports::imports(document.value());
        let vars = variable::get_vars(document.value(), &point);

        match hover::base(document.value(), &point, &vars, &imports, &self.class_map) {
            Ok(hover) => return Some(hover),
            Err(e) => {
                eprintln!("Error while hover: {e:?}");
                None
            }
        }
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
        let vars = variable::get_vars(document.value(), &point);

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
                eprintln!("Error while completion: {e:?}");
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
        let vars = variable::get_vars(document.value(), &point);
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
            eprintln!("References could not get callchain");
            return None;
        };
        let Some(class) = &self.class_map.get(&document.class_path) else {
            eprintln!("Could not find class {}", document.class_path);
            return None;
        };
        match references::call_chain_references(
            uri,
            &point,
            &call_chain,
            &vars,
            &imports,
            class,
            &self.class_map,
            &self.reference_map,
        ) {
            Ok(definition) => return Some(definition),
            Err(e) => {
                eprintln!("Error while completion: {e:?}");
            }
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
            ProjectKind::Gradle => gradle::project::get_paths(),
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
            Ok(hover) => return Some(hover),
            Err(e) => {
                eprintln!("Error while hover: {e:?}");
                None
            }
        }
    }
}
