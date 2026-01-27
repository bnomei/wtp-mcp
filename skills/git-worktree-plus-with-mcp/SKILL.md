---
name: wtp-git-worktree-plus-with-mcp 
description: Use this skill when the user wants to manage Git worktrees through the wtp MCP server (list, create, remove, resolve paths, init .wtp.yml, or prepare merge commands), especially when safety defaults around hooks and branch deletion matter.
---

# WTP Worktree Management (MCP-First)

This skill manages Git worktrees by calling the wtp MCP tools instead of hand-rolled `git worktree` shell flows. Prefer the MCP tools and resources in this repo so path layout, policy checks, and wtp-specific behavior stay correct.

## When To Use

Use this skill when the user asks to:
- Create a worktree for an existing branch or a new branch.
- See what worktrees exist and pick one.
- Remove a worktree (with optional branch deletion).
- Resolve the actual path to a worktree before changing directories.
- Initialize or review `.wtp.yml`.
- Get a safe merge command for a worktree branch.
- Install shell integration for `wtp`.

## MCP Tools To Prefer

Always prefer these MCP tools over direct shell commands:
- `list-worktrees`: Discover valid worktree selectors and branches.
- `init-config`: Run once before heavy use, then review `.wtp.yml`.
- `add-worktree`: Create a worktree from an existing branch (`branch`) or create a new one (`new_branch`, optional `from`). Provide exactly one of `branch` or `new_branch`.
- `get-worktree-path`: Resolve the absolute path for a selector (or omit `name` for main). Do not guess paths.
- `remove-worktree`: Remove by selector from `list-worktrees`. Use `force` only when needed.
- `merge-worktree`: Produce the merge command for a worktree’s branch; run it from the target branch worktree.
- `shell-hook` and `shell-init`: Return scripts for shell integration (`bash`, `zsh`, or `fish`).

## MCP Resources To Use Opportunistically

If the client supports resources, use them to reduce tool calls:
- `wtp://overview`: Quick snapshot of worktrees plus security settings.
- `wtp://worktrees/resolved`: Worktree list with absolute paths.
- `wtp://worktrees`: Raw worktree list.
- `wtp://worktree/{name}`: Details for a single worktree.
- `wtp://worktrees/by-branch-prefix`: Grouped view by branch prefix.

## Default Workflow

Follow this order unless the user asks otherwise:

1. Inventory. Call `list-worktrees` first to get valid selectors and avoid guessing. If helpful, read `wtp://overview` to see both layout and security posture.
2. Configure (if needed). If `.wtp.yml` does not exist or the user wants a specific layout, run `init-config`. Afterward, review `.wtp.yml` and ask before enabling hooks or destructive options.
3. Create. For an existing branch, call `add-worktree` with `branch`. For a new branch, call `add-worktree` with `new_branch` and optional `from`. Immediately call `get-worktree-path` with the returned `name`, then ask whether to switch the working directory there.
4. Navigate. Use `get-worktree-path` to resolve the destination, then change the working directory.
5. Remove. Re-run `list-worktrees` to confirm the selector, then call `remove-worktree` with `name`. Only set `with_branch` or `force_branch` after checking policy and getting explicit confirmation.
6. Merge. Call `merge-worktree` with the selector, then run the returned command from the target branch worktree after user confirmation.

## Safety And Policy Guardrails

Treat these as hard rules:
- Hooks can execute arbitrary commands. Do not enable or rely on hooks unless the user explicitly opts in.
- Branch deletion is destructive. Only set `with_branch` or `force_branch` after explicit confirmation.
- If a tool call fails due to policy, explain why and point the user at their MCP config security settings rather than trying to bypass it.

## Tool Call Patterns (Examples)

Keep examples short and aligned with the actual tool inputs.

Create from an existing branch:

```json
{"branch":"feature/my-branch"}
```

Create a new branch from `main`:

```json
{"new_branch":"feature/my-branch","from":"main"}
```

Resolve a path:

```json
{"name":"feature/my-branch"}
```

Remove a worktree safely:

```json
{"name":"feature/my-branch"}
```

Force removal (only when necessary):

```json
{"name":"feature/my-branch","force":true}
```

Delete the branch too (only with explicit confirmation and allowed policy):

```json
{"name":"feature/my-branch","with_branch":true}
```

## Communication Checklist

Keep the user oriented and safe:
- Say which selector you are using and where it came from (`list-worktrees`).
- After `add-worktree`, report `name`, `branch`, and the resolved path.
- Before destructive actions, restate the exact effect and the branch/worktree involved.
