use std::{process::Command, str::FromStr};

use nom::branch::alt;
use nom::{
    bytes::complete::{tag, take_until},
    multi::separated_list0,
    sequence::delimited,
    IResult,
};
use serde::{Deserialize, Serialize};

use crate::MavenError;

pub fn load<'a>() -> Result<Dependency<'a>, MavenError> {
    let log: String = get_cli_output()?;
    let cut: String = cut_output(log);
    // let mut output = File::create("/tmp/tree")?;
    // write!(output, "{}", cut)?;
    let input: &'static str = Box::leak(cut.into_boxed_str());
    let out = parser(input);
    match out {
        Ok(o) => Ok(o.1),
        Err(e) => Err(MavenError::TreeParseError(e)),
    }
}

fn get_cli_output() -> Result<String, MavenError> {
    // mvn dependency:tree -DoutputType=dot -b
    let output = Command::new("mvn")
        .arg("dependency:tree")
        .arg("-DoutputType=dot")
        // .arg("-b")
        .output()?;

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
pub struct Dependency<'a> {
    #[serde(borrow)]
    pub base: Pom<'a>,
    pub deps: Vec<Pom<'a>>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Pom<'a> {
    pub group_id: &'a str,
    pub artivact_id: &'a str,
    pub version: &'a str,
    pub scope: DependencyScope,
}

/// https://maven.apache.org/guides/introduction/introduction-to-dependency-mechanism.html#dependency-scope
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
    type Err = MavenError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "compile" => Ok(Self::Compile),
            "provided" => Ok(Self::Provided),
            "runtime" => Ok(Self::Runtime),
            "test" => Ok(Self::Test),
            "system" => Ok(Self::System),
            "import" => Ok(Self::Import),
            _ => Err(MavenError::UnknownDependencyScope),
        }
    }
}

fn parse_pom(input: &str) -> IResult<&str, Pom> {
    let (input, group_id) = take_until(":")(input)?;
    let (input, _) = tag(":")(input)?;
    let (input, artivact_id) = take_until(":")(input)?;
    let (input, _) = tag(":")(input)?;
    let (input, _type) = take_until(":")(input)?;
    let (input, _) = tag(":")(input)?;
    let (input, version) = take_until("\"")(input)?;
    Ok((
        input,
        Pom {
            group_id,
            artivact_id,
            version,
            scope: DependencyScope::Test,
        },
    ))
}
fn parse_pom_b(input: &str) -> IResult<&str, Pom> {
    let (input, group_id) = take_until(":")(input)?;
    let (input, _) = tag(":")(input)?;
    let (input, artivact_id) = take_until(":")(input)?;
    let (input, _) = tag(":")(input)?;
    let (input, _type) = take_until(":")(input)?;
    let (input, _) = tag(":")(input)?;
    let (input, version) = take_until(":")(input)?;
    let (input, _) = tag(":")(input)?;
    let (input, scope) = take_until("\"")(input)?;
    Ok((
        input,
        Pom {
            group_id,
            artivact_id,
            version,
            scope: scope.parse().expect("UnknownDependencyScope"),
        },
    ))
}
fn parse_relation(input: &str) -> IResult<&str, Pom> {
    let (input, _) = take_until(" -> ")(input)?;
    let (input, _) = tag(" -> ")(input)?;
    let (input, out) = delimited(tag("\""), parse_pom_b, tag("\""))(input)?;

    Ok((input, out))
}

fn parser(input: &str) -> IResult<&str, Dependency> {
    let (input, _) = tag("[INFO] digraph ")(input)?;
    let (input, base) = delimited(tag("\""), parse_pom, tag("\""))(input)?;
    let (input, _) = alt((tag(" {\n[INFO]  "), tag(" { \n[INFO] \t")))(input)?;
    let (input, deps) = separated_list0(
        alt((tag(" ;\n[INFO]  "), tag(" ; \n[INFO] \t"))),
        parse_relation,
    )(input)?;

    let (input, _) = take_until("[INFO]")(input)?;
    let (input, _) = tag("[INFO]  }")(input)?;
    let (input, _) = take_until("\n")(input)?;
    let (input, _) = tag("\n")(input)?;
    Ok((input, Dependency { base, deps }))
}

#[cfg(test)]
mod tests {
    use crate::tree::{cut_output, parser};

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
        let out = parser(&cut);
        let out = out.unwrap();
        insta::assert_yaml_snapshot!(out.1);
    }

    #[test]
    fn parse_diagram_with_tab() {
        let inp = include_str!("../tests/tverify-tap.bacic.txt");
        let out = parser(inp);
        let out = out.unwrap();
        insta::assert_yaml_snapshot!(out.1);
    }
}
