#![no_main]

use lambda_discipline::source::SourceMap;
use lambda_discipline::syntax::parse_module;
use lbd_fuzz::{assert_diagnostics_valid, on_big_stack};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let text = text.to_owned();
    on_big_stack(move || {
        let mut sources = SourceMap::new();
        let file = sources.add_virtual("<fuzz>", text.as_str());
        let (module, diagnostics) = parse_module(file, &text);
        assert_diagnostics_valid(&sources, &diagnostics);
        let _ = module.to_string();
    });
});
