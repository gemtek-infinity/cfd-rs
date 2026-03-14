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
    AccessSubcommand, Cli, CliError, CliOutput, Command, DB_CONNECT_REMOVED_MSG, IngressSubcommand,
    IpRouteSubcommand, ManagementSubcommand, PROGRAM_NAME, PROXY_DNS_REMOVED_MSG, RouteSubcommand,
    TailSubcommand, TunnelSubcommand, VnetSubcommand, parse_args, render_help, render_short_version,
    render_version_output, stub_not_implemented,
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
        Command::Version { short: true } => CliOutput::success(render_short_version()),
        Command::Version { short: false } => CliOutput::success(render_version_output(PROGRAM_NAME)),
        Command::Validate => execute_startup_command(&cli, CliMode::Validate),

        Command::Tunnel(TunnelSubcommand::Run | TunnelSubcommand::Bare) => {
            execute_startup_command(&cli, CliMode::Run)
        }

        // Removed features — exact error messages from Go baseline.
        Command::ProxyDns | Command::Tunnel(TunnelSubcommand::ProxyDns) => {
            CliOutput::failure(String::new(), PROXY_DNS_REMOVED_MSG.to_owned(), 1)
        }
        Command::Tunnel(TunnelSubcommand::DbConnect) => {
            CliOutput::failure(String::new(), DB_CONNECT_REMOVED_MSG.to_owned(), 1)
        }

        // Everything else is recognized but not yet implemented.
        other => CliOutput::failure(String::new(), stub_not_implemented(&full_command_label(other)), 1),
    }
}

/// Build a human-readable label for any command variant, including sub-tree
/// depth.  Used for stub-not-implemented messages.
fn full_command_label(command: &Command) -> String {
    match command {
        Command::Access(sub) => match sub {
            AccessSubcommand::Login => "access login".into(),
            AccessSubcommand::Curl => "access curl".into(),
            AccessSubcommand::Token => "access token".into(),
            AccessSubcommand::Tcp => "access tcp".into(),
            AccessSubcommand::SshConfig => "access ssh-config".into(),
            AccessSubcommand::SshGen => "access ssh-gen".into(),
            AccessSubcommand::Bare => "access".into(),
        },
        Command::Tail(sub) => match sub {
            TailSubcommand::Token => "tail token".into(),
            TailSubcommand::Bare => "tail".into(),
        },
        Command::Management(sub) => match sub {
            ManagementSubcommand::Token => "management token".into(),
            ManagementSubcommand::Bare => "management".into(),
        },
        Command::Tunnel(TunnelSubcommand::Route(sub)) => match sub {
            RouteSubcommand::Dns => "tunnel route dns".into(),
            RouteSubcommand::Lb => "tunnel route lb".into(),
            RouteSubcommand::Ip(ip) => match ip {
                IpRouteSubcommand::Add => "tunnel route ip add".into(),
                IpRouteSubcommand::Show => "tunnel route ip show".into(),
                IpRouteSubcommand::Delete => "tunnel route ip delete".into(),
                IpRouteSubcommand::Get => "tunnel route ip get".into(),
                IpRouteSubcommand::Bare => "tunnel route ip".into(),
            },
            RouteSubcommand::Bare => "tunnel route".into(),
        },
        Command::Tunnel(TunnelSubcommand::Vnet(sub)) => match sub {
            VnetSubcommand::Add => "tunnel vnet add".into(),
            VnetSubcommand::List => "tunnel vnet list".into(),
            VnetSubcommand::Delete => "tunnel vnet delete".into(),
            VnetSubcommand::Update => "tunnel vnet update".into(),
            VnetSubcommand::Bare => "tunnel vnet".into(),
        },
        Command::Tunnel(TunnelSubcommand::Ingress(sub)) => match sub {
            IngressSubcommand::Validate => "tunnel ingress validate".into(),
            IngressSubcommand::Rule => "tunnel ingress rule".into(),
            IngressSubcommand::Bare => "tunnel ingress".into(),
        },
        Command::Tunnel(sub) => format!("tunnel {sub}"),
        _ => format!("{command}"),
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
