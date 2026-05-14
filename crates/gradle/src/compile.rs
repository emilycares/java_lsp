use std::process::{Command, Stdio};

use compile::{CompileErrorMessage, parse_compile_errors};

#[must_use]
pub fn compile_java(executable_gradle: &str) -> Option<Vec<CompileErrorMessage>> {
    run_compile_java(executable_gradle).map(|log| parse_compile_errors(&log))
}

/// Telling gradle to give java compiler errors
fn run_compile_java(executable_gradle: &str) -> Option<String> {
    // ./gradlew compileJava -q
    let child = Command::new(executable_gradle)
        .arg("compileJava")
        .arg("-q")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    let output = child.wait_with_output().ok()?;
    eprintln!("{}", String::from_utf8_lossy(output.stderr.as_slice()));

    Some(String::from_utf8_lossy(&output.stderr).to_string())
}

#[cfg(test)]
mod tests {
    use compile::parse_compile_errors;
    use expect_test::expect;

    #[test]
    fn gradle_compile() {
        let inp = include_str!("../tests/compile_basic.txt");
        let out = parse_compile_errors(inp);
        let expected = expect![[r#"
            [
                CompileErrorMessage {
                    path: "/home/emily/tmp/vanilla-gradle/app/src/main/java/org/example/Other.java",
                    message: "illegal start of type",
                    row: 4,
                    col: 2,
                },
                CompileErrorMessage {
                    path: "/home/emily/tmp/vanilla-gradle/app/src/main/java/org/example/App.java",
                    message: "';' expected",
                    row: 4,
                    col: 19,
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn cb_compile_failure() {
        let input = r#"
* What went wrong:
Execution failed for task ':compileTestJava'.
> Compilation failed; see the compiler output below.
  /home/emily/tmp/java-test/toolarium-icap-client/src/test/java/com/github/toolarium/icap/client/TestOptions.java:25: error: class TestOptionsAA is public, should be declared in a file named TestOptionsAA.java
  public class TestOptionsAA extends AbstractICAPClientTet {
         ^
  /home/emily/tmp/java-test/toolarium-icap-client/src/test/java/com/github/toolarium/icap/client/TestOptions.java:25: error: cannot find symbol
  public class TestOptionsAA extends AbstractICAPClientTet {
                                     ^
    symbol: class AbstractICAPClientTet
  /home/emily/tmp/java-test/toolarium-icap-client/src/test/java/com/github/toolarium/icap/client/TestOptions.java:34: error: cannot find symbol
          ICAPRemoteServiceConfiguration remoteServiceConfiguration = getICAPClient().options();
                                                                      ^
    symbol:   method getICAPClient()
    location: class TestOptionsAA                                               
  /home/emily/tmp/java-test/toolarium-icap-client/src/test/java/com/github/toolarium/icap/client/TestOptions.java:53: error: cannot find symbol
              ICAPClientFactory.getInstance().getICAPClient("localhost", 1345, SERVICE).options();
                                                                               ^
    symbol:   variable SERVICE
    location: class TestOptionsAA
  4 errors

* Try:
"#;
        let out = parse_compile_errors(input);
        let expected = expect![[r#"
            [
                CompileErrorMessage {
                    path: "/home/emily/tmp/java-test/toolarium-icap-client/src/test/java/com/github/toolarium/icap/client/TestOptions.java",
                    message: "class TestOptionsAA is public, should be declared in a file named TestOptionsAA.java",
                    row: 25,
                    col: 9,
                },
                CompileErrorMessage {
                    path: "/home/emily/tmp/java-test/toolarium-icap-client/src/test/java/com/github/toolarium/icap/client/TestOptions.java",
                    message: "cannot find symbol",
                    row: 25,
                    col: 37,
                },
                CompileErrorMessage {
                    path: "/home/emily/tmp/java-test/toolarium-icap-client/src/test/java/com/github/toolarium/icap/client/TestOptions.java",
                    message: "cannot find symbol",
                    row: 34,
                    col: 70,
                },
                CompileErrorMessage {
                    path: "/home/emily/tmp/java-test/toolarium-icap-client/src/test/java/com/github/toolarium/icap/client/TestOptions.java",
                    message: "cannot find symbol",
                    row: 53,
                    col: 79,
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }
}
