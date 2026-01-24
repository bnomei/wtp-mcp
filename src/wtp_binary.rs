use crate::config::WtpConfig;
use crate::errors::{Result, WtpMcpError};
use std::path::{Path, PathBuf};
use std::process::Command;

const BINARY_NAME: &str = if cfg!(windows) { "wtp.exe" } else { "wtp" };

/// Resolved `wtp` binary path and optional version metadata.
#[derive(Debug, Clone)]
pub struct WtpBinary {
    /// Filesystem path to the executable.
    pub path: PathBuf,
    /// Cached version string if known.
    pub version: Option<String>,
}

impl WtpBinary {
    /// Resolve the `wtp` binary from config, cache, or PATH.
    pub fn resolve(config: &WtpConfig) -> Result<WtpBinary> {
        // 1. Check config override
        if let Some(ref path) = config.path
            && is_executable(path)
        {
            return Ok(WtpBinary {
                path: path.clone(),
                version: None,
            });
        }

        // 2. Check cache directory
        if let Some(cache_path) = cache_dir() {
            let binary_path = cache_path.join(BINARY_NAME);
            if is_executable(&binary_path) {
                return Ok(WtpBinary {
                    path: binary_path,
                    version: None,
                });
            }
        }

        // 3. Check PATH
        if let Ok(path) = which::which(BINARY_NAME) {
            return Ok(WtpBinary {
                path,
                version: None,
            });
        }

        Err(WtpMcpError::BinaryNotFound {
            message: "wtp binary not found in config path, cache directory, or PATH".to_string(),
        })
    }

    /// Query the binary for its version via `wtp --version`.
    pub fn version(&self) -> Result<String> {
        let output = Command::new(&self.path)
            .arg("--version")
            .output()
            .map_err(|e| WtpMcpError::CommandFailed {
                exit_code: -1,
                message: format!("failed to execute wtp --version: {}", e),
                stderr: String::new(),
            })?;

        if !output.status.success() {
            return Err(WtpMcpError::CommandFailed {
                exit_code: output.status.code().unwrap_or(-1),
                message: "wtp --version failed".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_version(&stdout).ok_or_else(|| WtpMcpError::ParseError {
            message: "failed to parse version output".to_string(),
            raw_output: stdout.to_string(),
        })
    }
}

/// Return the cache directory used for downloaded binaries.
pub fn cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|p| p.join("wtp-mcp").join("bin"))
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = path.metadata() {
            return metadata.permissions().mode() & 0o111 != 0;
        }
        false
    }

    #[cfg(not(unix))]
    {
        path.exists()
    }
}

fn parse_version(output: &str) -> Option<String> {
    // Expects output like "wtp 1.2.3" or just "1.2.3"
    let trimmed = output.trim();
    if trimmed.starts_with("wtp ") {
        Some(trimmed.strip_prefix("wtp ")?.trim().to_string())
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_dir_returns_path() {
        let cache = cache_dir();
        assert!(cache.is_some());
        let path = cache.unwrap();
        assert!(path.ends_with("wtp-mcp/bin") || path.to_string_lossy().contains("wtp-mcp"));
    }

    #[test]
    fn test_parse_version_with_prefix() {
        assert_eq!(parse_version("wtp 1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(parse_version("wtp 1.2.3\n"), Some("1.2.3".to_string()));
    }

    #[test]
    fn test_parse_version_without_prefix() {
        assert_eq!(parse_version("1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(parse_version("  1.2.3  \n"), Some("1.2.3".to_string()));
    }

    #[test]
    fn test_resolve_with_invalid_config_path_falls_back() {
        // When config path is invalid, resolve should still find wtp on PATH (if available)
        // or in cache dir. This test verifies the fallback logic works.
        let config = WtpConfig {
            path: Some(PathBuf::from("/nonexistent/path/wtp")),
        };
        let result = WtpBinary::resolve(&config);
        // Result depends on whether wtp is on PATH or in cache - both are valid outcomes
        // The important thing is that we don't panic and the fallback logic runs
        if let Ok(binary) = result {
            // Found on PATH or in cache - config path was correctly skipped
            assert_ne!(binary.path, PathBuf::from("/nonexistent/path/wtp"));
        }
        // If Err, that's also valid - wtp simply isn't available
    }
}
