# Repository Guidelines

## Project Structure & Module Organization
- `.github/workflows/`: CI and release workflows.
- `docs/`: release docs (`RELEASE.md`).
- `src/`: Rust source (server, CLI entry, config/security, wtp integration, tools).
- `src/tools/`: MCP tool implementations (`utility`, `worktree`, `wtp_management`).
- `tests/`: unit + integration tests (some invoke the `wtp` binary).
- `scripts/`: release and npm packaging helpers.
- `npm/base/`: npm wrapper package (launcher + optionalDependencies).
- `npm/platform/<os-arch>/`: platform packages containing the binary.
- `package.json`: npm workspace metadata (private).

## Build, Test, and Development Commands
- `cargo build`: build the Rust binary.
- `cargo build --release`: release build (binary in `target/release/wtp-mcp-rs`).
- `cargo test`: run unit and integration tests.
- `cargo test --test integration`: run integration tests (skips if `wtp` is not on PATH).
- `cargo fmt --all -- --check`: enforce Rust formatting (matches CI).
- `cargo clippy --all-targets --all-features -- -D warnings`: lint with Clippy (matches CI).
- `node scripts/check-version-sync.js`: verify Rust/npm version alignment (Cargo.toml vs `npm/base/package.json`).

## Coding Style & Naming Conventions
- Rust: format with `rustfmt`, lint with `clippy`; prefer explicit error handling and clear structs for tool inputs/outputs.
- Keep files ASCII where possible and follow existing naming patterns (kebab-case for binaries, snake_case for Rust modules).
- Use small, focused modules (e.g., `wtp_runner.rs`, `wtp_parser.rs`, `wtp_download.rs`, `wtp_binary.rs`).

## Testing Guidelines
- Unit tests live alongside modules (`src/...`) with `#[test]`.
- Integration tests live in `tests/` (e.g., `tests/integration.rs`, `tests/wtp_management.rs`).
- Name tests by behavior (`parse_list_compact`, `remove_denies_branch_delete`).
- Tests that invoke `wtp` should skip if it is unavailable on PATH.

## Commit & Pull Request Guidelines
- Git history is minimal (only an “Initial commit”), so no established convention yet.
- Use short, imperative commit subjects (e.g., “Add wtp list parser”).
- PRs should include a brief summary, test results, and any config/security impacts (hooks/branch deletion).

## Security & Configuration Tips
- `.wtp.yml` hooks can run commands; keep hook execution disabled by default unless explicitly allowed.
- Branch deletion is destructive; gate it behind `security.allow_branch_delete` and document the risk.
- Server behavior is configurable via `--config <path>` TOML (repo root, wtp path, update checks, security policy).
