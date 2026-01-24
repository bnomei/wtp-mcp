//! Public API surface for the wtp-mcp-rs library.

/// Configuration loading and CLI merge helpers.
pub mod config;
/// Error types and Result alias for the library.
pub mod errors;
/// Resource handlers that expose read-only repo/worktree views.
pub mod resources;
/// Security policy checks and guards.
pub mod security;
/// MCP server implementation and tool registration.
pub mod server;
/// Tool implementations and shared input/output types.
pub mod tools;
/// Shared data types used across tools and responses.
pub mod types;
/// Low-level wtp command wrappers and parsers.
pub mod wtp;
mod wtp_binary;
mod wtp_parser;
mod wtp_runner;

pub use server::WtpServer;
