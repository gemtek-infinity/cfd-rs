use std::ffi::OsString;

use crate::runtime;
use crate::startup::{StartupSurface, render_run_output, render_validate_output, resolve_startup};

use super::{Cli, CliOutput, Command, parse_args};
use super::{error::CliError, help::render_help};

const PROGRAM_NAME: &str = "cloudflared";

pub(crate) fn execute(args: impl IntoIterator<Item = OsString>) -> CliOutput {
    match parse_args(args) {
        Ok(cli) => execute_command(cli),
        Err(message) => CliError::usage(message).into_output(),
    }
}

fn execute_command(cli: Cli) -> CliOutput {
    match cli.command {
        Command::Help => CliOutput::success(render_help(PROGRAM_NAME)),
        Command::Version => CliOutput::success(format!("{PROGRAM_NAME} {}\n", env!("CARGO_PKG_VERSION"))),
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
