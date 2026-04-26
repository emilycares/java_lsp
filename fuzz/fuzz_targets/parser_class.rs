#![no_main]

extern crate parser;

use dto::SourceDestination;
use libfuzzer_sys::fuzz_target;
use my_string::smol_str::SmolStr;

fuzz_target!(|data: &[u8]| {
    let _ = parser::class::load_class(
        data,
        SmolStr::new_inline("c.e.E"),
        SourceDestination::None,
        false,
    );
});
