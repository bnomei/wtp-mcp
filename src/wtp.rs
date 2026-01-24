//! Low-level helpers for interacting with the `wtp` CLI.

/// Resolve cache directory and binary metadata for `wtp`.
pub use crate::wtp_binary::{WtpBinary, cache_dir};
/// Parse `wtp list` output into structured worktree data.
pub use crate::wtp_parser::parse_list;
/// Execute `wtp` commands and capture output.
pub use crate::wtp_runner::{CommandOutput, WtpRunner};
