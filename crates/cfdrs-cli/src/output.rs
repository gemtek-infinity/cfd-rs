#[derive(Debug)]
pub struct CliOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u8,
}

impl CliOutput {
    pub fn success(stdout: String) -> Self {
        Self {
            stdout,
            stderr: String::new(),
            exit_code: 0,
        }
    }

    pub fn failure(stdout: String, stderr: String, exit_code: u8) -> Self {
        Self {
            stdout,
            stderr,
            exit_code,
        }
    }

    pub fn usage_failure(stderr: String) -> Self {
        Self::failure(String::new(), stderr, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- CLI-030: usage failure behavior ---
    // Go: urfave/cli returns exit code 2 for usage errors (unknown commands, bad
    // flags). Errors go to stderr; stdout is empty on usage failures.

    #[test]
    fn usage_failure_exit_code_is_2() {
        let output = CliOutput::usage_failure("bad flag".into());
        assert_eq!(output.exit_code, 2);
    }

    #[test]
    fn usage_failure_stdout_is_empty() {
        let output = CliOutput::usage_failure("unknown command".into());
        assert!(output.stdout.is_empty());
    }

    #[test]
    fn usage_failure_stderr_contains_message() {
        let output = CliOutput::usage_failure("flag provided but not defined: --bad".into());
        assert!(output.stderr.contains("flag provided but not defined: --bad"));
    }

    #[test]
    fn success_exit_code_is_0() {
        let output = CliOutput::success("OK".into());
        assert_eq!(output.exit_code, 0);
    }

    #[test]
    fn config_error_exit_code_is_1() {
        let output = CliOutput::failure(String::new(), "config parse error".into(), 1);
        assert_eq!(output.exit_code, 1);
    }
}
