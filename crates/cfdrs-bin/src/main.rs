#![forbid(unsafe_code)]

mod protocol;
mod proxy;
mod runtime;
mod startup;
mod transport;

// Admitted for non-blocking file writer layer; not yet wired (hand-rolled
// size-based rotation matches Go parity).
use tracing_appender as _;

use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use cfdrs_cli::{
    CLASSIC_TUNNEL_DEPRECATED_MSG, Cli, CliError, CliOutput, Command, DB_CONNECT_REMOVED_MSG, GlobalFlags,
    HelpTarget, INGRESS_RULE_NARG_ERROR_MSG, IngressSubcommand, IpRouteSubcommand, PROGRAM_NAME,
    PROXY_DNS_REMOVED_LOG_MSG, PROXY_DNS_REMOVED_MSG, ROUTE_DNS_NARG_ERROR_MSG, ROUTE_IP_ADD_NARG_ERROR_MSG,
    ROUTE_IP_DELETE_NARG_ERROR_MSG, ROUTE_IP_GET_NARG_ERROR_MSG, ROUTE_LB_NARG_ERROR_MSG, RouteSubcommand,
    ServiceAction, TUNNEL_CLEANUP_NARG_ERROR_MSG, TUNNEL_CMD_ERROR_MSG, TUNNEL_CREATE_NARG_ERROR_MSG,
    TUNNEL_DELETE_NARG_ERROR_MSG, TUNNEL_INFO_NARG_ERROR_MSG, TUNNEL_RUN_HOSTNAME_WARNING_MSG,
    TUNNEL_RUN_IDENTITY_ERROR_MSG, TUNNEL_RUN_NARG_ERROR_MSG, TUNNEL_TOKEN_FILE_READ_ERROR_PREFIX,
    TUNNEL_TOKEN_INVALID_MSG, TUNNEL_TOKEN_NARG_ERROR_MSG, TunnelSubcommand, VNET_ADD_NARG_ERROR_MSG,
    VNET_DELETE_NARG_ERROR_MSG, VNET_UPDATE_NARG_ERROR_MSG, VnetSubcommand, parse_args, render_access_help,
    render_help, render_short_version, render_subcommand_help, render_tunnel_help, render_version_output,
    stub_not_implemented, subcommand_usage_error, tunnel_run_usage_error,
};
use cfdrs_his::environment::current_executable;
use cfdrs_his::service::{
    ProcessRunner, SERVICE_CONFIG_PATH, ServiceTemplateArgs, build_args_for_config, build_args_for_token,
    copy_file, install_linux_service, uninstall_linux_service,
};
use cfdrs_shared::{ConfigError, TunnelToken};
use mimalloc::MiMalloc;

use crate::startup::{
    PreparedRuntimeStartup, StartupSurface, prepare_runtime_startup, render_run_output,
    render_validate_output, resolve_startup,
};

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
        Ok(mut cli) => {
            cli.flags.apply_env_defaults();
            cli.flags.apply_defaults();
            execute_command(cli)
        }
        Err(message) => CliError::usage(message).into_output(),
    }
}

fn execute_command(cli: Cli) -> CliOutput {
    match &cli.command {
        Command::Help(HelpTarget::Root) => CliOutput::success(render_help(PROGRAM_NAME)),
        Command::Help(HelpTarget::Tunnel) => CliOutput::success(render_tunnel_help(PROGRAM_NAME)),
        Command::Help(HelpTarget::Access) => CliOutput::success(render_access_help(PROGRAM_NAME)),
        Command::Help(target) => CliOutput::success(render_subcommand_help(target)),
        Command::Version { short: true } => CliOutput::success(render_short_version()),
        Command::Version { short: false } => CliOutput::success(render_version_output(PROGRAM_NAME)),
        Command::Validate => execute_startup_command(&cli, CliMode::Validate),

        Command::Service(ServiceAction::Install) => execute_service_install(&cli),
        Command::Service(ServiceAction::Uninstall) => execute_service_uninstall(),

        Command::Tunnel(_) => dispatch_tunnel_subcommand(cli),

        // Go baseline: handleServiceMode() in main.go — daemon-style
        // config-watcher loop when invoked with zero args and zero flags.
        // Discovers/creates config via FindOrCreateConfigPath(), then enters
        // the runtime with watcher and signal handling already wired.
        Command::ServiceMode => execute_startup_command(&cli, CliMode::Run),

        // Removed features — exact error messages from Go baseline.
        Command::ProxyDns => {
            eprintln!("{PROXY_DNS_REMOVED_LOG_MSG}");
            CliOutput::failure(String::new(), PROXY_DNS_REMOVED_MSG.to_owned(), 1)
        }

        // Everything else is recognized but not yet implemented.
        other => CliOutput::failure(String::new(), stub_not_implemented(&full_command_label(other)), 1),
    }
}

