# wtp-mcp-rs

[![Crates.io Version](https://img.shields.io/crates/v/wtp-mcp-rs)](https://crates.io/crates/wtp-mcp-rs)
[![CI](https://img.shields.io/github/actions/workflow/status/bnomei/wtp-mcp/ci.yml?branch=main)](https://github.com/bnomei/wtp-mcp/actions/workflows/ci.yml)
[![Crates.io Downloads](https://img.shields.io/crates/d/wtp-mcp-rs)](https://crates.io/crates/wtp-mcp-rs)
[![License](https://img.shields.io/crates/l/wtp-mcp-rs)](https://crates.io/crates/wtp-mcp-rs)
[![Discord](https://flat.badgen.net/badge/discord/bnomei?color=7289da&icon=discord&label)](https://discordapp.com/users/bnomei)
[![Buymecoffee](https://flat.badgen.net/badge/icon/donate?icon=buymeacoffee&color=FF813F&label)](https://www.buymeacoffee.com/bnomei)

A Model Context Protocol (MCP) server for [wtp](https://github.com/satococoa/wtp) (Worktree Plus), written in Rust. It lets AI assistants manage Git worktrees via the wtp CLI.

**For**:
- Rust users who want `cargo install` and a native CLI.
- npm users who want `npm install -g` with prebuilt platform binaries.
- Git users who want safe, scriptable worktree automation.

Works in any Git repo. The wtp binary is required and must be on PATH (or set via `--wtp-path` / config).

## Highlights

- **Safety defaults**: Hooks and branch deletion are disabled by default and must be explicitly enabled.
- **Rich resources**: Query worktree lists, resolved paths, grouped views, and repo overview.

## Installation

### Cargo (crates.io)
```bash
cargo install wtp-mcp-rs
```

### Homebrew
```bash
brew install bnomei/wtp-mcp/wtp-mcp-rs
```

### npm
```bash
npm install -g @bnomei/wtp-mcp-rs
```

### npx (no install)
```bash
npx -y @bnomei/wtp-mcp-rs@latest --repo-root /path/to/repo
```

### GitHub Releases
Download a prebuilt archive from the GitHub Releases page, extract it, and place `wtp-mcp-rs` on your `PATH`.

### From source
```bash
git clone https://github.com/bnomei/wtp-mcp.git
cd wtp-mcp
cargo build --release
```

## Quick Start

1) Add this MCP configuration. Examples for common MCP clients (pick one):

```bash
# Claude Code
claude mcp add --transport stdio --scope project wtp -- wtp-mcp-rs

# Codex CLI
codex mcp add wtp -- wtp-mcp-rs

# OpenCode (interactive)
opencode mcp add

# Amp (workspace)
amp mcp add --workspace wtp -- wtp-mcp-rs
```

```json
{
  "mcpServers": {
    "wtp": {
      "command": "wtp-mcp-rs"
    }
  }
}
```

2) Run the MCP client from your repo root (or pass `--repo-root` / `--config` if you want to pin a different path). For Codex CLI, MCP config is global; use `codex --cd /path/to/repo` if you want to set the working root without changing directories.
3) Ensure `wtp` is installed and on `PATH` (or set `--wtp-path` / config).
4) Optional: run `init-config` once to generate `.wtp.yml`, then edit `defaults.base_dir` and hooks before using `add-worktree`.

## Usage

### MCP Configuration

Add the Quick Start snippet to your MCP client configuration. Example below includes all supported args (remove the ones you don't need):

```json
{
  "mcpServers": {
    "wtp": {
      "command": "wtp-mcp-rs",
      "args": [
        "--wtp-path",
        "/usr/local/bin/wtp",
        "--config",
        "/path/to/config.toml"
      ]
    }
  }
}
```

If your MCP client launches servers from the repository root (project/workspace scope), you can omit `--repo-root` because `wtp-mcp-rs` defaults to the current working directory.

### CLI Options

| Option | Description | Default |
|--------|-------------|---------|
| `--repo-root <path>` | Repository root directory | Current working directory |
| `--wtp-path <path>` | Override path to wtp binary | Auto-detect |
| `--config <path>` | Path to TOML configuration file | None |

### Worktree Configuration (.wtp.yml)

`wtp` controls worktree layout and hooks via `.wtp.yml` in the repo root.

- `init-config` runs `wtp init` to generate `.wtp.yml`. The default template usually sets `defaults.base_dir` to `../worktrees` and includes example hooks.
- Edit the file to choose your worktree base directory (e.g., `.worktrees` for in-repo worktrees).
- Hooks are optional post-create/remove actions (copy files, symlink dirs, run setup commands). They are **blocked by default** in this server unless `security.allow_hooks = true`.
- If `.wtp.yml` is missing, `add-worktree` creates a minimal config with `defaults.base_dir = .worktrees` and no hooks.

Minimal example:

```yaml
version: "1.0"
defaults:
  base_dir: .worktrees
```

### Merging changes from worktrees

Typical flow:
- Each worktree is a branch checkout in its own directory.
- Commit in that worktree.
- Merge into your target branch from any worktree (usually the "main" one):
  - `git merge feature/x`
  - or open a PR and merge on GitHub.

### Sample Configuration (config.toml)

```toml
# Repository root (optional, defaults to cwd)
repo_root = "/path/to/repo"

[wtp]
# Path to wtp binary (optional, auto-detected from PATH)
path = "/usr/local/bin/wtp"

[security]
# Allow execution of wtp hooks (default: false)
allow_hooks = false
# Allow branch deletion with worktree removal (default: false)
allow_branch_delete = false
```

### Security defaults (why destructive actions are blocked)

By default, potentially destructive operations are disabled:

- **Hook execution**: Disabled by default. Hooks can execute arbitrary code.
- **Branch deletion**: Disabled by default. The `--with-branch` and `--force-branch` flags on `remove-worktree` require explicit enablement.

To enable these features, add to your configuration file:

```toml
[security]
allow_hooks = true
allow_branch_delete = true
```

## Tools

- **list-worktrees** - List all worktrees in the repository
- **add-worktree** - Create a new worktree for an existing or new branch
- **remove-worktree** - Remove a worktree (optionally with its branch)
- **merge-worktree** - Return a git merge command for a worktree branch
- **init-config** - Initialize wtp configuration in the repository
- **get-worktree-path** - Get the absolute path to a worktree
- **shell-hook** - Get shell hook script (bash/zsh/fish)
- **shell-init** - Get shell initialization script (bash/zsh/fish)

## Resources

The server exposes the following MCP resources:

| URI | Description |
|-----|-------------|
| `wtp://worktrees` | List of all worktrees (raw) |
| `wtp://worktrees/resolved` | List of worktrees with resolved absolute paths |
| `wtp://worktree/{name}` | Details for a specific worktree |
| `wtp://overview` | Repository overview with worktrees and security settings |
| `wtp://worktrees/by-branch-prefix` | Worktrees grouped by branch prefix (feature/, bugfix/, etc.) |

## Development

### Running Tests

Unit tests (no wtp required):
```bash
cargo test --lib
```

Integration tests (requires wtp installed; tests skip if missing):
```bash
cargo test --test integration
```

## License

MIT License - see [LICENSE](LICENSE) for details.
