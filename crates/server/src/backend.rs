use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use call_chain::get_call_chain;
use common::{Dependency, TaskProgress, project_cache_dir, project_kind::ProjectKind};
use compile::CompileErrorMessage;
use dashmap::{DashMap, DashSet};
use document::{
    ClassSource, Document, DocumentError, get_class_path, open_document,
    read_document_or_open_class,
};
use lsp_extra::{SERVER_NAME, source_to_uri, to_ast_point};
use lsp_server::{Connection, Message};
use lsp_types::{
    ClientCapabilities, CodeActionParams, CodeActionResponse, CompletionParams, CompletionResponse,
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, DocumentFormattingParams,
    DocumentSymbolParams, DocumentSymbolResponse, ExecuteCommandParams, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverParams, Location, Position, ProgressParams,
    ProgressParamsValue, ProgressToken, PublishDiagnosticsParams, Range, ReferenceParams,
    SignatureHelp, SignatureHelpParams, TextEdit, Uri, WorkDoneProgress, WorkDoneProgressBegin,
    WorkDoneProgressEnd, WorkDoneProgressReport, WorkspaceSymbolParams, WorkspaceSymbolResponse,
    notification::{Notification, Progress, PublishDiagnostics},
};
use maven::{
    project::MavenProjectError,
    tree::MavenTreeError,
    update::{self, MavenUpdateError},
};
use my_string::MyString;
use parser::dto::Class;
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde_json::Value;
use tokio::task::JoinSet;

use crate::{
    codeaction::{self, CodeActionContext},
    command::{self, COMMAND_RELOAD_DEPENDENCIES, UPDATE_DEPENDENCIES},
    completion,
    definition::{self, DefinitionContext},
    hover::{self, class_action},
    references::{self, ReferenceUnit, ReferencesContext, ReferencesError},
    signature,
};

pub struct Backend {
    pub error_files: DashSet<String>,
    pub project_kind: ProjectKind,
    pub document_map: DashMap<MyString, Document>,
    pub class_map: Arc<DashMap<MyString, Class>>,
    pub reference_map: Arc<DashMap<MyString, Vec<ReferenceUnit>>>,
    pub client_capabilities: Arc<Option<ClientCapabilities>>,
    pub connection: Arc<Connection>,
    pub project_dir: PathBuf,
}

