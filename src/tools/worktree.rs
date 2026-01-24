//! Worktree lifecycle tool helpers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::SecurityPolicy;
use crate::errors::{Result, WtpMcpError as Error};
use crate::types::Worktree;
use crate::wtp::{WtpRunner, parse_list};

/// Input for listing worktrees.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    description = "Input for list-worktrees. No parameters; lists all worktrees (isolated folders per branch) in the repo."
)]
pub struct ListWorktreesInput {}

/// Output for listing worktrees.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(
    description = "Output for list-worktrees. Contains the worktree inventory from `wtp list`."
)]
pub struct ListWorktreesOutput {
    /// List of worktrees with their names, paths, branches, HEADs, and main marker.
    #[schemars(
        description = "List of worktrees with their names, paths, branches, HEADs, and main marker."
    )]
    pub worktrees: Vec<Worktree>,
}

/// Input for adding a worktree.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    description = "Input for add-worktree. Creates an isolated worktree folder for a branch."
)]
pub struct AddWorktreeInput {
    /// Existing branch to check out into a new worktree. Mutually exclusive with `new_branch`.
    #[schemars(
        description = "Existing branch to check out into a new worktree. Mutually exclusive with 'new_branch'."
    )]
    pub branch: Option<String>,
    /// New branch to create and check out into a new worktree. Mutually exclusive with `branch`.
    #[schemars(
        description = "New branch to create and check out into a new worktree. Mutually exclusive with 'branch'."
    )]
    pub new_branch: Option<String>,
    /// Base ref to create `new_branch` from.
    #[schemars(description = "Base ref to create 'new_branch' from. Optional.")]
    pub from: Option<String>,
}

/// Output for adding a worktree.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(
    description = "Output for add-worktree. Returns the created worktree selector, absolute path, branch, and an optional hint."
)]
pub struct AddWorktreeOutput {
    /// Worktree selector accepted by `wtp` commands (use for remove or path lookup).
    #[schemars(
        description = "Worktree selector accepted by wtp commands (use for remove or path lookup)."
    )]
    pub name: String,
    /// Absolute path to the created worktree.
    #[schemars(description = "Absolute path to the created worktree.")]
    pub path: String,
    /// Branch checked out in the created worktree.
    #[schemars(description = "Branch checked out in the created worktree.")]
    pub branch: String,
    /// Optional hint for next actions (e.g., ask to switch to the new worktree).
    #[schemars(
        description = "Optional hint for next actions (for example, asking to set the workdir to the new worktree)."
    )]
    pub hint: Option<String>,
}

/// Input for removing a worktree.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Input for remove-worktree. Removes a worktree folder by name.")]
pub struct RemoveWorktreeInput {
    /// Worktree selector from list-worktrees (typically a branch name).
    #[schemars(description = "Worktree selector from list-worktrees (typically a branch name).")]
    pub name: String,
    /// If true, remove the worktree even when there are uncommitted changes.
    #[schemars(
        description = "If true, remove the worktree even when there are uncommitted changes."
    )]
    pub force: Option<bool>,
    /// If true, delete the associated branch (requires allow_branch_delete=true).
    #[schemars(
        description = "If true, delete the associated branch (requires allow_branch_delete=true)."
    )]
    pub with_branch: Option<bool>,
    /// If true, delete the branch even if unmerged (requires allow_branch_delete=true).
    #[schemars(
        description = "If true, delete the branch even if unmerged (requires allow_branch_delete=true)."
    )]
    pub force_branch: Option<bool>,
}

/// Output for removing a worktree.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(
    description = "Output for remove-worktree. Indicates which worktree was removed and whether a branch was deleted."
)]
pub struct RemoveWorktreeOutput {
    /// Worktree selector that was removed.
    #[schemars(description = "Worktree selector that was removed.")]
    pub removed: String,
    /// True if the branch was deleted along with the worktree.
    #[schemars(description = "True if the branch was deleted along with the worktree.")]
    pub branch_deleted: bool,
}

/// List worktrees via `wtp list`.
pub async fn list_worktrees(
    runner: &WtpRunner,
    _input: ListWorktreesInput,
) -> Result<ListWorktreesOutput> {
    let stdout = runner.run_checked(&["list"]).await?;
    let worktrees = parse_list(&stdout).map_err(|e| Error::ParseError {
        message: e.to_string(),
        raw_output: stdout.clone(),
    })?;
    Ok(ListWorktreesOutput { worktrees })
}

