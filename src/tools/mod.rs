//! Tool implementations and shared input/output types.

/// Utility tool helpers (config and shell integration).
pub mod utility;
/// Worktree lifecycle tool helpers.
pub mod worktree;

pub use utility::{
    GetWorktreePathInput, InitConfigInput, InitConfigOutput, ShellHookInput, ShellInitInput,
    ShellScriptOutput, WorktreePathOutput,
};
pub use utility::{get_worktree_path, init_config, shell_hook, shell_init};

pub use worktree::{
    AddWorktreeInput, AddWorktreeOutput, ListWorktreesInput, ListWorktreesOutput,
    RemoveWorktreeInput, RemoveWorktreeOutput,
};
pub use worktree::{add_worktree, list_worktrees, remove_worktree};