impl Backend {
    pub fn new(connection: Connection, project_kind: ProjectKind, project_dir: PathBuf) -> Self {
        Self {
            connection: Arc::new(connection),
            error_files: DashSet::new(),
            project_kind,
            document_map: DashMap::new(),
            class_map: Arc::new(DashMap::new()),
            reference_map: Arc::new(DashMap::new()),
            client_capabilities: Arc::new(None),
            project_dir,
        }
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
    pub fn progress_start_option_token(
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
        percentage: u32,
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
        percentage: u32,
    ) {
        let token = ProgressToken::String(task.to_owned());
        Self::progress_update_percentage_token(con, &token, task, message, percentage);
    }
    fn progress_update_percentage_token(
        con: &Arc<Connection>,
        token: &ProgressToken,
        task: &str,
        message: String,
        percentage: u32,
    ) {
        eprintln!("Report progress on: {task} {percentage:?} status: {message}");
        if let Ok(params) = serde_json::to_value(ProgressParams {
            token: token.to_owned(),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                WorkDoneProgressReport {
                    cancellable: None,
                    message: Some(message),
                    percentage: Some(percentage),
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
    pub fn progress_end_option_token(
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

    fn compile(&self, path: &str) -> Vec<CompileErrorMessage> {
        match &self.project_kind {
            ProjectKind::Maven { executable } => {
                match maven::compile::generate_classpath(executable) {
                    Ok(classpath) => match compile::maven_compile_java_file(path, &classpath) {
                        Ok(errors) => return errors,
                        Err(e) => eprintln!("Compile error: {e:?}"),
                    },
                    e => eprintln!("Failed to load classpath {e:?}"),
                }
            }
            ProjectKind::Gradle { executable, .. } => {
                if let Some(errors) = gradle::compile::compile_java(executable) {
                    return errors;
                }
            }
            ProjectKind::Unknown => match compile::compile_java_file(path) {
                Ok(errors) => return errors,
                Err(e) => eprintln!("Compile error: {e:?}"),
            },
        }
        vec![]
    }

    fn publish_compile_errors(
        &self,
        errors: Vec<CompileErrorMessage>,
        current_file: &Uri,
    ) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        let mut emap = HashMap::<String, Vec<CompileErrorMessage>>::new();
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
                    if &uri == current_file {
                        out.extend(errs.iter().map(compile_error_to_diagnostic));
                    } else {
                        let errs: Vec<Diagnostic> =
                            errs.iter().map(compile_error_to_diagnostic).collect();
                        Self::send_diagnostic(&self.connection.clone(), uri, errs);
                    }
                } else {
                    Self::send_diagnostic(&self.connection.clone(), uri, vec![]);
                }
            }
        }
        out
    }

    pub async fn initialized(
        progress: Option<ProgressToken>,
        con: Arc<Connection>,
        project_kind: ProjectKind,
        class_map: Arc<DashMap<MyString, Class>>,
        reference_map: Arc<DashMap<MyString, Vec<ReferenceUnit>>>,
        project_dir: &Path,
        path: &OsString,
    ) {
        let progress = Arc::new(progress);
        Self::progress_start_option_token(&con, &progress, "Init");
        let mut handles = JoinSet::new();
        {
            let con = con.clone();
            let class_map = class_map.clone();
            let path = path.to_owned();
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
                    _ = jdk::load_classes(&class_map, sender, &path) => {}
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
            let project_dir = project_dir.to_owned();
            handles.spawn(async move {
                Self::progress_start_option_token(&con.clone(), &progress, &task);
                let (sender, receiver) =
                    tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
                        percentage: 0,
                        error: false,
                        message: "...".to_string(),
                    });
                let cache = project_cache_dir();

                tokio::select! {
                    () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
                    () = project_deps(con.clone(), sender, project_kind.clone(), class_map.clone(), true, &project_dir, &cache) => {}
                }
                Self::progress_end_option_token(&con, &progress, &task);
            });
        }

        {
            let con = con.clone();
            let project_dir = project_dir.to_owned();
            handles.spawn(async move {
                let task = "Load project files";
                let progress = Arc::new(Option::Some(ProgressToken::String(task.to_owned())));
                Self::progress_start_option_token(&con, &progress, task);
                Self::progress_update_percentage_option_token(
                    &con.clone(),
                    &progress,
                    task,
                    "Load project paths".to_string(),
                    1,
                );
                let project_classes = match project_kind {
                    ProjectKind::Maven { .. } => maven::project::load_project_folders(&project_dir),
                    ProjectKind::Gradle { .. } => {
                        gradle::project::load_project_folders(&project_dir)
                    }
                    ProjectKind::Unknown => loader::load_java_files(PathBuf::from("./")),
                };
                Self::progress_update_percentage_option_token(
                    &con.clone(),
                    &progress,
                    task,
                    "Initializing reference map".to_string(),
                    50,
                );
                match references::init_reference_map(&project_classes, &class_map, &reference_map) {
                    Ok(()) => (),
                    Err(e) => eprintln!("Got reference error: {e:?}"),
                }
                Self::progress_update_percentage_option_token(
                    &con.clone(),
                    &progress,
                    task,
                    format!("Populating class map number: {}", project_classes.len()),
                    90,
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
        match open_document(
            &document_map_key,
            &params.text_document.text,
            &self.document_map,
        ) {
            Ok(()) => {}
            Err(DocumentError::Diagnostic(diag)) => {
                self.handle_diagnostic(params.text_document.uri.clone(), Some(*diag));
            }
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
        let Some(mut document) = self
            .document_map
            .get_mut(&get_document_map_key(&params.text_document.uri))
        else {
            eprintln!("on_change document not found");
            return;
        };
        let mut errors = Vec::new();
        if let Err(DocumentError::Diagnostic(diag)) =
            document.apply_text_changes(&params.content_changes)
        {
            errors.push(*diag);
        }
        Self::send_diagnostic(
            &self.connection.clone(),
            params.text_document.uri.clone(),
            errors,
        );
    }

    pub fn did_save(&self, params: &DidSaveTextDocumentParams) {
        let path = params.text_document.uri.path();
        let path_str = path.as_str();
        let errors = self.compile(path_str);
        let mut current_file_diagnostics =
            self.publish_compile_errors(errors, &params.text_document.uri);

        match read_document_or_open_class(path.as_str(), &self.document_map) {
            Ok(ClassSource::Owned(doc)) => {
                let class =
                    parser::update_project_java_file(PathBuf::from(path.as_str()), &doc.ast);
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
            Ok(ClassSource::Ref(doc)) => {
                let class =
                    parser::update_project_java_file(PathBuf::from(path.as_str()), &doc.ast);
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
            Err(DocumentError::Diagnostic(diag)) => {
                current_file_diagnostics.push(*diag);
            }
            Err(e) => {
                eprintln!("Error while save: {e:?}");
            }
        }
        Self::send_diagnostic(
            &self.connection,
            params.text_document.uri.clone(),
            current_file_diagnostics,
        );
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

        match hover::base(&document.ast, &point, &vars, &imports, &self.class_map) {
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
            document.rope.to_string(),
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
                if let Some(value) = references::class_path(
                    &class.class_path,
                    &self.reference_map,
                    &self.class_map,
                    &self.document_map,
                ) {
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
            Err(ReferencesError::Document(DocumentError::Diagnostic(diag))) => {
                self.handle_diagnostic(uri, Some(*diag));
                None
            }
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
        let symbols = jwalk::WalkDir::new(self.project_dir.join("src"))
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
                    ClassSource::Owned(d) => {
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

    pub fn execute_command(&self, params: ExecuteCommandParams) -> Option<Value> {
        let progress = params.work_done_progress_params.work_done_token;
        match params.command.as_str() {
            COMMAND_RELOAD_DEPENDENCIES => command::reload_dependencies(
                &self.connection,
                progress,
                &self.project_kind,
                &self.class_map.clone(),
                &self.project_dir,
            ),
            UPDATE_DEPENDENCIES => {
                command::update_dependencies(
                    &self.connection,
                    progress,
                    &self.project_kind,
                    &self.class_map.clone(),
                    &self.project_dir,
                );
                None
            }
            u => {
                eprintln!("Unhandled command: {u}");
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

fn compile_error_to_diagnostic(e: &CompileErrorMessage) -> Diagnostic {
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
                i.percentage,
            );
        }
    })
    .await
    .expect("Forward failed");
}

pub async fn project_deps(
    con: Arc<Connection>,
    sender: tokio::sync::watch::Sender<TaskProgress>,
    project_kind: ProjectKind,
    class_map: Arc<DashMap<MyString, Class>>,
    use_cache: bool,
    project_dir: &Path,
    project_cache_dir: &Path,
) {
    match project_kind {
        ProjectKind::Maven { executable } => {
            match maven::project::project_deps(
                class_map,
                sender,
                use_cache,
                &executable,
                project_dir,
                project_cache_dir,
            )
            .await
            {
                Ok(()) => (),
                Err(e) => {
                    eprintln!("Got error while loading maven project: {e:?}");
                    let mut diagnostics = Vec::new();
                    let range = Range::default();
                    match e {
                        MavenProjectError::Tree(MavenTreeError::GotError(e)) => {
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
                        MavenProjectError::Tree(MavenTreeError::Cli(e)) => {
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
                        MavenProjectError::Tree(MavenTreeError::UnknownDependencyScope(scope)) => {
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
                        MavenProjectError::Tree(MavenTreeError::Utf8(_)) => (),
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
        ProjectKind::Gradle { executable, .. } => match gradle::project::project_deps(
            &class_map,
            &executable,
            sender,
            use_cache,
            project_dir,
            project_cache_dir,
        )
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
pub async fn update_report(
    project_kind: ProjectKind,
    con: Arc<Connection>,
    repos: Arc<Vec<String>>,
    tree: Vec<Dependency>,
    sender: tokio::sync::watch::Sender<TaskProgress>,
) {
    let res = update::update(repos, tree, sender).await;
    let Err(res) = res else {
        return;
    };
    let mut diagnostics = Vec::new();
    let range = Range::default();
    match res {
        MavenUpdateError::ClientBuilder(error)
        | MavenUpdateError::ReqBuilder(error)
        | MavenUpdateError::ShaBody(error)
        | MavenUpdateError::JarBody(error)
        | MavenUpdateError::Request(error) => {
            diagnostics.push(Diagnostic::new(
                range,
                Some(DiagnosticSeverity::ERROR),
                None,
                Some(String::from(SERVER_NAME)),
                error.to_string(),
                None,
                None,
            ));
        }
        MavenUpdateError::WriteHash(error)
        | MavenUpdateError::WriteJar(error)
        | MavenUpdateError::CreateDir(error)
        | MavenUpdateError::WriteEtag(error) => {
            let message = format!("Io error while update: {error}");
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
        MavenUpdateError::MTwo(mtwo_error) => {
            let message = format!("m2 error while update: {mtwo_error:?}");
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
    }
    let source = match project_kind {
        ProjectKind::Maven { .. } => Some(PathBuf::from("./pom.xml")),
        ProjectKind::Gradle {
            path_build_gradle, ..
        } => Some(path_build_gradle),
        ProjectKind::Unknown => None,
    };
    if let Some(source) = source
        && let Ok(source) = fs::canonicalize(source)
        && let Some(source) = source.to_str()
        && let Ok(uri) = source_to_uri(source)
    {
        Backend::send_diagnostic(&con, uri, diagnostics);
    }
}
