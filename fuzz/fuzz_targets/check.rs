#![no_main]

use lbd_fuzz::{check, on_big_stack};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let text = text.to_owned();
    on_big_stack(move || check(&text));
});