/// Dispatch all `Command::Tunnel(*)` variants — Go baseline:
/// `TunnelCommand()` dispatch tree in `cmd/cloudflared/tunnel/cmd.go`.
fn dispatch_tunnel_subcommand(cli: Cli) -> CliOutput {
    match &cli.command {
        Command::Tunnel(TunnelSubcommand::Run) => execute_tunnel_run(cli),
        Command::Tunnel(TunnelSubcommand::Bare) => execute_tunnel_bare(&cli),

        // Removed features — exact error messages from Go baseline.
        Command::Tunnel(TunnelSubcommand::ProxyDns) => {
            eprintln!("{PROXY_DNS_REMOVED_LOG_MSG}");
            CliOutput::failure(String::new(), PROXY_DNS_REMOVED_MSG.to_owned(), 1)
        }
        Command::Tunnel(TunnelSubcommand::DbConnect) => {
            // Go baseline: cliutil.RemovedCommand("db-connect") uses cli.Exit(..., -1)
            // which shells see as exit code 255 (unsigned byte truncation of -1).
            CliOutput::failure(String::new(), DB_CONNECT_REMOVED_MSG.to_owned(), 255)
        }

        // NArg validation — UsageError pattern (exit 255).
        Command::Tunnel(TunnelSubcommand::Create) => {
            validate_narg_exact(&cli.flags, 1, "tunnel create", TUNNEL_CREATE_NARG_ERROR_MSG).unwrap_or_else(
                || CliOutput::failure(String::new(), stub_not_implemented("tunnel create"), 1),
            )
        }
        Command::Tunnel(TunnelSubcommand::Delete) => {
            validate_narg_min(&cli.flags, 1, "tunnel delete", TUNNEL_DELETE_NARG_ERROR_MSG).unwrap_or_else(
                || CliOutput::failure(String::new(), stub_not_implemented("tunnel delete"), 1),
            )
        }
        Command::Tunnel(TunnelSubcommand::Cleanup) => {
            validate_narg_min(&cli.flags, 1, "tunnel cleanup", TUNNEL_CLEANUP_NARG_ERROR_MSG).unwrap_or_else(
                || CliOutput::failure(String::new(), stub_not_implemented("tunnel cleanup"), 1),
            )
        }
        Command::Tunnel(TunnelSubcommand::Token) => {
            validate_narg_exact(&cli.flags, 1, "tunnel token", TUNNEL_TOKEN_NARG_ERROR_MSG)
                .unwrap_or_else(|| CliOutput::failure(String::new(), stub_not_implemented("tunnel token"), 1))
        }
        Command::Tunnel(TunnelSubcommand::Info) => {
            validate_narg_exact(&cli.flags, 1, "tunnel info", TUNNEL_INFO_NARG_ERROR_MSG)
                .unwrap_or_else(|| CliOutput::failure(String::new(), stub_not_implemented("tunnel info"), 1))
        }

        // Route subcommands.
        Command::Tunnel(TunnelSubcommand::Route(sub)) => dispatch_route_subcommand(sub, &cli.flags),

        // Vnet subcommands.
        Command::Tunnel(TunnelSubcommand::Vnet(sub)) => dispatch_vnet_subcommand(sub, &cli.flags),

        // Ingress subcommands.
        Command::Tunnel(TunnelSubcommand::Ingress(IngressSubcommand::Rule)) => {
            if cli.flags.rest_args.is_empty() {
                return CliOutput::failure(String::new(), INGRESS_RULE_NARG_ERROR_MSG.to_owned(), 1);
            }
            CliOutput::failure(String::new(), stub_not_implemented("tunnel ingress rule"), 1)
        }

        // Everything else is recognized but not yet implemented.
        other => CliOutput::failure(String::new(), stub_not_implemented(&full_command_label(other)), 1),
    }
}

/// Dispatch `tunnel route *` subcommands.
fn dispatch_route_subcommand(sub: &RouteSubcommand, flags: &GlobalFlags) -> CliOutput {
    match sub {
        RouteSubcommand::Dns => validate_narg_exact(flags, 2, "tunnel route dns", ROUTE_DNS_NARG_ERROR_MSG)
            .unwrap_or_else(|| {
                CliOutput::failure(String::new(), stub_not_implemented("tunnel route dns"), 1)
            }),
        RouteSubcommand::Lb => validate_narg_exact(flags, 3, "tunnel route lb", ROUTE_LB_NARG_ERROR_MSG)
            .unwrap_or_else(|| CliOutput::failure(String::new(), stub_not_implemented("tunnel route lb"), 1)),
        // Route IP — errors.New pattern (exit 1).
        RouteSubcommand::Ip(IpRouteSubcommand::Add) => {
            if flags.rest_args.len() < 2 {
                return CliOutput::failure(String::new(), ROUTE_IP_ADD_NARG_ERROR_MSG.to_owned(), 1);
            }
            CliOutput::failure(String::new(), stub_not_implemented("tunnel route ip add"), 1)
        }
        RouteSubcommand::Ip(IpRouteSubcommand::Delete) => {
            if flags.rest_args.len() != 1 {
                return CliOutput::failure(String::new(), ROUTE_IP_DELETE_NARG_ERROR_MSG.to_owned(), 1);
            }
            CliOutput::failure(String::new(), stub_not_implemented("tunnel route ip delete"), 1)
        }
        RouteSubcommand::Ip(IpRouteSubcommand::Get) => {
            if flags.rest_args.len() != 1 {
                return CliOutput::failure(String::new(), ROUTE_IP_GET_NARG_ERROR_MSG.to_owned(), 1);
            }
            CliOutput::failure(String::new(), stub_not_implemented("tunnel route ip get"), 1)
        }
        _ => CliOutput::failure(String::new(), stub_not_implemented("tunnel route"), 1),
    }
}

