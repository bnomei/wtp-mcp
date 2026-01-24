//! Policy enforcement for hooks and branch deletion.

use std::path::Path;

use serde::Deserialize;

use crate::config::SecurityPolicy;
use crate::errors::{Result, WtpMcpError};

/// Security policy guard used by tools to validate operations.
#[derive(Debug, Clone)]
pub struct PolicyGuard {
    /// Whether `.wtp.yml` hooks are allowed to run.
    pub allow_hooks: bool,
    /// Whether branch deletion is allowed.
    pub allow_branch_delete: bool,
}

#[derive(Debug, Deserialize)]
struct WtpYml {
    hooks: Option<serde_yaml::Value>,
}

impl PolicyGuard {
    /// Construct a guard from the loaded security policy.
    pub fn from_config(security: &SecurityPolicy) -> Self {
        Self {
            allow_hooks: security.allow_hooks,
            allow_branch_delete: security.allow_branch_delete,
        }
    }

    /// Validate that hooks are allowed (or not configured) for the repo.
    pub fn check_hooks(&self, repo_root: &Path) -> Result<()> {
        if self.allow_hooks {
            return Ok(());
        }

        let wtp_yml_path = repo_root.join(".wtp.yml");
        if !wtp_yml_path.exists() {
            return Ok(());
        }

        let contents = std::fs::read_to_string(&wtp_yml_path)?;
        let parsed: WtpYml =
            serde_yaml::from_str(&contents).map_err(|e| WtpMcpError::ParseError {
                message: format!("Failed to parse .wtp.yml: {}", e),
                raw_output: contents.clone(),
            })?;

        if parsed.hooks.is_some() {
            return Err(WtpMcpError::PolicyViolation {
                message:
                    "Hooks are configured in .wtp.yml but hooks are not allowed by security policy"
                        .to_string(),
            });
        }

        Ok(())
    }

    /// Validate that branch deletion is allowed when requested.
    pub fn check_branch_delete(&self, with_branch: bool) -> Result<()> {
        if !with_branch {
            return Ok(());
        }

        if self.allow_branch_delete {
            return Ok(());
        }

        Err(WtpMcpError::PolicyViolation {
            message: "Branch deletion is not allowed by security policy".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::fs;
    use tempfile::TempDir;

    #[fixture]
    fn default_security() -> SecurityPolicy {
        SecurityPolicy {
            allow_hooks: false,
            allow_branch_delete: false,
        }
    }

    #[fixture]
    fn restrictive_guard() -> PolicyGuard {
        PolicyGuard {
            allow_hooks: false,
            allow_branch_delete: false,
        }
    }

    #[fixture]
    fn permissive_guard() -> PolicyGuard {
        PolicyGuard {
            allow_hooks: true,
            allow_branch_delete: true,
        }
    }

    #[fixture]
    fn temp_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[rstest]
    fn from_config_creates_guard(default_security: SecurityPolicy) {
        let guard = PolicyGuard::from_config(&default_security);
        assert!(!guard.allow_hooks);
        assert!(!guard.allow_branch_delete);
    }

    #[rstest]
    fn from_config_permissive() {
        let security = SecurityPolicy {
            allow_hooks: true,
            allow_branch_delete: true,
        };
        let guard = PolicyGuard::from_config(&security);
        assert!(guard.allow_hooks);
        assert!(guard.allow_branch_delete);
    }

    #[rstest]
    fn check_hooks_allowed(permissive_guard: PolicyGuard, temp_dir: TempDir) {
        fs::write(
            temp_dir.path().join(".wtp.yml"),
            "hooks:\n  post_add: echo hi",
        )
        .unwrap();
        assert!(permissive_guard.check_hooks(temp_dir.path()).is_ok());
    }

    #[rstest]
    fn check_hooks_no_file(restrictive_guard: PolicyGuard, temp_dir: TempDir) {
        assert!(restrictive_guard.check_hooks(temp_dir.path()).is_ok());
    }

    #[rstest]
    #[case("worktree_base: ../worktrees", true)]
    #[case("hooks:\n  post_add: echo hi", false)]
    #[case("hooks:\n  pre_remove: cleanup.sh", false)]
    fn check_hooks_with_yml_content(
        restrictive_guard: PolicyGuard,
        temp_dir: TempDir,
        #[case] yml_content: &str,
        #[case] should_pass: bool,
    ) {
        fs::write(temp_dir.path().join(".wtp.yml"), yml_content).unwrap();
        let result = restrictive_guard.check_hooks(temp_dir.path());
        assert_eq!(result.is_ok(), should_pass);
        if !should_pass {
            assert!(matches!(
                result.unwrap_err(),
                WtpMcpError::PolicyViolation { .. }
            ));
        }
    }

    #[rstest]
    fn check_branch_delete_not_requested(restrictive_guard: PolicyGuard) {
        assert!(restrictive_guard.check_branch_delete(false).is_ok());
    }

    #[rstest]
    fn check_branch_delete_allowed(permissive_guard: PolicyGuard) {
        assert!(permissive_guard.check_branch_delete(true).is_ok());
    }

    #[rstest]
    fn check_branch_delete_blocked(restrictive_guard: PolicyGuard) {
        let result = restrictive_guard.check_branch_delete(true);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            WtpMcpError::PolicyViolation { .. }
        ));
    }
}
