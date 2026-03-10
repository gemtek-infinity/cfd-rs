#![forbid(unsafe_code)]

mod runtime;

use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::ExitCode;

use cloudflared_config::{
    ConfigError, ConfigSource, DiscoveryAction, DiscoveryRequest, IngressService, NormalizationWarning,
    discover_config, load_normalized_config,
};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

const PROGRAM_NAME: &str = "cloudflared";

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
        Err(error) => error.into_output(),
    }
}

fn execute_command(cli: Cli) -> CliOutput {
    match cli.command {
        Command::Help => CliOutput::success(render_help()),
        Command::Version => CliOutput::success(format!("{PROGRAM_NAME} {}\n", env!("CARGO_PKG_VERSION"))),
        Command::Validate => match resolve_startup(cli.config_path) {
            Ok(startup) => CliOutput::success(render_validate_output(&startup)),
            Err(error) => error.into_output(),
        },
        Command::Run => match resolve_startup(cli.config_path) {
            Ok(startup) => execute_runtime_command(startup),
            Err(error) => error.into_output(),
        },
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

fn resolve_startup(config_path: Option<PathBuf>) -> Result<StartupSurface, CliError> {
    let request = DiscoveryRequest {
        explicit_config: config_path,
        ..DiscoveryRequest::default()
    };
    let discovery = discover_config(&request).map_err(CliError::config)?;
    let normalized =
        load_normalized_config(&discovery.path, discovery.source.clone()).map_err(CliError::config)?;

    Ok(StartupSurface {
        discovery,
        normalized,
    })
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Cli, CliError> {
    let mut args = args.into_iter();
    let _ = args.next();

    let mut config_path = None;
    let mut command = None;
    let mut help_requested = false;
    let mut version_requested = false;

    while let Some(arg) = args.next() {
        if arg == OsStr::new("--config") {
            let value = args
                .next()
                .ok_or_else(|| CliError::usage("missing value for --config"))?;
            set_config_path(&mut config_path, PathBuf::from(value))?;
            continue;
        }

        if let Some(path) = parse_equals_flag(&arg, "--config") {
            set_config_path(&mut config_path, PathBuf::from(path))?;
            continue;
        }

        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" | "help" => {
                help_requested = true;
            }
            "--version" | "-V" | "version" => {
                version_requested = true;
            }
            "validate" => {
                set_command(&mut command, Command::Validate)?;
            }
            "run" => {
                set_command(&mut command, Command::Run)?;
            }
            other if other.starts_with('-') => {
                return Err(CliError::usage(format!("unknown flag: {other}")));
            }
            other => {
                return Err(CliError::usage(format!("unknown command or argument: {other}")));
            }
        }
    }

    if help_requested {
        return Ok(Cli {
            command: Command::Help,
            config_path,
        });
    }
    if version_requested {
        return Ok(Cli {
            command: Command::Version,
            config_path,
        });
    }

    Ok(Cli {
        command: command.unwrap_or(Command::Help),
        config_path,
    })
}

fn parse_equals_flag<'a>(arg: &'a OsStr, name: &str) -> Option<&'a str> {
    let arg = arg.to_str()?;
    arg.strip_prefix(name)?.strip_prefix('=')
}

fn set_config_path(slot: &mut Option<PathBuf>, path: PathBuf) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::usage("--config may only be provided once"));
    }
    *slot = Some(path);
    Ok(())
}

fn set_command(slot: &mut Option<Command>, command: Command) -> Result<(), CliError> {
    if let Some(existing) = slot
        && *existing != command
    {
        return Err(CliError::usage(format!(
            "multiple commands were provided: {} and {}",
            existing.as_str(),
            command.as_str()
        )));
    }
    *slot = Some(command);
    Ok(())
}

fn render_help() -> String {
    let mut text = String::new();
    text.push_str(&format!("{PROGRAM_NAME} {}\n", env!("CARGO_PKG_VERSION")));
    text.push_str("Linux production-alpha runtime/lifecycle shell for Big Phase 3.2\n\n");
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
        "  run       Enter the real runtime/lifecycle owner, supervise the current deferred tunnel-core \
         boundary, and exit honestly where later slices are still unimplemented.\n",
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
    text.push_str("Deferred beyond Phase 3.2:\n");
    text.push_str(
        "  quiche tunnel core, Pingora integration, wire/protocol boundary, security/compliance operational \
         boundary, standard-format crate integration, packaging, and deployment tooling\n",
    );
    text
}

