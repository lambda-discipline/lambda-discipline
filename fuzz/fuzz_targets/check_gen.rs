#![no_main]

//! Structure-aware fuzzing: the input bytes drive `lbd-smith`, so every
//! program reaches the type checker. Well-typed programs must be accepted,
//! with and without annotations, and their ill-typed mutants rejected.

use arbitrary::Unstructured;
use lbd_fuzz::{check, on_big_stack};
use lbd_smith::Config;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    let Ok(mut program) = lbd_smith::program(&mut u, &Config::default()) else {
        return;
    };
    let Ok(mutant) = lbd_smith::mutate(&mut u, &program) else {
        return;
    };
    let plain = program.to_string();
    program.annotate();
    let annotated = program.to_string();
    let mutant = mutant.to_string();
    on_big_stack(move || {
        assert!(check(&plain), "rejected a well-typed program:\n{plain}");
        assert!(
            check(&annotated),
            "rejected a well-typed program:\n{annotated}"
        );
        assert!(!check(&mutant), "accepted an ill-typed program:\n{mutant}");
    });
});
