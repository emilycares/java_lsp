use std::sync::Arc;

use once_cell::sync::Lazy;

pub static CONFIG: Lazy<Arc<Configuration>> = Lazy::new(|| Arc::new(Configuration::default()));

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
