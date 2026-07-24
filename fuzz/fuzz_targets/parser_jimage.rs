#![no_main]

use libfuzzer_sys::fuzz_target;
use my_string::NuVec;

fuzz_target!(|data: &[u8]| {
    let _ = jimage::parser(data, 0, &NuVec::new_static(b""), true);
});
