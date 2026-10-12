use std::process::ExitCode;

fn main() -> ExitCode {
    lambda_discipline::cli::main()
}

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
