#![no_main]

extern crate parser;

use dto::SourceDestination;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = parser::class::load_class(
        data,
        "ch.emilycares.Everything".into(),
        SourceDestination::None,
        false,
    );
});
