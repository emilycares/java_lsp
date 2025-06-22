use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc};

use call_chain::get_call_chain;
use common::{TaskProgress, project_kind::ProjectKind};
use compile::CompileError;
use dashmap::{DashMap, DashSet};
use document::{ClassSource, Document};
use lsp_server::{Connection, Message};
use lsp_types::{
    ClientCapabilities, CodeActionParams, CodeActionResponse, CompletionParams, CompletionResponse,
    Diagnostic, DidChangeTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    DocumentFormattingParams, DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverParams, Location, Position, ProgressParams,
    ProgressParamsValue, ProgressToken, PublishDiagnosticsParams, Range, ReferenceParams,
    SignatureHelp, SignatureHelpParams, SymbolInformation, TextDocumentContentChangeEvent,
    TextDocumentItem, TextEdit, Uri, WorkDoneProgress, WorkDoneProgressBegin, WorkDoneProgressEnd,
    WorkDoneProgressReport, WorkspaceSymbolParams, WorkspaceSymbolResponse,
    notification::{Notification, Progress, PublishDiagnostics},
};
use parking_lot::Mutex;
use parser::dto::Class;
use position::PositionSymbol;
use tree_sitter_util::lsp::to_treesitter_point;

use crate::{
    codeaction::{self, CodeActionContext},
    completion,
    definition::{self, DefinitionContext, source_to_uri},
    hover::{self, class_action},
    references::{self, ReferenceUnit, ReferencesContext},
    signature,
};

pub struct Backend {
    pub error_files: DashSet<String>,
    pub project_kind: ProjectKind,
    pub document_map: DashMap<String, Document>,
    pub class_map: Arc<DashMap<String, Class>>,
    pub reference_map: Arc<DashMap<String, Vec<ReferenceUnit>>>,
    pub client_capabilities: Arc<Option<ClientCapabilities>>,
    pub connection: Arc<Connection>,
}

