#![no_main]

use libfuzzer_sys::fuzz_target;
use my_string::smol_str::SmolStr;

fuzz_target!(|data: &[u8]| {
    let _ = jimage::parser(data, 0, &SmolStr::new_inline(""), true);
});
