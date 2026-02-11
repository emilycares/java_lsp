// #![deny(warnings)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::implicit_hasher)]
mod backend;
mod codeaction;
pub mod command;
pub mod completion;
mod definition;
mod hover;
pub mod references;
mod router;
pub mod signature;

use std::{ffi::OsString, path::PathBuf, sync::Arc};

use lsp_server::{Connection, IoThreads};
use lsp_types::InitializeParams;

use crate::{backend::Backend, router::get_server_capabilities};

/// Accept connection over stdio
///
/// # Panics
/// When it could not init project
pub fn stdio() -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let Ok(project_dir) = std::env::current_dir() else {
        return Ok(());
    };
    let Some(path) = std::env::var_os("PATH") else {
        return Ok(());
    };
    let (connection, io_threads) = Connection::stdio();
    main(connection, io_threads, project_dir, path)
}

/// Server main
///
/// # Panics
/// When it could not init project
pub fn main(
    connection: Connection,
    io_threads: IoThreads,
    project_dir: PathBuf,
    path: OsString,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let project_kind = common::project_kind::get_project_kind(&project_dir, &path);
    if let Err(e) = project_kind {
        eprintln!("Error with project init: {e:?}");
        std::process::exit(1);
    }
    let project_kind = project_kind.expect("Program should already have exited");
    eprintln!("Start java_lsp with project_kind: {project_kind:?}");
    let backend = Backend::new(connection, project_kind, project_dir);

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(get_server_capabilities()).unwrap_or_default();
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
    main_loop(backend, params, path)?;
    io_threads.join()?;

    // Shut down gracefully.
    eprintln!("shutting down server");
    Ok(())
}

fn main_loop(
    mut backend: Backend,
    params: InitializeParams,
    path: OsString,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    backend.client_capabilities = Arc::new(Some(params.capabilities));
    let connection = backend.connection.clone();
    let project_kind = backend.project_kind.clone();
    let class_map = backend.class_map.clone();
    let reference_map = backend.reference_map.clone();
    let project_dir = backend.project_dir.clone();
    tokio::spawn(async move {
        Backend::initialized(
            params.work_done_progress_params.work_done_token,
            connection,
            project_kind,
            &class_map,
            reference_map,
            &project_dir,
            &path,
        )
        .await;
    });
    router::route(&backend)?;
    Ok(())
}