impl Backend {
    pub fn new(connection: Connection, project_kind: ProjectKind) -> Self {
        Self {
            connection: Arc::new(connection),
            error_files: DashSet::new(),
            project_kind,
            document_map: DashMap::new(),
            class_map: Arc::new(DashMap::new()),
            reference_map: Arc::new(DashMap::new()),
            client_capabilities: Arc::new(None),
        }
    }

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
                            eprintln!("Failed to setup document: {e:?}");
                        }
                    }
                }
                Err(e) => eprintln!("Got error parsing document {ipath}, {e:?}"),
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

    fn progress_start(con: Arc<Connection>, task: String) {
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
            let _ = con
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
        eprintln!("Report progress on: {task} {percentage:?} status: {message}");
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

    fn progress_end(con: Arc<Connection>, task: String) {
        eprintln!("End progress on: {}", task);
        if let Ok(params) = serde_json::to_value(ProgressParams {
            token: ProgressToken::String(task),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                message: None,
            })),
        }) {
            let _ = con
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

    fn publish_compile_errors(&self, errors: Vec<CompileError>) {
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

    pub async fn initialized(
        con: Arc<Connection>,
        project_kind: ProjectKind,
        class_map: Arc<dashmap::DashMap<std::string::String, parser::dto::Class>>,
        reference_map: Arc<dashmap::DashMap<String, Vec<ReferenceUnit>>>,
    ) {
        Backend::progress_start(con.clone(), "Init".to_string());
        let task = "Load jdk".to_string();

        Backend::progress_start(con.clone(), task.clone());
        let _ = jdk::load_classes(&class_map).await;
        Backend::progress_end(con.clone(), task.clone());

        if project_kind != ProjectKind::Unknown {
            let task = format!("Load {} dependencies", project_kind);
            Backend::progress_start(con.clone(), task.clone());
            let (sender, reciever) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
                persentage: 0,
                error: false,
                message: "...".to_string(),
            });
            let reciever = Arc::new(Mutex::new(reciever));
            let atask = Arc::new(task.clone());

            tokio::select! {
                _ = read_forward(reciever, con.clone(), atask)  => {},
                _ = fetch_deps(sender, project_kind.clone(), class_map.clone()) => {}
            }
            Backend::progress_end(con.clone(), task);
        }

        let task = "Load project files".to_string();
        Backend::progress_start(con.clone(), task.clone());
        let project_classes = match project_kind {
            ProjectKind::Maven => maven::project::load_project_folders(),
            ProjectKind::Gradle {
                path_build_gradle: _,
            } => gradle::project::load_project_folders(),
            ProjectKind::Unknown => vec![],
        };
        Backend::progress_update_persentage(
            con.clone(),
            task.clone(),
            "Initializing reference map".to_string(),
            None,
        );
        match references::init_refernece_map(&project_classes, &class_map, &reference_map) {
            Ok(_) => (),
            Err(e) => eprintln!("Got reference error: {e:?}"),
        }
        Backend::progress_update_persentage(
            con.clone(),
            task.clone(),
            "Populating class map".to_string(),
            None,
        );
        for class in project_classes {
            class_map.insert(class.class_path.clone(), class);
        }
        Backend::progress_end(con.clone(), task);

        Backend::progress_end(con, "Init".to_string());
    }

    pub fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_open(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
            language_id: params.text_document.language_id,
        });
    }

    pub fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.on_change(params.text_document.uri.to_string(), params.content_changes);
    }

    pub fn did_save(&self, params: DidSaveTextDocumentParams) {
        let path = params.text_document.uri.path();
        // The path on windows should not look like this: /C:/asdas remove the leading slash

        let errors = self.compile(path.as_str());
        self.publish_compile_errors(errors);

        let Some(mut document) = self.document_map.get_mut(params.text_document.uri.as_str())
        else {
            eprintln!("no doc found");
            return;
        };
        match parser::update_project_java_file(PathBuf::from(path.as_str()), document.as_bytes()) {
            Ok(class) => {
                document.class_path.clone_from(&class.class_path);
                let class_path = class.class_path.clone();
                match references::reference_update_class(
                    &class,
                    &self.class_map,
                    &self.reference_map,
                ) {
                    Ok(_) => {}
                    Err(e) => eprintln!("Got reference error: {e:?}"),
                };
                self.class_map.insert(class_path, class);
            }
            Err(e) => eprintln!("Save file parse error {e:?}"),
        }
    }

    pub fn hover(&self, params: HoverParams) -> Option<Hover> {
        let uri = params.text_document_position_params.text_document.uri;
        let document = self.document_map.get_mut(uri.as_str())?;
        let point = to_treesitter_point(params.text_document_position_params.position);
        let imports = imports::imports(document.value());
        let vars = match variables::get_vars(document.value(), &point) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Could not get vars: {e:?}");
                None
            }
        }?;

        match hover::base(document.value(), &point, &vars, &imports, &self.class_map) {
            Ok(hover) => Some(hover),
            Err(e) => {
                eprintln!("Error while hover: {e:?}");
                None
            }
        }
    }

    pub fn formatting(&self, params: DocumentFormattingParams) -> Option<Vec<TextEdit>> {
        let uri = params.text_document.uri;
        let Some(mut document) = self.document_map.get_mut(uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let lines = document.text.lines().len();
        if let Err(e) = format::format(document.path.clone()) {
            eprintln!("Formatter error: {e:?}");
            return None;
        }
        if let Err(e) = document.reload_file_from_disk() {
            eprintln!("Formatter unable to reload from disk: {e:?}");
            return None;
        }

        // self.connection.
        Some(vec![TextEdit::new(
            Range::new(Position::new(0, 0), Position::new((lines - 1) as u32, 0)),
            document.str_data.clone(),
        )])
    }

    pub fn completion(&self, params: CompletionParams) -> Option<CompletionResponse> {
        let params = params.text_document_position;
        let uri = params.text_document.uri;
        let Some(document) = self.document_map.get_mut(uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let mut out = vec![];
        let point = to_treesitter_point(params.position);
        let vars = match variables::get_vars(document.value(), &point) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Could not get vars: {e:?}");
                None
            }
        }?;

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
            out.extend(completion::static_methods(
                &imports,
                &document.tree,
                &self.class_map,
            ));
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

    pub fn goto_definition(&self, params: GotoDefinitionParams) -> Option<GotoDefinitionResponse> {
        let params = params.text_document_position_params;
        let uri = params.text_document.uri;
        let Some(document) = self.document_map.get_mut(uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
        };

        let point = to_treesitter_point(params.position);
        let imports = imports::imports(document.value());
        let vars = match variables::get_vars(document.value(), &point) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Could not get vars: {e:?}");
                None
            }
        }?;
        let Some(class) = &self.class_map.get(&document.class_path) else {
            eprintln!("Could not find class {}", document.class_path);
            return None;
        };

        let context = DefinitionContext {
            document_uri: uri,
            point: &point,
            vars: &vars,
            imports: &imports,
            class,
            class_map: &self.class_map,
            document_map: &self.document_map,
        };

        match definition::class(document.value(), &context) {
            Ok(definition) => return Some(definition),
            Err(e) => {
                eprintln!("Error while class definition: {e:?}");
            }
        }
        let Some(call_chain) = get_call_chain(&document.tree, document.as_bytes(), &point) else {
            eprintln!("Defintion could not get callchain");
            return None;
        };
        match definition::call_chain_definition(&call_chain, &context) {
            Ok(definition) => return Some(definition),
            Err(e) => {
                eprintln!("Error while definition: {e:?}");
            }
        }
        None
    }

    pub fn referneces(&self, params: ReferenceParams) -> Option<Vec<Location>> {
        let params = params.text_document_position;
        let uri = params.text_document.uri;
        let Some(document) = self.document_map.get_mut(uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let point = to_treesitter_point(params.position);
        let imports = imports::imports(document.value());
        let vars = match variables::get_vars(document.value(), &point) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Could not get vars: {e:?}");
                None
            }
        }?;
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
            Err(e) => eprintln!("Got refrence class error: {e:?}"),
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
                eprintln!("Got refrence call_chain error: {e:?}");
                None
            }
        }
    }

    pub fn code_action(&self, params: CodeActionParams) -> Option<CodeActionResponse> {
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
        let vars = match variables::get_vars(document.value(), &point) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Could not get vars: {e:?}");
                None
            }
        }?;

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
                eprintln!("Got error code_action replace with value: {e:?}");
            }
        }

        None
    }

    pub fn document_symbol(&self, params: DocumentSymbolParams) -> Option<DocumentSymbolResponse> {
        let Some(document) = self.document_map.get_mut(params.text_document.uri.as_str()) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let uri = params.text_document.uri;

        let symbols = position::get_symbols(document.as_bytes(), &document.tree);
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

    #[allow(clippy::unnecessary_wraps)]
    pub fn workspace_document_symbol(
        &self,
        params: &WorkspaceSymbolParams,
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
            .filter_map(|i| {
                let Ok(uri) = source_to_uri(&i) else {
                    return None;
                };
                Some((
                    i.clone(),
                    document::read_document_or_open_class(
                        &i,
                        String::new(),
                        &self.document_map,
                        uri.as_str(),
                    ),
                ))
            })
            .filter_map(|(path, source)| match source {
                ClassSource::Owned(d) => Some((path, position::get_symbols(d.as_bytes(), &d.tree))),
                ClassSource::Ref(d) => Some((path, position::get_symbols(d.as_bytes(), &d.tree))),
                ClassSource::Err(_) => None,
            })
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
                            eprintln!("Errors with workspace document symbol: {e:?}");
                            vec![]
                        }
                    },
                )
            })
            .filter_map(|(path, symbols)| {
                let uri = Uri::from_str(&format!("file:///{path}")).ok()?;
                Some(position::symbols_to_document_symbols(symbols, uri))
            })
            .flatten()
            .collect();
        Some(WorkspaceSymbolResponse::Flat(symbols))
    }

    pub fn signature_help(&self, params: SignatureHelpParams) -> Option<SignatureHelp> {
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

pub async fn read_forward(
    rx: Arc<Mutex<tokio::sync::watch::Receiver<TaskProgress>>>,
    con: Arc<Connection>,
    task: Arc<String>,
) {
    tokio::spawn(async move {
        let ex = &mut rx.lock();

        loop {
            let i = ex.borrow_and_update();
            if !i.has_changed() {
                continue;
            }
            let task = task.clone();
            let con = con.clone();
            if i.error {
                Backend::global_error(con, i.message.clone());
            } else {
                Backend::progress_update_persentage_a(
                    con,
                    task,
                    i.message.clone(),
                    Some(i.persentage.try_into().unwrap()),
                );
            }
        }
    })
    .await
    .expect("Forward failed");
}

pub async fn fetch_deps(
    sender: tokio::sync::watch::Sender<TaskProgress>,
    project_kind: ProjectKind,
    class_map: Arc<dashmap::DashMap<std::string::String, parser::dto::Class>>,
) {
    tokio::spawn(async move {
        match project_kind {
            ProjectKind::Maven => {
                match maven::fetch::fetch_deps(class_map, sender, true, true).await {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("Got error while loading maven project: {e:?}");
                    }
                }
            }
            ProjectKind::Gradle {
                path_build_gradle: path,
            } => match gradle::fetch::fetch_deps(&class_map, path, sender).await {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("Got error while loading gradle project: {e:?}");
                }
            },
            ProjectKind::Unknown => (),
        }
    })
    .await
    .expect("asdf");
}
