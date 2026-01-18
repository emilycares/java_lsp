use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum MavenMetadataError {
    Xml(serde_xml_rs::Error),
}

#[derive(Debug, PartialEq, Eq)]
pub struct MetadataInfo {
    pub classes: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename = "metadata")]
struct MavenMetadata {
    pub versioning: MavenMetadataVersioning,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct MavenMetadataVersioning {
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
    #[serde(rename = "snapshotVersions")]
    pub snapshot_versions: MavenMetadataSnapshotVersions,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct MavenMetadataSnapshotVersions {
    #[serde(rename = "snapshotVersion")]
    pub snapshot_version: Vec<MavenMetadataSnapshotVersion>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct MavenMetadataSnapshotVersion {
    pub classifier: Option<String>,
    pub extension: String,
    pub value: String,
    pub updated: String,
}

pub fn get_metadata_info(
    content: &str,
    artifact_id: &str,
) -> Result<MetadataInfo, MavenMetadataError> {
    let parsed: MavenMetadata = serde_xml_rs::from_str(content).map_err(MavenMetadataError::Xml)?;

    let classes = parsed
        .versioning
        .snapshot_versions
        .snapshot_version
        .iter()
        .find(|i| {
            i.classifier.is_none()
                && i.updated == parsed.versioning.last_updated
                && i.extension == "jar"
        })
        .map(|i| format!("{artifact_id}-{}.jar", i.value));
    let source = parsed
        .versioning
        .snapshot_versions
        .snapshot_version
        .iter()
        .find(|i| {
            i.classifier.iter().any(|i| i == "sources")
                && i.updated == parsed.versioning.last_updated
                && i.extension == "jar"
        })
        .map(|i| format!("{artifact_id}-{}-sources.jar", i.value));

    Ok(MetadataInfo { classes, source })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::metadata::{
        MavenMetadata, MavenMetadataSnapshotVersion, MavenMetadataSnapshotVersions,
        MavenMetadataVersioning, MetadataInfo, get_metadata_info,
    };

    #[test]
    fn load() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
        <metadata modelVersion="1.1.0">
          <groupId>eu.org</groupId>
          <artifactId>org-api</artifactId>
          <versioning>
            <lastUpdated>20251212232552</lastUpdated>
            <snapshot>
              <timestamp>20251212.232552</timestamp>
              <buildNumber>85</buildNumber>
            </snapshot>
            <snapshotVersions>
              <snapshotVersion>
                <extension>pom</extension>
                <value>1.11-20251212.232552-85</value>
                <updated>20251212232552</updated>
              </snapshotVersion>
              <snapshotVersion>
                <extension>jar</extension>
                <value>1.11-20251212.232552-85</value>
                <updated>20251212232552</updated>
              </snapshotVersion>
              <snapshotVersion>
                <classifier>sources</classifier>
                <extension>jar</extension>
                <value>1.11-20251212.232552-85</value>
                <updated>20251212232552</updated>
              </snapshotVersion>
              <snapshotVersion>
                <classifier>javadoc</classifier>
                <extension>jar</extension>
                <value>1.11-20251212.232552-85</value>
                <updated>20251212232552</updated>
              </snapshotVersion>
            </snapshotVersions>
          </versioning>
          <version>1.11-SNAPSHOT</version>
        </metadata>
        "#;
        let expect = MavenMetadata {
            versioning: MavenMetadataVersioning {
                last_updated: "20251212232552".to_owned(),
                snapshot_versions: MavenMetadataSnapshotVersions {
                    snapshot_version: vec![
                        MavenMetadataSnapshotVersion {
                            classifier: None,
                            extension: "pom".to_string(),
                            value: "1.11-20251212.232552-85".to_string(),
                            updated: "20251212232552".to_string(),
                        },
                        MavenMetadataSnapshotVersion {
                            classifier: None,
                            extension: "jar".to_string(),
                            value: "1.11-20251212.232552-85".to_string(),
                            updated: "20251212232552".to_string(),
                        },
                        MavenMetadataSnapshotVersion {
                            classifier: Some("sources".to_owned()),
                            extension: "jar".to_string(),
                            value: "1.11-20251212.232552-85".to_string(),
                            updated: "20251212232552".to_string(),
                        },
                        MavenMetadataSnapshotVersion {
                            classifier: Some("javadoc".to_owned()),
                            extension: "jar".to_string(),
                            value: "1.11-20251212.232552-85".to_string(),
                            updated: "20251212232552".to_string(),
                        },
                    ],
                },
            },
        };

        let out: MavenMetadata = serde_xml_rs::from_str(&content).unwrap();
        assert_eq!(out, expect);

        let info = get_metadata_info(content, "org-api").unwrap();
        assert_eq!(
            info,
            MetadataInfo {
                classes: Some("org-api-1.11-20251212.232552-85.jar".to_owned()),
                source: Some("org-api-1.11-20251212.232552-85-sources.jar".to_owned())
            }
        );
    }
}
