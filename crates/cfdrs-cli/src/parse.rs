mod value_flags;

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use self::value_flags::try_parse_value_flag;
use super::types::{GlobalFlags, HelpTarget, TunnelSubcommand};
use super::{Cli, Command, surface_contract};

/// Short-circuit builder for value-flag matching.
///
/// Each method is a no-op once a previous method has already matched.
/// After all candidates are tried, call `.matched()` to learn whether
/// the argument was consumed.
struct FlagMatcher<'a, I> {
    arg: &'a OsStr,
    args: &'a mut I,
    flags: &'a mut GlobalFlags,
    matched: bool,
}

impl<'a, I: Iterator<Item = OsString>> FlagMatcher<'a, I> {
    fn new(arg: &'a OsStr, args: &'a mut I, flags: &'a mut GlobalFlags) -> Self {
        Self {
            arg,
            args,
            flags,
            matched: false,
        }
    }

    fn string(
        &mut self,
        name: &str,
        target: impl FnOnce(&mut GlobalFlags) -> &mut Option<String>,
    ) -> Result<&mut Self, String> {
        if !self.matched
            && let Some(v) = try_string_flag(self.arg, self.args, name)?
        {
            *target(self.flags) = Some(v);
            self.matched = true;
        }
        Ok(self)
    }

    fn string_alias(
        &mut self,
        name: &str,
        alias: &str,
        target: impl FnOnce(&mut GlobalFlags) -> &mut Option<String>,
    ) -> Result<&mut Self, String> {
        if !self.matched {
            let v =
                try_string_flag(self.arg, self.args, name)?.or(try_string_flag(self.arg, self.args, alias)?);

            if let Some(v) = v {
                *target(self.flags) = Some(v);
                self.matched = true;
            }
        }
        Ok(self)
    }

    fn path(
        &mut self,
        name: &str,
        target: impl FnOnce(&mut GlobalFlags) -> &mut Option<PathBuf>,
    ) -> Result<&mut Self, String> {
        if !self.matched
            && let Some(v) = try_string_flag(self.arg, self.args, name)?
        {
            set_path_flag(target(self.flags), v, name)?;
            self.matched = true;
        }
        Ok(self)
    }

    fn path_alias(
        &mut self,
        name: &str,
        alias: &str,
        target: impl FnOnce(&mut GlobalFlags) -> &mut Option<PathBuf>,
    ) -> Result<&mut Self, String> {
        if !self.matched {
            let v =
                try_string_flag(self.arg, self.args, name)?.or(try_string_flag(self.arg, self.args, alias)?);

            if let Some(v) = v {
                set_path_flag(target(self.flags), v, name)?;
                self.matched = true;
            }
        }
        Ok(self)
    }

    fn push(
        &mut self,
        name: &str,
        target: impl FnOnce(&mut GlobalFlags) -> &mut Vec<String>,
    ) -> Result<&mut Self, String> {
        if !self.matched
            && let Some(v) = try_string_flag(self.arg, self.args, name)?
        {
            target(self.flags).push(v);
            self.matched = true;
        }
        Ok(self)
    }

    fn push_alias(
        &mut self,
        name: &str,
        alias: &str,
        target: impl FnOnce(&mut GlobalFlags) -> &mut Vec<String>,
    ) -> Result<&mut Self, String> {
        if !self.matched {
            let v =
                try_string_flag(self.arg, self.args, name)?.or(try_string_flag(self.arg, self.args, alias)?);

            if let Some(v) = v {
                target(self.flags).push(v);
                self.matched = true;
            }
        }
        Ok(self)
    }

    fn u16_val(
        &mut self,
        name: &str,
        target: impl FnOnce(&mut GlobalFlags) -> &mut Option<u16>,
    ) -> Result<&mut Self, String> {
        if !self.matched
            && let Some(v) = try_string_flag(self.arg, self.args, name)?
        {
            *target(self.flags) = Some(
                v.parse::<u16>()
                    .map_err(|_| format!("invalid value for {name}: {v}"))?,
            );
            self.matched = true;
        }
        Ok(self)
    }

