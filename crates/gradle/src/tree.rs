use std::process::Command;

#[derive(Debug)]
pub enum GradleTreeError {
    CliFailed(std::io::Error),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Dependency {
    pub group_id: String,
    pub artivact_id: String,
    pub version: String,
}

pub fn load(executable_gradle: &str) -> Result<Vec<Dependency>, GradleTreeError> {
    let log: String = get_cli_output(executable_gradle)?;
    let out = parse_tree(&log);
    Ok(out)
}

fn get_cli_output(executable_gradle: &str) -> Result<String, GradleTreeError> {
    // ./gradlew dependencies --console plain
    match Command::new(executable_gradle)
        .arg("dependencies")
        .arg("--console")
        .arg("plain")
        // .arg("-b")
        .output()
    {
        Ok(output) => Ok(String::from_utf8_lossy(&output.stdout).to_string()),
        Err(e) => Err(GradleTreeError::CliFailed(e)),
    }
}

fn parse_tree(inp: &str) -> Vec<Dependency> {
    let mut out = vec![];

    let mut capture = false;

    for line in inp.lines() {
        if line.contains(" - ") && line.ends_with('.') {
            capture = true;
        }

        if line.starts_with("(c) - A dependency constraint") {
            break;
        }

        if capture {
            if line.contains(" - ")
                || line.is_empty()
                || !line.contains('-')
                || line.starts_with("No dependencies")
            {
            } else {
                let line = line
                    .replace(['\\', ' ', '+', '|'], "")
                    .replace("(*)", "")
                    .replace("(n)", "")
                    .replace("(c)", "");
                let mut spl = line.splitn(3, ':');
                if let Some(group_id) = spl.next()
                    && let Some(artivact_id) = spl.next()
                    && let Some(version) = spl.next()
                {
                    out.push(Dependency {
                        group_id: group_id.trim_start_matches('-').to_string(),
                        artivact_id: artivact_id.trim_start_matches('-').to_string(),
                        version: version.trim_start_matches('-').to_string(),
                    });
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::tree::{Dependency, parse_tree};

    #[test]
    fn parse_diagram() {
        let inp = include_str!("../tests/dependencies_report.txt");
        let out = parse_tree(inp);
        assert_eq!(
            out,
            vec![
                Dependency {
                    group_id: "com.github.toolarium".to_string(),
                    artivact_id: "toolarium-enum-configuration".to_string(),
                    version: "1.2.0".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-databind".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-annotations".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson".to_string(),
                    artivact_id: "jackson-bom".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-annotations".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-core".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-databind".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.datatype".to_string(),
                    artivact_id: "jackson-datatype-jsr310".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-core".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson".to_string(),
                    artivact_id: "jackson-bom".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson".to_string(),
                    artivact_id: "jackson-bom".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-core".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.datatype".to_string(),
                    artivact_id: "jackson-datatype-jsr310".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-annotations".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-core".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-databind".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson".to_string(),
                    artivact_id: "jackson-bom".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "org.slf4j".to_string(),
                    artivact_id: "slf4j-api".to_string(),
                    version: "2.0.10".to_string(),
                },
                Dependency {
                    group_id: "com.puppycrawl.tools".to_string(),
                    artivact_id: "checkstyle".to_string(),
                    version: "10.3.3".to_string(),
                },
                Dependency {
                    group_id: "info.picocli".to_string(),
                    artivact_id: "picocli".to_string(),
                    version: "4.6.3".to_string(),
                },
                Dependency {
                    group_id: "org.antlr".to_string(),
                    artivact_id: "antlr4-runtime".to_string(),
                    version: "4.10.1".to_string(),
                },
                Dependency {
                    group_id: "commons-beanutils".to_string(),
                    artivact_id: "commons-beanutils".to_string(),
                    version: "1.9.4".to_string(),
                },
                Dependency {
                    group_id: "commons-collections".to_string(),
                    artivact_id: "commons-collections".to_string(),
                    version: "3.2.2".to_string(),
                },
                Dependency {
                    group_id: "com.google.guava".to_string(),
                    artivact_id: "guava".to_string(),
                    version: "31.1-jre".to_string(),
                },
                Dependency {
                    group_id: "com.google.guava".to_string(),
                    artivact_id: "failureaccess".to_string(),
                    version: "1.0.1".to_string(),
                },
                Dependency {
                    group_id: "com.google.guava".to_string(),
                    artivact_id: "listenablefuture".to_string(),
                    version: "9999.0-empty-to-avoid-conflict-with-guava".to_string(),
                },
                Dependency {
                    group_id: "com.google.code.findbugs".to_string(),
                    artivact_id: "jsr305".to_string(),
                    version: "3.0.2".to_string(),
                },
                Dependency {
                    group_id: "org.checkerframework".to_string(),
                    artivact_id: "checker-qual".to_string(),
                    version: "3.12.0".to_string(),
                },
                Dependency {
                    group_id: "com.google.errorprone".to_string(),
                    artivact_id: "error_prone_annotations".to_string(),
                    version: "2.11.0".to_string(),
                },
                Dependency {
                    group_id: "com.google.j2objc".to_string(),
                    artivact_id: "j2objc-annotations".to_string(),
                    version: "1.3".to_string(),
                },
                Dependency {
                    group_id: "org.reflections".to_string(),
                    artivact_id: "reflections".to_string(),
                    version: "0.10.2".to_string(),
                },
                Dependency {
                    group_id: "org.javassist".to_string(),
                    artivact_id: "javassist".to_string(),
                    version: "3.28.0-GA".to_string(),
                },
                Dependency {
                    group_id: "com.google.code.findbugs".to_string(),
                    artivact_id: "jsr305".to_string(),
                    version: "3.0.2".to_string(),
                },
                Dependency {
                    group_id: "net.sf.saxon".to_string(),
                    artivact_id: "Saxon-HE".to_string(),
                    version: "11.4".to_string(),
                },
                Dependency {
                    group_id: "org.xmlresolver".to_string(),
                    artivact_id: "xmlresolver".to_string(),
                    version: "4.4.3".to_string(),
                },
                Dependency {
                    group_id: "org.apache.httpcomponents.client5".to_string(),
                    artivact_id: "httpclient5".to_string(),
                    version: "5.1.3".to_string(),
                },
                Dependency {
                    group_id: "org.apache.httpcomponents.core5".to_string(),
                    artivact_id: "httpcore5".to_string(),
                    version: "5.1.3".to_string(),
                },
                Dependency {
                    group_id: "org.apache.httpcomponents.core5".to_string(),
                    artivact_id: "httpcore5-h2".to_string(),
                    version: "5.1.3".to_string(),
                },
                Dependency {
                    group_id: "org.apache.httpcomponents.core5".to_string(),
                    artivact_id: "httpcore5".to_string(),
                    version: "5.1.3".to_string(),
                },
                Dependency {
                    group_id: "commons-codec".to_string(),
                    artivact_id: "commons-codec".to_string(),
                    version: "1.15".to_string(),
                },
                Dependency {
                    group_id: "org.apache.httpcomponents.core5".to_string(),
                    artivact_id: "httpcore5".to_string(),
                    version: "5.1.3".to_string(),
                },
                Dependency {
                    group_id: "com.github.toolarium".to_string(),
                    artivact_id: "toolarium-enum-configuration".to_string(),
                    version: "1.2.0".to_string(),
                },
                Dependency {
                    group_id: "org.slf4j".to_string(),
                    artivact_id: "slf4j-api".to_string(),
                    version: "2.0.13".to_string(),
                },
                Dependency {
                    group_id: "com.github.toolarium".to_string(),
                    artivact_id: "toolarium-enum-configuration".to_string(),
                    version: "1.2.0".to_string(),
                },
                Dependency {
                    group_id: "org.slf4j".to_string(),
                    artivact_id: "slf4j-api".to_string(),
                    version: "2.0.13".to_string(),
                },
                Dependency {
                    group_id: "org.jacoco".to_string(),
                    artivact_id: "org.jacoco.agent".to_string(),
                    version: "0.8.9".to_string(),
                },
                Dependency {
                    group_id: "org.jacoco".to_string(),
                    artivact_id: "org.jacoco.ant".to_string(),
                    version: "0.8.9".to_string(),
                },
                Dependency {
                    group_id: "org.jacoco".to_string(),
                    artivact_id: "org.jacoco.core".to_string(),
                    version: "0.8.9".to_string(),
                },
                Dependency {
                    group_id: "org.ow2.asm".to_string(),
                    artivact_id: "asm".to_string(),
                    version: "9.5".to_string(),
                },
                Dependency {
                    group_id: "org.ow2.asm".to_string(),
                    artivact_id: "asm-commons".to_string(),
                    version: "9.5".to_string(),
                },
                Dependency {
                    group_id: "org.ow2.asm".to_string(),
                    artivact_id: "asm".to_string(),
                    version: "9.5".to_string(),
                },
                Dependency {
                    group_id: "org.ow2.asm".to_string(),
                    artivact_id: "asm-tree".to_string(),
                    version: "9.5".to_string(),
                },
                Dependency {
                    group_id: "org.ow2.asm".to_string(),
                    artivact_id: "asm".to_string(),
                    version: "9.5".to_string(),
                },
                Dependency {
                    group_id: "org.ow2.asm".to_string(),
                    artivact_id: "asm-tree".to_string(),
                    version: "9.5".to_string(),
                },
                Dependency {
                    group_id: "org.jacoco".to_string(),
                    artivact_id: "org.jacoco.report".to_string(),
                    version: "0.8.9".to_string(),
                },
                Dependency {
                    group_id: "org.jacoco".to_string(),
                    artivact_id: "org.jacoco.core".to_string(),
                    version: "0.8.9".to_string(),
                },
                Dependency {
                    group_id: "org.jacoco".to_string(),
                    artivact_id: "org.jacoco.agent".to_string(),
                    version: "0.8.9".to_string(),
                },
                Dependency {
                    group_id: "net.sf.jptools".to_string(),
                    artivact_id: "jptools".to_string(),
                    version: "1.7.11".to_string(),
                },
                Dependency {
                    group_id: "net.sourceforge.jexcelapi".to_string(),
                    artivact_id: "jxl".to_string(),
                    version: "2.6.12".to_string(),
                },
                Dependency {
                    group_id: "org.apache.poi".to_string(),
                    artivact_id: "poi".to_string(),
                    version: "3.9".to_string(),
                },
                Dependency {
                    group_id: "commons-codec".to_string(),
                    artivact_id: "commons-codec".to_string(),
                    version: "1.5".to_string(),
                },
                Dependency {
                    group_id: "org.apache.poi".to_string(),
                    artivact_id: "poi-ooxml".to_string(),
                    version: "3.9".to_string(),
                },
                Dependency {
                    group_id: "org.apache.poi".to_string(),
                    artivact_id: "poi".to_string(),
                    version: "3.9".to_string(),
                },
                Dependency {
                    group_id: "org.apache.poi".to_string(),
                    artivact_id: "poi-ooxml-schemas".to_string(),
                    version: "3.9".to_string(),
                },
                Dependency {
                    group_id: "org.apache.xmlbeans".to_string(),
                    artivact_id: "xmlbeans".to_string(),
                    version: "2.3.0".to_string(),
                },
                Dependency {
                    group_id: "stax".to_string(),
                    artivact_id: "stax-api".to_string(),
                    version: "1.0.1".to_string(),
                },
                Dependency {
                    group_id: "dom4j".to_string(),
                    artivact_id: "dom4j".to_string(),
                    version: "1.6.1".to_string(),
                },
                Dependency {
                    group_id: "xml-apis".to_string(),
                    artivact_id: "xml-apis".to_string(),
                    version: "1.0.b2".to_string(),
                },
                Dependency {
                    group_id: "org.slf4j".to_string(),
                    artivact_id: "slf4j-api".to_string(),
                    version: "2.0.13".to_string(),
                },
                Dependency {
                    group_id: "org.slf4j".to_string(),
                    artivact_id: "slf4j-api".to_string(),
                    version: "2.0.13".to_string(),
                },
                Dependency {
                    group_id: "org.junit.jupiter".to_string(),
                    artivact_id: "junit-jupiter-api".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit".to_string(),
                    artivact_id: "junit-bom".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit.jupiter".to_string(),
                    artivact_id: "junit-jupiter-api".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit.platform".to_string(),
                    artivact_id: "junit-platform-commons".to_string(),
                    version: "1.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.apiguardian".to_string(),
                    artivact_id: "apiguardian-api".to_string(),
                    version: "1.1.0".to_string(),
                },
                Dependency {
                    group_id: "org.opentest4j".to_string(),
                    artivact_id: "opentest4j".to_string(),
                    version: "1.2.0".to_string(),
                },
                Dependency {
                    group_id: "org.junit.platform".to_string(),
                    artivact_id: "junit-platform-commons".to_string(),
                    version: "1.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit".to_string(),
                    artivact_id: "junit-bom".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.apiguardian".to_string(),
                    artivact_id: "apiguardian-api".to_string(),
                    version: "1.1.0".to_string(),
                },
                Dependency {
                    group_id: "com.github.toolarium".to_string(),
                    artivact_id: "toolarium-enum-configuration".to_string(),
                    version: "1.2.0".to_string(),
                },
                Dependency {
                    group_id: "org.junit.jupiter".to_string(),
                    artivact_id: "junit-jupiter-api".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "com.github.toolarium".to_string(),
                    artivact_id: "toolarium-enum-configuration".to_string(),
                    version: "1.2.0".to_string(),
                },
                Dependency {
                    group_id: "org.slf4j".to_string(),
                    artivact_id: "slf4j-api".to_string(),
                    version: "2.0.13".to_string(),
                },
                Dependency {
                    group_id: "org.junit.jupiter".to_string(),
                    artivact_id: "junit-jupiter-api".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit".to_string(),
                    artivact_id: "junit-bom".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit.jupiter".to_string(),
                    artivact_id: "junit-jupiter-api".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit.jupiter".to_string(),
                    artivact_id: "junit-jupiter-engine".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit.platform".to_string(),
                    artivact_id: "junit-platform-commons".to_string(),
                    version: "1.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit.platform".to_string(),
                    artivact_id: "junit-platform-engine".to_string(),
                    version: "1.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit.platform".to_string(),
                    artivact_id: "junit-platform-launcher".to_string(),
                    version: "1.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.apiguardian".to_string(),
                    artivact_id: "apiguardian-api".to_string(),
                    version: "1.1.0".to_string(),
                },
                Dependency {
                    group_id: "org.opentest4j".to_string(),
                    artivact_id: "opentest4j".to_string(),
                    version: "1.2.0".to_string(),
                },
                Dependency {
                    group_id: "org.junit.platform".to_string(),
                    artivact_id: "junit-platform-commons".to_string(),
                    version: "1.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit".to_string(),
                    artivact_id: "junit-bom".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.apiguardian".to_string(),
                    artivact_id: "apiguardian-api".to_string(),
                    version: "1.1.0".to_string(),
                },
                Dependency {
                    group_id: "com.github.toolarium".to_string(),
                    artivact_id: "toolarium-enum-configuration".to_string(),
                    version: "1.2.0".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-databind".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-annotations".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson".to_string(),
                    artivact_id: "jackson-bom".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-annotations".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-core".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-databind".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.datatype".to_string(),
                    artivact_id: "jackson-datatype-jsr310".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-core".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson".to_string(),
                    artivact_id: "jackson-bom".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson".to_string(),
                    artivact_id: "jackson-bom".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-core".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.datatype".to_string(),
                    artivact_id: "jackson-datatype-jsr310".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-annotations".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-core".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson.core".to_string(),
                    artivact_id: "jackson-databind".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "com.fasterxml.jackson".to_string(),
                    artivact_id: "jackson-bom".to_string(),
                    version: "2.17.1".to_string(),
                },
                Dependency {
                    group_id: "org.slf4j".to_string(),
                    artivact_id: "slf4j-api".to_string(),
                    version: "2.0.10->2.0.13".to_string(),
                },
                Dependency {
                    group_id: "org.junit.jupiter".to_string(),
                    artivact_id: "junit-jupiter-engine".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit".to_string(),
                    artivact_id: "junit-bom".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.apiguardian".to_string(),
                    artivact_id: "apiguardian-api".to_string(),
                    version: "1.1.0".to_string(),
                },
                Dependency {
                    group_id: "org.junit.platform".to_string(),
                    artivact_id: "junit-platform-engine".to_string(),
                    version: "1.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit".to_string(),
                    artivact_id: "junit-bom".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.apiguardian".to_string(),
                    artivact_id: "apiguardian-api".to_string(),
                    version: "1.1.0".to_string(),
                },
                Dependency {
                    group_id: "org.opentest4j".to_string(),
                    artivact_id: "opentest4j".to_string(),
                    version: "1.2.0".to_string(),
                },
                Dependency {
                    group_id: "org.junit.platform".to_string(),
                    artivact_id: "junit-platform-commons".to_string(),
                    version: "1.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit.jupiter".to_string(),
                    artivact_id: "junit-jupiter-api".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.junit".to_string(),
                    artivact_id: "junit-bom".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "org.apiguardian".to_string(),
                    artivact_id: "apiguardian-api".to_string(),
                    version: "1.1.0".to_string(),
                },
                Dependency {
                    group_id: "org.junit.platform".to_string(),
                    artivact_id: "junit-platform-engine".to_string(),
                    version: "1.7.1".to_string(),
                },
                Dependency {
                    group_id: "ch.qos.logback".to_string(),
                    artivact_id: "logback-classic".to_string(),
                    version: "1.5.6".to_string(),
                },
                Dependency {
                    group_id: "ch.qos.logback".to_string(),
                    artivact_id: "logback-core".to_string(),
                    version: "1.5.6".to_string(),
                },
                Dependency {
                    group_id: "org.slf4j".to_string(),
                    artivact_id: "slf4j-api".to_string(),
                    version: "2.0.13".to_string(),
                },
                Dependency {
                    group_id: "org.junit.jupiter".to_string(),
                    artivact_id: "junit-jupiter-engine".to_string(),
                    version: "5.7.1".to_string(),
                },
                Dependency {
                    group_id: "ch.qos.logback".to_string(),
                    artivact_id: "logback-classic".to_string(),
                    version: "1.5.6".to_string(),
                },
            ]
        );
    }
}
