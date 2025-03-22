use zed_extension_api::{self as zed, Command, LanguageServerId, Result, Worktree};

struct JavaLsp {}

impl zed::Extension for JavaLsp {
    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        _worktree: &Worktree,
    ) -> Result<Command> {
        Ok(zed::Command {
            command: r#"java_lsp"#.to_owned(),
            args: vec![],
            env: vec![],
        })
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {}
    }
}

zed::register_extension!(JavaLsp);
