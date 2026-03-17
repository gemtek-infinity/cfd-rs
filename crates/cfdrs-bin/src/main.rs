#![forbid(unsafe_code)]

mod access_commands;
#[allow(dead_code)] // Wired incrementally during CLI Foundation closure.
mod api_client;
mod protocol;
mod proxy;
mod route_vnet_commands;
mod runtime;
mod startup;
mod tail_management;
mod transport;
mod tunnel_commands;
mod tunnel_local_commands;
mod tunnel_login;

// Admitted for non-blocking file writer layer; not yet wired (hand-rolled
// size-based rotation matches Go parity).
use tracing_appender as _;

use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use cfdrs_cli::{
    AccessSubcommand, CLASSIC_TUNNEL_DEPRECATED_MSG, Cli, CliError, CliOutput, Command,
    DB_CONNECT_REMOVED_MSG, GlobalFlags, HelpTarget, INGRESS_RULE_NARG_ERROR_MSG, IngressSubcommand,
    IpRouteSubcommand, ManagementSubcommand, PROGRAM_NAME, PROXY_DNS_REMOVED_LOG_MSG, PROXY_DNS_REMOVED_MSG,
    ROUTE_DNS_NARG_ERROR_MSG, ROUTE_IP_ADD_NARG_ERROR_MSG, ROUTE_IP_DELETE_NARG_ERROR_MSG,
    ROUTE_IP_GET_NARG_ERROR_MSG, ROUTE_LB_NARG_ERROR_MSG, RouteSubcommand, ServiceAction,
    TUNNEL_CLEANUP_NARG_ERROR_MSG, TUNNEL_CMD_ERROR_MSG, TUNNEL_CREATE_NARG_ERROR_MSG,
    TUNNEL_DELETE_NARG_ERROR_MSG, TUNNEL_INFO_NARG_ERROR_MSG, TUNNEL_RUN_HOSTNAME_WARNING_MSG,
    TUNNEL_RUN_IDENTITY_ERROR_MSG, TUNNEL_RUN_NARG_ERROR_MSG, TUNNEL_TOKEN_FILE_READ_ERROR_PREFIX,
    TUNNEL_TOKEN_INVALID_MSG, TUNNEL_TOKEN_NARG_ERROR_MSG, TailSubcommand, TunnelSubcommand,
    VNET_ADD_NARG_ERROR_MSG, VNET_DELETE_NARG_ERROR_MSG, VNET_UPDATE_NARG_ERROR_MSG, VnetSubcommand,
    parse_args, render_access_help, render_help, render_management_help, render_management_token_help,
    render_short_version, render_subcommand_help, render_tunnel_help, render_version_output,
    stub_not_implemented, subcommand_usage_error, tunnel_run_usage_error,
};
use cfdrs_his::environment::current_executable;
use cfdrs_his::service::{
    ProcessRunner, SERVICE_CONFIG_PATH, ServiceTemplateArgs, build_args_for_config, build_args_for_token,
    copy_file, install_linux_service, uninstall_linux_service,
};
use cfdrs_his::updater::{
    ManualUpdateOutcome, UPDATE_EXIT_FAILURE, UPDATE_EXIT_SUCCESS, Updater, WorkersUpdateRequest,
    WorkersUpdater, run_manual_update,
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
        Command::Help(HelpTarget::Management) => CliOutput::success(render_management_help(PROGRAM_NAME)),
        Command::Help(HelpTarget::ManagementToken) => {
            CliOutput::success(render_management_token_help(PROGRAM_NAME))
        }
        Command::Help(target) => CliOutput::success(render_subcommand_help(target)),
        Command::Version { short: true } => CliOutput::success(render_short_version()),
        Command::Version { short: false } => CliOutput::success(render_version_output(PROGRAM_NAME)),
        Command::Validate => execute_startup_command(&cli, CliMode::Validate),
        Command::Update => execute_update(&cli.flags),

        Command::Service(ServiceAction::Install) => execute_service_install(&cli),
        Command::Service(ServiceAction::Uninstall) => execute_service_uninstall(),

        // Go baseline: login at root level dispatches same as tunnel login
        // (main.go: loginCommand() → login.go: login()).
        Command::Login => tunnel_login::execute_tunnel_login(
            cli.flags.fedramp,
            cli.flags.login_url.as_deref(),
            cli.flags.callback_url.as_deref(),
            &tunnel_login::XdgOpenLauncher,
        ),

        Command::Tunnel(_) => dispatch_tunnel_subcommand(cli),

        // Go baseline: `access` command family from `access/cmd.go`.
        // Bare `access` shows help (urfave/cli default for commands with
        // subcommands); each subcommand dispatches explicitly.
        Command::Access(AccessSubcommand::Bare) => CliOutput::success(render_access_help(PROGRAM_NAME)),
        Command::Access(AccessSubcommand::Login) => access_commands::execute_access_login(&cli.flags),
        Command::Access(AccessSubcommand::Curl) => access_commands::execute_access_curl(&cli.flags),
        Command::Access(AccessSubcommand::Token) => access_commands::execute_access_token(&cli.flags),
        Command::Access(AccessSubcommand::Tcp) => access_commands::execute_access_tcp(&cli.flags),
        Command::Access(AccessSubcommand::SshConfig) => {
            access_commands::execute_access_ssh_config(&cli.flags)
        }
        Command::Access(AccessSubcommand::SshGen) => access_commands::execute_access_ssh_gen(&cli.flags),

        // Go baseline: `tail` command family from `tail/cmd.go`.
        // Bare `tail [TUNNEL-ID]` runs the streaming action; `tail token`
        // is a hidden subcommand that fetches a management JWT.
        Command::Tail(TailSubcommand::Token) => tail_management::execute_tail_token(&cli.flags),
        Command::Tail(TailSubcommand::Bare) => tail_management::execute_tail(&cli.flags),

        // Go baseline: `management` command family from `management/cmd.go`.
        // Entirely hidden; `management token` fetches a management JWT.
        Command::Management(ManagementSubcommand::Token) => {
            tail_management::execute_management_token(&cli.flags)
        }
        Command::Management(ManagementSubcommand::Bare) => {
            CliOutput::success(render_management_help(PROGRAM_NAME))
        }

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
    }
}

