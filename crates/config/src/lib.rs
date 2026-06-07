#[derive(Debug, PartialEq, Clone)]
pub struct Configuration {
    pub formatter: FormatterConfig,
    pub editor_runs_commands: bool,
}

impl Configuration {
    pub fn missing(field: &str) {
        eprintln!(
            r#"Missing configuration for {}, Please configure in LSP InitializeParams.initializationOptions, example: {{ "formatter": "google" }}"#,
            field
        )
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum FormatterConfig {
    None,
    Google,
    Idea,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            formatter: FormatterConfig::None,
            editor_runs_commands: false,
        }
    }
}
