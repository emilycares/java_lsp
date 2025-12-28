use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};

use call_chain::get_call_chain;
use common::{TaskProgress, project_kind::ProjectKind};
use compile::CompileError;
use dashmap::{DashMap, DashSet};
use document::{ClassSource, Document, get_class_path, read_document_or_open_class};
use loader::LoaderError;
use lsp_extra::{SERVER_NAME, source_to_uri};
use lsp_server::{Connection, Message};
use lsp_types::{
    ClientCapabilities, CodeActionParams, CodeActionResponse, CompletionParams, CompletionResponse,
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, DocumentFormattingParams,
    DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse,
    Hover, HoverParams, Location, Position, ProgressParams, ProgressParamsValue, ProgressToken,
    PublishDiagnosticsParams, Range, ReferenceParams, SignatureHelp, SignatureHelpParams,
    TextDocumentContentChangeEvent, TextEdit, Uri, WorkDoneProgress, WorkDoneProgressBegin,
    WorkDoneProgressEnd, WorkDoneProgressReport, WorkspaceSymbolParams, WorkspaceSymbolResponse,
    notification::{Notification, Progress, PublishDiagnostics},
};
use maven::{fetch::MavenFetchError, tree::MavenTreeError};
use my_string::MyString;
use parser::dto::Class;
use rayon::iter::{ParallelBridge, ParallelIterator};
use tokio::task::JoinSet;

use crate::{
    codeaction::{self, CodeActionContext},
    completion,
    definition::{self, DefinitionContext},
    hover::{self, class_action},
    references::{self, ReferenceUnit, ReferencesContext},
    signature, to_ast_point,
};

pub struct Backend {
    pub error_files: DashSet<String>,
    pub project_kind: ProjectKind,
    pub document_map: DashMap<MyString, Document>,
    pub class_map: Arc<DashMap<MyString, Class>>,
    pub reference_map: Arc<DashMap<MyString, Vec<ReferenceUnit>>>,
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

    fn on_change(&self, uri: Uri, changes: &[TextDocumentContentChangeEvent]) {
        let Some(mut document) = self.document_map.get_mut(&get_document_map_key(&uri)) else {
            eprintln!("on_change document not found");
            return;
        };
        let mut errors = Vec::new();
        if let Ok(Some(d)) = document.apply_text_changes(changes) {
            errors.push(d);
        }
        Self::send_diagnostic(&self.connection.clone(), uri, errors);
    }