/// Dispatch `tunnel vnet *` subcommands.
fn dispatch_vnet_subcommand(sub: &VnetSubcommand, flags: &GlobalFlags) -> CliOutput {
    match sub {
        VnetSubcommand::Add => {
            if flags.rest_args.is_empty() {
                return CliOutput::failure(String::new(), VNET_ADD_NARG_ERROR_MSG.to_owned(), 1);
            }
            CliOutput::failure(String::new(), stub_not_implemented("tunnel vnet add"), 1)
        }
        VnetSubcommand::Delete => {
            if flags.rest_args.is_empty() {
                return CliOutput::failure(String::new(), VNET_DELETE_NARG_ERROR_MSG.to_owned(), 1);
            }
            CliOutput::failure(String::new(), stub_not_implemented("tunnel vnet delete"), 1)
        }
        VnetSubcommand::Update => {
            if flags.rest_args.len() != 1 {
                return CliOutput::failure(String::new(), VNET_UPDATE_NARG_ERROR_MSG.to_owned(), 1);
            }
            CliOutput::failure(String::new(), stub_not_implemented("tunnel vnet update"), 1)
        }
        _ => CliOutput::failure(String::new(), stub_not_implemented("tunnel vnet"), 1),
    }
}

/// Build a human-readable label for any command variant, including sub-tree
/// depth.  Used for stub-not-implemented messages.
fn full_command_label(command: &Command) -> String {
    command.full_label()
}

/// Go baseline: `cliutil.UsageError` — returns `Some(CliOutput)` when
/// `rest_args.len() != expected`, with exit code 255.
fn validate_narg_exact(
    flags: &GlobalFlags,
    expected: usize,
    cmd_path: &str,
    message: &str,
) -> Option<CliOutput> {
    if flags.rest_args.len() != expected {
        Some(CliOutput::failure(
            String::new(),
            subcommand_usage_error(cmd_path, message),
            255,
        ))
    } else {
        None
    }
}

/// Go baseline: `cliutil.UsageError` — returns `Some(CliOutput)` when
/// `rest_args.len() < minimum`, with exit code 255.
fn validate_narg_min(
    flags: &GlobalFlags,
    minimum: usize,
    cmd_path: &str,
    message: &str,
) -> Option<CliOutput> {
    if flags.rest_args.len() < minimum {
        Some(CliOutput::failure(
            String::new(),
            subcommand_usage_error(cmd_path, message),
            255,
        ))
    } else {
        None
    }
}

fn execute_startup_command(cli: &Cli, mode: CliMode) -> CliOutput {
    match resolve_startup(cli.flags.config_path.clone()) {
        Ok(startup) => match mode {
            CliMode::Validate => CliOutput::success(render_validate_output(&startup)),
            CliMode::Run => execute_runtime_command(startup, &cli.flags),
        },
        Err(error) => CliError::config(error).into_output(),
    }
}

/// Go baseline: `runCommand()` in subcommands.go lines 748–788.
///
/// Validation order (matching Go exactly):
///   1. NArg > 1 → UsageError
///   2. `--token` → `--token-file` → positional arg → config tunnel ID
///   3. missing identity → UsageError
fn execute_tunnel_run(cli: Cli) -> CliOutput {
    let flags = &cli.flags;

    // Step 1: NArg validation — Go rejects more than one positional arg.
    if flags.rest_args.len() > 1 {
        return CliOutput::failure(
            String::new(),
            tunnel_run_usage_error(TUNNEL_RUN_NARG_ERROR_MSG),
            255,
        );
    }

    // Step 2: Token resolution — --token > --token-file (Go lines 760–776).
    let token_str = resolve_run_token_string(flags);

    match token_str {
        Ok(Some(ref s)) if !s.is_empty() => {
            // Token string found — validate it (Go line 777–778).
            if TunnelToken::decode(s).is_err() {
                return CliOutput::failure(
                    String::new(),
                    tunnel_run_usage_error(TUNNEL_TOKEN_INVALID_MSG),
                    255,
                );
            }
            // Valid token — proceed through startup (runtime will use token).
            return execute_startup_command(&cli, CliMode::Run);
        }
        Ok(Some(_) | None) => {
            // No token — fall through to positional arg / config check.
        }
        Err(err) => {
            // Token file read error — exit 255 (Go line 770).
            return CliOutput::failure(
                String::new(),
                tunnel_run_usage_error(&format!("{TUNNEL_TOKEN_FILE_READ_ERROR_PREFIX}{err}")),
                255,
            );
        }
    }

    // Step 3: Positional arg — tunnel name/ID (Go line 781).
    if !flags.rest_args.is_empty() {
        return execute_startup_command(&cli, CliMode::Run);
    }

    // Step 4: Config tunnel ID (Go lines 783–787).
    match resolve_startup(flags.config_path.clone()) {
        Ok(startup) if startup.normalized.tunnel.is_some() => execute_runtime_command(startup, flags),
        _ => CliOutput::failure(
            String::new(),
            tunnel_run_usage_error(TUNNEL_RUN_IDENTITY_ERROR_MSG),
            255,
        ),
    }
}

/// Resolve the token string from `--token` or `--token-file` flags.
///
/// Go baseline: subcommands.go lines 760–776 — `--token` takes precedence
/// over `--token-file`.  Returns `Ok(None)` when neither flag is set.
fn resolve_run_token_string(flags: &GlobalFlags) -> Result<Option<String>, std::io::Error> {
    if let Some(token) = flags.token.as_deref() {
        return Ok(Some(token.to_owned()));
    }

    if let Some(token_path) = flags.token_file.as_ref() {
        let data = std::fs::read_to_string(token_path)?;
        return Ok(Some(data.trim().to_owned()));
    }

    Ok(None)
}

