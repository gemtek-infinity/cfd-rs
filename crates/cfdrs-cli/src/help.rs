use crate::surface_contract;

pub fn render_help(program_name: &str) -> String {
    surface_contract::render_help_text(program_name)
}

pub fn render_tunnel_help(program_name: &str) -> String {
    surface_contract::render_tunnel_help_text(program_name)
}

pub fn render_access_help(program_name: &str) -> String {
    surface_contract::render_access_help_text(program_name)
}
