#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
mod backend;
mod codeaction;
pub mod command;
pub mod completion;
mod definition;
mod hover;
pub mod references;
mod router;
pub mod signature;

use std::sync::Arc;

use lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionProviderCapability, CompletionOptions,
    ExecuteCommandOptions, HoverProviderCapability, InitializeParams, OneOf, ServerCapabilities,
    SignatureHelpOptions, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions,
};

use lsp_server::{Connection, IoThreads};

use crate::{backend::Backend, command::COMMAND_RELOAD_DEPENDENCIES};

/// Accept connection over stdio
///
/// # Panics
/// When it could not init project
pub fn stdio() -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();
    main(connection, io_threads)
}

/// Server main
///
/// # Panics
/// When it could not init project
pub fn main(
    connection: Connection,
    io_threads: IoThreads,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let project_kind = common::project_kind::get_project_kind();
    if let Err(e) = project_kind {
        eprintln!("Error with project init: {e:?}");
        std::process::exit(1);
    }
    let project_kind = project_kind.expect("Program should already have exited");
    eprintln!("Start java_lsp with project_kind: {project_kind:?}");
    let backend = Backend::new(connection, project_kind);

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                will_save: None,
                will_save_wait_until: None,
                save: None,
            },
        )),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
            ..CodeActionOptions::default()
        })),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some([' ', '.', '('].iter().map(ToString::to_string).collect()),
            ..CompletionOptions::default()
        }),
        document_symbol_provider: Some(OneOf::Left(true)),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        // Not ready
        // document_formatting_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".to_owned(), ",".to_owned(), "<".to_owned()]),
            ..Default::default()
        }),
        document_highlight_provider: None,
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec![COMMAND_RELOAD_DEPENDENCIES.to_owned()],
            work_done_progress_options: lsp_types::WorkDoneProgressOptions {
                work_done_progress: Some(true),
            },
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
        serde_json::from_value(initialization_params).unwrap_or_default();
    main_loop(backend, params)?;
    io_threads.join()?;

    // Shut down gracefully.
    eprintln!("shutting down server");
    Ok(())
}

fn main_loop(
    mut backend: Backend,
    params: InitializeParams,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    backend.client_capabilities = Arc::new(Some(params.capabilities));
    let connection = backend.connection.clone();
    let project_kind = backend.project_kind.clone();
    let class_map = backend.class_map.clone();
    let reference_map = backend.reference_map.clone();
    tokio::spawn(async move {
        Backend::initialized(
            params.work_done_progress_params.work_done_token,
            connection,
            project_kind,
            class_map.clone(),
            reference_map,
        )
        .await;
    });
    router::route(&backend)?;
    Ok(())
}