fn render_validate_output(startup: &StartupSurface) -> String {
    let mut lines = vec![String::from("OK: admitted alpha startup surface validated")];
    lines.extend(render_startup_lines(startup));
    lines.join("\n") + "\n"
}

fn render_run_output(startup: &StartupSurface, report: &runtime::RuntimeExecution) -> String {
    let mut lines = vec![String::from("Resolved admitted alpha startup surface")];
    lines.extend(render_startup_lines(startup));
    lines.extend(report.summary_lines.iter().cloned());
    lines.join("\n") + "\n"
}

fn render_startup_lines(startup: &StartupSurface) -> Vec<String> {
    let mut lines = vec![
        format!(
            "config-source: {}",
            config_source_label(&startup.discovery.source)
        ),
        format!("config-path: {}", startup.discovery.path.display()),
        format!("ingress-rules: {}", startup.normalized.ingress.len()),
    ];

    if startup.discovery.action == DiscoveryAction::CreateDefaultConfig {
        lines.push(String::from("created-default-config: yes"));
    }

    match warning_summary(&startup.normalized.warnings) {
        Some(summary) => lines.push(format!("warnings: {summary}")),
        None => lines.push(String::from("warnings: none")),
    }

    if startup.normalized.ingress.len() == 1
        && startup.normalized.ingress[0].service == IngressService::HttpStatus(503)
    {
        lines.push(String::from(
            "ingress-default: catch-all http_status:503 is admitted when no ingress rules are configured",
        ));
    }

    lines
}

fn warning_summary(warnings: &[NormalizationWarning]) -> Option<String> {
    let mut parts = Vec::new();

    for warning in warnings {
        match warning {
            NormalizationWarning::UnknownTopLevelKeys(keys) => {
                parts.push(format!("unknown-top-level-keys={}", keys.join(",")));
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

fn config_source_label(source: &ConfigSource) -> &'static str {
    match source {
        ConfigSource::ExplicitPath(_) => "explicit",
        ConfigSource::DiscoveredPath(_) => "discovered",
        ConfigSource::AutoCreatedPath(_) => "auto-created",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Help,
    Version,
    Validate,
    Run,
}

impl Command {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::Version => "version",
            Self::Validate => "validate",
            Self::Run => "run",
        }
    }
}

#[derive(Debug)]
struct Cli {
    command: Command,
    config_path: Option<PathBuf>,
}

#[derive(Debug)]
pub(crate) struct StartupSurface {
    discovery: cloudflared_config::DiscoveryOutcome,
    normalized: cloudflared_config::NormalizedConfig,
}

#[derive(Debug)]
struct CliOutput {
    stdout: String,
    stderr: String,
    exit_code: u8,
}

impl CliOutput {
    fn success(stdout: String) -> Self {
        Self {
            stdout,
            stderr: String::new(),
            exit_code: 0,
        }
    }

    fn failure(stdout: String, stderr: String, exit_code: u8) -> Self {
        Self {
            stdout,
            stderr,
            exit_code,
        }
    }
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Config(ConfigError),
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn config(error: ConfigError) -> Self {
        Self::Config(error)
    }

    fn into_output(self) -> CliOutput {
        match self {
            Self::Usage(message) => CliOutput {
                stdout: String::new(),
                stderr: format!(
                    "error: {message}\nRun `cloudflared help` for the admitted Phase 3.2 surface.\n"
                ),
                exit_code: 2,
            },
            Self::Config(error) => CliOutput {
                stdout: String::new(),
                stderr: format!(
                    "error: startup validation failed [{}]: {error}\n",
                    error.category()
                ),
                exit_code: 1,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, parse_args};
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn parse(parts: &[&str]) -> super::Cli {
        let args = std::iter::once(OsString::from("cloudflared"))
            .chain(parts.iter().map(OsString::from))
            .collect::<Vec<_>>();
        parse_args(args).expect("arguments should parse")
    }

    #[test]
    fn config_flag_can_appear_before_command() {
        let cli = parse(&["--config", "/tmp/config.yml", "validate"]);

        assert_eq!(cli.command, Command::Validate);
        assert_eq!(cli.config_path, Some(PathBuf::from("/tmp/config.yml")));
    }

    #[test]
    fn config_flag_can_appear_after_command() {
        let cli = parse(&["run", "--config=/tmp/config.yml"]);

        assert_eq!(cli.command, Command::Run);
        assert_eq!(cli.config_path, Some(PathBuf::from("/tmp/config.yml")));
    }
}
