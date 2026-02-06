use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use common::{Dependency, TaskProgress, deps_dir};
use my_string::smol_str::ToSmolStr;
use parser::SourceDestination;
use reqwest::{Client, StatusCode};
use tokio::task::JoinSet;

use crate::{
    m2::{MTwoError, get_maven_m2_folder, pom_classes_jar, pom_m2_sha1},
    repository::Repository,
};
use crate::{
    m2::{PomMTwo, pom_m2, pom_sources_jar},
    metadata,
};

#[derive(Debug)]
pub enum MavenUpdateError {
    MTwo(MTwoError),
    ClientBuilder(reqwest::Error),
    ReqBuilder(reqwest::Error),
    Request(reqwest::Error),
    ShaBody(reqwest::Error),
    JarBody(reqwest::Error),
    WriteHash(std::io::Error),
    WriteJar(std::io::Error),
    CreateDir(std::io::Error),
    WriteEtag(std::io::Error),
}

pub async fn update(
    repos: Arc<Vec<Repository>>,
    tree: &[Dependency],
    sender: tokio::sync::watch::Sender<TaskProgress>,
) -> Result<(), MavenUpdateError> {
    let client = reqwest::Client::builder()
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .map_err(MavenUpdateError::ClientBuilder)?;
    let client = Arc::new(client);
    let m2 = get_maven_m2_folder().map_err(MavenUpdateError::MTwo)?;
    let mut handles = JoinSet::new();
    let tasks_number = u32::try_from(tree.len() + 1).unwrap_or(1);
    let completed_number = Arc::new(AtomicU32::new(0));
    let deps_path = deps_dir();

    for pom in tree {
        let deps_bas = Arc::new(deps_base(pom, &deps_path));
        let pom_mtwo = Arc::new(pom_m2(pom, &m2));
        let one = stage_one(pom, &deps_bas, &pom_mtwo);
        let mut ignore_etag = false;
        let f_source = Arc::new(pom_sources_jar(pom, &pom_mtwo));
        let d_source = Arc::new(deps_get_source(&deps_bas));
        match one {
            UpdateStateOne::WasUpdated | UpdateStateOne::JarNotFound => ignore_etag = true,
            UpdateStateOne::NoOwnHash
            | UpdateStateOne::CheckUpdate
            | UpdateStateOne::SourceNotFound => (),
            UpdateStateOne::FailedToReadSha
            | UpdateStateOne::FailedToReadOwnHash
            | UpdateStateOne::FailedToReadJar => continue,
        }

        let pom = Arc::new(pom.to_owned());
        let jar = Arc::new(pom_classes_jar(&pom, &pom_mtwo));
        let mut found = false;
        for repo in repos.as_ref() {
            {
                let repo = Arc::new(repo.clone());
                let source_url = pom_source_jar_url(&pom, &repo.url);
                if matches!(one, UpdateStateOne::SourceNotFound) {
                    let fetched = fetch_extract_source(
                        f_source.clone(),
                        pom_mtwo.clone(),
                        d_source.clone(),
                        deps_bas.clone(),
                        client.clone(),
                        repo.clone(),
                        &source_url,
                    )
                    .await;
                    if fetched {
                        break;
                    }
                }
            }
            let jar_url = pom_jar_url(&pom, &repo.url);
            let mut two = stage_two(
                pom.clone(),
                pom_mtwo.clone(),
                repo,
                jar.clone(),
                &deps_bas,
                ignore_etag,
                &client,
                &jar_url,
            )
            .await;

            let mut source_url = pom_source_jar_url(&pom, &repo.url);

            if matches!(two, Ok(UpdateStateTwo::NotFound)) && pom.version.ends_with("SNAPSHOT") {
                let url = pom_snapshot_maven_metadata_xml_url(&pom, &repo.url);
                let mut builder = client.get(url);
                if let Some(cred) = &repo.credentials {
                    builder =
                        builder.basic_auth(cred.username.clone(), Some(cred.password.clone()));
                }
                if let Ok(req) = builder.build()
                    && let Ok(resp) = client.execute(req).await
                    && resp.status().is_success()
                    && let Ok(content) = resp.text().await
                    && let Ok(info) = metadata::get_metadata_info(&content, &pom.artivact_id)
                    && let Some(classes) = info.classes
                {
                    two = stage_two(
                        pom.clone(),
                        pom_mtwo.clone(),
                        repo,
                        jar.clone(),
                        &deps_bas,
                        ignore_etag,
                        &client,
                        &pom_url_base(&pom, &repo.url, &classes),
                    )
                    .await;
                    if let Some(src) = info.source {
                        source_url = pom_url_base(&pom, &repo.url, &src);
                    }
                }
            }

            let a = completed_number.fetch_add(1, Ordering::Relaxed);
            let _ = sender.send(TaskProgress {
                percentage: (100 * a) / (tasks_number + 1),
                error: false,
                message: pom.artivact_id.clone(),
            });
            let repo = Arc::new(repo.clone());
            let res = handle_repo_retry(
                &mut handles,
                two,
                deps_bas.clone(),
                jar.clone(),
                d_source.clone(),
                f_source.clone(),
                pom.clone(),
                pom_mtwo.clone(),
                repo,
                &client,
                source_url,
            );

            if res {
                found = true;
            } else {
                break;
            }
        }
        if !found && matches!(one, UpdateStateOne::WasUpdated) {
            handles.spawn(async move {
                index_jar(pom, &deps_bas, &jar, &d_source).await;
            });
        }
    }
    let _ = handles.join_all().await;

    Ok(())
}

