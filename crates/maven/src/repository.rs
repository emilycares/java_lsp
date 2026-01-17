use std::path::Path;

use crate::{pom, settings};

#[derive(Debug)]
pub enum RepositoryError {
    Settings(settings::M2SettingsError),
    Pom(pom::PomError),
}

#[derive(Clone)]
pub struct Repository {
    pub url: String,
    pub credentials: Option<RepositoryCredentials>,
}
#[derive(Clone)]
pub struct RepositoryCredentials {
    pub username: String,
    pub password: String,
}

#[must_use]
pub fn central() -> Repository {
    Repository {
        url: "https://repo.maven.apache.org/maven2/".to_owned(),
        credentials: None,
    }
}

pub fn load_repositories(
    m2_folder: &Path,
    project_dir: &Path,
) -> Result<Vec<Repository>, RepositoryError> {
    let mut out = Vec::new();

    let project = pom::load_pom_xml(project_dir).map_err(RepositoryError::Pom)?;

    let Some(repositories) = project.repositories else {
        return Ok(out);
    };
    if repositories.repository.is_empty() {
        return Ok(out);
    }

    let se = settings::load_settings_xml(m2_folder).map_err(RepositoryError::Settings)?;
    let Some(servers) = se.servers else {
        return Ok(out);
    };

    for repo in repositories.repository {
        let mut url = repo.url;
        if !url.ends_with('/') {
            url.push('/');
        }
        let Some(s) = servers.server.iter().find(|i| i.id == repo.id) else {
            out.push(Repository {
                url,
                credentials: None,
            });
            continue;
        };
        out.push(Repository {
            url,
            credentials: Some(RepositoryCredentials {
                username: s.username.clone(),
                password: s.password.clone(),
            }),
        });
    }

    out.push(central());

    Ok(out)
}
