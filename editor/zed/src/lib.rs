use zed_extension_api::{self as zed, Command, LanguageServerId, Os, Result, Worktree};

struct JavaLsp {}

impl zed::Extension for JavaLsp {
    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        // When there are releases here is how to fetch the releas
        // https://github.com/zed-extensions/terraform/blob/5ea4664f1cf43e77456631f50639c970f944eea5/src/terraform.rs
        let lset = zed::settings::LspSettings::for_worktree("java_lsp", worktree)?;
        if let Some(s) = lset.binary
            && let Some(bin) = s.path
        {
            return Ok(zed::Command {
                command: bin,
                args: vec![],
                env: vec![],
            });
        }
        let command = match zed::current_platform().0 {
            Os::Mac | Os::Linux => "java_lsp",
            Os::Windows => "java_lsp.exe",
        };
        if let Some(pcommand) = worktree.which(command) {
            return Ok(zed::Command {
                command: pcommand.to_string(),
                args: vec![],
                env: vec![],
            });
        }
        return Ok(zed::Command {
            command: "java_lsp command not found in path".to_string(),
            args: vec![],
            env: vec![],
        });
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {}
    }
}

zed::register_extension!(JavaLsp);
