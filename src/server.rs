//! MCP server implementation for wtp-mcp-rs.
//!
//! This module registers all tools and resources using the rmcp crate.

use std::path::PathBuf;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{
    Annotated, ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParams,
    RawResource, RawResourceTemplate, ReadResourceRequestParams, ReadResourceResult,
    ResourceContents, ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData as McpError, ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::config::{Config, SecurityPolicy, WtpConfig};
use crate::resources;
use crate::security::PolicyGuard;
use crate::tools::{
    AddWorktreeInput, AddWorktreeOutput, GetWorktreePathInput, InitConfigInput, InitConfigOutput,
    ListWorktreesInput, ListWorktreesOutput, MergeWorktreeInput, MergeWorktreeOutput,
    RemoveWorktreeInput, RemoveWorktreeOutput, ShellHookInput, ShellInitInput, ShellScriptOutput,
    WorktreePathOutput,
};
use crate::wtp::{WtpBinary, WtpRunner};

// ============================================================================
// Tool Input Schemas
// ============================================================================

/// Parameters for the `add-worktree` tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    description = "Parameters for add-worktree. Creates an isolated worktree folder for a branch. Intended flow: init-config, inspect/configure .wtp.yml (defaults.base_dir and optional hooks), then add-worktree. Provide exactly one of 'branch' or 'new_branch'. After creation, consider asking the user if they want to set the workdir to the new worktree."
)]
pub struct AddWorktreeParams {
    /// Existing branch name to check out in a new worktree. Mutually exclusive with `new_branch`.
    #[schemars(
        description = "Existing branch name to check out in a new worktree. Mutually exclusive with 'new_branch'."
    )]
    pub branch: Option<String>,
    /// New branch name to create and check out in a new worktree. Mutually exclusive with `branch`.
    #[schemars(
        description = "New branch name to create and check out in a new worktree. Mutually exclusive with 'branch'."
    )]
    pub new_branch: Option<String>,
    /// Base ref (branch, tag, or commit) to create `new_branch` from.
    #[schemars(
        description = "Base ref (branch, tag, or commit) to create 'new_branch' from. Optional."
    )]
    pub from: Option<String>,
}

/// Parameters for the `remove-worktree` tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    description = "Parameters for remove-worktree. Removes an isolated worktree folder by name (use list-worktrees to discover names). If .wtp.yml defines hooks, allow_hooks must be true."
)]
pub struct RemoveWorktreeParams {
    /// Worktree selector as returned by list-worktrees (typically a branch name).
    #[schemars(
        description = "Worktree selector as returned by list-worktrees (typically a branch name)."
    )]
    pub name: String,
    /// If true, remove the worktree even when there are uncommitted changes.
    #[schemars(
        description = "If true, remove the worktree even when there are uncommitted changes."
    )]
    pub force: Option<bool>,
    /// If true, delete the associated branch (requires allow_branch_delete=true).
    #[schemars(
        description = "If true, delete the associated branch as well (requires allow_branch_delete=true)."
    )]
    pub with_branch: Option<bool>,
    /// If true, delete the branch even if unmerged (requires allow_branch_delete=true).
    #[schemars(
        description = "If true, delete the branch even if unmerged (requires allow_branch_delete=true)."
    )]
    pub force_branch: Option<bool>,
}

/// Parameters for the `merge-worktree` tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    description = "Parameters for merge-worktree. Resolves the worktree branch and returns a git merge command."
)]
pub struct MergeWorktreeParams {
    /// Worktree selector as returned by list-worktrees (typically a branch name).
    #[schemars(
        description = "Worktree selector as returned by list-worktrees (typically a branch name)."
    )]
    pub name: String,
}

/// Parameters for the `get-worktree-path` tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    description = "Parameters for get-worktree-path. Resolves an absolute path for a worktree selector."
)]
pub struct GetWorktreePathParams {
    /// Optional worktree selector. Omit to return the main worktree path.
    #[schemars(description = "Optional worktree selector. Omit to return the main worktree path.")]
    pub name: Option<String>,
}

/// Parameters for shell integration tools (`shell-hook` and `shell-init`).
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    description = "Parameters for shell-hook and shell-init. These return wtp shell integration scripts."
)]
pub struct ShellParams {
    /// Shell type: bash, zsh, or fish.
    #[schemars(description = "Shell type: bash, zsh, or fish.")]
    pub shell: String,
}

// (ensure-wtp / update-wtp tools removed: no auto-download or update checks)

// ============================================================================
// Server Implementation
// ============================================================================

/// MCP server that exposes `wtp` commands as tools and resources.
#[derive(Clone)]
pub struct WtpServer {
    /// Repository root path the server operates against.
    pub repo_root: PathBuf,
    /// Path to the `wtp` binary used for commands.
    pub wtp_path: PathBuf,
    /// Resolved `wtp` configuration for re-resolving the binary.
    pub wtp_config: WtpConfig,
    /// Security policy flags for hooks and branch deletion.
    pub security: SecurityPolicy,
    tool_router: ToolRouter<Self>,
}

