use std::path::Path;

use serde::Deserialize;

#[derive(Debug)]
pub enum PomError {
    IO(std::io::Error),
    Xml(serde_xml_rs::Error),
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename = "project")]
pub struct Pom {
    pub repositories: Option<PomRepositories>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct PomRepositories {
    pub repository: Vec<PomRepository>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct PomRepository {
    pub id: String,
    pub url: String,
}

pub fn load_pom_xml(project_dir: &Path) -> Result<Pom, PomError> {
    let path = project_dir.join("pom.xml");
    let file = std::fs::File::open(path).map_err(PomError::IO)?;
    serde_xml_rs::from_reader(file).map_err(PomError::Xml)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::pom::{Pom, PomRepositories, PomRepository};

    #[test]
    fn load() {
        let content = "
        <project>
            <repositories>
                <repository>
                    <id>org-public</id>
                    <url>https://repo.org.eu/public</url>
                </repository>
                <repository>
                    <id>org-private</id>
                    <url>https://repo.org.eu/org</url>
                </repository>
            </repositories>
        </project>
        ";
        let expect = Pom {
            repositories: Some(PomRepositories {
                repository: vec![
                    PomRepository {
                        id: "org-public".to_string(),
                        url: "https://repo.org.eu/public".to_string(),
                    },
                    PomRepository {
                        id: "org-private".to_string(),
                        url: "https://repo.org.eu/org".to_string(),
                    },
                ],
            }),
        };

        let out: Pom = serde_xml_rs::from_str(content).unwrap();

        assert_eq!(out, expect);
    }

    #[test]
    fn no_repositories() {
        let content = "
        <project>
        </project>
        ";
        let expect = Pom { repositories: None };

        let out: Pom = serde_xml_rs::from_str(content).unwrap();

        assert_eq!(out, expect);
    }
}
