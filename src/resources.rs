//! MCP resource helpers that aggregate `wtp` data.

use std::collections::HashMap;

use crate::config::SecurityPolicy;
use crate::errors::{Result, WtpMcpError};
use crate::types::{Overview, ResolvedWorktree, SecuritySummary, Worktree};
use crate::wtp::{WtpRunner, parse_list};

/// Fetch and parse worktrees using `wtp list`.
pub async fn get_worktrees(runner: &WtpRunner) -> Result<Vec<Worktree>> {
    let output = runner.run_checked(&["list"]).await?;
    parse_list(&output).map_err(|e| WtpMcpError::ParseError {
        message: e.to_string(),
        raw_output: output,
    })
}

/// Fetch worktrees and resolve absolute paths with `wtp cd`.
pub async fn get_worktrees_resolved(runner: &WtpRunner) -> Result<Vec<ResolvedWorktree>> {
    let worktrees = get_worktrees(runner).await?;
    let mut resolved = Vec::with_capacity(worktrees.len());

    for wt in worktrees {
        let absolute_path = if wt.is_main || wt.path.starts_with('/') {
            wt.path.clone()
        } else {
            let cd_output = runner.run(&["cd", &wt.name]).await?;
            if cd_output.exit_code == 0 {
                cd_output.stdout.trim().to_string()
            } else {
                wt.path.clone()
            }
        };

        resolved.push(ResolvedWorktree {
            name: wt.name,
            path: wt.path,
            branch: wt.branch,
            head: wt.head,
            is_main: wt.is_main,
            absolute_path,
        });
    }

    Ok(resolved)
}

/// Look up a resolved worktree by its name.
pub async fn get_worktree_by_name(
    runner: &WtpRunner,
    name: &str,
) -> Result<Option<ResolvedWorktree>> {
    let worktrees = get_worktrees_resolved(runner).await?;
    Ok(worktrees
        .into_iter()
        .find(|wt| wt.name == name || wt.branch == name || wt.path == name))
}

/// Build an overview snapshot including worktrees and security settings.
pub async fn get_overview(runner: &WtpRunner, security: &SecurityPolicy) -> Result<Overview> {
    let worktrees = get_worktrees_resolved(runner).await?;

    let wtp_version = runner.run(&["--version"]).await.ok().and_then(|output| {
        if output.exit_code == 0 {
            Some(output.stdout.trim().to_string())
        } else {
            None
        }
    });

    let repo_root = worktrees
        .iter()
        .find(|wt| wt.is_main)
        .map(|wt| wt.absolute_path.clone())
        .unwrap_or_default();

    Ok(Overview {
        repo_root,
        wtp_version,
        worktrees,
        security: SecuritySummary::from(security),
    })
}

/// Group worktrees by the prefix of their branch name.
pub async fn get_worktrees_by_branch_prefix(
    runner: &WtpRunner,
) -> Result<HashMap<String, Vec<Worktree>>> {
    let worktrees = get_worktrees(runner).await?;
    let mut grouped: HashMap<String, Vec<Worktree>> = HashMap::new();

    for wt in worktrees {
        let prefix = extract_branch_prefix(&wt.branch);
        grouped.entry(prefix).or_default().push(wt);
    }

    Ok(grouped)
}

fn extract_branch_prefix(branch: &str) -> String {
    branch.split('/').next().unwrap_or(branch).to_string()
}

/// Resource URI for the raw worktree list.
pub const URI_WORKTREES: &str = "wtp://worktrees";
/// Resource URI for resolved worktrees with absolute paths.
pub const URI_WORKTREES_RESOLVED: &str = "wtp://worktrees/resolved";
/// Resource URI template for a specific worktree.
pub const URI_WORKTREE_TEMPLATE: &str = "wtp://worktree/{name}";
/// Resource URI for the overview snapshot.
pub const URI_OVERVIEW: &str = "wtp://overview";
/// Resource URI for grouping by branch prefix.
pub const URI_WORKTREES_BY_BRANCH_PREFIX: &str = "wtp://worktrees/by-branch-prefix";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_branch_prefix() {
        assert_eq!(extract_branch_prefix("feature/auth"), "feature");
        assert_eq!(extract_branch_prefix("bugfix/123"), "bugfix");
        assert_eq!(extract_branch_prefix("main"), "main");
        assert_eq!(extract_branch_prefix("release/v1.0.0"), "release");
    }

    #[test]
    fn test_security_summary_from_policy() {
        let policy = SecurityPolicy {
            allow_hooks: true,
            allow_branch_delete: false,
        };
        let summary = SecuritySummary::from(&policy);
        assert!(summary.allow_hooks);
        assert!(!summary.allow_branch_delete);
    }
}
