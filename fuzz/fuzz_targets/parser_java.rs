#![no_main]

extern crate parser;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(tokens) = ast::lexer::lex(data) {
        let _ = ast::parse_file(&tokens);
    }
});
