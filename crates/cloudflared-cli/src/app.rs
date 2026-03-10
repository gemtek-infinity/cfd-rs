use std::ffi::OsString;

use cloudflared_config::ConfigError;

use crate::cli::{Cli, Command, parse_args};
use crate::output::CliOutput;
use crate::runtime;
use crate::startup::{StartupSurface, render_run_output, render_validate_output, resolve_startup};

const PROGRAM_NAME: &str = "cloudflared";

pub(crate) fn execute(args: impl IntoIterator<Item = OsString>) -> CliOutput {
    match parse_args(args) {
        Ok(cli) => execute_command(cli),
        Err(message) => AppError::usage(message).into_output(),
    }
}

fn execute_command(cli: Cli) -> CliOutput {
    match cli.command {
        Command::Help => CliOutput::success(render_help()),
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
        Err(error) => AppError::config(error).into_output(),
    }
}

fn execute_runtime_command(startup: StartupSurface) -> CliOutput {
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

fn render_help() -> String {
    let mut text = String::new();
    text.push_str(&format!("{PROGRAM_NAME} {}\n", env!("CARGO_PKG_VERSION")));
    text.push_str("Linux production-alpha QUIC tunnel core with Pingora proxy seam\n\n");
    text.push_str("Usage:\n");
    text.push_str("  cloudflared [--config FILEPATH] validate\n");
    text.push_str("  cloudflared [--config FILEPATH] run\n");
    text.push_str("  cloudflared help\n");
    text.push_str("  cloudflared version\n\n");
    text.push_str("Admitted commands:\n");
    text.push_str(
        "  validate  Resolve config, load YAML, normalize ingress, and report startup readiness.\n",
    );
    text.push_str(
        "  run       Enter the runtime-owned QUIC transport core with a Pingora proxy seam and stop \
         honestly where later wire and origin slices are still unimplemented.\n",
    );
    text.push_str("  version   Print the workspace version.\n");
    text.push_str("  help      Print this help text.\n\n");
    text.push_str("Admitted flags and defaults:\n");
    text.push_str("  --config FILEPATH  Use an explicit YAML config path.\n");
    text.push_str(
        "  default discovery  Search ~/.cloudflared, ~/.cloudflare-warp, ~/cloudflare-warp, \
         /etc/cloudflared, /usr/local/etc/cloudflared.\n",
    );
    text.push_str(
        "  default create     If no config exists, write /usr/local/etc/cloudflared/config.yml with \
         logDirectory: /var/log/cloudflared.\n\n",
    );
    text.push_str("Admitted environment:\n");
    text.push_str("  HOME  Expands the leading ~ in default config search directories.\n\n");
    text.push_str("Deferred beyond current phase:\n");
    text.push_str(
        "  Wire/protocol boundary, security/compliance operational boundary, standard-format crate \
         integration, packaging, and deployment tooling\n",
    );
    text
}

enum CliMode {
    Validate,
    Run,
}

enum AppError {
    Usage(String),
    Config(ConfigError),
}

impl AppError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn config(error: ConfigError) -> Self {
        Self::Config(error)
    }

    fn into_output(self) -> CliOutput {
        match self {
            Self::Usage(message) => CliOutput::usage_failure(format!(
                "error: {message}\nRun `cloudflared help` for the admitted command surface.\n"
            )),
            Self::Config(error) => CliOutput::failure(
                String::new(),
                format!(
                    "error: startup validation failed [{}]: {error}\n",
                    error.category()
                ),
                1,
            ),
        }
    }
}
