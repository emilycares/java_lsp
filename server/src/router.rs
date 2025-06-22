use lsp_types::ReferenceParams;
use lsp_types::request::{References, SignatureHelpRequest};
use lsp_types::{
    CodeActionParams, CompletionParams, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, DocumentFormattingParams, DocumentSymbolParams,
    GotoDefinitionParams, HoverParams, SignatureHelpParams, WorkspaceSymbolParams,
    notification::{DidChangeTextDocument, DidOpenTextDocument, DidSaveTextDocument, Notification},
    request::{
        CodeActionRequest, Completion, DocumentSymbolRequest, Formatting, GotoDefinition,
        HoverRequest, Request, WorkspaceSymbolRequest,
    },
};

use lsp_server::{Message, Response};

use crate::backend::Backend;

pub fn route(backend: Backend) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    for msg in &backend.connection.receiver {
        match msg {
            Message::Request(req) => {
                if backend.connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                match req.method.as_str() {
                    HoverRequest::METHOD => {
                        if let Ok(params) = serde_json::from_value::<HoverParams>(req.params) {
                            let result = backend.hover(params);
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
                            let result = backend.formatting(params);
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
                            let result = backend.goto_definition(params);
                            let _ = backend.connection.sender.send(Message::Response(Response {
                                id: req.id,
                                result: serde_json::to_value(result).ok(),
                                error: None,
                            }));
                        }
                    }
                    Completion::METHOD => {
                        if let Ok(params) = serde_json::from_value::<CompletionParams>(req.params) {
                            let result = backend.completion(params);
                            let _ = backend.connection.sender.send(Message::Response(Response {
                                id: req.id,
                                result: serde_json::to_value(result).ok(),
                                error: None,
                            }));
                        }
                    }
                    References::METHOD => {
                        if let Ok(params) = serde_json::from_value::<ReferenceParams>(req.params) {
                            let result = backend.referneces(params);
                            let _ = backend.connection.sender.send(Message::Response(Response {
                                id: req.id,
                                result: serde_json::to_value(result).ok(),
                                error: None,
                            }));
                        }
                    }
                    CodeActionRequest::METHOD => {
                        if let Ok(params) = serde_json::from_value::<CodeActionParams>(req.params) {
                            let result = backend.code_action(params);
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
                            let result = backend.document_symbol(params);
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
                            let result = backend.workspace_document_symbol(&params);
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
                            let result = backend.signature_help(params);
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
                            backend.did_save(params);
                        }
                    }
                    _ => {}
                };
            }
        }
    }
    Ok(())
}
