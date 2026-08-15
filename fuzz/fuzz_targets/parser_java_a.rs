#![no_main]
use ast::lexer::PositionToken;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: Vec::<PositionToken>| {
    let _ = ast::parse_file(&data);
});
