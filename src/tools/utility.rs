//! Utility tool helpers for config and shell integration.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::errors::{Result, WtpMcpError};
use crate::wtp::WtpRunner;

/// Input for `init-config`.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    description = "Input for init-config. No parameters; creates a .wtp.yml in the repo root for worktree layout and optional hooks."
)]
pub struct InitConfigInput {}

/// Output for `init-config`.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Output for init-config. Path to the created .wtp.yml file.")]
pub struct InitConfigOutput {
    /// Absolute or relative path to the created config file.
    #[schemars(description = "Absolute or relative path to the created config file.")]
    pub path: String,
}

/// Input for `get-worktree-path`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetWorktreePathInput {
    /// Worktree selector from list-worktrees. Omit to return the main worktree path.
    #[schemars(
        description = "Worktree selector from list-worktrees. Omit to return the main worktree path."
    )]
    pub name: Option<String>,
}

/// Output for `get-worktree-path`.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Output for get-worktree-path. Absolute path to a worktree.")]
pub struct WorktreePathOutput {
    /// Absolute path to the resolved worktree.
    #[schemars(description = "Absolute path to the resolved worktree.")]
    pub path: String,
}

/// Input for `shell-hook`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShellHookInput {
    /// Shell type: bash, zsh, or fish.
    #[schemars(description = "Shell type: bash, zsh, or fish.")]
    pub shell: String,
}

/// Input for `shell-init`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShellInitInput {
    /// Shell type: bash, zsh, or fish.
    #[schemars(description = "Shell type: bash, zsh, or fish.")]
    pub shell: String,
}

/// Output for `shell-hook` or `shell-init`.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Output for shell-hook or shell-init. Script content returned by wtp.")]
pub struct ShellScriptOutput {
    /// Shell type the script targets.
    #[schemars(description = "Shell type the script targets.")]
    pub shell: String,
    /// Script content to install in the shell configuration.
    #[schemars(description = "Script content to install in the shell configuration.")]
    pub script: String,
}

const VALID_SHELLS: [&str; 3] = ["bash", "zsh", "fish"];

fn validate_shell(shell: &str) -> Result<()> {
    if VALID_SHELLS.contains(&shell) {
        Ok(())
    } else {
        Err(WtpMcpError::PolicyViolation {
            message: format!(
                "invalid shell '{}': must be one of {}",
                shell,
                VALID_SHELLS.join(", ")
            ),
        })
    }
}

/// Initialize wtp configuration in the repository.
pub async fn init_config(runner: &WtpRunner, _input: InitConfigInput) -> Result<InitConfigOutput> {
    let output = runner.run_checked(&["init"]).await?;
    // `wtp init` creates .wtp.yml in the repo root
    // Return the path from stdout or construct it
    Ok(InitConfigOutput {
        path: output.trim().to_string(),
    })
}

/// Get the absolute path to a worktree.
pub async fn get_worktree_path(
    runner: &WtpRunner,
    input: GetWorktreePathInput,
) -> Result<WorktreePathOutput> {
    let args: Vec<&str> = match &input.name {
        Some(name) => vec!["cd", name.as_str()],
        None => vec!["cd"],
    };
    let output = runner.run_checked(&args).await?;
    Ok(WorktreePathOutput {
        path: output.trim().to_string(),
    })
}

/// Get shell hook script for the specified shell.
pub async fn shell_hook(runner: &WtpRunner, input: ShellHookInput) -> Result<ShellScriptOutput> {
    validate_shell(&input.shell)?;
    let output = runner.run_checked(&["hook", &input.shell]).await?;
    Ok(ShellScriptOutput {
        shell: input.shell,
        script: output,
    })
}

/// Get shell initialization script for the specified shell.
pub async fn shell_init(runner: &WtpRunner, input: ShellInitInput) -> Result<ShellScriptOutput> {
    validate_shell(&input.shell)?;
    let output = runner.run_checked(&["shell-init", &input.shell]).await?;
    Ok(ShellScriptOutput {
        shell: input.shell,
        script: output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_shell_valid() {
        assert!(validate_shell("bash").is_ok());
        assert!(validate_shell("zsh").is_ok());
        assert!(validate_shell("fish").is_ok());
    }

    #[test]
    fn test_validate_shell_invalid() {
        assert!(validate_shell("sh").is_err());
        assert!(validate_shell("powershell").is_err());
        assert!(validate_shell("").is_err());
    }
}