/// Dispatch all `Command::Tunnel(*)` variants — Go baseline:
/// `TunnelCommand()` dispatch tree in `cmd/cloudflared/tunnel/cmd.go`.
fn dispatch_tunnel_subcommand(cli: Cli) -> CliOutput {
    match &cli.command {
        Command::Tunnel(TunnelSubcommand::Run) => execute_tunnel_run(cli),
        Command::Tunnel(TunnelSubcommand::Bare) => execute_tunnel_bare(&cli),

        // Go baseline: tunnel login → login.go: login().
        Command::Tunnel(TunnelSubcommand::Login) => tunnel_login::execute_tunnel_login(
            cli.flags.fedramp,
            cli.flags.login_url.as_deref(),
            cli.flags.callback_url.as_deref(),
            &tunnel_login::XdgOpenLauncher,
        ),

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

        // Tunnel CRUD subcommands (create, list, delete, cleanup, token, info).
        Command::Tunnel(
            sub @ (TunnelSubcommand::Create
            | TunnelSubcommand::List
            | TunnelSubcommand::Delete
            | TunnelSubcommand::Cleanup
            | TunnelSubcommand::Token
            | TunnelSubcommand::Info),
        ) => dispatch_tunnel_crud(sub, &cli.flags),

        // Route subcommands.
        Command::Tunnel(TunnelSubcommand::Route(sub)) => dispatch_route_subcommand(sub, &cli.flags),

        // Vnet subcommands.
        Command::Tunnel(TunnelSubcommand::Vnet(sub)) => dispatch_vnet_subcommand(sub, &cli.flags),

        // Ingress subcommands.
        Command::Tunnel(TunnelSubcommand::Ingress(IngressSubcommand::Validate)) => {
            tunnel_local_commands::execute_ingress_validate(&cli.flags)
        }
        Command::Tunnel(TunnelSubcommand::Ingress(IngressSubcommand::Rule)) => {
            if cli.flags.rest_args.len() != 1 {
                return CliOutput::failure(String::new(), INGRESS_RULE_NARG_ERROR_MSG.to_owned(), 1);
            }
            tunnel_local_commands::execute_ingress_rule(&cli.flags)
        }
        Command::Tunnel(TunnelSubcommand::Ingress(IngressSubcommand::Bare)) => {
            CliOutput::success(render_subcommand_help(&HelpTarget::TunnelIngress))
        }

        Command::Tunnel(TunnelSubcommand::Ready) => tunnel_local_commands::execute_tunnel_ready(&cli.flags),
        Command::Tunnel(TunnelSubcommand::Diag) => tunnel_local_commands::execute_tunnel_diag(&cli.flags),

        // Everything else is recognized but not yet implemented.
        other => CliOutput::failure(String::new(), stub_not_implemented(&full_command_label(other)), 1),
    }
}

