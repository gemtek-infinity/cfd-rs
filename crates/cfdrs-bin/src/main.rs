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
    Cli, CliError, CliOutput, Command, DB_CONNECT_REMOVED_MSG, PROGRAM_NAME, PROXY_DNS_REMOVED_MSG,
    TunnelSubcommand, parse_args, render_help, render_version_output, stub_not_implemented,
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
    match &cli.command {
        Command::Help => CliOutput::success(render_help(PROGRAM_NAME)),
        Command::Version => CliOutput::success(render_version_output(PROGRAM_NAME)),
        Command::Validate => execute_startup_command(&cli, CliMode::Validate),

        Command::Tunnel(TunnelSubcommand::Run) => execute_startup_command(&cli, CliMode::Run),

        // Bare tunnel invocation without subcommand — currently delegates to run.
        // Go baseline: TunnelCommand() which starts quick tunnel or named tunnel.
        Command::Tunnel(TunnelSubcommand::Bare) => execute_startup_command(&cli, CliMode::Run),

        // Service mode — empty invocation enters daemon mode.
        // Go baseline: handleServiceMode() in main.go.
        Command::ServiceMode => CliOutput::failure(String::new(), stub_not_implemented("(service-mode)"), 1),

        // Stubs for commands that exist in the Go baseline but are not yet
        // implemented in the Rust rewrite. Each prints a clear message so
        // callers know the command is recognized but deferred.
        Command::Update => CliOutput::failure(String::new(), stub_not_implemented("update"), 1),
        Command::Login => CliOutput::failure(String::new(), stub_not_implemented("login"), 1),
        Command::Access => CliOutput::failure(String::new(), stub_not_implemented("access"), 1),
        Command::Tail => CliOutput::failure(String::new(), stub_not_implemented("tail"), 1),
        Command::Management => CliOutput::failure(String::new(), stub_not_implemented("management"), 1),

        Command::Service(_) => CliOutput::failure(String::new(), stub_not_implemented("service"), 1),

        // Removed features — exact error messages from Go baseline.
        Command::ProxyDns => CliOutput::failure(String::new(), PROXY_DNS_REMOVED_MSG.to_owned(), 1),
        Command::Tunnel(TunnelSubcommand::DbConnect) => {
            CliOutput::failure(String::new(), DB_CONNECT_REMOVED_MSG.to_owned(), 1)
        }

        // Tunnel subcommands not yet implemented.
        Command::Tunnel(sub) => {
            let label = format!("tunnel {sub}");
            CliOutput::failure(String::new(), stub_not_implemented(&label), 1)
        }
    }
}

fn execute_startup_command(cli: &Cli, mode: CliMode) -> CliOutput {
    match resolve_startup(cli.flags.config_path.clone()) {
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
