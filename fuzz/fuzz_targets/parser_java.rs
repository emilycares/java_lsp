#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut tokens = Vec::new();
    if ast::lexer::lex_mut::<false>(data, &mut tokens).is_ok() {
        let _ = ast::parse_file(&tokens);
    }
});
