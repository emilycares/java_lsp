#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(tokens) = ast::lexer::lex(data, true) {
        let _ = ast::parse_file(&tokens);
    }
});
