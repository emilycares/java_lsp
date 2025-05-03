use std::{process::Command, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::EXECUTABLE_MAVEN;

#[derive(Debug)]
pub enum MavenTreeError {
    Cli(std::io::Error),
    UnknownDependencyScope,
}

pub fn load() -> Result<Dependency, MavenTreeError> {
    let log: String = get_cli_output()?;
    let cut: String = cut_output(log);

    parser(cut)
}

fn parser(cut: String) -> Result<Dependency, MavenTreeError> {
    let mut out: Vec<Pom> = vec![];
    for line in cut.lines() {
        let line = line.trim_start_matches("[INFO]").trim();
        let Some((_, line)) = line.split_once("-> \"") else {
            continue;
        };
        let mut spl = line.split(":");
        let group_id = spl.next().unwrap_or_default().to_string();
        let artivact_id = spl.next().unwrap_or_default().to_string();
        spl.next();
        let version = spl.next().unwrap_or_default().to_string();
        let scope = spl.next().unwrap_or_default();
        let Some((scope, _)) = scope.split_once("\"") else {
            continue;
        };
        let scope: DependencyScope = scope.parse()?;
        out.push(Pom {
            group_id,
            artivact_id,
            version,
            scope,
        });
    }
    Ok(Dependency { deps: out })
}

fn get_cli_output() -> Result<String, MavenTreeError> {
    // mvn dependency:tree -DoutputType=dot
    let output = Command::new(EXECUTABLE_MAVEN)
        .arg("dependency:tree")
        .arg("-DoutputType=dot")
        // .arg("-b")
        .output()
        .map_err(MavenTreeError::Cli)?;
    dbg!(String::from_utf8_lossy(&output.stderr).to_string());

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn cut_output(inp: String) -> String {
    let mut out = String::new();

    let mut capture = false;

    for line in inp.lines() {
        if line.starts_with("[INFO] digraph") {
            capture = true;
        }

        if capture {
            out.push_str(line);
            out.push('\n');
        }

        if line.starts_with("[INFO]  }") {
            break;
        }
    }

    out
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Dependency {
    pub deps: Vec<Pom>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Pom {
    pub group_id: String,
    pub artivact_id: String,
    pub version: String,
    pub scope: DependencyScope,
}

/// <https://maven.apache.org/guides/introduction/introduction-to-dependency-mechanism.html#dependency-scope>
#[derive(Default, PartialEq, Debug, Serialize, Deserialize)]
pub enum DependencyScope {
    #[default]
    Compile,
    Provided,
    /// No need to be indexed
    Runtime,
    /// Only considered in test
    Test,
    System,
    Import,
}

impl FromStr for DependencyScope {
    type Err = MavenTreeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "compile" => Ok(Self::Compile),
            "provided" => Ok(Self::Provided),
            "runtime" => Ok(Self::Runtime),
            "test" => Ok(Self::Test),
            "system" => Ok(Self::System),
            "import" => Ok(Self::Import),
            other => {
                eprintln!("Other dep scope: {}", other);
                Err(MavenTreeError::UnknownDependencyScope)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::tree::{cut_output, parser, Dependency, DependencyScope, Pom};

    #[test]
    fn cut_basic() {
        let inp = include_str!("../tests/tverify.bacic.txt");

        let out = cut_output(inp.to_string());

        assert!(!out.contains("Building getting-started"));
        assert!(!out.contains("BUILD SUCCESS"));
    }

    #[test]
    fn parse_diagram() {
        let inp = include_str!("../tests/tverify.bacic.txt");
        let cut = cut_output(inp.to_string());
        let out = parser(cut);
        let out = out.unwrap();
        assert_eq!(
            out,
            Dependency {
                deps: vec![
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-rest".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-arc".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-junit5".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.rest-assured".to_string(),
                        artivact_id: "rest-assured".to_string(),
                        version: "5.4.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-rest-common".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.resteasy.reactive".to_string(),
                        artivact_id: "resteasy-reactive-vertx".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-vertx-http".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-jsonp".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-virtual-threads".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.resteasy.reactive".to_string(),
                        artivact_id: "resteasy-reactive-common".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-mutiny".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-vertx".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.resteasy.reactive".to_string(),
                        artivact_id: "resteasy-reactive-common-types".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.reactivestreams".to_string(),
                        artivact_id: "reactive-streams".to_string(),
                        version: "1.0.4".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "mutiny-zero-flow-adapters".to_string(),
                        version: "1.1.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-annotation".to_string(),
                        version: "2.3.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-smallrye-context-propagation".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "mutiny-smallrye-context-propagation".to_string(),
                        version: "2.6.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye".to_string(),
                        artivact_id: "smallrye-context-propagation".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye".to_string(),
                        artivact_id: "smallrye-context-propagation-api".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye".to_string(),
                        artivact_id: "smallrye-context-propagation-storage".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-netty".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-codec-haproxy".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-vertx-latebound-mdc-provider".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye".to_string(),
                        artivact_id: "smallrye-fault-tolerance-vertx".to_string(),
                        version: "6.3.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-codec".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "com.aayushatharva.brotli4j".to_string(),
                        artivact_id: "brotli4j".to_string(),
                        version: "1.16.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "com.aayushatharva.brotli4j".to_string(),
                        artivact_id: "service".to_string(),
                        version: "1.16.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "com.aayushatharva.brotli4j".to_string(),
                        artivact_id: "native-linux-x86_64".to_string(),
                        version: "1.16.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-web".to_string(),
                        version: "4.5.7".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-core".to_string(),
                        version: "3.12.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.resteasy.reactive".to_string(),
                        artivact_id: "resteasy-reactive".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.vertx.utils".to_string(),
                        artivact_id: "quarkus-vertx-utils".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.enterprise".to_string(),
                        artivact_id: "jakarta.enterprise.cdi-api".to_string(),
                        version: "4.1.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.ws.rs".to_string(),
                        artivact_id: "jakarta.ws.rs-api".to_string(),
                        version: "3.1.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.annotation".to_string(),
                        artivact_id: "jakarta.annotation-api".to_string(),
                        version: "3.0.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jboss.logging".to_string(),
                        artivact_id: "commons-logging-jboss-logging".to_string(),
                        version: "1.0.0.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.xml.bind".to_string(),
                        artivact_id: "jakarta.xml.bind-api".to_string(),
                        version: "4.0.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jboss.logging".to_string(),
                        artivact_id: "jboss-logging".to_string(),
                        version: "3.6.0.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-web-common".to_string(),
                        version: "4.5.7".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-auth-common".to_string(),
                        version: "4.5.7".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-bridge-common".to_string(),
                        version: "4.5.7".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-runtime".to_string(),
                        version: "3.12.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "vertx-mutiny-generator".to_string(),
                        version: "3.12.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-codegen".to_string(),
                        version: "4.5.7".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.enterprise".to_string(),
                        artivact_id: "jakarta.enterprise.lang-model".to_string(),
                        version: "4.1.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.el".to_string(),
                        artivact_id: "jakarta.el-api".to_string(),
                        version: "5.0.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.interceptor".to_string(),
                        artivact_id: "jakarta.interceptor-api".to_string(),
                        version: "2.2.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.activation".to_string(),
                        artivact_id: "jakarta.activation-api".to_string(),
                        version: "2.1.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-security-runtime-spi".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-tls-registry".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-credentials".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-vertx-context".to_string(),
                        version: "2.3.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.security".to_string(),
                        artivact_id: "quarkus-security".to_string(),
                        version: "2.1.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-web".to_string(),
                        version: "3.12.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.github.crac".to_string(),
                        artivact_id: "org-crac".to_string(),
                        version: "0.1.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-constraint".to_string(),
                        version: "2.3.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-web-common".to_string(),
                        version: "3.12.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-auth-common".to_string(),
                        version: "3.12.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-bridge-common".to_string(),
                        version: "3.12.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-uri-template".to_string(),
                        version: "3.12.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-uri-template".to_string(),
                        version: "4.5.7".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.eclipse.parsson".to_string(),
                        artivact_id: "parsson".to_string(),
                        version: "1.1.6".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.json".to_string(),
                        artivact_id: "jakarta.json-api".to_string(),
                        version: "2.1.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-core".to_string(),
                        version: "4.5.7".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-common".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-buffer".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-transport".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-handler".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-handler-proxy".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-codec-http".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-codec-http2".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-resolver".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-resolver-dns".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "com.fasterxml.jackson.core".to_string(),
                        artivact_id: "jackson-core".to_string(),
                        version: "2.17.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-transport-native-unix-common".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-codec-socks".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-codec-dns".to_string(),
                        version: "4.1.108.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.arc".to_string(),
                        artivact_id: "arc".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-core".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.eclipse.microprofile.context-propagation".to_string(),
                        artivact_id: "microprofile-context-propagation-api".to_string(),
                        version: "1.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.transaction".to_string(),
                        artivact_id: "jakarta.transaction-api".to_string(),
                        version: "2.0.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "mutiny".to_string(),
                        version: "2.6.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jctools".to_string(),
                        artivact_id: "jctools-core".to_string(),
                        version: "4.0.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.inject".to_string(),
                        artivact_id: "jakarta.inject-api".to_string(),
                        version: "2.0.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-os".to_string(),
                        version: "2.3.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-ide-launcher".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-development-mode-spi".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.config".to_string(),
                        artivact_id: "smallrye-config".to_string(),
                        version: "3.8.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jboss.logmanager".to_string(),
                        artivact_id: "jboss-logmanager".to_string(),
                        version: "3.0.6.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jboss.logging".to_string(),
                        artivact_id: "jboss-logging-annotations".to_string(),
                        version: "2.2.1.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jboss.threads".to_string(),
                        artivact_id: "jboss-threads".to_string(),
                        version: "3.6.1.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.slf4j".to_string(),
                        artivact_id: "slf4j-api".to_string(),
                        version: "2.0.6".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jboss.slf4j".to_string(),
                        artivact_id: "slf4j-jboss-logmanager".to_string(),
                        version: "2.0.0.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.wildfly.common".to_string(),
                        artivact_id: "wildfly-common".to_string(),
                        version: "1.7.0.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-bootstrap-runner".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-fs-util".to_string(),
                        version: "0.0.10".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.config".to_string(),
                        artivact_id: "smallrye-config-core".to_string(),
                        version: "3.8.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.eclipse.microprofile.config".to_string(),
                        artivact_id: "microprofile-config-api".to_string(),
                        version: "3.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-classloader".to_string(),
                        version: "2.3.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.config".to_string(),
                        artivact_id: "smallrye-config-common".to_string(),
                        version: "3.8.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-cpu".to_string(),
                        version: "2.3.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-expression".to_string(),
                        version: "2.3.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-net".to_string(),
                        version: "2.3.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-ref".to_string(),
                        version: "2.3.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-function".to_string(),
                        version: "2.3.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-bootstrap-core".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.eclipse.sisu".to_string(),
                        artivact_id: "org.eclipse.sisu.inject".to_string(),
                        version: "0.9.0.M3".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-test-common".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-junit5-properties".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.jupiter".to_string(),
                        artivact_id: "junit-jupiter".to_string(),
                        version: "5.10.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "com.thoughtworks.xstream".to_string(),
                        artivact_id: "xstream".to_string(),
                        version: "1.4.20".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-classloader-commons".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-bootstrap-app-model".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-io".to_string(),
                        version: "2.3.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-core-deployment".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-bootstrap-maven-resolver".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-bootstrap-gradle-resolver".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.smallrye".to_string(),
                        artivact_id: "jandex".to_string(),
                        version: "3.2.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "commons-io".to_string(),
                        artivact_id: "commons-io".to_string(),
                        version: "2.16.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.aesh".to_string(),
                        artivact_id: "readline".to_string(),
                        version: "2.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.aesh".to_string(),
                        artivact_id: "aesh".to_string(),
                        version: "2.8.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus.gizmo".to_string(),
                        artivact_id: "gizmo".to_string(),
                        version: "1.8.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.ow2.asm".to_string(),
                        artivact_id: "asm".to_string(),
                        version: "9.7".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.ow2.asm".to_string(),
                        artivact_id: "asm-commons".to_string(),
                        version: "9.7".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-hibernate-validator-spi".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-class-change-agent".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-devtools-utilities".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-builder".to_string(),
                        version: "3.12.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.graalvm.sdk".to_string(),
                        artivact_id: "nativeimage".to_string(),
                        version: "23.1.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.platform".to_string(),
                        artivact_id: "junit-platform-launcher".to_string(),
                        version: "1.10.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.fusesource.jansi".to_string(),
                        artivact_id: "jansi".to_string(),
                        version: "2.4.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.ow2.asm".to_string(),
                        artivact_id: "asm-util".to_string(),
                        version: "9.7".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.ow2.asm".to_string(),
                        artivact_id: "asm-analysis".to_string(),
                        version: "9.7".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.ow2.asm".to_string(),
                        artivact_id: "asm-tree".to_string(),
                        version: "9.7".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.graalvm.sdk".to_string(),
                        artivact_id: "word".to_string(),
                        version: "23.1.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.smallrye.beanbag".to_string(),
                        artivact_id: "smallrye-beanbag-maven".to_string(),
                        version: "1.5.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-embedder".to_string(),
                        version: "3.9.8".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.eclipse.sisu".to_string(),
                        artivact_id: "org.eclipse.sisu.plexus".to_string(),
                        version: "0.9.0.M3".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-settings-builder".to_string(),
                        version: "3.9.8".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-resolver-provider".to_string(),
                        version: "3.9.8".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-connector-basic".to_string(),
                        version: "1.9.20".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-transport-wagon".to_string(),
                        version: "1.9.20".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.wagon".to_string(),
                        artivact_id: "wagon-http".to_string(),
                        version: "3.5.3".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.wagon".to_string(),
                        artivact_id: "wagon-file".to_string(),
                        version: "3.5.3".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.smallrye.beanbag".to_string(),
                        artivact_id: "smallrye-beanbag-sisu".to_string(),
                        version: "1.5.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "javax.inject".to_string(),
                        artivact_id: "javax.inject".to_string(),
                        version: "1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-artifact".to_string(),
                        version: "3.9.8".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-builder-support".to_string(),
                        version: "3.9.8".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-model".to_string(),
                        version: "3.9.8".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-model-builder".to_string(),
                        version: "3.9.8".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-repository-metadata".to_string(),
                        version: "3.9.8".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-settings".to_string(),
                        version: "3.9.8".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-api".to_string(),
                        version: "1.9.20".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-impl".to_string(),
                        version: "1.9.20".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-spi".to_string(),
                        version: "1.9.20".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-util".to_string(),
                        version: "1.9.20".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-transport-http".to_string(),
                        version: "1.9.20".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.wagon".to_string(),
                        artivact_id: "wagon-provider-api".to_string(),
                        version: "3.5.3".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.wagon".to_string(),
                        artivact_id: "wagon-http-shared".to_string(),
                        version: "3.5.3".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-interpolation".to_string(),
                        version: "1.26".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-utils".to_string(),
                        version: "3.5.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-xml".to_string(),
                        version: "4.0.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-cipher".to_string(),
                        version: "2.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-sec-dispatcher".to_string(),
                        version: "2.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.smallrye.beanbag".to_string(),
                        artivact_id: "smallrye-beanbag".to_string(),
                        version: "1.5.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-named-locks".to_string(),
                        version: "1.9.20".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-xml-impl".to_string(),
                        version: "4.0.0-alpha-5".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-api-xml".to_string(),
                        version: "4.0.0-alpha-5".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-api-meta".to_string(),
                        version: "4.0.0-alpha-5".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-core".to_string(),
                        version: "3.9.8".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-plugin-api".to_string(),
                        version: "3.9.8".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.shared".to_string(),
                        artivact_id: "maven-shared-utils".to_string(),
                        version: "3.4.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "com.google.inject".to_string(),
                        artivact_id: "guice".to_string(),
                        version: "5.1.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "com.google.guava".to_string(),
                        artivact_id: "guava".to_string(),
                        version: "33.2.1-jre".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "com.google.guava".to_string(),
                        artivact_id: "failureaccess".to_string(),
                        version: "1.0.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "javax.annotation".to_string(),
                        artivact_id: "javax.annotation-api".to_string(),
                        version: "1.3.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-classworlds".to_string(),
                        version: "2.6.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "commons-cli".to_string(),
                        artivact_id: "commons-cli".to_string(),
                        version: "1.8.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-component-annotations".to_string(),
                        version: "2.1.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "aopalliance".to_string(),
                        artivact_id: "aopalliance".to_string(),
                        version: "1.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.jupiter".to_string(),
                        artivact_id: "junit-jupiter-api".to_string(),
                        version: "5.10.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.jupiter".to_string(),
                        artivact_id: "junit-jupiter-params".to_string(),
                        version: "5.10.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.jupiter".to_string(),
                        artivact_id: "junit-jupiter-engine".to_string(),
                        version: "5.10.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.opentest4j".to_string(),
                        artivact_id: "opentest4j".to_string(),
                        version: "1.3.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.platform".to_string(),
                        artivact_id: "junit-platform-commons".to_string(),
                        version: "1.10.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apiguardian".to_string(),
                        artivact_id: "apiguardian-api".to_string(),
                        version: "1.1.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.platform".to_string(),
                        artivact_id: "junit-platform-engine".to_string(),
                        version: "1.10.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.github.x-stream".to_string(),
                        artivact_id: "mxparser".to_string(),
                        version: "1.2.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "xmlpull".to_string(),
                        artivact_id: "xmlpull".to_string(),
                        version: "1.1.3.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.groovy".to_string(),
                        artivact_id: "groovy".to_string(),
                        version: "4.0.16".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.groovy".to_string(),
                        artivact_id: "groovy-xml".to_string(),
                        version: "4.0.16".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.httpcomponents".to_string(),
                        artivact_id: "httpclient".to_string(),
                        version: "4.5.14".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.httpcomponents".to_string(),
                        artivact_id: "httpmime".to_string(),
                        version: "4.5.14".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.hamcrest".to_string(),
                        artivact_id: "hamcrest".to_string(),
                        version: "2.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.ccil.cowan.tagsoup".to_string(),
                        artivact_id: "tagsoup".to_string(),
                        version: "1.2.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.rest-assured".to_string(),
                        artivact_id: "json-path".to_string(),
                        version: "5.4.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.rest-assured".to_string(),
                        artivact_id: "xml-path".to_string(),
                        version: "5.4.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.httpcomponents".to_string(),
                        artivact_id: "httpcore".to_string(),
                        version: "4.4.16".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "commons-codec".to_string(),
                        artivact_id: "commons-codec".to_string(),
                        version: "1.17.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.groovy".to_string(),
                        artivact_id: "groovy-json".to_string(),
                        version: "4.0.16".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.rest-assured".to_string(),
                        artivact_id: "rest-assured-common".to_string(),
                        version: "5.4.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.commons".to_string(),
                        artivact_id: "commons-lang3".to_string(),
                        version: "3.14.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                ]
            }
        );
    }

    #[test]
    fn parse_diagram_with_tab() {
        let inp = include_str!("../tests/tverify-tap.bacic.txt");
        let out = parser(inp.to_string());
        let out = out.unwrap();
        assert_eq!(
            out,
            Dependency {
                deps: vec![
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-resteasy-reactive".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-arc".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-resteasy-reactive-qute".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-junit5".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.rest-assured".to_string(),
                        artivact_id: "rest-assured".to_string(),
                        version: "5.4.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-resteasy-reactive-common".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.resteasy.reactive".to_string(),
                        artivact_id: "resteasy-reactive-vertx".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-vertx-http".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-jsonp".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-virtual-threads".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.resteasy.reactive".to_string(),
                        artivact_id: "resteasy-reactive-common".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-mutiny".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-vertx".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.resteasy.reactive".to_string(),
                        artivact_id: "resteasy-reactive-common-types".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.reactivestreams".to_string(),
                        artivact_id: "reactive-streams".to_string(),
                        version: "1.0.4".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "mutiny-zero-flow-adapters".to_string(),
                        version: "1.0.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-annotation".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-smallrye-context-propagation".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "mutiny-smallrye-context-propagation".to_string(),
                        version: "2.5.6".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye".to_string(),
                        artivact_id: "smallrye-context-propagation".to_string(),
                        version: "2.1.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye".to_string(),
                        artivact_id: "smallrye-context-propagation-api".to_string(),
                        version: "2.1.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye".to_string(),
                        artivact_id: "smallrye-context-propagation-storage".to_string(),
                        version: "2.1.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-netty".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-codec-haproxy".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-vertx-latebound-mdc-provider".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye".to_string(),
                        artivact_id: "smallrye-fault-tolerance-vertx".to_string(),
                        version: "6.2.6".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-codec".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "com.aayushatharva.brotli4j".to_string(),
                        artivact_id: "brotli4j".to_string(),
                        version: "1.12.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "com.aayushatharva.brotli4j".to_string(),
                        artivact_id: "service".to_string(),
                        version: "1.12.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "com.aayushatharva.brotli4j".to_string(),
                        artivact_id: "native-linux-x86_64".to_string(),
                        version: "1.12.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-web".to_string(),
                        version: "4.5.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-core".to_string(),
                        version: "3.8.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.resteasy.reactive".to_string(),
                        artivact_id: "resteasy-reactive".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.enterprise".to_string(),
                        artivact_id: "jakarta.enterprise.cdi-api".to_string(),
                        version: "4.0.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.ws.rs".to_string(),
                        artivact_id: "jakarta.ws.rs-api".to_string(),
                        version: "3.1.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.annotation".to_string(),
                        artivact_id: "jakarta.annotation-api".to_string(),
                        version: "2.1.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jboss.logging".to_string(),
                        artivact_id: "commons-logging-jboss-logging".to_string(),
                        version: "1.0.0.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.xml.bind".to_string(),
                        artivact_id: "jakarta.xml.bind-api".to_string(),
                        version: "4.0.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jboss.logging".to_string(),
                        artivact_id: "jboss-logging".to_string(),
                        version: "3.5.3.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-web-common".to_string(),
                        version: "4.5.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-auth-common".to_string(),
                        version: "4.5.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-bridge-common".to_string(),
                        version: "4.5.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-runtime".to_string(),
                        version: "3.8.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "vertx-mutiny-generator".to_string(),
                        version: "3.8.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-codegen".to_string(),
                        version: "4.5.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.enterprise".to_string(),
                        artivact_id: "jakarta.enterprise.lang-model".to_string(),
                        version: "4.0.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.el".to_string(),
                        artivact_id: "jakarta.el-api".to_string(),
                        version: "5.0.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.interceptor".to_string(),
                        artivact_id: "jakarta.interceptor-api".to_string(),
                        version: "2.1.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.activation".to_string(),
                        artivact_id: "jakarta.activation-api".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-security-runtime-spi".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-credentials".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-vertx-context".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.security".to_string(),
                        artivact_id: "quarkus-security".to_string(),
                        version: "2.0.3.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-web".to_string(),
                        version: "3.8.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.github.crac".to_string(),
                        artivact_id: "org-crac".to_string(),
                        version: "0.1.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-constraint".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-web-common".to_string(),
                        version: "3.8.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-auth-common".to_string(),
                        version: "3.8.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-bridge-common".to_string(),
                        version: "3.8.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "smallrye-mutiny-vertx-uri-template".to_string(),
                        version: "3.8.0".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-uri-template".to_string(),
                        version: "4.5.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.eclipse.parsson".to_string(),
                        artivact_id: "parsson".to_string(),
                        version: "1.1.5".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.json".to_string(),
                        artivact_id: "jakarta.json-api".to_string(),
                        version: "2.1.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.vertx".to_string(),
                        artivact_id: "vertx-core".to_string(),
                        version: "4.5.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-common".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-buffer".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-transport".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-handler".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-handler-proxy".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-codec-http".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-codec-http2".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-resolver".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-resolver-dns".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "com.fasterxml.jackson.core".to_string(),
                        artivact_id: "jackson-core".to_string(),
                        version: "2.16.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-transport-native-unix-common".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-codec-socks".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.netty".to_string(),
                        artivact_id: "netty-codec-dns".to_string(),
                        version: "4.1.106.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.arc".to_string(),
                        artivact_id: "arc".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-core".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.eclipse.microprofile.context-propagation".to_string(),
                        artivact_id: "microprofile-context-propagation-api".to_string(),
                        version: "1.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.transaction".to_string(),
                        artivact_id: "jakarta.transaction-api".to_string(),
                        version: "2.0.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.reactive".to_string(),
                        artivact_id: "mutiny".to_string(),
                        version: "2.5.6".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "jakarta.inject".to_string(),
                        artivact_id: "jakarta.inject-api".to_string(),
                        version: "2.0.1".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-os".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-ide-launcher".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-development-mode-spi".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.config".to_string(),
                        artivact_id: "smallrye-config".to_string(),
                        version: "3.5.4".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jboss.logmanager".to_string(),
                        artivact_id: "jboss-logmanager".to_string(),
                        version: "3.0.4.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jboss.logging".to_string(),
                        artivact_id: "jboss-logging-annotations".to_string(),
                        version: "2.2.1.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jboss.threads".to_string(),
                        artivact_id: "jboss-threads".to_string(),
                        version: "3.5.1.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.slf4j".to_string(),
                        artivact_id: "slf4j-api".to_string(),
                        version: "2.0.6".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.jboss.slf4j".to_string(),
                        artivact_id: "slf4j-jboss-logmanager".to_string(),
                        version: "2.0.0.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.wildfly.common".to_string(),
                        artivact_id: "wildfly-common".to_string(),
                        version: "1.7.0.Final".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-bootstrap-runner".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-fs-util".to_string(),
                        version: "0.0.10".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.config".to_string(),
                        artivact_id: "smallrye-config-core".to_string(),
                        version: "3.5.4".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "org.eclipse.microprofile.config".to_string(),
                        artivact_id: "microprofile-config-api".to_string(),
                        version: "3.0.3".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-classloader".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.config".to_string(),
                        artivact_id: "smallrye-config-common".to_string(),
                        version: "3.5.4".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-cpu".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-expression".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-net".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-ref".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-function".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-qute".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus.qute".to_string(),
                        artivact_id: "qute-core".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-bootstrap-core".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.eclipse.sisu".to_string(),
                        artivact_id: "org.eclipse.sisu.inject".to_string(),
                        version: "0.9.0.M2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-test-common".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-junit5-properties".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.jupiter".to_string(),
                        artivact_id: "junit-jupiter".to_string(),
                        version: "5.10.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "com.thoughtworks.xstream".to_string(),
                        artivact_id: "xstream".to_string(),
                        version: "1.4.20".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-bootstrap-app-model".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.smallrye.common".to_string(),
                        artivact_id: "smallrye-common-io".to_string(),
                        version: "2.1.2".to_string(),
                        scope: DependencyScope::Compile,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-core-deployment".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-bootstrap-maven-resolver".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-bootstrap-gradle-resolver".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.smallrye".to_string(),
                        artivact_id: "jandex".to_string(),
                        version: "3.1.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "commons-io".to_string(),
                        artivact_id: "commons-io".to_string(),
                        version: "2.15.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.aesh".to_string(),
                        artivact_id: "readline".to_string(),
                        version: "2.4".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.aesh".to_string(),
                        artivact_id: "aesh".to_string(),
                        version: "2.7".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus.gizmo".to_string(),
                        artivact_id: "gizmo".to_string(),
                        version: "1.7.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.ow2.asm".to_string(),
                        artivact_id: "asm".to_string(),
                        version: "9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.ow2.asm".to_string(),
                        artivact_id: "asm-commons".to_string(),
                        version: "9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-class-change-agent".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-devtools-utilities".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.quarkus".to_string(),
                        artivact_id: "quarkus-builder".to_string(),
                        version: "3.7.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.graalvm.sdk".to_string(),
                        artivact_id: "graal-sdk".to_string(),
                        version: "23.0.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.platform".to_string(),
                        artivact_id: "junit-platform-launcher".to_string(),
                        version: "1.10.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.fusesource.jansi".to_string(),
                        artivact_id: "jansi".to_string(),
                        version: "2.4.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.ow2.asm".to_string(),
                        artivact_id: "asm-util".to_string(),
                        version: "9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.ow2.asm".to_string(),
                        artivact_id: "asm-analysis".to_string(),
                        version: "9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.ow2.asm".to_string(),
                        artivact_id: "asm-tree".to_string(),
                        version: "9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.smallrye.beanbag".to_string(),
                        artivact_id: "smallrye-beanbag-maven".to_string(),
                        version: "1.3.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-embedder".to_string(),
                        version: "3.9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.eclipse.sisu".to_string(),
                        artivact_id: "org.eclipse.sisu.plexus".to_string(),
                        version: "0.9.0.M2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-settings-builder".to_string(),
                        version: "3.9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-resolver-provider".to_string(),
                        version: "3.9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-connector-basic".to_string(),
                        version: "1.9.18".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-transport-wagon".to_string(),
                        version: "1.9.18".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.wagon".to_string(),
                        artivact_id: "wagon-http".to_string(),
                        version: "3.5.3".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.wagon".to_string(),
                        artivact_id: "wagon-file".to_string(),
                        version: "3.5.3".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.smallrye.beanbag".to_string(),
                        artivact_id: "smallrye-beanbag-sisu".to_string(),
                        version: "1.3.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "javax.inject".to_string(),
                        artivact_id: "javax.inject".to_string(),
                        version: "1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-artifact".to_string(),
                        version: "3.9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-builder-support".to_string(),
                        version: "3.9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-model".to_string(),
                        version: "3.9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-model-builder".to_string(),
                        version: "3.9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-repository-metadata".to_string(),
                        version: "3.9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-settings".to_string(),
                        version: "3.9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-api".to_string(),
                        version: "1.9.18".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-impl".to_string(),
                        version: "1.9.18".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-spi".to_string(),
                        version: "1.9.18".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-util".to_string(),
                        version: "1.9.18".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-transport-http".to_string(),
                        version: "1.9.10".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.wagon".to_string(),
                        artivact_id: "wagon-provider-api".to_string(),
                        version: "3.5.3".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.wagon".to_string(),
                        artivact_id: "wagon-http-shared".to_string(),
                        version: "3.5.3".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-interpolation".to_string(),
                        version: "1.26".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-utils".to_string(),
                        version: "3.5.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-xml".to_string(),
                        version: "4.0.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-cipher".to_string(),
                        version: "2.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-sec-dispatcher".to_string(),
                        version: "2.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.smallrye.beanbag".to_string(),
                        artivact_id: "smallrye-beanbag".to_string(),
                        version: "1.3.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.resolver".to_string(),
                        artivact_id: "maven-resolver-named-locks".to_string(),
                        version: "1.9.18".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-xml-impl".to_string(),
                        version: "4.0.0-alpha-5".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-api-xml".to_string(),
                        version: "4.0.0-alpha-5".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-api-meta".to_string(),
                        version: "4.0.0-alpha-5".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-core".to_string(),
                        version: "3.9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven".to_string(),
                        artivact_id: "maven-plugin-api".to_string(),
                        version: "3.9.6".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.maven.shared".to_string(),
                        artivact_id: "maven-shared-utils".to_string(),
                        version: "3.3.4".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "com.google.inject".to_string(),
                        artivact_id: "guice".to_string(),
                        version: "5.1.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "com.google.guava".to_string(),
                        artivact_id: "guava".to_string(),
                        version: "33.0.0-jre".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "com.google.guava".to_string(),
                        artivact_id: "failureaccess".to_string(),
                        version: "1.0.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "javax.annotation".to_string(),
                        artivact_id: "javax.annotation-api".to_string(),
                        version: "1.3.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-classworlds".to_string(),
                        version: "2.6.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "commons-cli".to_string(),
                        artivact_id: "commons-cli".to_string(),
                        version: "1.5.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.codehaus.plexus".to_string(),
                        artivact_id: "plexus-component-annotations".to_string(),
                        version: "2.1.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "aopalliance".to_string(),
                        artivact_id: "aopalliance".to_string(),
                        version: "1.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.jupiter".to_string(),
                        artivact_id: "junit-jupiter-api".to_string(),
                        version: "5.10.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.jupiter".to_string(),
                        artivact_id: "junit-jupiter-params".to_string(),
                        version: "5.10.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.jupiter".to_string(),
                        artivact_id: "junit-jupiter-engine".to_string(),
                        version: "5.10.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.opentest4j".to_string(),
                        artivact_id: "opentest4j".to_string(),
                        version: "1.3.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.platform".to_string(),
                        artivact_id: "junit-platform-commons".to_string(),
                        version: "1.10.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apiguardian".to_string(),
                        artivact_id: "apiguardian-api".to_string(),
                        version: "1.1.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.junit.platform".to_string(),
                        artivact_id: "junit-platform-engine".to_string(),
                        version: "1.10.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.github.x-stream".to_string(),
                        artivact_id: "mxparser".to_string(),
                        version: "1.2.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "xmlpull".to_string(),
                        artivact_id: "xmlpull".to_string(),
                        version: "1.1.3.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.groovy".to_string(),
                        artivact_id: "groovy".to_string(),
                        version: "4.0.16".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.groovy".to_string(),
                        artivact_id: "groovy-xml".to_string(),
                        version: "4.0.16".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.httpcomponents".to_string(),
                        artivact_id: "httpclient".to_string(),
                        version: "4.5.14".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.httpcomponents".to_string(),
                        artivact_id: "httpmime".to_string(),
                        version: "4.5.14".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.hamcrest".to_string(),
                        artivact_id: "hamcrest".to_string(),
                        version: "2.2".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.ccil.cowan.tagsoup".to_string(),
                        artivact_id: "tagsoup".to_string(),
                        version: "1.2.1".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.rest-assured".to_string(),
                        artivact_id: "json-path".to_string(),
                        version: "5.4.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.rest-assured".to_string(),
                        artivact_id: "xml-path".to_string(),
                        version: "5.4.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.httpcomponents".to_string(),
                        artivact_id: "httpcore".to_string(),
                        version: "4.4.16".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "commons-codec".to_string(),
                        artivact_id: "commons-codec".to_string(),
                        version: "1.16.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.groovy".to_string(),
                        artivact_id: "groovy-json".to_string(),
                        version: "4.0.16".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "io.rest-assured".to_string(),
                        artivact_id: "rest-assured-common".to_string(),
                        version: "5.4.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                    Pom {
                        group_id: "org.apache.commons".to_string(),
                        artivact_id: "commons-lang3".to_string(),
                        version: "3.14.0".to_string(),
                        scope: DependencyScope::Test,
                    },
                ]
            }
        );
    }
}
