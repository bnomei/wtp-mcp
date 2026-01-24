//! Shared data types used across tools, resources, and parsers.

use schemars::JsonSchema;
use serde::Serialize;

use crate::config::SecurityPolicy;

/// Worktree entry as reported by `wtp list`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(
    description = "Worktree entry as reported by `wtp list`. Each worktree represents an isolated folder tied to a branch."
)]
pub struct Worktree {
    /// Worktree selector accepted by `wtp` commands (typically a branch name).
    #[schemars(
        description = "Worktree selector accepted by wtp commands (typically a branch name)."
    )]
    pub name: String,
    /// Path as printed by `wtp` (may be relative or abbreviated).
    #[schemars(description = "Path as printed by wtp (may be relative or abbreviated).")]
    pub path: String,
    /// Git branch checked out in this worktree.
    #[schemars(description = "Git branch checked out in this worktree.")]
    pub branch: String,
    /// HEAD commit hash for this worktree.
    #[schemars(description = "HEAD commit hash for this worktree.")]
    pub head: String,
    /// True if this is the main worktree.
    #[schemars(description = "True if this is the main worktree.")]
    pub is_main: bool,
}

/// Worktree entry with a resolved absolute path.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Worktree entry with resolved absolute path for direct navigation.")]
pub struct ResolvedWorktree {
    /// Worktree selector accepted by `wtp` commands (typically a branch name).
    #[schemars(
        description = "Worktree selector accepted by wtp commands (typically a branch name)."
    )]
    pub name: String,
    /// Path as printed by `wtp` (may be relative or abbreviated).
    #[schemars(description = "Path as printed by wtp (may be relative or abbreviated).")]
    pub path: String,
    /// Git branch checked out in this worktree.
    #[schemars(description = "Git branch checked out in this worktree.")]
    pub branch: String,
    /// HEAD commit hash for this worktree.
    #[schemars(description = "HEAD commit hash for this worktree.")]
    pub head: String,
    /// True if this is the main worktree.
    #[schemars(description = "True if this is the main worktree.")]
    pub is_main: bool,
    /// Absolute filesystem path for this worktree.
    #[schemars(description = "Absolute filesystem path for this worktree.")]
    pub absolute_path: String,
}

/// Summary of security policy flags exposed via resources.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SecuritySummary {
    /// True when hooks are allowed by policy.
    pub allow_hooks: bool,
    /// True when branch deletion is allowed by policy.
    pub allow_branch_delete: bool,
}

/// Repository overview resource.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Overview {
    /// Absolute path of the main worktree, if available.
    pub repo_root: String,
    /// `wtp` version string if resolved.
    pub wtp_version: Option<String>,
    /// Resolved worktree inventory.
    pub worktrees: Vec<ResolvedWorktree>,
    /// Security policy summary for this repo.
    pub security: SecuritySummary,
}

impl From<&SecurityPolicy> for SecuritySummary {
    fn from(policy: &SecurityPolicy) -> Self {
        Self {
            allow_hooks: policy.allow_hooks,
            allow_branch_delete: policy.allow_branch_delete,
        }
    }
}

impl From<Worktree> for ResolvedWorktree {
    fn from(wt: Worktree) -> Self {
        Self {
            name: wt.name,
            absolute_path: wt.path.clone(),
            path: wt.path,
            branch: wt.branch,
            head: wt.head,
            is_main: wt.is_main,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_summary_from_policy() {
        let policy = SecurityPolicy {
            allow_hooks: true,
            allow_branch_delete: false,
        };
        let summary = SecuritySummary::from(&policy);
        assert!(summary.allow_hooks);
        assert!(!summary.allow_branch_delete);
    }

    #[test]
    fn resolved_worktree_from_worktree() {
        let worktree = Worktree {
            name: "wt".to_string(),
            path: "path".to_string(),
            branch: "main".to_string(),
            head: "abc123".to_string(),
            is_main: true,
        };

        let resolved = ResolvedWorktree::from(worktree);
        assert_eq!(resolved.name, "wt");
        assert_eq!(resolved.path, "path");
        assert_eq!(resolved.absolute_path, "path");
        assert!(resolved.is_main);
    }
}
