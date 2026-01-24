use rstest::{fixture, rstest};
use std::process::Command;
use tempfile::TempDir;

#[fixture]
fn test_repo() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    std::fs::write(repo_path.join("README.md"), "# Test Repo").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    temp_dir
}

fn wtp_available() -> bool {
    which::which("wtp").is_ok()
}

fn run_wtp(repo_path: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new("wtp")
        .args(args)
        .current_dir(repo_path)
        .output()
        .expect("Failed to run wtp")
}

#[rstest]
fn test_list_worktrees_empty(test_repo: TempDir) {
    if !wtp_available() {
        eprintln!("Skipping: wtp not found on PATH");
        return;
    }

    let output = run_wtp(test_repo.path(), &["list"]);

    assert!(output.status.success(), "wtp list failed: {:?}", output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("main") || stdout.contains("master"),
        "Expected main/master worktree in output: {}",
        stdout
    );
}

#[rstest]
fn test_add_and_remove_worktree(test_repo: TempDir) {
    if !wtp_available() {
        eprintln!("Skipping: wtp not found on PATH");
        return;
    }

    let repo_path = test_repo.path();

    let add_output = run_wtp(repo_path, &["add", "-b", "feature-test"]);
    assert!(
        add_output.status.success(),
        "wtp add failed: {:?}",
        add_output
    );

    let list_output = run_wtp(repo_path, &["list"]);
    assert!(list_output.status.success());
    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        list_stdout.contains("feature-test"),
        "Expected feature-test worktree in list: {}",
        list_stdout
    );

    let remove_output = run_wtp(repo_path, &["remove", "feature-test"]);
    assert!(
        remove_output.status.success(),
        "wtp remove failed: {:?}",
        remove_output
    );

    let final_list = run_wtp(repo_path, &["list"]);
    let final_stdout = String::from_utf8_lossy(&final_list.stdout);
    assert!(
        !final_stdout.contains("feature-test"),
        "feature-test should be removed: {}",
        final_stdout
    );
}

#[rstest]
fn test_get_worktree_path(test_repo: TempDir) {
    if !wtp_available() {
        eprintln!("Skipping: wtp not found on PATH");
        return;
    }

    let repo_path = test_repo.path();

    let output = run_wtp(repo_path, &["list"]);
    assert!(output.status.success(), "wtp list failed: {:?}", output);

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("@*") || stdout.contains("main") || stdout.contains("master"),
        "Expected main worktree marker (@*) or branch name in output: {}",
        stdout
    );
}
