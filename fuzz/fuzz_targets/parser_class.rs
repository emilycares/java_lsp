#![no_main]

use dto::SourceDestination;
use libfuzzer_sys::fuzz_target;
use my_string::NuVec;

fuzz_target!(|data: &[u8]| {
    let _ = class::load_class(
        data,
        NuVec::new_static(b"c.e.E"),
        SourceDestination::None,
        false,
    );
});