/// Returns true if the `Pom` is not found in repo
#[allow(clippy::too_many_arguments)]
fn handle_repo_retry(
    handles: &mut JoinSet<()>,
    two: Result<UpdateStateTwo, MavenUpdateError>,
    deps_bas: Arc<PathBuf>,
    jar: Arc<PathBuf>,
    d_source: Arc<PathBuf>,
    f_source: Arc<PathBuf>,
    pom: Arc<Dependency>,
    pom_mtwo: Arc<PomMTwo>,
    repo: Arc<Repository>,
    client: &Arc<Client>,
    source_url: String,
) -> bool {
    match two {
        Ok(UpdateStateTwo::Updated) => {
            {
                let d_source = d_source.clone();
                let deps_bas = deps_bas.clone();
                let client = client.clone();
                handles.spawn(async move {
                    fetch_extract_source(
                        f_source,
                        pom_mtwo,
                        d_source,
                        deps_bas,
                        client,
                        repo,
                        &source_url,
                    )
                    .await;
                });
            }
            handles.spawn(async move {
                index_jar(pom, &deps_bas, &jar, &d_source).await;
            });
        }
        Ok(UpdateStateTwo::AlreadyLatest) => {
            let cfc = deps_get_cfc(&deps_bas, &pom);
            if !cfc.exists() {
                handles.spawn(async move {
                    index_jar(pom, &deps_bas, &jar, &d_source).await;
                });
            }
        }
        Ok(UpdateStateTwo::NotFound) => return true,
        Err(e) => eprintln!("Got error: {e:?}"),
    }
    false
}

async fn fetch_extract_source(
    f_source: Arc<PathBuf>,
    pom_mtwo: Arc<PathBuf>,
    d_source: Arc<PathBuf>,
    deps_bas: Arc<PathBuf>,
    client: Arc<Client>,
    repo: Arc<Repository>,
    url: &str,
) -> bool {
    match fetch_source(&pom_mtwo, &repo, &f_source, &deps_bas, &client, url).await {
        Ok(UpdateStateSource::Updated) => {
            let _ = tokio::fs::remove_dir(&d_source.as_path()).await;
            eprintln!("Extract: {}", f_source.display());
            match zip_util::extract_jar(&f_source, &d_source).await {
                Ok(()) => (),
                Err(e) => eprintln!("unable to extract jar {e:?}"),
            }
            return true;
        }
        Ok(UpdateStateSource::NotFound) => {}
        Err(e) => eprintln!("Get error: {e:?}"),
    }
    false
}

async fn index_jar(pom: Arc<Dependency>, deps_bas: &PathBuf, jar: &PathBuf, d_source: &DepsSource) {
    let Some(source) = d_source.as_path().to_str() else {
        return;
    };
    match loader::load_classes_jar(
        jar,
        SourceDestination::RelativeInFolder(source.to_smolstr()),
    )
    .await
    {
        Ok(classes) => {
            let cfc = deps_get_cfc(deps_bas, &pom);
            if let Err(e) = loader::save_class_folder(&cfc, &classes) {
                eprintln!("Failed to save cache for {}, {e:?}", cfc.display());
            }
        }
        Err(e) => eprintln!("Get error: {e:?}"),
    }
}

#[derive(Debug)]
pub enum UpdateStateOne {
    NoOwnHash,
    WasUpdated,
    CheckUpdate,
    JarNotFound,
    SourceNotFound,
    FailedToReadSha,
    FailedToReadOwnHash,
    FailedToReadJar,
}

