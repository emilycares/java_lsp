#![no_main]

extern crate parser;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = parser::class::load_module(data);
});
