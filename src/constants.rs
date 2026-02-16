pub const DEFAULT_BASE_URL: &str = "https://verilib.org";

// CLI binary name - could also get this from env!("CARGO_PKG_NAME")
pub const CLI_NAME: &str = env!("CARGO_PKG_NAME");

// Dynamic error message generators
pub fn auth_required_msg() -> String {
    format!("No API key found. Please run '{} auth' first", CLI_NAME)
}

pub fn init_required_msg() -> String {
    format!("Project not initialized. Please run '{} init <repo_id>' first", CLI_NAME)
}

// Docker configuration
pub const DEFAULT_DOCKER_IMAGE: &str = "verilib/probe-verus:1";
