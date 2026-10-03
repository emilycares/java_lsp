#![no_main]

use libfuzzer_sys::fuzz_target;
use my_string::NuVec;

fuzz_target!(|data: &[u8]| {
    let _ = decompiler::decompile_class(data, &NuVec::Static(b""));
});