/// Go baseline: TunnelCommand() dispatch tree in cmd/cloudflared/tunnel/cmd.go.
///
/// Priority order:
///   1. `--name` set → adhoc named tunnel (stub)
///   2. `--url` or `--hello-world` → quick tunnel (stub)
///   3. config has TunnelID → redirect to `tunnel run`
///   4. `--hostname` set → classic tunnel deprecation error
///   5. fallthrough → error with usage guidance
fn execute_tunnel_bare(cli: &Cli) -> CliOutput {
    let flags = &cli.flags;

    // Branch 1: --name → adhoc named tunnel (not yet implemented)
    if flags.tunnel_name.is_some() {
        return CliOutput::failure(String::new(), stub_not_implemented("tunnel (adhoc named)"), 1);
    }

    // Branch 2: --url or --hello-world → quick tunnel (not yet implemented)
    if flags.url.is_some() || flags.hello_world {
        return CliOutput::failure(String::new(), stub_not_implemented("tunnel (quick tunnel)"), 1);
    }

    // Branch 3: config has TunnelID → run from config
    match resolve_startup(flags.config_path.clone()) {
        Ok(startup) if startup.normalized.tunnel.is_some() => {
            return execute_runtime_command(startup, flags);
        }
        _ => {}
    }

    // Branch 4: --hostname → classic tunnel deprecation
    if flags.hostname.is_some() {
        return CliOutput::failure(String::new(), CLASSIC_TUNNEL_DEPRECATED_MSG.to_owned(), 1);
    }

    // Branch 5: no valid argument → usage error
    CliOutput::failure(String::new(), TUNNEL_CMD_ERROR_MSG.to_owned(), 1)
}

