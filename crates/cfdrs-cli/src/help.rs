use crate::surface_contract;

pub fn render_help(program_name: &str) -> String {
    surface_contract::render_help_text(program_name)
}
