#![no_main]
use arbitrary::{Arbitrary, Unstructured};
use ast::lexer::PositionToken;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);
    if let Ok(tokens) = Vec::<PositionToken>::arbitrary(&mut unstructured) {
        let _ = ast::parse_file(&tokens);
    }

});

