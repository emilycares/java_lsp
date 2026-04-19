use lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionParams, CodeActionProviderCapability,
    CompletionOptions, CompletionParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, DocumentFormattingParams,
    DocumentLinkOptions, DocumentLinkParams, DocumentSymbolParams, ExecuteCommandOptions,
    ExecuteCommandParams, GotoDefinitionParams, HoverParams, HoverProviderCapability,
    InlayHintParams, OneOf, ReferenceParams, ServerCapabilities, SignatureHelpOptions,
    SignatureHelpParams, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    WorkDoneProgressOptions, WorkspaceSymbolParams,
    notification::{
        Cancel, DidChangeConfiguration, DidChangeTextDocument, DidCloseTextDocument,
        DidOpenTextDocument, DidSaveTextDocument, Notification, SetTrace,
    },
    request::{
        CodeActionRequest, Completion, DocumentLinkRequest, DocumentSymbolRequest, ExecuteCommand,
        Formatting, GotoDefinition, HoverRequest, InlayHintRequest, References, Request,
        SignatureHelpRequest, WorkspaceSymbolRequest,
    },
};

use lsp_server::{Message, RequestId, Response};
use serde_json::{Value, from_value, to_value};

use crate::{
    backend::Backend,
    command::{COMMAND_RELOAD_DEPENDENCIES, COMMAND_UPDATE_DEPENDENCIES},
};

pub fn get_server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                will_save: None,
                will_save_wait_until: None,
                save: Some(lsp_types::TextDocumentSyncSaveOptions::Supported(true)),
            },
        )),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
            ..CodeActionOptions::default()
        })),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![' '.to_string(), '.'.to_string(), '('.to_string()]),
            ..CompletionOptions::default()
        }),
        document_symbol_provider: Some(OneOf::Left(true)),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        // Not ready
        // document_formatting_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!['('.to_string(), ','.to_string(), '<'.to_string()]),
            ..Default::default()
        }),
        document_highlight_provider: None,
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec![
                COMMAND_RELOAD_DEPENDENCIES.to_owned(),
                COMMAND_UPDATE_DEPENDENCIES.to_owned(),
            ],
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: Some(true),
            },
        }),
        document_link_provider: Some(DocumentLinkOptions {
            resolve_provider: None,
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
        }),
        inlay_hint_provider: Some(OneOf::Left(true)),
        ..Default::default()
    }
}
pub fn route(backend: &Backend) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    loop {
        let Ok(msg) = backend.connection.receiver.recv() else {
            break;
        };
        match msg {
            Message::Request(req) => {
                if backend.connection.handle_shutdown(&req)? {
                    break;
                }

                match req.method.as_str() {
                    HoverRequest::METHOD => {
                        if let Ok(params) = from_value::<HoverParams>(req.params) {
                            let result = backend.hover(params);
                            send(backend, req.id, to_value(result).ok());
                        }
                    }
                    Formatting::METHOD => {
                        if let Ok(params) = from_value::<DocumentFormattingParams>(req.params) {
                            let result = backend.formatting(params);
                            send(backend, req.id, to_value(result).ok());
                        }
                    }
                    GotoDefinition::METHOD => {
                        if let Ok(params) = from_value::<GotoDefinitionParams>(req.params) {
                            let result = backend.goto_definition(params);
                            send(backend, req.id, to_value(result).ok());
                        }
                    }
                    Completion::METHOD => {
                        if let Ok(params) = from_value::<CompletionParams>(req.params) {
                            let result = backend.completion(params);
                            send(backend, req.id, to_value(result).ok());
                        }
                    }
                    References::METHOD => {
                        if let Ok(params) = from_value::<ReferenceParams>(req.params) {
                            let result = backend.references(params);
                            send(backend, req.id, to_value(result).ok());
                        }
                    }
                    CodeActionRequest::METHOD => {
                        if let Ok(params) = from_value::<CodeActionParams>(req.params) {
                            let result = backend.code_action(params);
                            send(backend, req.id, to_value(result).ok());
                        }
                    }
                    DocumentSymbolRequest::METHOD => {
                        if let Ok(params) = from_value::<DocumentSymbolParams>(req.params) {
                            let result = backend.document_symbol(params);
                            send(backend, req.id, to_value(result).ok());
                        }
                    }
                    WorkspaceSymbolRequest::METHOD => {
                        if let Ok(params) = from_value::<WorkspaceSymbolParams>(req.params) {
                            let result = backend.workspace_document_symbol(&params);
                            send(backend, req.id, to_value(result).ok());
                        }
                    }
                    SignatureHelpRequest::METHOD => {
                        if let Ok(params) = from_value::<SignatureHelpParams>(req.params) {
                            let result = backend.signature_help(params);
                            send(backend, req.id, to_value(result).ok());
                        }
                    }
                    ExecuteCommand::METHOD => {
                        if let Ok(params) = from_value::<ExecuteCommandParams>(req.params) {
                            let result = backend.execute_command(params);
                            send(backend, req.id, to_value(result).ok());
                        }
                    }
                    DocumentLinkRequest::METHOD => {
                        if let Ok(params) = from_value::<DocumentLinkParams>(req.params) {
                            let result = backend.document_link(params);
                            send(backend, req.id, to_value(result).ok());
                        }
                    }
                    InlayHintRequest::METHOD => {
                        if let Ok(params) = from_value::<InlayHintParams>(req.params) {
                            let result = backend.inlay_hint(params);
                            send(backend, req.id, to_value(result).ok());
                        }
                    }
                    r => {
                        eprintln!("Got unsupported request: {r}");
                    }
                }
            }
            Message::Response(resp) => {
                eprintln!("got response: {resp:?}");
            }
            Message::Notification(not) => match not.method.as_str() {
                DidOpenTextDocument::METHOD => {
                    if let Ok(params) = from_value::<DidOpenTextDocumentParams>(not.params) {
                        backend.did_open(&params);
                    }
                }
                DidCloseTextDocument::METHOD => {
                    if let Ok(params) = from_value::<DidCloseTextDocumentParams>(not.params) {
                        backend.did_close(&params);
                    }
                }
                DidChangeTextDocument::METHOD => {
                    if let Ok(params) = from_value::<DidChangeTextDocumentParams>(not.params) {
                        backend.did_change(&params);
                    }
                }
                DidSaveTextDocument::METHOD => {
                    if let Ok(params) = from_value::<DidSaveTextDocumentParams>(not.params) {
                        backend.did_save(&params);
                    }
                }
                DidChangeConfiguration::METHOD | SetTrace::METHOD | Cancel::METHOD => {}
                r => {
                    eprintln!("Got unsupported notification: {r}");
                }
            },
        }
    }
    Ok(())
}

fn send(backend: &Backend, id: RequestId, result: Option<Value>) {
    let _ = backend.connection.sender.send(Message::Response(Response {
        id,
        result,
        error: None,
    }));
}