    fn u32_val(
        &mut self,
        name: &str,
        target: impl FnOnce(&mut GlobalFlags) -> &mut Option<u32>,
    ) -> Result<&mut Self, String> {
        if !self.matched
            && let Some(v) = try_string_flag(self.arg, self.args, name)?
        {
            *target(self.flags) = Some(parse_u32(&v, name)?);
            self.matched = true;
        }
        Ok(self)
    }

    fn u64_val(
        &mut self,
        name: &str,
        target: impl FnOnce(&mut GlobalFlags) -> &mut Option<u64>,
    ) -> Result<&mut Self, String> {
        if !self.matched
            && let Some(v) = try_string_flag(self.arg, self.args, name)?
        {
            *target(self.flags) = Some(parse_u64(&v, name)?);
            self.matched = true;
        }
        Ok(self)
    }

    fn matched(&self) -> bool {
        self.matched
    }
}

#[derive(Default)]
struct ParseState {
    flags: GlobalFlags,
    command: Option<Command>,
    help_requested: bool,
    version_requested: bool,
    short_version: bool,
    any_flag_set: bool,
    /// After a top-level command word is consumed, remaining positional
    /// args for subcommand parsing land here before being resolved.
    awaiting_subcommand: Option<SubcommandContext>,
}

/// Tracks which top-level command was seen so subsequent positional
/// args can be resolved as subcommands.
#[derive(Debug)]
enum SubcommandContext {
    Tunnel,
    Service,
    Access,
    Tail,
    Management,
    Route,
    RouteIp,
    Vnet,
    Ingress,
}

pub fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Cli, String> {
    let mut args = args.into_iter();
    let _ = args.next();

    let mut state = ParseState::default();

    while let Some(arg) = args.next() {
        handle_argument(arg, &mut args, &mut state)?;
    }

    Ok(finalize_cli(state))
}

fn handle_argument(
    arg: OsString,
    args: &mut impl Iterator<Item = OsString>,
    state: &mut ParseState,
) -> Result<(), String> {
    let token = arg.to_string_lossy();
    let token_str = token.as_ref();

    if handle_priority_flag(token_str, state) {
        return Ok(());
    }

    if try_parse_flag(arg.as_os_str(), args, state)? {
        return Ok(());
    }

    if dispatch_command_token(token_str, state)? {
        return Ok(());
    }

    handle_unrecognized_token(token_str, state)
}

fn handle_priority_flag(token_str: &str, state: &mut ParseState) -> bool {
    if surface_contract::is_help_token(token_str) {
        state.help_requested = true;
        return true;
    }

    if surface_contract::is_version_token(token_str) {
        state.version_requested = true;
        return true;
    }

    if surface_contract::is_short_version_token(token_str) {
        state.short_version = true;
        return true;
    }

    false
}

fn dispatch_command_token(token_str: &str, state: &mut ParseState) -> Result<bool, String> {
    if state.awaiting_subcommand.is_some() {
        resolve_subcommand(token_str, state)?;
        return Ok(true);
    }

    if let Some(command) = surface_contract::parse_command_token(token_str) {
        apply_top_level_command(command, state)?;
        return Ok(true);
    }

    Ok(false)
}

fn handle_unrecognized_token(token_str: &str, state: &mut ParseState) -> Result<(), String> {
    if state.command.is_some() {
        state.flags.rest_args.push(token_str.to_owned());
        return Ok(());
    }

    if token_str.starts_with('-') {
        return Err(surface_contract::unknown_flag_message(token_str));
    }

    Err(surface_contract::unknown_argument_message(token_str))
}

