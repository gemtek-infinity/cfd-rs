#[derive(Debug)]
pub(crate) struct CliOutput {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) exit_code: u8,
}

impl CliOutput {
    pub(crate) fn success(stdout: String) -> Self {
        Self {
            stdout,
            stderr: String::new(),
            exit_code: 0,
        }
    }

    pub(crate) fn failure(stdout: String, stderr: String, exit_code: u8) -> Self {
        Self {
            stdout,
            stderr,
            exit_code,
        }
    }

    pub(crate) fn usage_failure(stderr: String) -> Self {
        Self::failure(String::new(), stderr, 2)
    }
}