impl WtpServer {
    /// Create a new server from a loaded config.
    pub fn new(config: Config) -> Self {
        let repo_root = config
            .repo_root
            .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

        let wtp_config = config.wtp.clone();
        let wtp_path = WtpBinary::resolve(&wtp_config)
            .map(|b| b.path)
            .unwrap_or_else(|_| PathBuf::from("wtp"));

        Self {
            repo_root,
            wtp_path,
            wtp_config,
            security: config.security,
            tool_router: Self::tool_router(),
        }
    }

    fn get_runner(&self) -> WtpRunner {
        // Re-resolve the wtp path in case config/path changes; otherwise fall back.
        let resolved_path = WtpBinary::resolve(&self.wtp_config)
            .map(|b| b.path)
            .unwrap_or_else(|_| self.wtp_path.clone());
        WtpRunner::new(resolved_path, self.repo_root.clone())
    }
}

#[tool_router]
impl WtpServer {
    #[tool(
        name = "list-worktrees",
        description = "List worktrees (isolated folders per branch) in this repo so an agent can pick a target directory. Paths are returned as printed by wtp and may be relative or abbreviated; use get-worktree-path to resolve absolute paths. Returns JSON { worktrees: [ { name, path, branch, head, is_main } ] }.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_worktrees(&self) -> Result<Json<ListWorktreesOutput>, McpError> {
        let runner = self.get_runner();
        let output = crate::tools::list_worktrees(&runner, ListWorktreesInput {})
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Json(output))
    }

    #[tool(
        name = "add-worktree",
        description = "Create a new isolated worktree folder for a branch. Provide 'branch' for an existing branch or 'new_branch' (+ optional 'from') to create one. Intended flow: run init-config, inspect and configure .wtp.yml (defaults.base_dir and optional hooks), then call add-worktree. If .wtp.yml is missing, this tool creates a minimal one with defaults.base_dir=.worktrees and no hooks. Hooks are optional post-create actions that can copy files (e.g., .env), symlink shared dirs, or run setup commands; when configured, wtp runs them after add-worktree completes. They improve new worktree readiness but are blocked unless allow_hooks=true. Consider asking the user if they want to set the workdir to the new worktree. Returns JSON { name, path, branch, hint }.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn add_worktree(
        &self,
        params: Parameters<AddWorktreeParams>,
    ) -> Result<Json<AddWorktreeOutput>, McpError> {
        let runner = self.get_runner();
        let policy = PolicyGuard::from_config(&self.security);
        policy
            .check_hooks(runner.repo_root())
            .map_err(|e| McpError::invalid_request(e.to_string(), None))?;
        let input = AddWorktreeInput {
            branch: params.0.branch,
            new_branch: params.0.new_branch,
            from: params.0.from,
        };
        let output = crate::tools::add_worktree(&runner, input, &self.security)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Json(output))
    }

    #[tool(
        name = "remove-worktree",
        description = "Remove a worktree folder by name (from list-worktrees). Use 'force' to discard uncommitted changes. Use 'with_branch'/'force_branch' to delete the branch (requires allow_branch_delete=true). If hooks are configured for removal, wtp runs them during remove-worktree for cleanup, but they are blocked unless allow_hooks=true. Returns JSON { removed, branch_deleted }.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn remove_worktree(
        &self,
        params: Parameters<RemoveWorktreeParams>,
    ) -> Result<Json<RemoveWorktreeOutput>, McpError> {
        let runner = self.get_runner();
        let policy = PolicyGuard::from_config(&self.security);
        policy
            .check_branch_delete(
                params.0.with_branch.unwrap_or(false) || params.0.force_branch.unwrap_or(false),
            )
            .map_err(|e| McpError::invalid_request(e.to_string(), None))?;
        policy
            .check_hooks(runner.repo_root())
            .map_err(|e| McpError::invalid_request(e.to_string(), None))?;
        let input = RemoveWorktreeInput {
            name: params.0.name,
            force: params.0.force,
            with_branch: params.0.with_branch,
            force_branch: params.0.force_branch,
        };
        let output = crate::tools::remove_worktree(&runner, input, &self.security)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Json(output))
    }

    #[tool(
        name = "merge-worktree",
        description = "Return the git merge command for a worktree branch. Provide a worktree selector; run the returned command from the target branch worktree (usually main). Returns JSON { branch, command, hint }.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn merge_worktree(
        &self,
        params: Parameters<MergeWorktreeParams>,
    ) -> Result<Json<MergeWorktreeOutput>, McpError> {
        let runner = self.get_runner();
        let input = MergeWorktreeInput {
            name: params.0.name,
        };
        let output = crate::tools::merge_worktree(&runner, input)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Json(output))
    }

    #[tool(
        name = "init-config",
        description = "Initialize wtp configuration (.wtp.yml) in the repo by running wtp init. The generated file includes example hooks (automations that can copy files, link shared dirs, or run setup commands) and a default base_dir (often ../worktrees). Hooks in .wtp.yml run after add-worktree (and during remove-worktree if configured). Inspect/edit the file to set defaults.base_dir and remove hooks (or enable allow_hooks) before add-worktree. Returns JSON { path }.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn init_config(&self) -> Result<Json<InitConfigOutput>, McpError> {
        let runner = self.get_runner();
        let output = crate::tools::utility::init_config(&runner, InitConfigInput {})
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Json(output))
    }

    #[tool(
        name = "get-worktree-path",
        description = "Resolve the absolute path to a worktree by name (omit name for main worktree). Use this instead of guessing paths; wtp manages layout based on its config. Returns JSON { path }.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_worktree_path(
        &self,
        params: Parameters<GetWorktreePathParams>,
    ) -> Result<Json<WorktreePathOutput>, McpError> {
        let runner = self.get_runner();
        let input = GetWorktreePathInput {
            name: params.0.name,
        };
        let output = crate::tools::utility::get_worktree_path(&runner, input)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Json(output))
    }

    #[tool(
        name = "shell-hook",
        description = "Get the wtp shell hook script for bash/zsh/fish. This enables wtp's shell integration niceties (as provided by wtp) when installed in a user's shell config. Returns JSON { shell, script }.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn shell_hook(
        &self,
        params: Parameters<ShellParams>,
    ) -> Result<Json<ShellScriptOutput>, McpError> {
        let runner = self.get_runner();
        let input = ShellHookInput {
            shell: params.0.shell,
        };
        let output = crate::tools::utility::shell_hook(&runner, input)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Json(output))
    }

    #[tool(
        name = "shell-init",
        description = "Get the wtp shell init script for bash/zsh/fish. This provides shell functions/aliases used by wtp; optional for interactive convenience. Returns JSON { shell, script }.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn shell_init(
        &self,
        params: Parameters<ShellParams>,
    ) -> Result<Json<ShellScriptOutput>, McpError> {
        let runner = self.get_runner();
        let input = ShellInitInput {
            shell: params.0.shell,
        };
        let output = crate::tools::utility::shell_init(&runner, input)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Json(output))
    }
}

