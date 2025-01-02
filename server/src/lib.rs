mod call_chain;
mod codeaction;
pub mod completion;
mod definition;
pub mod document;
mod hover;
mod imports;
mod position;
mod tyres;
mod utils;
mod variable;

use std::collections::HashMap;
use std::path::PathBuf;

use common::compile::CompileError;
use common::project_kind::ProjectKind;
use dashmap::{DashMap, DashSet};
use document::Document;
use notification::Progress;
use parser::dto::Class;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
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
        document.apply_text_changes(&changes);
    }

    async fn on_open(&self, params: TextDocumentItem) {
        let rope = ropey::Rope::from_str(&params.text);
        let key = params.uri.to_string();
        if let Some(mut document) = self.document_map.get_mut(&key) {
            document.replace_text(rope);
        } else {
            self.document_map
                .insert(key, Document::setup_rope(&params.text, rope).unwrap());
        }
    }

    fn _get_opened_document(
        &self,
        uri: &str,
    ) -> Option<dashmap::mapref::one::Ref<'_, std::string::String, Document>> {
        // when file is open
        if let Some(document) = self.document_map.get(uri) {
            return Some(document);
        };
        None
    }

    async fn get_document(
        &self,
        uri: Url,
    ) -> Option<dashmap::mapref::one::Ref<'_, std::string::String, Document>> {
        // when file is open
        if let Some(document) = self._get_opened_document(uri.as_str()) {
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
        if let Some(document) = self._get_opened_document(uri.as_str()) {
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
            if let Ok(uri) = Url::parse(&format!("file:/{}", path)) {
                if let Some(errs) = emap.get(path) {
                    let errs: Vec<Diagnostic> = errs
                        .iter()
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
                ProjectKind::Unknown => None,
            };
            if let Some(cm) = cm {
                for pair in cm.into_iter() {
                    self.class_map.insert(pair.0, pair.1);
                }
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

        self.client
            .send_notification::<Progress>(ProgressParams {
                token: ProgressToken::String("Load project files".to_string()),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                    WorkDoneProgressBegin {
                        title: "Load project files".to_string(),
                        cancellable: None,
                        message: None,
                        percentage: None,
                    },
                )),
            })
            .await;

        let project_classes = match self.project_kind {
            ProjectKind::Maven => maven::project::load_project_folders(),
            ProjectKind::Gradle => vec![],
            ProjectKind::Unknown => vec![],
        };
        for class in project_classes {
            self.class_map.insert(class.class_path.clone(), class);
        }

        self.client
            .send_notification::<Progress>(ProgressParams {
                token: ProgressToken::String("Load project files".to_string()),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message: None,
                })),
            })
            .await;

        eprintln!("Init done");
    }

    async fn shutdown(&self) -> Result<()> {
        panic!("Stop");
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_open(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.text_document.text,
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
        self.publish_compile_errors(errors).await;

        let Some(document) = self.get_document(params.text_document.uri.clone()).await else {
            eprintln!("no doc found");
            return;
        };
        dbg!(&path);
        if let Some(class) =
            parser::update_project_java_file(PathBuf::from(path), document.as_bytes())
        {
            eprintln!(
                "save class: {} {} {}",
                class.name, class.class_path, class.source
            );
            self.class_map.insert(class.class_path.clone(), class);
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(document) = self.get_document(uri).await else {
            return Ok(None);
        };
        let point = to_treesitter_point(params.text_document_position_params.position);
        let imports = imports::imports(document.value());

        let class_hover = hover::class(document.value(), &point, &imports, &self.class_map);
        return Ok(class_hover);
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let Some(document) = self.get_document(uri).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
        };
        let Some(lines) = document.text.lines().len().try_into().ok() else {
            return Ok(None);
        };
        let Some(text) = format::format(document.text.to_string(), format::Formatter::Topiary)
        else {
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
        let Some(document) = self.get_document(uri).await else {
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
        let Some(document) = self.get_document(uri).await else {
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
        let Some(document) = self.get_document(params.text_document.uri.clone()).await else {
            eprintln!("Document is not opened.");
            return Ok(None);
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
            return Ok(Some(imps));
        }

        Ok(None)
    }
}