fn execute_runtime_command(startup: StartupSurface, flags: &GlobalFlags) -> CliOutput {
    let PreparedRuntimeStartup {
        startup,
        runtime_config,
        log_config,
        transport_log_level,
    } = match prepare_runtime_startup(startup, flags) {
        Ok(prepared) => prepared,
        Err(error) => return CliError::config(error).into_output(),
    };

    runtime::install_runtime_logging(&log_config, transport_log_level);

    // Go baseline: ReadConfigFile() double-parses the YAML and logs warnings
    // for unknown top-level keys.  The Rust equivalent collects them during
    // normalization; emit them here once logging is installed.
    for warning in &startup.normalized.warnings {
        match warning {
            cfdrs_shared::NormalizationWarning::UnknownTopLevelKeys(keys) => {
                tracing::warn!("Your configuration file has unknown top-level keys: {:?}", keys,);
            }
        }
    }

    // Go baseline: runCommand() in subcommands.go line 757 — warn when
    // --hostname is set but a Named Tunnel is configured.  Non-fatal.
    if flags.hostname.is_some() {
        tracing::warn!("{TUNNEL_RUN_HOSTNAME_WARNING_MSG}");
    }

    let report = runtime::run(runtime_config);
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

struct ServiceInstallRequest {
    template_args: ServiceTemplateArgs,
    auto_update: bool,
}

fn execute_service_install(cli: &Cli) -> CliOutput {
    match build_service_install_request(cli).and_then(|request| {
        let runner = ProcessRunner;
        install_linux_service(&request.template_args, request.auto_update, &runner)
    }) {
        Ok(()) => CliOutput::success("Linux service for cloudflared installed successfully\n".to_owned()),
        Err(error) => CliError::config(error).into_output(),
    }
}

fn execute_service_uninstall() -> CliOutput {
    let runner = ProcessRunner;

    match uninstall_linux_service(&runner) {
        Ok(()) => CliOutput::success("Linux service for cloudflared uninstalled successfully\n".to_owned()),
        Err(error) => CliError::config(error).into_output(),
    }
}

fn build_service_install_request(cli: &Cli) -> Result<ServiceInstallRequest, ConfigError> {
    let executable_path = current_executable()?;
    let extra_args = build_service_install_extra_args(cli)?;

    Ok(ServiceInstallRequest {
        template_args: ServiceTemplateArgs {
            path: executable_path,
            extra_args,
        },
        auto_update: !cli.flags.no_update_service,
    })
}

fn build_service_install_extra_args(cli: &Cli) -> Result<Vec<String>, ConfigError> {
    if let Some(token) = resolve_service_install_token(cli)? {
        return Ok(build_args_for_token(&token));
    }

    let startup = resolve_startup(cli.flags.config_path.clone())?;
    ensure_service_install_config_requirements(&startup)?;
    copy_service_config_if_needed(&startup.discovery.path)?;

    Ok(build_args_for_config())
}

fn resolve_service_install_token(cli: &Cli) -> Result<Option<String>, ConfigError> {
    if let Some(token) = cli.flags.rest_args.first() {
        return validate_service_install_token(token).map(Some);
    }

    if let Some(token) = cli.flags.token.as_deref() {
        return validate_service_install_token(token).map(Some);
    }

    if let Some(token_path) = cli.flags.token_file.as_ref() {
        let token =
            std::fs::read_to_string(token_path).map_err(|source| ConfigError::read(token_path, source))?;
        return validate_service_install_token(token.trim()).map(Some);
    }

    Ok(None)
}

fn validate_service_install_token(token: &str) -> Result<String, ConfigError> {
    TunnelToken::decode(token.trim())?;
    Ok(token.trim().to_owned())
}

fn ensure_service_install_config_requirements(startup: &StartupSurface) -> Result<(), ConfigError> {
    if startup.normalized.tunnel.is_some() && startup.normalized.credentials.credentials_file.is_some() {
        return Ok(());
    }

    Err(ConfigError::invariant(format!(
        "Configuration file {} must contain entries for the tunnel to run and its associated \
         credentials:\ntunnel: TUNNEL-UUID\ncredentials-file: CREDENTIALS-FILE\n",
        startup.discovery.path.display()
    )))
}

fn copy_service_config_if_needed(source_path: &Path) -> Result<(), ConfigError> {
    let destination_path = Path::new(SERVICE_CONFIG_PATH);

    if source_path == destination_path {
        return Ok(());
    }

    if destination_path.exists() {
        return Err(ConfigError::invariant(format!(
            "Possible conflicting configuration in {} and {}. Either remove {} or run `cloudflared --config \
             {} service install`",
            source_path.display(),
            destination_path.display(),
            destination_path.display(),
            destination_path.display()
        )));
    }

    copy_file(source_path, destination_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded_tunnel_token() -> String {
        TunnelToken {
            account_tag: "account".to_owned(),
            tunnel_secret: cfdrs_shared::TunnelSecret::from_bytes([1, 2, 3, 4]),
            tunnel_id: uuid::Uuid::nil(),
            endpoint: None,
        }
        .encode()
        .expect("token should encode")
    }

    fn service_install_cli() -> Cli {
        Cli {
            command: Command::Service(ServiceAction::Install),
            flags: GlobalFlags::default(),
        }
    }

    #[test]
    fn service_install_token_prefers_rest_args() {
        let mut cli = service_install_cli();
        cli.flags.rest_args.push(encoded_tunnel_token());
        cli.flags.token = Some("ignored".to_owned());

        let token = resolve_service_install_token(&cli).expect("token should resolve");
        assert_eq!(token, Some(cli.flags.rest_args[0].clone()));
    }

    #[test]
    fn service_install_token_can_be_loaded_from_file() {
        let mut cli = service_install_cli();
        let token = encoded_tunnel_token();
        let token_path = std::env::temp_dir().join("cfdrs-service-install-token.txt");
        std::fs::write(&token_path, format!("{token}\n")).expect("token file should be written");
        cli.flags.token_file = Some(token_path.clone());

        let resolved = resolve_service_install_token(&cli).expect("token should resolve from file");
        assert_eq!(resolved, Some(token));

        let _ = std::fs::remove_file(token_path);
    }

    #[test]
    fn service_install_rejects_invalid_token() {
        let mut cli = service_install_cli();
        cli.flags.token = Some("not-a-valid-token".to_owned());

        let error = resolve_service_install_token(&cli).expect_err("invalid token should fail");
        assert_eq!(error.category().to_string(), "token-decode");
    }

    #[test]
    fn copy_service_config_skips_service_path() {
        let result = copy_service_config_if_needed(Path::new(SERVICE_CONFIG_PATH));
        assert!(result.is_ok());
    }

    #[test]
    fn runtime_command_label_for_service_install_stays_stable() {
        let label = full_command_label(&Command::Service(ServiceAction::Install));
        assert_eq!(label, "service install");
    }

    // --- CLI-032: tunnel run NArg validation ---

    #[test]
    fn tunnel_run_rejects_multiple_positional_args() {
        // Go baseline: c.NArg() > 1 → UsageError (exit -1 = 255)
        let output = execute(
            ["cloudflared", "tunnel", "run", "arg1", "arg2"]
                .into_iter()
                .map(OsString::from),
        );

        assert_eq!(
            output.exit_code, 255,
            "Go baseline exit code is -1 (255 unsigned)"
        );
        assert!(
            output.stderr.contains("accepts only one argument"),
            "stderr must contain NArg error: {:?}",
            output.stderr
        );
        assert!(
            output.stderr.contains("See 'cloudflared tunnel run --help'."),
            "stderr must contain help suffix: {:?}",
            output.stderr
        );
    }

    #[test]
    fn tunnel_run_allows_single_positional_arg() {
        // Go baseline: c.NArg() == 1 is the tunnel name/ID — valid
        let output = execute(
            ["cloudflared", "tunnel", "run", "my-tunnel"]
                .into_iter()
                .map(OsString::from),
        );

        // Should NOT get the NArg error (may get config discovery error, that's fine)
        assert_ne!(
            output.exit_code, 255,
            "single positional arg must not trigger NArg rejection"
        );
        assert!(
            !output.stderr.contains("accepts only one argument"),
            "single positional arg must not trigger NArg error"
        );
    }

    #[test]
    fn tunnel_run_allows_zero_positional_args() {
        // Go baseline: c.NArg() == 0 is valid (uses config tunnel ID or token)
        // — may exit 255 with identity error, but NOT with NArg error
        let output = execute(["cloudflared", "tunnel", "run"].into_iter().map(OsString::from));

        assert!(
            !output.stderr.contains("accepts only one argument"),
            "zero positional args must not trigger NArg error"
        );
    }

    // --- CLI-032: NArg validation for subcommands (cliutil.UsageError → exit 255)
    // ---

    /// Helper: execute with the given args and return the output.
    fn exec(args: &[&str]) -> CliOutput {
        execute(args.iter().map(OsString::from))
    }

    // tunnel create: NArg != 1 → 255
    #[test]
    fn tunnel_create_rejects_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "create"]);
        assert_eq!(out.exit_code, 255);
        assert!(out.stderr.contains(TUNNEL_CREATE_NARG_ERROR_MSG));
    }

    #[test]
    fn tunnel_create_rejects_two_args() {
        let out = exec(&["cloudflared", "tunnel", "create", "a", "b"]);
        assert_eq!(out.exit_code, 255);
        assert!(out.stderr.contains(TUNNEL_CREATE_NARG_ERROR_MSG));
    }

    #[test]
    fn tunnel_create_accepts_one_arg() {
        let out = exec(&["cloudflared", "tunnel", "create", "my-tunnel"]);
        assert_ne!(out.exit_code, 255);
    }

    // tunnel delete: NArg < 1 → 255
    #[test]
    fn tunnel_delete_rejects_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "delete"]);
        assert_eq!(out.exit_code, 255);
        assert!(out.stderr.contains(TUNNEL_DELETE_NARG_ERROR_MSG));
    }

    #[test]
    fn tunnel_delete_accepts_one_arg() {
        let out = exec(&["cloudflared", "tunnel", "delete", "my-tunnel"]);
        assert_ne!(out.exit_code, 255);
    }

    #[test]
    fn tunnel_delete_accepts_multiple_args() {
        let out = exec(&["cloudflared", "tunnel", "delete", "t1", "t2"]);
        assert_ne!(out.exit_code, 255);
    }

    // tunnel cleanup: NArg < 1 → 255
    #[test]
    fn tunnel_cleanup_rejects_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "cleanup"]);
        assert_eq!(out.exit_code, 255);
        assert!(out.stderr.contains(TUNNEL_CLEANUP_NARG_ERROR_MSG));
    }

    #[test]
    fn tunnel_cleanup_accepts_one_arg() {
        let out = exec(&["cloudflared", "tunnel", "cleanup", "t1"]);
        assert_ne!(out.exit_code, 255);
    }

    // tunnel token: NArg != 1 → 255
    #[test]
    fn tunnel_token_rejects_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "token"]);
        assert_eq!(out.exit_code, 255);
        assert!(out.stderr.contains(TUNNEL_TOKEN_NARG_ERROR_MSG));
    }

    #[test]
    fn tunnel_token_rejects_two_args() {
        let out = exec(&["cloudflared", "tunnel", "token", "a", "b"]);
        assert_eq!(out.exit_code, 255);
        assert!(out.stderr.contains(TUNNEL_TOKEN_NARG_ERROR_MSG));
    }

    #[test]
    fn tunnel_token_accepts_one_arg() {
        let out = exec(&["cloudflared", "tunnel", "token", "my-tunnel"]);
        assert_ne!(out.exit_code, 255);
    }

    // tunnel info: NArg != 1 → 255
    #[test]
    fn tunnel_info_rejects_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "info"]);
        assert_eq!(out.exit_code, 255);
        assert!(out.stderr.contains(TUNNEL_INFO_NARG_ERROR_MSG));
    }

    #[test]
    fn tunnel_info_accepts_one_arg() {
        let out = exec(&["cloudflared", "tunnel", "info", "my-tunnel"]);
        assert_ne!(out.exit_code, 255);
    }

    // tunnel route dns: NArg != 2 → 255
    #[test]
    fn route_dns_rejects_one_arg() {
        let out = exec(&["cloudflared", "tunnel", "route", "dns", "my-tunnel"]);
        assert_eq!(out.exit_code, 255);
        assert!(out.stderr.contains(ROUTE_DNS_NARG_ERROR_MSG));
    }

    #[test]
    fn route_dns_accepts_two_args() {
        let out = exec(&[
            "cloudflared",
            "tunnel",
            "route",
            "dns",
            "my-tunnel",
            "example.com",
        ]);
        assert_ne!(out.exit_code, 255);
    }

    // tunnel route lb: NArg != 3 → 255
    #[test]
    fn route_lb_rejects_two_args() {
        let out = exec(&["cloudflared", "tunnel", "route", "lb", "my-tunnel", "example.com"]);
        assert_eq!(out.exit_code, 255);
        assert!(out.stderr.contains(ROUTE_LB_NARG_ERROR_MSG));
    }

    #[test]
    fn route_lb_accepts_three_args() {
        let out = exec(&[
            "cloudflared",
            "tunnel",
            "route",
            "lb",
            "my-tunnel",
            "example.com",
            "my-pool",
        ]);
        assert_ne!(out.exit_code, 255);
    }

    // --- Route IP subcommands (Go baseline: errors.New → exit 1) ---

    #[test]
    fn route_ip_add_rejects_one_arg() {
        let out = exec(&["cloudflared", "tunnel", "route", "ip", "add", "cidr"]);
        assert_eq!(out.exit_code, 1);
        assert!(out.stderr.contains(ROUTE_IP_ADD_NARG_ERROR_MSG));
    }

    #[test]
    fn route_ip_add_accepts_two_args() {
        let out = exec(&[
            "cloudflared",
            "tunnel",
            "route",
            "ip",
            "add",
            "10.0.0.0/8",
            "my-tunnel",
        ]);
        assert!(
            !out.stderr.contains(ROUTE_IP_ADD_NARG_ERROR_MSG),
            "two positional args should pass NArg check"
        );
    }

    #[test]
    fn route_ip_delete_rejects_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "route", "ip", "delete"]);
        assert_eq!(out.exit_code, 1);
        assert!(out.stderr.contains(ROUTE_IP_DELETE_NARG_ERROR_MSG));
    }

    #[test]
    fn route_ip_delete_accepts_one_arg() {
        let out = exec(&["cloudflared", "tunnel", "route", "ip", "delete", "10.0.0.0/8"]);
        // Should not get NArg error — will be a stub error, exit 1, but different
        // message
        assert!(
            !out.stderr.contains(ROUTE_IP_DELETE_NARG_ERROR_MSG),
            "one arg should pass NArg check"
        );
    }

    #[test]
    fn route_ip_get_rejects_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "route", "ip", "get"]);
        assert_eq!(out.exit_code, 1);
        assert!(out.stderr.contains(ROUTE_IP_GET_NARG_ERROR_MSG));
    }

    #[test]
    fn route_ip_get_accepts_one_arg() {
        let out = exec(&["cloudflared", "tunnel", "route", "ip", "get", "10.0.0.1"]);
        assert!(
            !out.stderr.contains(ROUTE_IP_GET_NARG_ERROR_MSG),
            "one arg should pass NArg check"
        );
    }

    // --- Vnet subcommands (Go baseline: errors.New → exit 1) ---

    #[test]
    fn vnet_add_rejects_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "vnet", "add"]);
        assert_eq!(out.exit_code, 1);
        assert!(out.stderr.contains(VNET_ADD_NARG_ERROR_MSG));
    }

    #[test]
    fn vnet_add_accepts_one_arg() {
        let out = exec(&["cloudflared", "tunnel", "vnet", "add", "my-vnet"]);
        assert!(!out.stderr.contains(VNET_ADD_NARG_ERROR_MSG));
    }

    #[test]
    fn vnet_delete_rejects_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "vnet", "delete"]);
        assert_eq!(out.exit_code, 1);
        assert!(out.stderr.contains(VNET_DELETE_NARG_ERROR_MSG));
    }

    #[test]
    fn vnet_delete_accepts_one_arg() {
        let out = exec(&["cloudflared", "tunnel", "vnet", "delete", "my-vnet"]);
        assert!(!out.stderr.contains(VNET_DELETE_NARG_ERROR_MSG));
    }

    #[test]
    fn vnet_update_rejects_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "vnet", "update"]);
        assert_eq!(out.exit_code, 1);
        assert!(out.stderr.contains(VNET_UPDATE_NARG_ERROR_MSG));
    }

    #[test]
    fn vnet_update_rejects_two_args() {
        let out = exec(&["cloudflared", "tunnel", "vnet", "update", "a", "b"]);
        assert_eq!(out.exit_code, 1);
        assert!(out.stderr.contains(VNET_UPDATE_NARG_ERROR_MSG));
    }

    #[test]
    fn vnet_update_accepts_one_arg() {
        let out = exec(&["cloudflared", "tunnel", "vnet", "update", "my-vnet"]);
        assert!(!out.stderr.contains(VNET_UPDATE_NARG_ERROR_MSG));
    }

    // --- Ingress rule: empty args → exit 1 ---

    #[test]
    fn ingress_rule_rejects_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "ingress", "rule"]);
        assert_eq!(out.exit_code, 1);
        assert!(out.stderr.contains(INGRESS_RULE_NARG_ERROR_MSG));
    }

    #[test]
    fn ingress_rule_accepts_one_arg() {
        let out = exec(&[
            "cloudflared",
            "tunnel",
            "ingress",
            "rule",
            "http://localhost:8080",
        ]);
        assert!(!out.stderr.contains(INGRESS_RULE_NARG_ERROR_MSG));
    }

    // --- Subcommands with NO NArg validation in Go baseline ---

    #[test]
    fn tunnel_list_accepts_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "list"]);
        assert_ne!(out.exit_code, 255, "list has no NArg constraint");
    }

    #[test]
    fn tunnel_login_accepts_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "login"]);
        assert_ne!(out.exit_code, 255, "login has no NArg constraint");
    }

    #[test]
    fn route_ip_show_accepts_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "route", "ip", "show"]);
        assert_ne!(out.exit_code, 255, "route ip show has no NArg constraint");
    }

    #[test]
    fn vnet_list_accepts_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "vnet", "list"]);
        assert_ne!(out.exit_code, 255, "vnet list has no NArg constraint");
    }

    #[test]
    fn ingress_validate_accepts_zero_args() {
        let out = exec(&["cloudflared", "tunnel", "ingress", "validate"]);
        assert_ne!(out.exit_code, 255, "ingress validate has no NArg constraint");
    }

    // --- exit 255 help suffix verification ---

    #[test]
    fn usage_error_includes_help_suffix() {
        let out = exec(&["cloudflared", "tunnel", "create"]);
        assert_eq!(out.exit_code, 255);
        assert!(
            out.stderr.contains("See 'cloudflared tunnel create --help'."),
            "UsageError must include help suffix: {:?}",
            out.stderr
        );
    }

    // --- CLI-012 / CLI-032: tunnel run token precedence and identity
    // validation ---

    #[test]
    fn tunnel_run_with_valid_token_does_not_reject() {
        // Go baseline: --token <valid> → runWithCredentials (no error from token
        // path)
        let token = encoded_tunnel_token();
        let out = exec(&["cloudflared", "tunnel", "run", "--token", &token]);
        // Should proceed past token validation into startup (may get config error,
        // not token error)
        assert!(
            !out.stderr.contains(TUNNEL_TOKEN_INVALID_MSG),
            "valid token must not trigger invalid-token error"
        );
    }

    #[test]
    fn tunnel_run_with_invalid_token_exits_255() {
        // Go baseline: ParseToken fails → "Provided Tunnel token is not valid."
        // exit -1 (255)
        let out = exec(&["cloudflared", "tunnel", "run", "--token", "not-a-valid-token"]);
        assert_eq!(out.exit_code, 255);
        assert!(
            out.stderr.contains(TUNNEL_TOKEN_INVALID_MSG),
            "stderr must contain invalid token msg: {:?}",
            out.stderr
        );
        assert!(
            out.stderr.contains("See 'cloudflared tunnel run --help'."),
            "stderr must contain help suffix: {:?}",
            out.stderr
        );
    }

    #[test]
    fn tunnel_run_with_empty_token_falls_through() {
        // Go baseline: tokenStr == "" → falls through to positional arg / config
        let out = exec(&["cloudflared", "tunnel", "run", "--token", ""]);
        // Empty token → treated as no token → falls through to identity check
        assert!(
            !out.stderr.contains(TUNNEL_TOKEN_INVALID_MSG),
            "empty token must not trigger invalid-token error"
        );
    }

    #[test]
    fn tunnel_run_with_token_file_reads_token() {
        // Go baseline: --token-file → read file → use as tokenStr
        let token = encoded_tunnel_token();
        let token_path = std::env::temp_dir().join("cfdrs-run-token-file-test.txt");
        std::fs::write(&token_path, format!("{token}\n")).expect("write token file");

        let path_str = token_path.to_str().expect("path to str");
        let out = exec(&["cloudflared", "tunnel", "run", "--token-file", path_str]);

        // Valid token from file → should not get token error
        assert!(
            !out.stderr.contains(TUNNEL_TOKEN_INVALID_MSG),
            "token from file must not trigger invalid-token error"
        );
        let _ = std::fs::remove_file(token_path);
    }

    #[test]
    fn tunnel_run_with_invalid_token_file_exits_255() {
        // Go baseline: os.ReadFile fails → "Failed to read token file: <err>"
        let out = exec(&[
            "cloudflared",
            "tunnel",
            "run",
            "--token-file",
            "/nonexistent/path/to/token",
        ]);
        assert_eq!(out.exit_code, 255);
        assert!(
            out.stderr.contains(TUNNEL_TOKEN_FILE_READ_ERROR_PREFIX),
            "stderr must contain token file read error prefix: {:?}",
            out.stderr
        );
    }

    #[test]
    fn tunnel_run_token_flag_takes_precedence_over_token_file() {
        // Go baseline: --token is checked before --token-file
        let bad_file = std::env::temp_dir().join("cfdrs-run-token-precedence.txt");
        std::fs::write(&bad_file, "garbage-not-a-token\n").expect("write bad token file");

        let valid_token = encoded_tunnel_token();
        let path_str = bad_file.to_str().expect("path to str");
        let out = exec(&[
            "cloudflared",
            "tunnel",
            "run",
            "--token",
            &valid_token,
            "--token-file",
            path_str,
        ]);

        // --token is valid → should not get invalid-token error (--token-file is
        // ignored)
        assert!(
            !out.stderr.contains(TUNNEL_TOKEN_INVALID_MSG),
            "valid --token must take precedence over --token-file"
        );
        let _ = std::fs::remove_file(bad_file);
    }

    #[test]
    fn tunnel_run_no_identity_exits_255() {
        // Go baseline: no token, no positional arg, no config tunnel ID →
        // "requires the ID or name" error, exit -1 (255)
        let out = exec(&["cloudflared", "tunnel", "run"]);
        assert_eq!(
            out.exit_code, 255,
            "no identity must exit 255, got: exit={} stderr={:?}",
            out.exit_code, out.stderr
        );
        assert!(
            out.stderr.contains("requires the ID or name"),
            "stderr must contain identity error: {:?}",
            out.stderr
        );
    }

    #[test]
    fn bare_run_and_tunnel_run_both_dispatch() {
        // Go baseline: "cloudflared run" and "cloudflared tunnel run" produce
        // identical dispatch
        let out_bare = exec(&["cloudflared", "run"]);
        let out_tunnel = exec(&["cloudflared", "tunnel", "run"]);
        // Both should reach the same identity error (no token, no config)
        assert_eq!(out_bare.exit_code, out_tunnel.exit_code);
    }

    // --- CLI-001: service mode (bare invocation) ---

    #[test]
    fn service_mode_dispatches_to_startup() {
        // Go baseline: handleServiceMode() in main.go — empty invocation enters
        // service mode which discovers config and starts daemonically.
        // Without a config or discoverable default, this should produce a config
        // error, NOT the old stub error.
        let out = exec(&["cloudflared"]);
        assert!(
            !out.stderr.contains("service mode requires a configuration file"),
            "service mode stub must be replaced by real dispatch"
        );
    }

    #[test]
    fn service_mode_with_config_dispatches_to_runtime() {
        // Go baseline: handleServiceMode() with --config → load config and run.
        // Use a valid config that has a tunnel, expect it to reach the runtime
        // (not service mode stub error).
        let config_path = std::env::temp_dir().join("cfdrs-service-mode-test.yml");
        std::fs::write(
            &config_path,
            "tunnel: 00000000-0000-0000-0000-000000000000\ningress:\n  - service: http_status:503\n",
        )
        .expect("write test config");

        let path_str = config_path.to_str().expect("path to str");
        let out = exec(&["cloudflared", "--config", path_str]);

        assert!(
            !out.stderr.contains("service mode requires a configuration file"),
            "bare invocation with --config must not hit old stub"
        );
        let _ = std::fs::remove_file(config_path);
    }
}