/// Dispatch tunnel CRUD subcommands with NArg validation.
fn dispatch_tunnel_crud(sub: &TunnelSubcommand, flags: &GlobalFlags) -> CliOutput {
    match sub {
        TunnelSubcommand::Create => {
            validate_narg_exact(flags, 1, "tunnel create", TUNNEL_CREATE_NARG_ERROR_MSG)
                .unwrap_or_else(|| tunnel_commands::execute_tunnel_create(flags))
        }
        TunnelSubcommand::List => tunnel_commands::execute_tunnel_list(flags),
        TunnelSubcommand::Delete => {
            validate_narg_min(flags, 1, "tunnel delete", TUNNEL_DELETE_NARG_ERROR_MSG)
                .unwrap_or_else(|| tunnel_commands::execute_tunnel_delete(flags))
        }
        TunnelSubcommand::Cleanup => {
            validate_narg_min(flags, 1, "tunnel cleanup", TUNNEL_CLEANUP_NARG_ERROR_MSG)
                .unwrap_or_else(|| tunnel_commands::execute_tunnel_cleanup(flags))
        }
        TunnelSubcommand::Token => validate_narg_exact(flags, 1, "tunnel token", TUNNEL_TOKEN_NARG_ERROR_MSG)
            .unwrap_or_else(|| tunnel_commands::execute_tunnel_token(flags)),
        TunnelSubcommand::Info => validate_narg_exact(flags, 1, "tunnel info", TUNNEL_INFO_NARG_ERROR_MSG)
            .unwrap_or_else(|| tunnel_commands::execute_tunnel_info(flags)),
        _ => unreachable!("dispatch_tunnel_crud called with non-CRUD subcommand"),
    }
}

