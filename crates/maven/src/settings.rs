use serde::Deserialize;
use std::path::Path;

#[derive(Debug)]
pub enum M2SettingsError {
    IO(std::io::Error),
    Xml(serde_xml_rs::Error),
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename = "settings")]
pub struct M2Settings {
    pub servers: Option<M2Servers>,
    pub mirrors: Option<M2Mirrors>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct M2Servers {
    pub server: Vec<M2Server>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct M2Server {
    pub id: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct M2Mirrors {
    pub mirror: Vec<M2Mirror>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct M2Mirror {
    pub id: String,
    #[serde(rename = "mirrorOf")]
    pub mirror_of: String,
    pub url: String,
}

pub fn load_settings_xml(m2_folder: &Path) -> Result<M2Settings, M2SettingsError> {
    let path = m2_folder.join("settings.xml");
    let file = std::fs::File::open(path).map_err(M2SettingsError::IO)?;
    serde_xml_rs::from_reader(file).map_err(M2SettingsError::Xml)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::settings::{M2Mirror, M2Mirrors, M2Server, M2Servers, M2Settings};

    #[test]
    fn load() {
        let content = "
        <settings>
          <servers>
            <server>
              <id>org-private</id>
              <username>user</username>
              <password>123</password>
            </server>
            <server>
              <id>org-public</id>
              <username>user</username>
              <password>123</password>
            </server>
          </servers>
        </settings>
        ";
        let expect = M2Settings {
            servers: Some(M2Servers {
                server: vec![
                    M2Server {
                        id: "org-private".to_string(),
                        username: "user".to_string(),
                        password: "123".to_string(),
                    },
                    M2Server {
                        id: "org-public".to_string(),
                        username: "user".to_string(),
                        password: "123".to_string(),
                    },
                ],
            }),
            mirrors: None,
        };

        let out: M2Settings = serde_xml_rs::from_str(content).unwrap();

        assert_eq!(out, expect);
    }

    #[test]
    fn load_mirrors() {
        let content = "
        <settings>
          <mirrors>
            <mirror>
              <id>org-public</id>
              <mirrorOf>*</mirrorOf>
              <url>https://org.url</url>
            </mirror>
          </mirrors>
        </settings>
        ";
        let expect = M2Settings {
            servers: None,
            mirrors: Some(M2Mirrors {
                mirror: vec![M2Mirror {
                    id: "org-public".to_string(),
                    mirror_of: "*".to_string(),
                    url: "https://org.url".to_string(),
                }],
            }),
        };

        let out: M2Settings = serde_xml_rs::from_str(content).unwrap();

        assert_eq!(out, expect);
    }

    #[test]
    fn no_servers() {
        let content = "
        <settings>
        </settings>
        ";
        let expect = M2Settings {
            servers: None,
            mirrors: None,
        };

        let out: M2Settings = serde_xml_rs::from_str(content).unwrap();

        assert_eq!(out, expect);
    }
}
