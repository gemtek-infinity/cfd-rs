//! Command tree, help text, parsing, user-visible dispatch, shell-visible
//! errors, CLI-facing surface types, and exact command-surface parity.
//!
//! This crate owns the 32-row CLI parity surface: all user-visible command
//! behavior, help formatting, flag names and aliases, environment-variable
//! bindings, exit codes, and error text placement.

mod env_defaults;
mod error;
mod help;
mod output;
mod parse;
mod surface_contract;
mod types;

pub use self::error::CliError;
pub use self::help::render_help;
pub use self::output::CliOutput;
pub use self::parse::parse_args;
pub use self::surface_contract::{
    CLASSIC_TUNNEL_DEPRECATED_MSG, DB_CONNECT_REMOVED_MSG, PROGRAM_NAME, PROXY_DNS_REMOVED_LOG_MSG,
    PROXY_DNS_REMOVED_MSG, TUNNEL_CMD_ERROR_MSG, TUNNEL_RUN_HOSTNAME_WARNING_MSG,
    TUNNEL_RUN_IDENTITY_ERROR_MSG, TUNNEL_RUN_NARG_ERROR_MSG, TUNNEL_TOKEN_FILE_READ_ERROR_PREFIX,
    TUNNEL_TOKEN_INVALID_MSG, render_short_version, render_version_output, stub_not_implemented,
    tunnel_run_usage_error,
};
pub use self::types::{
    AccessSubcommand, Cli, Command, GlobalFlags, IngressSubcommand, IpRouteSubcommand, ManagementSubcommand,
    RouteSubcommand, ServiceAction, TailSubcommand, TunnelSubcommand, VnetSubcommand,
};
