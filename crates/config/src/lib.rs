#![deny(clippy::redundant_clone)]
use std::sync::LazyLock;

pub static CONFIG: LazyLock<Configuration> = LazyLock::new(Configuration::default);

#[derive(Debug, PartialEq, Clone)]
pub struct Configuration {
    pub formatter: FormatterConfig,
}

#[derive(Debug, PartialEq, Clone)]
pub enum FormatterConfig {
    Intelij,
    None,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            formatter: FormatterConfig::None,
        }
    }
}