/// Resolve a token as a subcommand within the current subcommand context.
/// Called only when `state.awaiting_subcommand` is `Some`.
fn resolve_subcommand(token_str: &str, state: &mut ParseState) -> Result<(), String> {
    // Take ownership to avoid holding an immutable borrow across mutable
    // sub-function calls (resolve_tunnel_subcommand / resolve_route_subcommand).
    let ctx = state.awaiting_subcommand.take().expect("caller checks Some");

    let resolved = match ctx {
        SubcommandContext::Tunnel => resolve_tunnel_subcommand(token_str, state),

        SubcommandContext::Service => {
            surface_contract::parse_service_subcommand(token_str).map(Command::Service)
        }
        SubcommandContext::Access => {
            surface_contract::parse_access_subcommand(token_str).map(Command::Access)
        }
        SubcommandContext::Tail => surface_contract::parse_tail_subcommand(token_str).map(Command::Tail),
        SubcommandContext::Management => {
            surface_contract::parse_management_subcommand(token_str).map(Command::Management)
        }

        SubcommandContext::Route => resolve_route_subcommand(token_str, state),

        SubcommandContext::RouteIp => surface_contract::parse_ip_route_subcommand(token_str)
            .map(|s| Command::Tunnel(TunnelSubcommand::Route(super::types::RouteSubcommand::Ip(s)))),
        SubcommandContext::Vnet => surface_contract::parse_vnet_subcommand(token_str)
            .map(|s| Command::Tunnel(TunnelSubcommand::Vnet(s))),
        SubcommandContext::Ingress => surface_contract::parse_ingress_subcommand(token_str)
            .map(|s| Command::Tunnel(TunnelSubcommand::Ingress(s))),
    };

    if let Some(cmd) = resolved {
        state.command = Some(cmd);
    } else {
        // Not resolved — restore the context for subsequent tokens.
        state.awaiting_subcommand = Some(ctx);
        state.flags.rest_args.push(token_str.to_owned());
    }

    Ok(())
}

/// Tunnel subcommands may enter deeper sub-subcommand parsing.
fn resolve_tunnel_subcommand(token_str: &str, state: &mut ParseState) -> Option<Command> {
    let sub = surface_contract::parse_tunnel_subcommand(token_str)?;

    match &sub {
        TunnelSubcommand::Route(_) => {
            state.awaiting_subcommand = Some(SubcommandContext::Route);
        }
        TunnelSubcommand::Vnet(_) => {
            state.awaiting_subcommand = Some(SubcommandContext::Vnet);
        }
        TunnelSubcommand::Ingress(_) => {
            state.awaiting_subcommand = Some(SubcommandContext::Ingress);
        }
        _ => {
            state.awaiting_subcommand = None;
        }
    }

    Some(Command::Tunnel(sub))
}

/// Route subcommands may enter IP sub-subcommand parsing.
fn resolve_route_subcommand(token_str: &str, state: &mut ParseState) -> Option<Command> {
    let sub = surface_contract::parse_route_subcommand(token_str)?;

    match &sub {
        super::types::RouteSubcommand::Ip(_) => {
            state.awaiting_subcommand = Some(SubcommandContext::RouteIp);
        }
        _ => {
            state.awaiting_subcommand = None;
        }
    }

    Some(Command::Tunnel(TunnelSubcommand::Route(sub)))
}

/// Apply a recognized top-level command word, entering subcommand parsing
/// mode for commands that have sub-subcommands.
fn apply_top_level_command(command: Command, state: &mut ParseState) -> Result<(), String> {
    match &command {
        Command::Tunnel(TunnelSubcommand::Bare) => {
            state.awaiting_subcommand = Some(SubcommandContext::Tunnel);
            state.command = Some(command);
        }

        Command::Service(_) => {
            state.awaiting_subcommand = Some(SubcommandContext::Service);
            state.command = Some(command);
        }

        Command::Access(_) => {
            state.awaiting_subcommand = Some(SubcommandContext::Access);
            state.command = Some(command);
        }

        Command::Tail(_) => {
            state.awaiting_subcommand = Some(SubcommandContext::Tail);
            state.command = Some(command);
        }

        Command::Management(_) => {
            state.awaiting_subcommand = Some(SubcommandContext::Management);
            state.command = Some(command);
        }

        _ => {
            set_command(&mut state.command, command)?;
        }
    }

    Ok(())
}

/// Try to parse the argument as a known flag.
/// Returns `true` if the argument was consumed as a flag.
fn try_parse_flag(
    arg: &OsStr,
    args: &mut impl Iterator<Item = OsString>,
    state: &mut ParseState,
) -> Result<bool, String> {
    if try_parse_value_flag(arg, args, state)? {
        return Ok(true);
    }

    if try_parse_bool_flag(arg, state) {
        state.any_flag_set = true;
        return Ok(true);
    }

    Ok(false)
}