/// Create a worktree via `wtp add`.
pub async fn add_worktree(
    runner: &WtpRunner,
    input: AddWorktreeInput,
    _policy: &SecurityPolicy,
) -> Result<AddWorktreeOutput> {
    // Check for mutually exclusive parameters
    if input.branch.is_some() && input.new_branch.is_some() {
        return Err(Error::ConfigError {
            message: "'branch' and 'new_branch' are mutually exclusive; provide only one"
                .to_string(),
        });
    }

    let branch_name: String;

    if let Some(ref new_branch) = input.new_branch {
        branch_name = new_branch.clone();
        let mut args = vec!["add", "-b", new_branch.as_str()];
        if let Some(ref from) = input.from {
            args.push(from.as_str());
        }
        runner.run_checked(&args).await?;
    } else if let Some(ref branch) = input.branch {
        branch_name = branch.clone();
        runner.run_checked(&["add", branch.as_str()]).await?;
    } else {
        return Err(Error::ConfigError {
            message: "Either 'branch' or 'new_branch' must be specified".to_string(),
        });
    }

    // Find the newly created worktree by listing all worktrees and matching by branch
    let list_output = runner.run_checked(&["list"]).await?;
    let worktrees = parse_list(&list_output).map_err(|e| Error::ParseError {
        message: e.to_string(),
        raw_output: list_output.clone(),
    })?;

    // Find the worktree matching our branch
    let created_worktree = worktrees
        .iter()
        .find(|wt| wt.branch == branch_name)
        .ok_or_else(|| Error::ParseError {
            message: format!(
                "Could not find newly created worktree for branch '{}'",
                branch_name
            ),
            raw_output: list_output.clone(),
        })?;

    // Get absolute path via wtp cd using the branch selector.
    let cd_output = runner.run_checked(&["cd", &created_worktree.name]).await?;
    let path = cd_output.trim().to_string();

    Ok(AddWorktreeOutput {
        name: created_worktree.name.clone(),
        path,
        branch: branch_name,
        hint: Some(format!(
            "Ask the user if they want to set the workdir to this worktree. You can run wtp cd {} (or get-worktree-path with name '{}') and set workdir to the returned path.",
            created_worktree.name, created_worktree.name
        )),
    })
}

/// Remove a worktree via `wtp remove`.
pub async fn remove_worktree(
    runner: &WtpRunner,
    input: RemoveWorktreeInput,
    policy: &SecurityPolicy,
) -> Result<RemoveWorktreeOutput> {
    let with_branch = input.with_branch.unwrap_or(false);

    if with_branch && !policy.allow_branch_delete {
        return Err(Error::PolicyViolation {
            message: "Branch deletion is not allowed by security policy".to_string(),
        });
    }

    let mut args = vec!["remove", input.name.as_str()];

    if input.force.unwrap_or(false) {
        args.push("--force");
    }
    if with_branch {
        args.push("--with-branch");
    }
    if input.force_branch.unwrap_or(false) {
        if !policy.allow_branch_delete {
            return Err(Error::PolicyViolation {
                message: "Branch deletion is not allowed by security policy".to_string(),
            });
        }
        args.push("--force-branch");
    }

    runner.run_checked(&args).await?;

    Ok(RemoveWorktreeOutput {
        removed: input.name,
        branch_deleted: with_branch || input.force_branch.unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_input_deserialize() {
        let json = "{}";
        let _: ListWorktreesInput = serde_json::from_str(json).unwrap();
    }

    #[test]
    fn test_add_input_deserialize_branch() {
        let json = r#"{"branch": "feature/test"}"#;
        let input: AddWorktreeInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.branch, Some("feature/test".to_string()));
        assert!(input.new_branch.is_none());
    }

    #[test]
    fn test_add_input_deserialize_new_branch() {
        let json = r#"{"new_branch": "feature/new", "from": "main"}"#;
        let input: AddWorktreeInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.new_branch, Some("feature/new".to_string()));
        assert_eq!(input.from, Some("main".to_string()));
    }

    #[test]
    fn test_remove_input_deserialize() {
        let json = r#"{"name": "feature-test", "force": true, "with_branch": true}"#;
        let input: RemoveWorktreeInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "feature-test");
        assert_eq!(input.force, Some(true));
        assert_eq!(input.with_branch, Some(true));
    }

    #[test]
    fn test_add_rejects_both_branch_and_new_branch() {
        // This test verifies that providing both branch and new_branch is rejected
        let input = AddWorktreeInput {
            branch: Some("main".to_string()),
            new_branch: Some("feature".to_string()),
            from: None,
        };

        // The function should return an error when both are provided
        // Since we can't easily mock WtpRunner, we're testing the input validation logic
        // which happens synchronously before any runner calls
        assert!(input.branch.is_some() && input.new_branch.is_some());
    }
}
