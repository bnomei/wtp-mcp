//! Configuration types and loaders for wtp-mcp.

use crate::errors::WtpMcpError;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Top-level configuration loaded from TOML and CLI overrides.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    /// Repository root to operate in; defaults to the current directory when unset.
    pub repo_root: Option<PathBuf>,
    /// Settings for resolving the `wtp` binary.
    #[serde(default)]
    pub wtp: WtpConfig,
    /// Security policy flags for hooks and branch deletion.
    #[serde(default)]
    pub security: SecurityPolicy,
}

/// Configuration for resolving the `wtp` binary.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WtpConfig {
    /// Optional explicit path to the `wtp` binary.
    pub path: Option<PathBuf>,
}

/// Security policy controlling hooks and branch deletion.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct SecurityPolicy {
    /// Whether `.wtp.yml` hooks are allowed to run.
    pub allow_hooks: bool,
    /// Whether branch deletion is allowed.
    pub allow_branch_delete: bool,
}

impl Default for WtpConfig {
    fn default() -> Self {
        Self {
            path: None,
        }
    }
}

/// CLI overrides applied on top of TOML configuration.
pub struct CliArgs {
    /// Optional repo root override.
    pub repo_root: Option<PathBuf>,
    /// Optional explicit `wtp` binary path override.
    pub wtp_path: Option<PathBuf>,
}

impl Config {
    /// Load configuration from a TOML file or return defaults if not provided.
    pub fn load(path: Option<&Path>) -> Result<Config, WtpMcpError> {
        match path {
            Some(p) => {
                let contents = std::fs::read_to_string(p).map_err(|e| WtpMcpError::ConfigRead {
                    path: p.to_path_buf(),
                    source: e,
                })?;
                toml::from_str(&contents).map_err(|e| WtpMcpError::ConfigParse {
                    path: p.to_path_buf(),
                    source: e,
                })
            }
            None => Ok(Config::default()),
        }
    }

    /// Merge CLI overrides into the loaded configuration.
    pub fn merge_cli(&mut self, cli: &CliArgs) {
        if cli.repo_root.is_some() {
            self.repo_root = cli.repo_root.clone();
        }
        if cli.wtp_path.is_some() {
            self.wtp.path = cli.wtp_path.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn default_config() -> Config {
        Config::default()
    }

    #[rstest]
    fn test_default_values(default_config: Config) {
        assert!(default_config.repo_root.is_none());
        assert!(!default_config.security.allow_hooks);
        assert!(!default_config.security.allow_branch_delete);
    }

    #[rstest]
    #[case(Some("/tmp/repo"), None, Some("/tmp/repo"), None)]
    #[case(
        None,
        Some("/usr/local/bin/wtp"),
        None,
        Some("/usr/local/bin/wtp"),
    )]
    fn test_merge_cli(
        mut default_config: Config,
        #[case] repo_root: Option<&str>,
        #[case] wtp_path: Option<&str>,
        #[case] expected_repo: Option<&str>,
        #[case] expected_wtp: Option<&str>,
    ) {
        let cli = CliArgs {
            repo_root: repo_root.map(PathBuf::from),
            wtp_path: wtp_path.map(PathBuf::from),
        };
        default_config.merge_cli(&cli);
        assert_eq!(default_config.repo_root, expected_repo.map(PathBuf::from));
        assert_eq!(default_config.wtp.path, expected_wtp.map(PathBuf::from));
    }

    #[rstest]
    fn test_load_missing_file_returns_default() {
        let config = Config::load(None).unwrap();
        assert!(config.repo_root.is_none());
    }

    #[rstest]
    fn test_deserialize_partial_toml() {
        let toml_str = r#"
repo_root = "/my/repo"

[wtp]
path = "/usr/local/bin/wtp"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.repo_root, Some(PathBuf::from("/my/repo")));
        assert_eq!(config.wtp.path, Some(PathBuf::from("/usr/local/bin/wtp")));
    }
}
