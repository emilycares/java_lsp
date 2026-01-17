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

pub fn load_settings_xml(m2_folder: &Path) -> Result<M2Settings, M2SettingsError> {
    let path = m2_folder.join("settings.xml");
    let file = std::fs::File::open(path).map_err(M2SettingsError::IO)?;
    serde_xml_rs::from_reader(file).map_err(M2SettingsError::Xml)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::settings::{M2Server, M2Servers, M2Settings};

    #[test]
    fn load() {
        let content = r#"
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
        "#;
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
        };

        let out: M2Settings = serde_xml_rs::from_str(content).unwrap();

        assert_eq!(out, expect);
    }
    #[test]
    fn no_servers() {
        let content = r#"
        <settings>
        </settings>
        "#;
        let expect = M2Settings { servers: None };

        let out: M2Settings = serde_xml_rs::from_str(content).unwrap();

        assert_eq!(out, expect);
    }
}
