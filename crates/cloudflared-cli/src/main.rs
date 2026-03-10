#![forbid(unsafe_code)]

mod app;
mod cli;
mod output;
mod runtime;
mod startup;
mod transport;

use std::env;
use std::process::ExitCode;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

fn main() -> ExitCode {
    let output = app::execute(env::args_os());

    if !output.stdout.is_empty() {
        print!("{}", output.stdout);
    }
    if !output.stderr.is_empty() {
        eprint!("{}", output.stderr);
    }

    ExitCode::from(output.exit_code)
}
