#![forbid(unsafe_code)]

mod protocol;
mod proxy;
mod runtime;
mod startup;
mod transport;

use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use cfdrs_cli::{
    AccessSubcommand, CLASSIC_TUNNEL_DEPRECATED_MSG, Cli, CliError, CliOutput, Command,
    DB_CONNECT_REMOVED_MSG, GlobalFlags, IngressSubcommand, IpRouteSubcommand, ManagementSubcommand,
    PROGRAM_NAME, PROXY_DNS_REMOVED_LOG_MSG, PROXY_DNS_REMOVED_MSG, RouteSubcommand, ServiceAction,
    TUNNEL_CMD_ERROR_MSG, TUNNEL_RUN_HOSTNAME_WARNING_MSG, TUNNEL_RUN_NARG_ERROR_MSG, TailSubcommand,
    TunnelSubcommand, VnetSubcommand, parse_args, render_help, render_short_version, render_version_output,
    stub_not_implemented, tunnel_run_usage_error,
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
        Command::Help => CliOutput::success(render_help(PROGRAM_NAME)),
        Command::Version { short: true } => CliOutput::success(render_short_version()),
        Command::Version { short: false } => CliOutput::success(render_version_output(PROGRAM_NAME)),
        Command::Validate => execute_startup_command(&cli, CliMode::Validate),

        Command::Service(ServiceAction::Install) => execute_service_install(&cli),
        Command::Service(ServiceAction::Uninstall) => execute_service_uninstall(),

        Command::Tunnel(TunnelSubcommand::Run) => execute_tunnel_run(cli),

        // Go baseline: TunnelCommand() dispatch tree in cmd/cloudflared/tunnel/cmd.go
        Command::Tunnel(TunnelSubcommand::Bare) => execute_tunnel_bare(&cli),

        // Go baseline: handleServiceMode() in main.go — daemon-style
        // config-watcher loop when invoked with zero args and zero flags.
        // Requires HIS watcher/reload infrastructure (HIS-041 through HIS-043).
        Command::ServiceMode => CliOutput::failure(
            String::new(),
            "service mode requires a configuration file; use 'cloudflared tunnel run' with --config or \
             --token instead"
                .to_owned(),
            1,
        ),

        // Removed features — exact error messages from Go baseline.
        Command::ProxyDns | Command::Tunnel(TunnelSubcommand::ProxyDns) => {
            // Go baseline: log.Error().Msg("DNS Proxy is no longer supported since version
            // ...") then returns errors.New(removedMessage).  urfave/cli exit
            // code is 1.
            eprintln!("{PROXY_DNS_REMOVED_LOG_MSG}");
            CliOutput::failure(String::new(), PROXY_DNS_REMOVED_MSG.to_owned(), 1)
        }
        Command::Tunnel(TunnelSubcommand::DbConnect) => {
            // Go baseline: cliutil.RemovedCommand("db-connect") uses cli.Exit(..., -1)
            // which shells see as exit code 255 (unsigned byte truncation of -1).
            CliOutput::failure(String::new(), DB_CONNECT_REMOVED_MSG.to_owned(), 255)
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
        Command::Service(action) => match action {
            ServiceAction::Install => "service install".into(),
            ServiceAction::Uninstall => "service uninstall".into(),
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
            CliMode::Run => execute_runtime_command(startup, &cli.flags),
        },
        Err(error) => CliError::config(error).into_output(),
    }
}

/// Go baseline: `runCommand()` in subcommands.go lines 748–788.
///
/// Validation order (matching Go exactly):
///   1. NArg > 1 → UsageError
///   2. `--hostname` set → warning log (non-fatal)
///   3. `--token` → `--token-file` → positional arg → config tunnel ID
///   4. missing identity → UsageError
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

    // Steps 2–4: token resolution and identity check happen after config
    // loading in execute_runtime_command / prepare_runtime_startup.
    // Token-only mode (bypassing config entirely) requires startup path
    // restructuring and is tracked as a remaining CLI-032 gap.

    execute_startup_command(&cli, CliMode::Run)
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
        let output = execute(["cloudflared", "tunnel", "run"].into_iter().map(OsString::from));

        assert_ne!(
            output.exit_code, 255,
            "zero positional args must not trigger NArg rejection"
        );
        assert!(
            !output.stderr.contains("accepts only one argument"),
            "zero positional args must not trigger NArg error"
        );
    }
}