#[must_use]
pub fn stage_one(pom: &Dependency, deps_bas: &DepsBas, pom_mtwo: &PomMTwo) -> UpdateStateOne {
    let own_hash = deps_get_hash(deps_bas, pom);
    if !own_hash.exists() {
        return UpdateStateOne::NoOwnHash;
    }
    let source = deps_get_source(deps_bas);
    if !source.exists() {
        return UpdateStateOne::SourceNotFound;
    }
    let jar = pom_classes_jar(pom, pom_mtwo);
    if !jar.exists() {
        return UpdateStateOne::JarNotFound;
    }
    let sha1 = pom_m2_sha1(pom, pom_mtwo);
    if sha1.exists() {
        let Ok(sha_content) = std::fs::read_to_string(&sha1) else {
            return UpdateStateOne::FailedToReadSha;
        };
        let Ok(sha_own_hash) = std::fs::read_to_string(&own_hash) else {
            return UpdateStateOne::FailedToReadOwnHash;
        };
        let check2 = sha_own_hash == sha_content;
        if check2 {
            UpdateStateOne::CheckUpdate
        } else {
            UpdateStateOne::WasUpdated
        }
    } else {
        let Ok(sha_own_hash) = std::fs::read_to_string(&own_hash) else {
            return UpdateStateOne::FailedToReadOwnHash;
        };
        let Ok(jar) = std::fs::read(jar) else {
            return UpdateStateOne::FailedToReadJar;
        };
        let digest = jar_sha1(&jar);
        let check2 = sha_own_hash == digest;
        if check2 {
            UpdateStateOne::CheckUpdate
        } else {
            UpdateStateOne::WasUpdated
        }
    }
}

#[derive(Debug)]
pub enum UpdateStateTwo {
    Updated,
    AlreadyLatest,
    NotFound,
}

#[allow(clippy::too_many_arguments)]
pub async fn stage_two(
    pom: Arc<Dependency>,
    pom_mtwo: Arc<PomMTwo>,
    repo: &Repository,
    jar: Arc<PathBuf>,
    deps_bas: &DepsBas,
    ignore_etag: bool,
    client: &Arc<Client>,
    jar_url: &str,
) -> Result<UpdateStateTwo, MavenUpdateError> {
    eprintln!("Fetch jar: {jar_url}");
    let mut builder = client.get(jar_url);
    if let Some(cred) = &repo.credentials {
        builder = builder.basic_auth(cred.username.clone(), Some(cred.password.clone()));
    }
    tokio::fs::create_dir_all(deps_bas)
        .await
        .map_err(MavenUpdateError::CreateDir)?;
    tokio::fs::create_dir_all(pom_mtwo.as_ref())
        .await
        .map_err(MavenUpdateError::CreateDir)?;
    let etag = deps_get_etag(deps_bas, &pom);
    if !ignore_etag
        && etag.exists()
        && let Ok(etag) = fs::read_to_string(&etag)
    {
        builder = builder.header("If-None-Match", etag);
    }
    let req = builder.build().map_err(MavenUpdateError::ReqBuilder)?;

    let resp = client
        .execute(req)
        .await
        .map_err(MavenUpdateError::Request)?;

    if resp.status() == StatusCode::NOT_MODIFIED {
        return Ok(UpdateStateTwo::AlreadyLatest);
    }
    if resp.status() == StatusCode::NOT_FOUND || !resp.status().is_success() {
        return Ok(UpdateStateTwo::NotFound);
    }
    let hash = deps_get_hash(deps_bas, &pom);
    let sha1 = pom_m2_sha1(&pom, &pom_mtwo);
    if let Some(sha) = resp.headers().get("etag") {
        tokio::fs::write(etag, sha)
            .await
            .map_err(MavenUpdateError::WriteEtag)?;
    }
    if let Some(sha) = resp.headers().get("x-checksum-sha1") {
        tokio::fs::write(hash, sha)
            .await
            .map_err(MavenUpdateError::WriteHash)?;
        tokio::fs::write(sha1, sha)
            .await
            .map_err(MavenUpdateError::WriteHash)?;
    } else {
        let mut builder = client.get(pom_jar_sha_url(&pom, &repo.url));
        if let Some(cred) = &repo.credentials {
            builder = builder.basic_auth(cred.username.clone(), Some(cred.password.clone()));
        }
        let req = builder.build().map_err(MavenUpdateError::ReqBuilder)?;
        let resp = client
            .execute(req)
            .await
            .map_err(MavenUpdateError::Request)?;

        let contents = resp.bytes().await.map_err(MavenUpdateError::ShaBody)?;
        tokio::fs::write(hash, &contents)
            .await
            .map_err(MavenUpdateError::WriteHash)?;
        tokio::fs::write(sha1, contents)
            .await
            .map_err(MavenUpdateError::WriteHash)?;
    }
    let contents = &resp.bytes().await.map_err(MavenUpdateError::JarBody)?;
    tokio::fs::write(jar.as_ref(), contents)
        .await
        .map_err(MavenUpdateError::WriteJar)?;

    Ok(UpdateStateTwo::Updated)
}

