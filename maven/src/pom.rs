use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("There was a error parsing a pom")]
    ParseError(#[from] serde_xml_rs::Error),
    #[error("Requires parrent")]
    ParentInformationNeeded(Parent),
}

#[allow(dead_code)]
pub fn parse(file: &str) -> Result<Pom, ParseError> {
    let pom: Pom = serde_xml_rs::from_str(file)?;
    Ok(pom)
}

#[allow(dead_code)]
/// https://maven.apache.org/guides/introduction/introduction-to-dependency-mechanism.html
pub fn resolve(pom: Pom, inputs: Vec<Pom>) -> Result<Pom, ParseError> {
    let mut out = pom.clone();

    // https://howtodoinjava.com/maven/maven-parent-child-pom-example/
    if let Some(parent) = pom.parent {
        let Some(parent_info) = inputs.iter().find(|i| {
            i.group_id == parent.group_id
                && i.artivact_id == parent.artifact_id
                && i.version == parent.version
        }) else {
            return Err(ParseError::ParentInformationNeeded(parent));
        };

        // TODO: Check if the parent can be overwriten by it's children
        out.properties.extend(parent_info.properties.clone());
        if let Some(pardep) = &parent_info.dependencies {
            if let Some(ref mut odep) = out.dependencies {
                odep.dependencies.extend(pardep.dependencies.clone());
            }
        }
    }

    Ok(out)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PrePom {
    pub parent: Option<Parent>,
    #[serde(rename = "dependencyManagement")]
    pub dependency_management: Option<DependencyManagement>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Pom {
    #[serde(rename = "modelVersion")]
    pub model_version: Option<String>,
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "artifactId")]
    pub artivact_id: String,
    pub packaging: Option<String>,
    pub name: Option<String>,
    pub version: String,
    pub parent: Option<Parent>,
    pub dependencies: Option<Dependencies>,
    #[serde(rename = "dependencyManagement")]
    pub dependency_management: Option<DependencyManagement>,
    #[serde(default)]
    pub properties: HashMap<String, String>,
}
impl Pom {
    #[allow(dead_code)]
    fn new() -> Pom {
        Self {
            model_version: None,
            group_id: String::new(),
            artivact_id: String::new(),
            packaging: None,
            name: None,
            version: String::new(),
            parent: None,
            dependencies: None,
            dependency_management: None,
            properties: HashMap::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Parent {
    #[serde(rename = "artifactId")]
    pub artifact_id: String,
    #[serde(rename = "groupId")]
    pub group_id: String,
    pub version: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DependencyManagement {
    #[serde(rename = "dependencies")]
    pub dependencies: Dependencies,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Dependencies {
    #[serde(rename = "dependency", default)]
    pub dependencies: Vec<Dependency>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Dependency {
    #[serde(rename = "artifactId")]
    pub artifact_id: String,
    #[serde(rename = "type")]
    pub dtype: Option<String>,
    #[serde(rename = "groupId")]
    pub group_id: String,
    pub optional: Option<bool>,
    #[serde(default)]
    pub scope: Option<DependencyScope>,
    #[serde(rename = "exclusions")]
    pub exclusions: Option<Exclusions>,
    pub version: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Exclusions {
    #[serde(rename = "exclusion", default)]
    pub exclusions: Vec<Exclusion>,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Exclusion {
    #[serde(rename = "artifactId")]
    pub artifact_id: String,
    #[serde(rename = "groupId")]
    pub group_id: String,
}

/// https://maven.apache.org/guides/introduction/introduction-to-dependency-mechanism.html#dependency-scope
#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub enum DependencyScope {
    #[default]
    #[serde(rename = "compile")]
    Compile,
    #[serde(rename = "provided")]
    Provided,
    #[serde(rename = "runtime")]
    /// No need to be indexed
    Runtime,
    #[serde(rename = "test")]
    /// Only considered in test
    Test,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "import")]
    Import,
}

#[cfg(test)]
mod tests {
    use crate::pom::{parse, resolve};

    #[test]
    fn parse_requires_parrent() {
        let pom = resolve(
            parse(include_str!("../tests/parse.pom.xml")).unwrap(),
            vec![],
        );
        dbg!(&pom);
        assert!(pom.is_err());
    }
}
