#![no_main]

use ast::types::AstFile;
use editorconfig::EditorConfigFilled;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: AstFile| {
    let _ = formatter::internal(&data, b"", &EditorConfigFilled::default());
});