#[derive(Debug)]
pub enum UpdateStateSource {
    Updated,
    NotFound,
}
pub async fn fetch_source(
    pom_mtwo: &PomMTwo,
    repo: &Repository,
    source: &DepsSource,
    deps_bas: &DepsBas,
    client: &Arc<Client>,
    url: &str,
) -> Result<UpdateStateSource, MavenUpdateError> {
    let mut builder = client.get(url);
    if let Some(cred) = &repo.credentials {
        builder = builder.basic_auth(cred.username.clone(), Some(cred.password.clone()));
    }
    tokio::fs::create_dir_all(deps_bas)
        .await
        .map_err(MavenUpdateError::CreateDir)?;
    tokio::fs::create_dir_all(pom_mtwo)
        .await
        .map_err(MavenUpdateError::CreateDir)?;
    let req = builder.build().map_err(MavenUpdateError::ReqBuilder)?;

    let resp = client
        .execute(req)
        .await
        .map_err(MavenUpdateError::Request)?;

    if resp.status() == StatusCode::NOT_FOUND || !resp.status().is_success() {
        return Ok(UpdateStateSource::NotFound);
    }
    let contents = &resp.bytes().await.map_err(MavenUpdateError::JarBody)?;
    tokio::fs::write(source, contents)
        .await
        .map_err(MavenUpdateError::WriteJar)?;

    Ok(UpdateStateSource::Updated)
}

fn pom_jar_url(pom: &Dependency, repo: &str) -> String {
    format!(
        "{repo}{}/{}/{}/{}-{}.jar",
        pom.group_id.replace('.', "/"),
        pom.artivact_id,
        pom.version,
        pom.artivact_id,
        pom.version
    )
}
fn pom_snapshot_maven_metadata_xml_url(pom: &Dependency, repo: &str) -> String {
    format!(
        "{repo}{}/{}/{}/maven-metadata.xml",
        pom.group_id.replace('.', "/"),
        pom.artivact_id,
        pom.version,
    )
}
fn pom_source_jar_url(pom: &Dependency, repo: &str) -> String {
    format!(
        "{repo}{}/{}/{}/{}-{}-sources.jar",
        pom.group_id.replace('.', "/"),
        pom.artivact_id,
        pom.version,
        pom.artivact_id,
        pom.version
    )
}
fn pom_url_base(pom: &Dependency, repo: &str, suffix: &str) -> String {
    format!(
        "{}{}/{}/{}/{}",
        repo,
        pom.group_id.replace('.', "/"),
        pom.artivact_id,
        pom.version,
        suffix
    )
}
fn pom_jar_sha_url(pom: &Dependency, repo: &str) -> String {
    format!(
        "{repo}{}/{}/{}/{}-{}.jar.sha1",
        pom.group_id.replace('.', "/"),
        pom.artivact_id,
        pom.version,
        pom.artivact_id,
        pom.version
    )
}

fn jar_sha1(jar: &[u8]) -> String {
    let mut m = sha1_smol::Sha1::new();
    m.update(jar);
    m.digest().to_string()
}
type DepsSource = PathBuf;
#[must_use]
pub fn deps_get_source(deps_bas: &DepsBas) -> DepsSource {
    deps_bas.join("source")
}

type DepsHash = PathBuf;
#[must_use]
pub fn deps_get_hash(deps_bas: &DepsBas, pom: &Dependency) -> DepsHash {
    let mut p = deps_bas.join("a");

    let file_name = format!("{}-{}.hash", pom.artivact_id, pom.version);
    p.set_file_name(file_name);

    p
}
type DepsCFC = PathBuf;
#[must_use]
pub fn deps_get_cfc(deps_bas: &DepsBas, pom: &Dependency) -> DepsCFC {
    let mut p = deps_bas.join("a");

    let file_name = format!("{}-{}.cfc", pom.artivact_id, pom.version);
    p.set_file_name(file_name);

    p
}

type DepsEtag = PathBuf;
#[must_use]
pub fn deps_get_etag(deps_bas: &DepsBas, pom: &Dependency) -> DepsEtag {
    let mut p = deps_bas.join("a");

    let file_name = format!("{}-{}.etag", pom.artivact_id, pom.version);
    p.set_file_name(file_name);

    p
}

pub type DepsBas = PathBuf;
#[must_use]
pub fn deps_base(pom: &Dependency, deps_path: &Path) -> DepsBas {
    let group_parts = pom.group_id.split('.');
    let mut p = deps_path.to_path_buf();
    for gp in group_parts {
        p = p.join(gp);
    }
    p = p.join(&pom.artivact_id).join(&pom.version);
    p
}
