use crate::subcommand_help;
use crate::surface_contract;
use crate::types::HelpTarget;

pub fn render_help(program_name: &str) -> String {
    surface_contract::render_help_text(program_name)
}

pub fn render_tunnel_help(program_name: &str) -> String {
    surface_contract::render_tunnel_help_text(program_name)
}

pub fn render_access_help(program_name: &str) -> String {
    surface_contract::render_access_help_text(program_name)
}

pub fn render_subcommand_help(target: &HelpTarget) -> String {
    subcommand_help::render_subcommand_help_text(target)
}