/// Try to consume the argument as a boolean flag (no value).
/// Returns `true` if matched.
fn try_parse_bool_flag(arg: &OsStr, state: &mut ParseState) -> bool {
    let arg_str = arg.to_string_lossy();

    match arg_str.as_ref() {
        "--no-autoupdate" => state.flags.no_autoupdate = true,
        "--hello-world" => state.flags.hello_world = true,
        "--no-tls-verify" => state.flags.no_tls_verify = true,
        "--no-chunked-encoding" => state.flags.no_chunked_encoding = true,
        "--http2-origin" => state.flags.http2_origin = true,
        "--post-quantum" | "-pq" => state.flags.post_quantum = Some(true),
        "--is-autoupdated" => state.flags.is_autoupdated = true,
        "--bastion" => state.flags.bastion = true,
        "--socks5" => state.flags.socks5 = true,
        "--proxy-no-happy-eyeballs" => state.flags.proxy_no_happy_eyeballs = true,
        "--quic-disable-pmtu-discovery" => state.flags.quic_disable_pmtu = true,
        "--no-update-service" => state.flags.no_update_service = true,
        "--proxy-dns" => state.flags.proxy_dns = true,
        _ => return false,
    }

    true
}

/// Try to extract a string value from `--flag VALUE` or `--flag=VALUE`.
fn try_string_flag(
    arg: &OsStr,
    args: &mut impl Iterator<Item = OsString>,
    name: &str,
) -> Result<Option<String>, String> {
    if arg == OsStr::new(name) {
        let value = args
            .next()
            .ok_or_else(|| surface_contract::missing_flag_value_message(name))?;
        return Ok(Some(value.to_string_lossy().into_owned()));
    }

    if let Some(value) = parse_equals_flag(arg, name) {
        return Ok(Some(value.to_owned()));
    }

    Ok(None)
}

fn finalize_cli(state: ParseState) -> Cli {
    let ParseState {
        flags,
        command,
        help_requested,
        version_requested,
        short_version,
        any_flag_set,
        awaiting_subcommand: _,
    } = state;

    if help_requested {
        let target = match &command {
            Some(Command::Tunnel(_)) => HelpTarget::Tunnel,
            Some(Command::Access(_)) => HelpTarget::Access,
            _ => HelpTarget::Root,
        };
        return Cli {
            command: Command::Help(target),
            flags,
        };
    }

    if version_requested {
        return Cli {
            command: Command::Version { short: short_version },
            flags,
        };
    }

    // --short / -s without --version is still version output.
    if short_version {
        return Cli {
            command: Command::Version { short: true },
            flags,
        };
    }

    let command = match command {
        Some(cmd) => cmd,
        None => {
            if any_flag_set || !flags.rest_args.is_empty() {
                // Flags present but no command word — implicit tunnel mode.
                // Go baseline: root action delegates to tunnel.TunnelCommand(c).
                Command::Tunnel(TunnelSubcommand::Bare)
            } else {
                // Truly empty invocation — service mode.
                // Go baseline: handleServiceMode() in main.go.
                Command::ServiceMode
            }
        }
    };

    Cli { command, flags }
}

fn parse_equals_flag<'a>(arg: &'a OsStr, name: &str) -> Option<&'a str> {
    let arg = arg.to_str()?;
    arg.strip_prefix(name)?.strip_prefix('=')
}

fn parse_u32(value: &str, flag_name: &str) -> Result<u32, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value for {flag_name}: {value}"))
}

fn parse_u64(value: &str, flag_name: &str) -> Result<u64, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value for {flag_name}: {value}"))
}

fn set_path_flag(slot: &mut Option<PathBuf>, value: String, flag_name: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(surface_contract::repeated_flag_message(flag_name));
    }

    *slot = Some(PathBuf::from(value));
    Ok(())
}

fn set_command(slot: &mut Option<Command>, command: Command) -> Result<(), String> {
    if let Some(existing) = slot
        && *existing != command
    {
        return Err(surface_contract::multiple_commands_message(existing, &command));
    }

    *slot = Some(command);
    Ok(())
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