    pub fn send_diagnostic(con: &Arc<Connection>, uri: Uri, diagnostics: Vec<Diagnostic>) {
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
    fn progress_start_option_token(
        con: &Arc<Connection>,
        token: &Arc<Option<ProgressToken>>,
        title: &str,
    ) {
        if let Some(token) = token.as_ref() {
            Self::progress_start_token(con, token, title);
            return;
        }
        Self::progress_start(con, title);
    }
    fn progress_start_token(con: &Arc<Connection>, token: &ProgressToken, title: &str) {
        eprintln!("Start progress on: {title}");
        if let Ok(params) = serde_json::to_value(ProgressParams {
            token: token.to_owned(),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(WorkDoneProgressBegin {
                title: title.to_owned(),
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

    fn progress_start(con: &Arc<Connection>, task: &str) {
        let token = ProgressToken::String(task.to_owned());
        Self::progress_start_token(con, &token, task);
    }
    fn progress_update_percentage_option_token(
        con: &Arc<Connection>,
        token: &Arc<Option<ProgressToken>>,
        task: &str,
        message: String,
        percentage: Option<u32>,
    ) {
        if let Some(token) = token.as_ref() {
            Self::progress_update_percentage_token(con, token, task, message, percentage);
            return;
        }
        Self::progress_update_percentage(con, task, message, percentage);
    }
    fn progress_update_percentage(
        con: &Arc<Connection>,
        task: &str,
        message: String,
        percentage: Option<u32>,
    ) {
        let token = ProgressToken::String(task.to_owned());
        Self::progress_update_percentage_token(con, &token, task, message, percentage);
    }
    fn progress_update_percentage_token(
        con: &Arc<Connection>,
        token: &ProgressToken,
        task: &str,
        message: String,
        percentage: Option<u32>,
    ) {
        eprintln!("Report progress on: {task} {percentage:?} status: {message}");
        if let Ok(params) = serde_json::to_value(ProgressParams {
            token: token.to_owned(),
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
    fn progress_end_option_token(
        con: &Arc<Connection>,
        token: &Arc<Option<ProgressToken>>,
        task: &str,
    ) {
        if let Some(token) = token.as_ref() {
            Self::progress_end_token(con, token, task);
            return;
        }
        Self::progress_end(con, task);
    }
    fn progress_end_token(con: &Arc<Connection>, token: &ProgressToken, task: &str) {
        eprintln!("End progress on: {task}");
        if let Ok(params) = serde_json::to_value(ProgressParams {
            token: token.to_owned(),
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

    fn progress_end(con: &Arc<Connection>, task: &str) {
        let token = ProgressToken::String(task.to_owned());
        Self::progress_end_token(con, &token, task);
    }

    fn compile(&self, path: &str) -> Vec<CompileError> {
        match &self.project_kind {
            ProjectKind::Maven { executable } => {
                if let Some(classpath) = maven::compile::generate_classpath(executable)
                    && let Some(errors) = compile::compile_java_file(path, &classpath)
                {
                    return errors;
                }
            }
            ProjectKind::Gradle {
                executable,
                path_build_gradle: _,
            } => {
                if let Some(errors) = gradle::compile::compile_java(executable) {
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
                emap.insert(e.path.clone(), vec![e]);
            }
        }

        for path in self.error_files.iter() {
            let Some(path) = path.get(..) else {
                continue;
            };
            if let Ok(uri) = source_to_uri(path) {
                if let Some(errs) = emap.get(path) {
                    let errs: Vec<Diagnostic> = errs
                        .iter()
                        .map(|e| {
                            let r = u32::try_from(e.row).unwrap_or_default();
                            let c = u32::try_from(e.col).unwrap_or_default();
                            let p = Position::new(r.saturating_sub(1), c);
                            Diagnostic::new(
                                Range::new(p, p),
                                Some(DiagnosticSeverity::ERROR),
                                None,
                                Some(String::from(SERVER_NAME)),
                                e.message.clone(),
                                None,
                                None,
                            )
                        })
                        .collect();
                    Self::send_diagnostic(&self.connection.clone(), uri, errs);
                } else {
                    Self::send_diagnostic(&self.connection.clone(), uri, vec![]);
                }
            }
        }
    }

    pub async fn initialized(
        progress: Option<ProgressToken>,
        con: Arc<Connection>,
        project_kind: ProjectKind,
        class_map: Arc<dashmap::DashMap<MyString, parser::dto::Class>>,
        reference_map: Arc<dashmap::DashMap<MyString, Vec<ReferenceUnit>>>,
    ) {
        let progress = Arc::new(progress);
        Self::progress_start_option_token(&con, &progress, "Init");
        let mut handles = JoinSet::new();
        {
            let con = con.clone();
            let class_map = class_map.clone();
            handles.spawn(async move {
                let task = "Load jdk";
                let progress = Arc::new(Option::Some(ProgressToken::String(task.to_owned())));
                let (sender, receiver) =
                    tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
                        percentage: 0,
                        error: false,
                        message: "...".to_string(),
                    });

                Self::progress_start_option_token(&con.clone(), &progress, task);
                tokio::select! {
                    () = read_forward(receiver, con.clone(), task.to_owned(), progress.clone())  => {},
                    _ = jdk::load_classes(&class_map, sender) => {}
                }
                Self::progress_end_option_token(&con.clone(), &progress, task);
            });
        }

        if project_kind != ProjectKind::Unknown {
            let con = con.clone();
            let task = format!("Load {project_kind} dependencies");
            let progress = Arc::new(Option::Some(ProgressToken::String(task.clone())));
            let project_kind = project_kind.clone();
            let class_map = class_map.clone();
            handles.spawn(async move {
                Self::progress_start_option_token(&con.clone(), &progress, &task);
                let (sender, receiver) =
                    tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
                        percentage: 0,
                        error: false,
                        message: "...".to_string(),
                    });

                tokio::select! {
                    () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
                    () = fetch_deps(con.clone(), sender, project_kind.clone(), class_map.clone()) => {}
                }
                Self::progress_end_option_token(&con, &progress, &task);
            });
        }

        {
            let con = con.clone();
            handles.spawn(async move {
                let task = "Load project files";
                let progress = Arc::new(Option::Some(ProgressToken::String(task.to_owned())));
                Self::progress_start_option_token(&con, &progress, task);
                Self::progress_update_percentage_option_token(
                    &con.clone(),
                    &progress,
                    task,
                    "Load project paths".to_string(),
                    None,
                );
                let project_classes = match project_kind {
                    ProjectKind::Maven { executable: _ } => maven::project::load_project_folders(),
                    ProjectKind::Gradle {
                        executable: _,
                        path_build_gradle: _,
                    } => gradle::project::load_project_folders(),
                    ProjectKind::Unknown => loader::load_java_files(PathBuf::from("./")),
                };
                Self::progress_update_percentage_option_token(
                    &con.clone(),
                    &progress,
                    task,
                    "Initializing reference map".to_string(),
                    None,
                );
                match references::init_reference_map(&project_classes, &class_map, &reference_map) {
                    Ok(()) => (),
                    Err(e) => eprintln!("Got reference error: {e:?}"),
                }
                Self::progress_update_percentage_option_token(
                    &con.clone(),
                    &progress,
                    task,
                    "Populating class map".to_string(),
                    None,
                );
                for class in project_classes {
                    class_map.insert(class.class_path.clone(), class);
                }
                Self::progress_end_option_token(&con.clone(), &progress, task);
            });
        }

        let _ = handles.join_all().await;

        Self::progress_end_option_token(&con, &progress, "Init");
    }

    pub fn did_open(&self, params: &DidOpenTextDocumentParams) {
        let document_map_key = get_document_map_key(&params.text_document.uri);
        match read_document_or_open_class(&document_map_key, &self.document_map) {
            Ok(ClassSource::Owned(doc, diag)) => {
                self.handle_diagnostic(params.text_document.uri.clone(), diag.as_ref().clone());
                match parser::update_project_java_file(
                    PathBuf::from(document_map_key),
                    doc.as_bytes(),
                ) {
                    Ok(class) => {
                        match references::reference_update_class(
                            &class,
                            &self.class_map,
                            &self.reference_map,
                        ) {
                            Ok(()) => {}
                            Err(e) => eprintln!("Got reference error: {e:?}"),
                        }
                        if let Some(key) = get_class_path(&doc.ast) {
                            self.class_map.insert(key, class);
                        }
                    }
                    Err(e) => eprintln!("Save file parse error {e:?}"),
                }
            }
            Ok(ClassSource::Ref(_)) => {}
            Err(e) => {
                eprintln!("Error while on_open: {e:?}");
            }
        }
    }
    pub fn did_close(&self, params: &DidCloseTextDocumentParams) {
        let key = get_document_map_key(&params.text_document.uri);
        eprintln!("Closing file: {key}");
        self.document_map.remove(&key);
    }

    pub fn did_change(&self, params: &DidChangeTextDocumentParams) {
        self.on_change(params.text_document.uri.clone(), &params.content_changes);
    }

    pub fn did_save(&self, params: &DidSaveTextDocumentParams) {
        let path = params.text_document.uri.path();
        let path_str = path.as_str();
        let errors = self.compile(path_str);
        self.publish_compile_errors(errors);

        match read_document_or_open_class(path.as_str(), &self.document_map) {
            Ok(ClassSource::Owned(doc, diag)) => {
                self.handle_diagnostic(params.text_document.uri.clone(), diag.as_ref().clone());
                match parser::update_project_java_file(PathBuf::from(path.as_str()), doc.as_bytes())
                {
                    Ok(class) => {
                        let class_path = class.class_path.clone();
                        match references::reference_update_class(
                            &class,
                            &self.class_map,
                            &self.reference_map,
                        ) {
                            Ok(()) => {}
                            Err(e) => eprintln!("Got reference error: {e:?}"),
                        }
                        self.class_map.insert(class_path, class);
                    }
                    Err(e) => eprintln!("Save file parse error {e:?}"),
                }
            }
            Ok(ClassSource::Ref(doc)) => {
                match parser::update_project_java_file(PathBuf::from(path.as_str()), doc.as_bytes())
                {
                    Ok(class) => {
                        let class_path = class.class_path.clone();
                        match references::reference_update_class(
                            &class,
                            &self.class_map,
                            &self.reference_map,
                        ) {
                            Ok(()) => {}
                            Err(e) => eprintln!("Got reference error: {e:?}"),
                        }
                        self.class_map.insert(class_path, class);
                    }
                    Err(e) => eprintln!("Save file parse error {e:?}"),
                }
            }
            Err(_) => (),
        }
    }

    pub fn hover(&self, params: HoverParams) -> Option<Hover> {
        let uri = params.text_document_position_params.text_document.uri;
        let document = self.document_map.get(&get_document_map_key(&uri))?;
        let point = to_ast_point(params.text_document_position_params.position);
        let imports = imports::imports(&document.ast);
        let vars = match variables::get_vars(&document.ast, &point) {
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
        let Some(mut document) = self.document_map.get_mut(&get_document_map_key(&uri)) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let lines = document.rope.lines().len();
        if let Err(e) = format::format(document.path.clone()) {
            eprintln!("Formatter error: {e:?}");
            return None;
        }
        if let Err(e) = document.reload_file_from_disk() {
            eprintln!("Formatter unable to reload from disk: {e:?}");
            return None;
        }

        let lines = u32::try_from(lines).unwrap_or_default();
        // self.connection.
        Some(vec![TextEdit::new(
            Range::new(Position::new(0, 0), Position::new(lines - 1, 0)),
            document.str_data.clone(),
        )])
    }

    pub fn completion(&self, params: CompletionParams) -> Option<CompletionResponse> {
        let params = params.text_document_position;
        let uri = params.text_document.uri;
        let Some(document) = self.document_map.get(&get_document_map_key(&uri)) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let mut out = vec![];
        let point = to_ast_point(params.position);
        let vars = match variables::get_vars(&document.ast, &point) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Could not get vars: {e:?}");
                None
            }
        }?;

        let imports = imports::imports(&document.ast);

        let Some(class_path) = get_class_path(&document.ast) else {
            eprintln!("Could not get class_path");
            return None;
        };
        let Some(class) = &self.class_map.get(&class_path) else {
            eprintln!("Could not find class {class_path}");
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
            eprintln!("Call chain is empty");
            out.extend(completion::static_methods(
                &document.ast,
                &imports,
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
        let Some(document) = self.document_map.get(&get_document_map_key(&uri)) else {
            eprintln!("Document is not opened.");
            return None;
        };

        let point = to_ast_point(params.position);
        let imports = imports::imports(&document.ast);
        let vars = match variables::get_vars(&document.ast, &point) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Could not get vars: {e:?}");
                None
            }
        }?;
        let Some(class_path) = get_class_path(&document.ast) else {
            eprintln!("Could not get class_path");
            return None;
        };
        let Some(class) = &self.class_map.get(&class_path) else {
            eprintln!("Could not find class {class_path}");
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

        match definition::class(&document.ast, &context, &self.document_map) {
            Ok(definition) => return Some(definition),
            Err(e) => {
                eprintln!("Error while class definition: {e:?}");
            }
        }
        let call_chain = get_call_chain(&document.ast, &point);
        match definition::call_chain_definition(&call_chain, &context) {
            Ok(definition) => return Some(definition),
            Err(e) => {
                eprintln!("Error while definition: {e:?}");
            }
        }
        None
    }

    pub fn references(&self, params: ReferenceParams) -> Option<Vec<Location>> {
        let params = params.text_document_position;
        let uri = params.text_document.uri;
        let Some(document) = self.document_map.get(&get_document_map_key(&uri)) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let point = to_ast_point(params.position);
        let imports = imports::imports(&document.ast);
        let vars = match variables::get_vars(&document.ast, &point) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Could not get vars: {e:?}");
                None
            }
        }?;
        match class_action(&document.ast, &point, &vars, &imports, &self.class_map) {
            Ok((class, _range)) => {
                if let Some(value) =
                    references::class_path(&class.class_path, &self.reference_map, &self.class_map)
                {
                    return Some(value);
                }
            }
            Err(e) => eprintln!("Got reference class error: {e:?}"),
        }
        let call_chain = get_call_chain(&document.ast, &point);
        let Some(class_path) = get_class_path(&document.ast) else {
            eprintln!("Could not get class_path");
            return None;
        };
        let Some(class) = &self.class_map.get(&class_path) else {
            eprintln!("Could not find class {class_path}");
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
                eprintln!("Got reference call_chain error: {e:?}");
                None
            }
        }
    }

    pub fn code_action(&self, params: CodeActionParams) -> Option<CodeActionResponse> {
        let Some(document) = self
            .document_map
            .get(&get_document_map_key(&params.text_document.uri))
        else {
            eprintln!("Document is not opened.");
            return None;
        };
        let current_file = params.text_document.uri;
        let point = to_ast_point(params.range.start);

        let imports = imports::imports(&document.ast);

        let Some(class_path) = get_class_path(&document.ast) else {
            eprintln!("Could not get class_path");
            return None;
        };
        let Some(class) = &self.class_map.get(&class_path) else {
            eprintln!("Could not find class {class_path}");
            return None;
        };
        let vars = match variables::get_vars(&document.ast, &point) {
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
        if let Some(imps) = codeaction::import_jtype(&document.ast, &context) {
            return Some(imps);
        }

        match codeaction::replace_with_value_type(&document.ast, &context) {
            Ok(None) => (),
            Ok(Some(e)) => return Some(vec![e]),
            Err(e) => {
                eprintln!("Got error code_action replace with value: {e:?}");
            }
        }

        None
    }

    pub fn document_symbol(&self, params: DocumentSymbolParams) -> Option<DocumentSymbolResponse> {
        let Some(document) = self
            .document_map
            .get(&get_document_map_key(&params.text_document.uri))
        else {
            eprintln!("Document is not opened.");
            return None;
        };
        let uri = params.text_document.uri;

        let mut symbols = vec![];
        let _ = position::get_class_position_ast(&document.ast, None, &mut symbols);
        let _ = position::get_method_position_ast(&document.ast, None, &mut symbols);
        let _ = position::get_field_position_ast(&document.ast, None, &mut symbols);
        let symbols = position::symbols_to_document_symbols(&symbols, &uri);
        Some(DocumentSymbolResponse::Flat(symbols))
    }

    #[allow(clippy::unnecessary_wraps)]
    pub fn workspace_document_symbol(
        &self,
        params: &WorkspaceSymbolParams,
    ) -> Option<WorkspaceSymbolResponse> {
        let current_dir = std::env::current_dir().ok()?;
        let symbols = jwalk::WalkDir::new(current_dir.join("src"))
            .into_iter()
            .par_bridge()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir())
            .filter(|i| {
                i.path()
                    .extension()
                    .filter(|i| i.eq_ignore_ascii_case("java"))
                    .is_some()
            })
            .filter_map(|e| e.path().to_str().map(ToString::to_string))
            .filter_map(|i| {
                let class_source =
                    document::read_document_or_open_class(&i, &self.document_map).ok()?;
                Some((i.clone(), class_source))
            })
            .filter_map(|(path, source)| {
                let mut out = vec![];
                match source {
                    ClassSource::Owned(d, _) => {
                        let _ = position::get_class_position_ast(&d.ast, None, &mut out);
                    }
                    ClassSource::Ref(d) => {
                        let _ = position::get_class_position_ast(&d.ast, None, &mut out);
                    }
                }
                Some((path, out))
            })
            .map(|(path, symbols)| {
                (
                    path,
                    symbols
                        .into_iter()
                        .filter(|i| i.name.contains(&params.query))
                        .collect::<Vec<_>>(),
                )
            })
            .filter_map(|(path, symbols)| {
                let uri = source_to_uri(&path).ok()?;
                Some(position::symbols_to_document_symbols(&symbols, &uri))
            })
            .flatten()
            .collect();
        Some(WorkspaceSymbolResponse::Flat(symbols))
    }

    pub fn signature_help(&self, params: SignatureHelpParams) -> Option<SignatureHelp> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(document) = self.document_map.get(&get_document_map_key(&uri)) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let point = to_ast_point(params.text_document_position_params.position);
        let Some(class_path) = get_class_path(&document.ast) else {
            eprintln!("Could not get class_path");
            return None;
        };
        let Some(class) = &self.class_map.get(&class_path) else {
            eprintln!("Could not find class {class_path}");
            return None;
        };

        match signature::signature_driver(&document, &point, class, &self.class_map) {
            Ok(hover) => Some(hover),
            Err(e) => {
                eprintln!("Error while signature_help: {e:?}");
                None
            }
        }
    }

    fn handle_diagnostic(&self, uri: Uri, diag: Option<Diagnostic>) {
        let mut diagnostics = Vec::new();
        diagnostics.extend(diag);
        Self::send_diagnostic(&self.connection, uri, diagnostics);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_document_map_key(uri: &Uri) -> String {
    uri.path().as_str().to_owned()
}
#[cfg(target_os = "windows")]
pub fn get_document_map_key(uri: &Uri) -> String {
    uri.path()
        .as_str()
        // remove leading slash
        .trim_start_matches('/')
        // url encoded colon
        .replacen("%3A", ":", 1)
}

pub async fn read_forward(
    mut rx: tokio::sync::watch::Receiver<TaskProgress>,
    con: Arc<Connection>,
    task: String,
    token: Arc<Option<ProgressToken>>,
) {
    tokio::spawn(async move {
        loop {
            if rx.changed().await.is_err() {
                break;
            }
            let i = rx.borrow();
            Backend::progress_update_percentage_option_token(
                &con.clone(),
                &token,
                &task,
                i.message.clone(),
                Some(i.percentage),
            );
        }
    })
    .await
    .expect("Forward failed");
}

pub async fn fetch_deps(
    con: Arc<Connection>,
    sender: tokio::sync::watch::Sender<TaskProgress>,
    project_kind: ProjectKind,
    class_map: Arc<dashmap::DashMap<MyString, parser::dto::Class>>,
) {
    match project_kind {
        ProjectKind::Maven { executable } => {
            match maven::fetch::fetch_deps(class_map, sender, true, true, &executable).await {
                Ok(()) => (),
                Err(e) => {
                    eprintln!("Got error while loading maven project: {e:?}");
                    let mut diagnostics = Vec::new();
                    let range = Range::default();
                    match e {
                        MavenFetchError::DownloadSources(e) => {
                            let message = format!("Unable download maven sources: {e}");
                            diagnostics.push(Diagnostic::new(
                                range,
                                Some(DiagnosticSeverity::ERROR),
                                None,
                                Some(String::from(SERVER_NAME)),
                                message,
                                None,
                                None,
                            ));
                        }
                        MavenFetchError::NoHomeFound => {
                            let message = "Unable to find home directory".to_owned();
                            diagnostics.push(Diagnostic::new(
                                range,
                                Some(DiagnosticSeverity::ERROR),
                                None,
                                Some(String::from(SERVER_NAME)),
                                message,
                                None,
                                None,
                            ));
                        }
                        MavenFetchError::Tree(MavenTreeError::Cli(e)) => {
                            let message = format!("Unable load maven dependency tree {e:?}");
                            diagnostics.push(Diagnostic::new(
                                range,
                                Some(DiagnosticSeverity::ERROR),
                                None,
                                Some(String::from(SERVER_NAME)),
                                message,
                                None,
                                None,
                            ));
                        }
                        MavenFetchError::Tree(MavenTreeError::UnknownDependencyScope(scope)) => {
                            let message = format!("Unsupported dependency scope found: {scope:?}");
                            diagnostics.push(Diagnostic::new(
                                range,
                                Some(DiagnosticSeverity::ERROR),
                                None,
                                Some(String::from(SERVER_NAME)),
                                message,
                                None,
                                None,
                            ));
                        }
                        MavenFetchError::ParserLoader(LoaderError::Zip { e, path }) => {
                            let severity = Some(DiagnosticSeverity::ERROR);
                            let message = format!(
                                "Unable to load zip or jmod classes from: {path}, error: {e}"
                            );
                            diagnostics.push(Diagnostic::new(
                                range, severity, None, None, message, None, None,
                            ));
                        }
                        MavenFetchError::NoM2Folder => {
                            let message = "Unable to find .m2 directory".to_owned();
                            diagnostics.push(Diagnostic::new(
                                range,
                                Some(DiagnosticSeverity::ERROR),
                                None,
                                Some(String::from(SERVER_NAME)),
                                message,
                                None,
                                None,
                            ));
                        }
                        MavenFetchError::FailedToResolveSources(_) => {
                            let message = "Unable to download maven sources".to_owned();
                            diagnostics.push(Diagnostic::new(
                                range,
                                Some(DiagnosticSeverity::ERROR),
                                None,
                                Some(String::from(SERVER_NAME)),
                                message,
                                None,
                                None,
                            ));
                        }
                        MavenFetchError::ParserLoader(
                            LoaderError::IO(_) | LoaderError::InvalidCfcCache,
                        ) => (),
                    }
                    let source = PathBuf::from("./pom.xml");
                    if let Ok(source) = fs::canonicalize(source)
                        && let Some(source) = source.to_str()
                        && let Ok(uri) = source_to_uri(source)
                    {
                        Backend::send_diagnostic(&con, uri, diagnostics);
                    }
                }
            }
        }
        ProjectKind::Gradle {
            executable,
            path_build_gradle,
        } => match gradle::fetch::fetch_deps(&class_map, path_build_gradle, &executable, sender)
            .await
        {
            Ok(()) => (),
            Err(e) => {
                eprintln!("Got error while loading gradle project: {e:?}");
            }
        },
        ProjectKind::Unknown => (),
    }
}
