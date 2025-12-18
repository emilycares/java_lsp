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
    SignatureHelp, SignatureHelpParams, TextDocumentContentChangeEvent, TextDocumentItem, TextEdit,
    Uri, WorkDoneProgress, WorkDoneProgressBegin, WorkDoneProgressEnd, WorkDoneProgressReport,
    WorkspaceSymbolParams, WorkspaceSymbolResponse,
    notification::{Notification, Progress, PublishDiagnostics},
};
use my_string::MyString;
use parser::{SourceDestination, dto::Class};
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::{
    codeaction::{self, CodeActionContext},
    completion,
    definition::{self, DefinitionContext, source_to_uri},
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

    fn on_change(&self, uri: &str, changes: &[TextDocumentContentChangeEvent]) {
        let Some(mut document) = self.document_map.get_mut(uri) else {
            return;
        };
        // TODO: Handle error
        let _ = document.apply_text_changes(changes);
    }

    fn on_open(&self, params: &TextDocumentItem) {
        let ipath = get_normal_path(&params.uri);
        let path = PathBuf::from(params.uri.path().as_str());
        if let Some(mut document) = self.document_map.get_mut(&ipath) {
            // TODO: Handle error
            let _ = document.replace_string(&params.text);
        } else {
            match parser::java::load_java(
                params.text.as_bytes(),
                SourceDestination::Here(ipath.clone()),
            ) {
                Ok(class) => match Document::setup(&params.text, path, class.class_path) {
                    Ok(doc) => {
                        self.document_map.insert(ipath, doc);
                    }
                    Err(e) => {
                        eprintln!("Failed to setup document: {e:?}");
                    }
                },
                Err(e) => eprintln!("Got error parsing document {ipath}, {e:?}"),
            }
        }
    }

    fn send_diagnostic(con: &Arc<Connection>, uri: Uri, diagnostics: Vec<Diagnostic>) {
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

    fn progress_start(con: &Arc<Connection>, task: String) {
        eprintln!("Start progress on: {task}");
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

    fn progress_update_percentage(
        con: &Arc<Connection>,
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

    fn progress_end(con: &Arc<Connection>, task: String) {
        eprintln!("End progress on: {task}");
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
                if let Some(classpath) = maven::compile::generate_classpath()
                    && let Some(errors) = compile::compile_java_file(path, &classpath)
                {
                    return errors;
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
                emap.insert(e.path.clone(), vec![e]);
            }
        }

        for path in self.error_files.iter() {
            let Some(path) = path.get(..) else {
                continue;
            };
            if let Ok(uri) = Uri::from_str(&format!("file:///{path}")) {
                if let Some(errs) = emap.get(path) {
                    let errs: Vec<Diagnostic> = errs
                        .iter()
                        .map(|e| {
                            let r = u32::try_from(e.row).unwrap_or_default();
                            let c = u32::try_from(e.col).unwrap_or_default();
                            let p = Position::new(r - 1, c);
                            Diagnostic::new_simple(Range::new(p, p), e.message.clone())
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
        con: Arc<Connection>,
        project_kind: ProjectKind,
        class_map: Arc<dashmap::DashMap<MyString, parser::dto::Class>>,
        reference_map: Arc<dashmap::DashMap<MyString, Vec<ReferenceUnit>>>,
    ) {
        Self::progress_start(&con.clone(), "Init".to_string());
        {
            let task = "Load jdk".to_string();
            let (sender, receiver) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
                percentage: 0,
                error: false,
                message: "...".to_string(),
            });

            Self::progress_start(&con.clone(), task.clone());
            tokio::select! {
                () = read_forward(receiver, con.clone(), task.clone())  => {},
                _ = jdk::load_classes(&class_map, sender) => {}
            }
            Self::progress_end(&con.clone(), task);
        }

        if project_kind != ProjectKind::Unknown {
            let task = format!("Load {project_kind} dependencies");
            Self::progress_start(&con.clone(), task.clone());
            let (sender, receiver) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
                percentage: 0,
                error: false,
                message: "...".to_string(),
            });

            tokio::select! {
                () = read_forward(receiver, con.clone(), task.clone())  => {},
                () = fetch_deps(sender, project_kind.clone(), class_map.clone()) => {}
            }
            Self::progress_end(&con.clone(), task);
        }

        {
            let task = "Load project files".to_string();
            Self::progress_start(&con.clone(), task.clone());
            Self::progress_update_percentage(
                &con.clone(),
                task.clone(),
                "Load project paths".to_string(),
                None,
            );
            let project_classes = match project_kind {
                ProjectKind::Maven => maven::project::load_project_folders(),
                ProjectKind::Gradle {
                    path_build_gradle: _,
                } => gradle::project::load_project_folders(),
                ProjectKind::Unknown => vec![],
            };
            Self::progress_update_percentage(
                &con.clone(),
                task.clone(),
                "Initializing reference map".to_string(),
                None,
            );
            match references::init_reference_map(&project_classes, &class_map, &reference_map) {
                Ok(()) => (),
                Err(e) => eprintln!("Got reference error: {e:?}"),
            }
            Self::progress_update_percentage(
                &con.clone(),
                task.clone(),
                "Populating class map".to_string(),
                None,
            );
            for class in project_classes {
                class_map.insert(class.class_path.clone(), class);
            }
            Self::progress_end(&con.clone(), task);
        }

        Self::progress_end(&con, "Init".to_string());
    }

    pub fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_open(&TextDocumentItem {
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
            language_id: params.text_document.language_id,
        });
    }

    pub fn did_change(&self, params: &DidChangeTextDocumentParams) {
        self.on_change(params.text_document.uri.as_str(), &params.content_changes);
    }

    pub fn did_save(&self, params: &DidSaveTextDocumentParams) {
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
                    Ok(()) => {}
                    Err(e) => eprintln!("Got reference error: {e:?}"),
                }
                self.class_map.insert(class_path, class);
            }
            Err(e) => eprintln!("Save file parse error {e:?}"),
        }
    }

    pub fn hover(&self, params: HoverParams) -> Option<Hover> {
        let uri = params.text_document_position_params.text_document.uri;
        let document = self.document_map.get(&get_normal_path(&uri))?;
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
        let Some(mut document) = self.document_map.get_mut(&get_normal_path(&uri)) else {
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
        let Some(document) = self.document_map.get(&get_normal_path(&uri)) else {
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
        let Some(document) = self.document_map.get(&get_normal_path(&uri)) else {
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
        let Some(document) = self.document_map.get(&get_normal_path(&uri)) else {
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
                eprintln!("Got reference call_chain error: {e:?}");
                None
            }
        }
    }

    pub fn code_action(&self, params: CodeActionParams) -> Option<CodeActionResponse> {
        let Some(document) = self
            .document_map
            .get(&get_normal_path(&params.text_document.uri))
        else {
            eprintln!("Document is not opened.");
            return None;
        };
        let current_file = params.text_document.uri;
        let point = to_ast_point(params.range.start);

        let imports = imports::imports(&document.ast);

        let Some(class) = &self.class_map.get(&document.class_path) else {
            eprintln!("Could not find class {}", document.class_path);
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
            .get(&get_normal_path(&params.text_document.uri))
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
                let uri = source_to_uri(&i).ok()?;
                let class_source = document::read_document_or_open_class(
                    &i,
                    MyString::new(),
                    &self.document_map,
                    uri.as_str(),
                )
                .ok()?;
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
                let uri = Uri::from_str(&format!("file:///{path}")).ok()?;
                Some(position::symbols_to_document_symbols(&symbols, &uri))
            })
            .flatten()
            .collect();
        Some(WorkspaceSymbolResponse::Flat(symbols))
    }

    pub fn signature_help(&self, params: SignatureHelpParams) -> Option<SignatureHelp> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(document) = self.document_map.get(&get_normal_path(&uri)) else {
            eprintln!("Document is not opened.");
            return None;
        };
        let point = to_ast_point(params.text_document_position_params.position);
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

#[cfg(not(target_os = "windows"))]
fn get_normal_path(uri: &Uri) -> String {
    uri.path().as_str().to_owned()
}
#[cfg(target_os = "windows")]
fn get_normal_path(uri: &Uri) -> String {
    uri.path()
        .as_str()
        // remove leading slash
        .trim_start_matches("/")
        // url encoded colon
        .replacen("%3A", ":", 1)
}

pub async fn read_forward(
    mut rx: tokio::sync::watch::Receiver<TaskProgress>,
    con: Arc<Connection>,
    task: String,
) {
    tokio::spawn(async move {
        loop {
            if rx.changed().await.is_err() {
                break;
            }
            let i = rx.borrow();
            Backend::progress_update_percentage(
                &con.clone(),
                task.clone(),
                i.message.clone(),
                Some(i.percentage),
            );
        }
    })
    .await
    .expect("Forward failed");
}

pub async fn fetch_deps(
    sender: tokio::sync::watch::Sender<TaskProgress>,
    project_kind: ProjectKind,
    class_map: Arc<dashmap::DashMap<MyString, parser::dto::Class>>,
) {
    tokio::spawn(async move {
        match project_kind {
            ProjectKind::Maven => {
                match maven::fetch::fetch_deps(class_map, sender, true, true).await {
                    Ok(()) => (),
                    Err(e) => {
                        eprintln!("Got error while loading maven project: {e:?}");
                    }
                }
            }
            ProjectKind::Gradle {
                path_build_gradle: path,
            } => match gradle::fetch::fetch_deps(&class_map, path, sender).await {
                Ok(()) => (),
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
