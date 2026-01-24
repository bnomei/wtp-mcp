//! Parse structured data from `wtp` CLI output.

use anyhow::Result;

use crate::types::Worktree;

/// Parse `wtp list` output into a vector of Worktree structs.
///
/// Supports legacy whitespace layouts (`<path> <branch> <head>`) and the
/// modern table layout with a STATUS column.
///
/// Main worktree is marked with `@` as name or contains "(main worktree)".
///
/// Uses robust whitespace splitting:
/// - Split on whitespace
/// - Last token = head (commit hash)
/// - If a STATUS column is present, third-to-last = branch, else second-to-last
/// - Remainder = path/name
pub fn parse_list(output: &str) -> Result<Vec<Worktree>> {
    let mut worktrees = Vec::new();
    let has_status_column = detect_status_column(output);

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if is_header_line(line) || is_separator_line(line) {
            continue;
        }

        // Remove "(main worktree)" marker if present for cleaner parsing
        let clean_line = line.replace("(main worktree)", "").trim().to_string();

        // Split on whitespace
        let tokens: Vec<&str> = clean_line.split_whitespace().collect();

        if tokens.len() < 3 {
            // Not enough tokens for a valid line, skip
            continue;
        }

        let has_status = has_status_column && tokens.len() >= 4;
        let (branch_idx, head_idx) = if has_status {
            (tokens.len() - 3, tokens.len() - 1)
        } else {
            (tokens.len() - 2, tokens.len() - 1)
        };

        // Last token = head (commit hash)
        let head = tokens[head_idx].to_string();
        // Branch token position depends on status column presence
        let branch = tokens[branch_idx].to_string();
        // Remainder = path/name
        let path_tokens = &tokens[..branch_idx];
        let path = path_tokens.join(" ");
        let path = path.trim_end_matches('*').to_string();

        // Check for main worktree marker
        let is_main =
            line.contains("(main worktree)") || line.starts_with("@ ") || path.starts_with('@');

        // Use the branch as the selector when available; fall back to path/alias.
        let name = select_worktree_name(&path, &branch);

        worktrees.push(Worktree {
            name,
            path,
            branch,
            head,
            is_main,
        });
    }

    Ok(worktrees)
}

fn detect_status_column(output: &str) -> bool {
    output.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("PATH")
            && trimmed.contains("BRANCH")
            && trimmed.contains("HEAD")
            && trimmed.contains("STATUS")
    })
}

fn is_header_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("PATH") && trimmed.contains("BRANCH") && trimmed.contains("HEAD")
}

fn is_separator_line(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.chars().all(|c| c == '-' || c == ' ')
}

fn select_worktree_name(path: &str, branch: &str) -> String {
    let branch = branch.trim();
    if !branch.is_empty() && branch != "-" {
        return branch.to_string();
    }
    let path = path.trim();
    if path.starts_with('@') {
        return "@".to_string();
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::empty("", 0)]
    #[case::whitespace_only("   \n  \n   ", 0)]
    #[case::single_worktree("/path main abc123", 1)]
    #[case::multiple("/path1 main abc\n/path2 feat def", 2)]
    #[case::compact("/path/to/wt main abc123\n/path/to/feature feat def456", 2)]
    #[case::bare_names("project-main main abc123\nfeature-branch feature def456", 2)]
    fn parse_list_count(#[case] input: &str, #[case] expected_count: usize) {
        let worktrees = parse_list(input).unwrap();
        assert_eq!(worktrees.len(), expected_count);
    }

    #[rstest]
    #[case::at_symbol("@ main abc123", true)]
    #[case::marker("/path main abc (main worktree)", true)]
    #[case::regular("/path feat def", false)]
    fn main_worktree_detection(#[case] input: &str, #[case] expected_is_main: bool) {
        let worktrees = parse_list(input).unwrap();
        assert_eq!(worktrees[0].is_main, expected_is_main);
    }

    #[rstest]
    fn normal_output_multiple_worktrees() {
        let output = r#"
/Users/dev/project/main         main      abc1234
/Users/dev/project/feature-auth feature/auth def5678
/Users/dev/project/bugfix       bugfix/123   ghi9012
"#;
        let worktrees = parse_list(output).unwrap();

        assert_eq!(worktrees.len(), 3);

        assert_eq!(worktrees[0].name, "main");
        assert_eq!(worktrees[0].path, "/Users/dev/project/main");
        assert_eq!(worktrees[0].branch, "main");
        assert_eq!(worktrees[0].head, "abc1234");
        assert!(!worktrees[0].is_main);

        assert_eq!(worktrees[1].name, "feature/auth");
        assert_eq!(worktrees[1].path, "/Users/dev/project/feature-auth");
        assert_eq!(worktrees[1].branch, "feature/auth");
        assert_eq!(worktrees[1].head, "def5678");

        assert_eq!(worktrees[2].name, "bugfix/123");
        assert_eq!(worktrees[2].branch, "bugfix/123");
    }

    #[rstest]
    fn table_output_with_status_column() {
        let output = r#"
PATH                                 BRANCH             STATUS    HEAD
----                                 ------             ------    ----
@*                                   main               managed   abc1234
../../priva...ees/feature-abc        feature-abc        unmanaged def5678
"#;
        let worktrees = parse_list(output).unwrap();

        assert_eq!(worktrees.len(), 2);
        assert_eq!(worktrees[0].name, "main");
        assert_eq!(worktrees[0].path, "@");
        assert_eq!(worktrees[0].branch, "main");
        assert_eq!(worktrees[0].head, "abc1234");
        assert!(worktrees[0].is_main);

        assert_eq!(worktrees[1].name, "feature-abc");
        assert_eq!(worktrees[1].path, "../../priva...ees/feature-abc");
        assert_eq!(worktrees[1].branch, "feature-abc");
        assert_eq!(worktrees[1].head, "def5678");
        assert!(!worktrees[1].is_main);
    }
}
