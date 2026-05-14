#[derive(Debug, PartialEq, Clone)]
pub struct Configuration {
    pub formatter: FormatterConfig,
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
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            formatter: FormatterConfig::None,
        }
    }
}
