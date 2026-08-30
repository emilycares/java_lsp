#![no_main]

use ast::types::AstFile;
use dto::SourceDestination;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: AstFile| {
    let _ = parser::java::load_java_tree(&data, SourceDestination::None);
});
