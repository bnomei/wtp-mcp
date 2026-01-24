//! Error types and conversions for MCP responses.

use rmcp::model::{Content, ErrorCode, ErrorData, IntoContents};
use serde_json::json;
use std::borrow::Cow;

/// Error type for all wtp-mcp operations.
#[derive(Debug, thiserror::Error)]
pub enum WtpMcpError {
    /// The `wtp` binary could not be located.
    #[error("wtp binary not found: {message}")]
    BinaryNotFound {
        /// Details about the lookup failure.
        message: String,
    },

    /// A `wtp` command exited with a non-zero status.
    #[error("wtp command failed (exit {exit_code}): {message}")]
    CommandFailed {
        /// Exit code returned by the command.
        exit_code: i32,
        /// Human-readable summary of the failure.
        message: String,
        /// Standard error captured from the command.
        stderr: String,
    },

    /// Failed to parse `wtp` output into structured data.
    #[error("failed to parse wtp output: {message}")]
    ParseError {
        /// Summary of the parse failure.
        message: String,
        /// Raw output that failed to parse.
        raw_output: String,
    },

    /// Operation blocked by security policy.
    #[error("policy violation: {message}")]
    PolicyViolation {
        /// Policy violation details.
        message: String,
    },

    /// Failed to download a required artifact (such as `wtp`).
    #[error("download failed: {message}")]
    DownloadFailed {
        /// Summary of the download failure.
        message: String,
    },

    /// Configuration is invalid or inconsistent.
    #[error("configuration error: {message}")]
    ConfigError {
        /// Summary of the configuration error.
        message: String,
    },

    /// Failed to read a config file from disk.
    #[error("failed to read config file at {path}: {source}")]
    ConfigRead {
        /// Path to the config file.
        path: std::path::PathBuf,
        /// Underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse a config file.
    #[error("failed to parse config file at {path}: {source}")]
    ConfigParse {
        /// Path to the config file.
        path: std::path::PathBuf,
        /// Underlying TOML parse error.
        #[source]
        source: toml::de::Error,
    },

    /// Unhandled IO error from filesystem or process operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<WtpMcpError> for ErrorData {
    fn from(err: WtpMcpError) -> Self {
        let code = match &err {
            WtpMcpError::BinaryNotFound { .. } => ErrorCode::INTERNAL_ERROR,
            WtpMcpError::CommandFailed { .. } => ErrorCode::INTERNAL_ERROR,
            WtpMcpError::ParseError { .. } => ErrorCode::PARSE_ERROR,
            WtpMcpError::PolicyViolation { .. } => ErrorCode::INVALID_REQUEST,
            WtpMcpError::DownloadFailed { .. } => ErrorCode::INTERNAL_ERROR,
            WtpMcpError::ConfigError { .. } => ErrorCode::INVALID_PARAMS,
            WtpMcpError::ConfigRead { .. } => ErrorCode::INTERNAL_ERROR,
            WtpMcpError::ConfigParse { .. } => ErrorCode::INVALID_PARAMS,
            WtpMcpError::Io(_) => ErrorCode::INTERNAL_ERROR,
        };

        let data = match &err {
            WtpMcpError::CommandFailed {
                exit_code, stderr, ..
            } => Some(json!({
                "exit_code": exit_code,
                "stderr": stderr,
            })),
            WtpMcpError::ParseError { raw_output, .. } => Some(json!({
                "raw_output": raw_output,
            })),
            _ => None,
        };

        ErrorData {
            code,
            message: Cow::Owned(err.to_string()),
            data,
        }
    }
}

impl IntoContents for WtpMcpError {
    fn into_contents(self) -> Vec<Content> {
        vec![Content::text(self.to_string())]
    }
}

/// Convenience alias for `WtpMcpError`.
pub type Error = WtpMcpError;
/// Convenience result alias for wtp-mcp operations.
pub type Result<T> = std::result::Result<T, WtpMcpError>;