#[tool_handler]
impl ServerHandler for WtpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("WTP MCP Server - Worktree management via wtp CLI".into()),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            ..Default::default()
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        fn make_resource(uri: &str, name: &str, description: &str) -> Annotated<RawResource> {
            Annotated::new(
                RawResource {
                    uri: uri.into(),
                    name: name.into(),
                    description: Some(description.into()),
                    mime_type: Some("application/json".into()),
                    size: None,
                    title: None,
                    icons: None,
                    meta: None,
                },
                None,
            )
        }

        Ok(ListResourcesResult {
            resources: vec![
                make_resource(resources::URI_WORKTREES, "worktrees", "List all worktrees"),
                make_resource(
                    resources::URI_WORKTREES_RESOLVED,
                    "worktrees-resolved",
                    "List all worktrees with resolved absolute paths",
                ),
                make_resource(
                    resources::URI_OVERVIEW,
                    "overview",
                    "Repository overview including worktrees and security settings",
                ),
                make_resource(
                    resources::URI_WORKTREES_BY_BRANCH_PREFIX,
                    "worktrees-by-branch-prefix",
                    "Worktrees grouped by branch prefix",
                ),
            ],
            next_cursor: None,
            meta: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![Annotated::new(
                RawResourceTemplate {
                    uri_template: resources::URI_WORKTREE_TEMPLATE.into(),
                    name: "worktree".into(),
                    description: Some("Get a specific worktree by name".into()),
                    mime_type: Some("application/json".into()),
                    title: None,
                    icons: None,
                },
                None,
            )],
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let uri = request.uri.as_str();
        let runner = self.get_runner();

        let json_content = match uri {
            resources::URI_WORKTREES => {
                let worktrees = resources::get_worktrees(&runner)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&worktrees)
            }
            resources::URI_WORKTREES_RESOLVED => {
                let worktrees = resources::get_worktrees_resolved(&runner)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&worktrees)
            }
            resources::URI_OVERVIEW => {
                let overview = resources::get_overview(&runner, &self.security)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&overview)
            }
            resources::URI_WORKTREES_BY_BRANCH_PREFIX => {
                let grouped = resources::get_worktrees_by_branch_prefix(&runner)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&grouped)
            }
            _ if uri.starts_with("wtp://worktree/") => {
                let name = uri.strip_prefix("wtp://worktree/").unwrap_or("");
                let worktree = resources::get_worktree_by_name(&runner, name)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                match worktree {
                    Some(wt) => serde_json::to_string_pretty(&wt),
                    None => {
                        return Err(McpError::resource_not_found(
                            format!("Worktree '{}' not found", name),
                            None,
                        ));
                    }
                }
            }
            _ => {
                return Err(McpError::resource_not_found(
                    format!("Unknown resource URI: {}", uri),
                    None,
                ));
            }
        }
        .map_err(|e| McpError::internal_error(format!("JSON serialization failed: {}", e), None))?;

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(json_content, uri)],
        })
    }
}