/// Dispatch `tunnel route *` subcommands.
fn dispatch_route_subcommand(sub: &RouteSubcommand, flags: &GlobalFlags) -> CliOutput {
    match sub {
        RouteSubcommand::Dns => validate_narg_exact(flags, 2, "tunnel route dns", ROUTE_DNS_NARG_ERROR_MSG)
            .unwrap_or_else(|| route_vnet_commands::execute_route_dns(flags)),
        RouteSubcommand::Lb => validate_narg_exact(flags, 3, "tunnel route lb", ROUTE_LB_NARG_ERROR_MSG)
            .unwrap_or_else(|| route_vnet_commands::execute_route_lb(flags)),
        RouteSubcommand::Ip(IpRouteSubcommand::Add) => {
            if flags.rest_args.len() < 2 {
                return CliOutput::failure(String::new(), ROUTE_IP_ADD_NARG_ERROR_MSG.to_owned(), 1);
            }
            route_vnet_commands::execute_route_ip_add(flags)
        }
        RouteSubcommand::Ip(IpRouteSubcommand::Show) => route_vnet_commands::execute_route_ip_show(flags),
        RouteSubcommand::Ip(IpRouteSubcommand::Delete) => {
            if flags.rest_args.len() != 1 {
                return CliOutput::failure(String::new(), ROUTE_IP_DELETE_NARG_ERROR_MSG.to_owned(), 1);
            }
            route_vnet_commands::execute_route_ip_delete(flags)
        }
        RouteSubcommand::Ip(IpRouteSubcommand::Get) => {
            if flags.rest_args.len() != 1 {
                return CliOutput::failure(String::new(), ROUTE_IP_GET_NARG_ERROR_MSG.to_owned(), 1);
            }
            route_vnet_commands::execute_route_ip_get(flags)
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
            route_vnet_commands::execute_vnet_add(flags)
        }
        VnetSubcommand::List => route_vnet_commands::execute_vnet_list(flags),
        VnetSubcommand::Delete => {
            if flags.rest_args.is_empty() {
                return CliOutput::failure(String::new(), VNET_DELETE_NARG_ERROR_MSG.to_owned(), 1);
            }
            route_vnet_commands::execute_vnet_delete(flags)
        }
        VnetSubcommand::Update => {
            if flags.rest_args.len() != 1 {
                return CliOutput::failure(String::new(), VNET_UPDATE_NARG_ERROR_MSG.to_owned(), 1);
            }
            route_vnet_commands::execute_vnet_update(flags)
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

fn execute_update(flags: &GlobalFlags) -> CliOutput {
    let target_path = match current_executable() {
        Ok(path) => path,
        Err(error) => return CliError::config(error).into_output(),
    };

    let request = WorkersUpdateRequest::new(
        env!("CARGO_PKG_VERSION"),
        target_path,
        flags.update_beta,
        flags.update_staging,
        flags.force,
        flags.update_version.clone(),
    );
    let updater = match WorkersUpdater::new(request) {
        Ok(updater) => updater,
        Err(error) => return CliError::config(error).into_output(),
    };

    execute_update_with_updater(&updater, cfdrs_his::updater::should_skip_update())
}

fn execute_update_with_updater(updater: &dyn Updater, package_managed: bool) -> CliOutput {
    match run_manual_update(updater, package_managed) {
        Ok(ManualUpdateOutcome::PackageManaged { message }) => CliOutput {
            stdout: String::new(),
            stderr: format!("{message}\n"),
            exit_code: 0,
        },
        Ok(ManualUpdateOutcome::NoUpdate { user_message }) => {
            CliOutput::success(render_update_stdout(user_message, "cloudflared is up to date"))
        }
        Ok(ManualUpdateOutcome::Updated {
            version,
            user_message,
        }) => CliOutput {
            stdout: render_update_stdout(
                user_message,
                &format!("cloudflared has been updated to version {version}"),
            ),
            stderr: String::new(),
            exit_code: UPDATE_EXIT_SUCCESS as u8,
        },
        Err(error) => CliOutput::failure(
            String::new(),
            format!("failed to update cloudflared: {error}\n"),
            UPDATE_EXIT_FAILURE as u8,
        ),
    }
}

fn render_update_stdout(user_message: Option<String>, status_line: &str) -> String {
    let mut lines = Vec::new();
    if let Some(message) = user_message
        && !message.is_empty()
    {
        lines.push(message);
    }
    lines.push(status_line.to_owned());
    lines.join("\n") + "\n"
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
            // Token string found — validate and decode (Go line 777–778).
            let token = match TunnelToken::decode(s) {
                Ok(t) => t,
                Err(_) => {
                    return CliOutput::failure(
                        String::new(),
                        tunnel_run_usage_error(TUNNEL_TOKEN_INVALID_MSG),
                        255,
                    );
                }
            };

            // Go baseline: sc.runWithCredentials(token.Credentials())
            // Token provides both tunnel identity and credentials directly.
            return execute_run_with_token(flags, token);
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
    let tunnel_ref = flags.rest_args.first().map(|s| s.as_str());

    // Step 4: Config tunnel ID fallback (Go lines 783–787).
    match resolve_startup(flags.config_path.clone()) {
        Ok(mut startup) => {
            // If positional arg provided, override config tunnel with it.
            if let Some(name_or_id) = tunnel_ref {
                startup.normalized.tunnel =
                    Some(cfdrs_shared::TunnelReference::from_raw(name_or_id.to_owned()));
            }

            if startup.normalized.tunnel.is_some() {
                execute_runtime_command(startup, flags)
            } else {
                CliOutput::failure(
                    String::new(),
                    tunnel_run_usage_error(TUNNEL_RUN_IDENTITY_ERROR_MSG),
                    255,
                )
            }
        }
        Err(error) => {
            if tunnel_ref.is_some() {
                // Positional arg present but config resolution failed.
                // Go still needs config context for ingress/runtime.
                CliError::config(error).into_output()
            } else {
                CliOutput::failure(
                    String::new(),
                    tunnel_run_usage_error(TUNNEL_RUN_IDENTITY_ERROR_MSG),
                    255,
                )
            }
        }
    }
}

/// Run with credentials extracted directly from a decoded token.
///
/// Go baseline: `sc.runWithCredentials(token.Credentials())` — the token
/// provides both the tunnel identity (UUID) and the credential material,
/// bypassing config-based credential discovery.
fn execute_run_with_token(flags: &GlobalFlags, token: TunnelToken) -> CliOutput {
    let mut startup = match resolve_startup(flags.config_path.clone()) {
        Ok(s) => s,
        Err(error) => return CliError::config(error).into_output(),
    };

    // Inject token-derived tunnel identity and credential file into the
    // startup surface so the runtime uses them without file-based discovery.
    startup.normalized.tunnel = Some(cfdrs_shared::TunnelReference::from_raw(
        token.tunnel_id.to_string(),
    ));

    // Write token credentials to a temp file so the runtime can load them
    // through the standard credential-file path.  Go passes credentials
    // in-memory; we bridge via a temporary credential file.
    let creds = token.to_credentials_file();
    match creds.to_pretty_json() {
        Ok(json) => {
            let cred_dir = std::env::temp_dir();
            let cred_path = cred_dir.join(format!("{}.json", token.tunnel_id));
            if std::fs::write(&cred_path, json).is_ok() {
                startup.normalized.credentials.credentials_file = Some(cred_path);
            }
        }
        Err(error) => return CliError::config(error).into_output(),
    }

    execute_runtime_command(startup, flags)
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
#[path = "main_tests.rs"]
mod tests;
