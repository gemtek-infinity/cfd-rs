#![forbid(unsafe_code)]

mod protocol;
mod proxy;
mod runtime;
mod startup;
mod transport;

use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

use cfdrs_cli::{
    Cli, CliError, CliOutput, Command, PROGRAM_NAME, parse_args, render_help, render_version_output,
};
use mimalloc::MiMalloc;

use crate::startup::{StartupSurface, render_run_output, render_validate_output, resolve_startup};

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

fn main() -> ExitCode {
    let output = execute(env::args_os());

    if !output.stdout.is_empty() {
        print!("{}", output.stdout);
    }

    if !output.stderr.is_empty() {
        eprint!("{}", output.stderr);
    }

    ExitCode::from(output.exit_code)
}

fn execute(args: impl IntoIterator<Item = OsString>) -> CliOutput {
    match parse_args(args) {
        Ok(cli) => execute_command(cli),
        Err(message) => CliError::usage(message).into_output(),
    }
}

fn execute_command(cli: Cli) -> CliOutput {
    match cli.command {
        Command::Help => CliOutput::success(render_help(PROGRAM_NAME)),
        Command::Version => CliOutput::success(render_version_output(PROGRAM_NAME)),
        Command::Validate => execute_startup_command(cli, CliMode::Validate),
        Command::Run => execute_startup_command(cli, CliMode::Run),
    }
}

fn execute_startup_command(cli: Cli, mode: CliMode) -> CliOutput {
    match resolve_startup(cli.config_path) {
        Ok(startup) => match mode {
            CliMode::Validate => CliOutput::success(render_validate_output(&startup)),
            CliMode::Run => execute_runtime_command(startup),
        },
        Err(error) => CliError::config(error).into_output(),
    }
}

fn execute_runtime_command(startup: StartupSurface) -> CliOutput {
    runtime::install_runtime_logging();
    let report = runtime::run(runtime::RuntimeConfig::new(
        startup.discovery.clone(),
        startup.normalized.clone(),
    ));
    let stdout = render_run_output(&startup, &report);

    match report.exit.stderr_message() {
        Some(stderr) => CliOutput::failure(stdout, stderr, report.exit.exit_code()),
        None => CliOutput::success(stdout),
    }
}

enum CliMode {
    Validate,
    Run,
}
