use std::{io::Cursor, sync::Arc};

use nickel_lang_core::{error::NullReporter, eval::cache::CacheImpl, program::Program};
use once_cell::sync::Lazy;
use serde::Deserialize;

pub static CONFIG: Lazy<Arc<Configuration>> = Lazy::new(|| Arc::new(Configuration::default()));

#[derive(Debug, serde::Deserialize, PartialEq, serde::Serialize, Clone)]
pub struct Configuration {
    pub formatter: FormatterConfig,
}

#[derive(Debug, serde::Deserialize, PartialEq, serde::Serialize, Clone)]
pub enum FormatterConfig {
    Topiary,
    Intelij,
    None,
}

/// <https://github.com/tweag/topiary/blob/f96e12c6b2730e42f7b6b425f07aadd723aee5db/topiary-config/src/lib.rs#L223>
impl Default for Configuration {
    fn default() -> Self {
        let def = include_bytes!("../config.ncl");
        let mut program = Program::<CacheImpl>::new_from_source(
            Cursor::new(def),
            "builtin",
            std::io::empty(),
            NullReporter {},
        )
        .expect("Evaluating the builtin configuration should be safe");
        let term = program
            .eval_full_for_export()
            .expect("Evaluating the builtin configuration should be safe");
        Configuration::deserialize(term)
            .expect("Evaluating the builtin configuration should be safe")
    }
}
