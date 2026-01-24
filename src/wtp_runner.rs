use std::path::PathBuf;
use std::process::Command;

use crate::errors::{Result, WtpMcpError};

/// Captured output from a `wtp` command execution.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Standard output as UTF-8 string.
    pub stdout: String,
    /// Standard error as UTF-8 string.
    pub stderr: String,
    /// Exit code returned by the process.
    pub exit_code: i32,
}

/// Executes `wtp` commands with a configured repo root.
pub struct WtpRunner {
    binary_path: PathBuf,
    repo_root: PathBuf,
}

impl WtpRunner {
    /// Create a runner for a specific binary path and repo root.
    pub fn new(binary_path: PathBuf, repo_root: PathBuf) -> Self {
        Self {
            binary_path,
            repo_root,
        }
    }

    /// Return the configured repo root.
    pub fn repo_root(&self) -> &PathBuf {
        &self.repo_root
    }

    /// Run `wtp` with the given arguments and capture output.
    pub async fn run(&self, args: &[&str]) -> Result<CommandOutput> {
        tracing::debug!(
            binary = %self.binary_path.display(),
            cwd = %self.repo_root.display(),
            ?args,
            "executing wtp command"
        );

        let output = Command::new(&self.binary_path)
            .args(args)
            .current_dir(&self.repo_root)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        tracing::debug!(exit_code, "wtp command completed");

        Ok(CommandOutput {
            stdout,
            stderr,
            exit_code,
        })
    }

    /// Run `wtp` and return stdout, failing on non-zero exit.
    pub async fn run_checked(&self, args: &[&str]) -> Result<String> {
        let output = self.run(args).await?;

        if output.exit_code != 0 {
            return Err(WtpMcpError::CommandFailed {
                exit_code: output.exit_code,
                message: format!("wtp {:?} failed", args),
                stderr: output.stderr,
            });
        }

        Ok(output.stdout)
    }
}
