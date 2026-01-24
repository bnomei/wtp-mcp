#[cfg(unix)]
mod stubbed {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use wtp_mcp_rs::config::SecurityPolicy;
    use wtp_mcp_rs::errors::WtpMcpError;
    use wtp_mcp_rs::resources;
    use wtp_mcp_rs::tools::{self, AddWorktreeInput, GetWorktreePathInput, RemoveWorktreeInput};
    use wtp_mcp_rs::wtp::WtpRunner;

    const LIST_OUTPUT: &str = "/repo/main main abc123 (main worktree)\n\
feature-branch feature/awesome def456\n\
/abs/bugfix bugfix/123 aaa111\n";

    struct StubWtp {
        _dir: TempDir,
        path: PathBuf,
        repo_root: PathBuf,
    }

    impl StubWtp {
        fn new(script: String) -> Self {
            let dir = TempDir::new().expect("temp dir");
            let repo_root = dir.path().join("repo");
            fs::create_dir_all(&repo_root).expect("repo dir");

            let path = dir.path().join("wtp");
            fs::write(&path, script).expect("write stub");

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&path).expect("metadata").permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&path, perms).expect("chmod");
            }

            Self {
                _dir: dir,
                path,
                repo_root,
            }
        }

        fn runner(&self) -> WtpRunner {
            WtpRunner::new(self.path.clone(), self.repo_root.clone())
        }
    }

    fn build_stub_script(list_output: &str, cd_default: &str, cd_map: &[(&str, &str)]) -> String {
        let mut script = String::from("#!/bin/sh\n");
        script.push_str("cmd=\"$1\"\n");
        script.push_str("if [ \"$#\" -gt 0 ]; then\n  shift\nfi\n");
        script.push_str("case \"$cmd\" in\n");

        script.push_str("list)\ncat <<'__WTP_LIST__'\n");
        script.push_str(list_output);
        if !list_output.ends_with('\n') {
            script.push('\n');
        }
        script.push_str("__WTP_LIST__\n;;\n");

        script.push_str("cd)\n");
        script.push_str("if [ -z \"$1\" ]; then\n");
        script.push_str(&format!("  echo \"{}\"\n  exit 0\nfi\n", cd_default));
        script.push_str("case \"$1\" in\n");
        for (name, path) in cd_map {
            script.push_str(&format!("  {} ) echo \"{}\";;\n", name, path));
        }
        script.push_str("  * ) exit 1;;\n");
        script.push_str("esac\n;;\n");

        script.push_str("--version)\n");
        script.push_str("echo \"wtp 1.2.3\"\n;;\n");

        script.push_str("hook)\n");
        script.push_str("echo \"hook-$1\"\n;;\n");

        script.push_str("shell-init)\n");
        script.push_str("echo \"init-$1\"\n;;\n");

        script.push_str("init)\n");
        script.push_str("echo \"/repo/.wtp.yml\"\n;;\n");

        script.push_str("add|remove)\nexit 0\n;;\n");

        script.push_str("fail)\n");
        script.push_str("echo \"fail\" >&2\nexit 2\n;;\n");

        script.push_str("*)\n");
        script.push_str("echo \"unknown command: $cmd\" >&2\nexit 1\n;;\n");
        script.push_str("esac\n");

        script
    }

    fn make_stub() -> StubWtp {
        let script = build_stub_script(
            LIST_OUTPUT,
            "/repo/main",
            &[
                ("feature-branch", "/repo/feature-branch"),
                ("feature/awesome", "/repo/feature-branch"),
            ],
        );
        StubWtp::new(script)
    }

    #[tokio::test]
    async fn list_worktrees_parses_stub_output() {
        let stub = make_stub();
        let output = tools::list_worktrees(&stub.runner(), tools::ListWorktreesInput {})
            .await
            .expect("list worktrees");

        assert_eq!(output.worktrees.len(), 3);
        assert_eq!(output.worktrees[1].branch, "feature/awesome");
    }

    #[tokio::test]
    async fn add_worktree_new_branch_resolves_path() {
        let stub = make_stub();
        let input = AddWorktreeInput {
            branch: None,
            new_branch: Some("feature/awesome".to_string()),
            from: Some("main".to_string()),
        };

        let output = tools::add_worktree(&stub.runner(), input, &SecurityPolicy::default())
            .await
            .expect("add worktree");

        assert_eq!(output.name, "feature/awesome");
        assert_eq!(output.path, "/repo/feature-branch");
        assert_eq!(output.branch, "feature/awesome");
    }

    #[tokio::test]
    async fn add_worktree_creates_default_wtp_config_when_missing() {
        let stub = make_stub();
        let input = AddWorktreeInput {
            branch: Some("feature/awesome".to_string()),
            new_branch: None,
            from: None,
        };

        let _ = tools::add_worktree(&stub.runner(), input, &SecurityPolicy::default())
            .await
            .expect("add worktree");

        let config_path = stub.repo_root.join(".wtp.yml");
        assert!(config_path.exists());
        let contents = fs::read_to_string(config_path).expect("read .wtp.yml");
        assert!(contents.contains("base_dir: .worktrees"));
    }

    #[tokio::test]
    async fn add_worktree_requires_branch_or_new_branch() {
        let stub = make_stub();
        let input = AddWorktreeInput {
            branch: None,
            new_branch: None,
            from: None,
        };

        let err = tools::add_worktree(&stub.runner(), input, &SecurityPolicy::default())
            .await
            .expect_err("expected error");

        assert!(matches!(err, WtpMcpError::ConfigError { .. }));
    }

    #[tokio::test]
    async fn remove_worktree_blocks_branch_delete_when_policy_forbids() {
        let stub = make_stub();
        let policy = SecurityPolicy {
            allow_hooks: false,
            allow_branch_delete: false,
        };
        let input = RemoveWorktreeInput {
            name: "feature/awesome".to_string(),
            force: None,
            with_branch: Some(true),
            force_branch: None,
        };

        let err = tools::remove_worktree(&stub.runner(), input, &policy)
            .await
            .expect_err("expected policy violation");

        assert!(matches!(err, WtpMcpError::PolicyViolation { .. }));
    }

    #[tokio::test]
    async fn get_worktree_path_uses_cd_output() {
        let stub = make_stub();
        let input = GetWorktreePathInput {
            name: Some("feature/awesome".to_string()),
        };

        let path = tools::get_worktree_path(&stub.runner(), input)
            .await
            .expect("get worktree path");

        assert_eq!(path.path, "/repo/feature-branch");
    }

    #[tokio::test]
    async fn shell_hook_and_init_return_stubbed_output() {
        let stub = make_stub();
        let hook = tools::shell_hook(
            &stub.runner(),
            tools::ShellHookInput {
                shell: "bash".to_string(),
            },
        )
        .await
        .expect("shell hook");

        let init = tools::shell_init(
            &stub.runner(),
            tools::ShellInitInput {
                shell: "bash".to_string(),
            },
        )
        .await
        .expect("shell init");

        assert!(hook.script.contains("hook-bash"));
        assert!(init.script.contains("init-bash"));
    }

    #[tokio::test]
    async fn init_config_returns_path() {
        let stub = make_stub();
        let output = tools::init_config(&stub.runner(), tools::InitConfigInput {})
            .await
            .expect("init config");

        assert_eq!(output.path, "/repo/.wtp.yml");
    }

    #[tokio::test]
    async fn runner_checked_propagates_nonzero_exit() {
        let stub = make_stub();
        let err = stub
            .runner()
            .run_checked(&["fail"])
            .await
            .expect_err("expected failure");

        match err {
            WtpMcpError::CommandFailed { exit_code, .. } => assert_eq!(exit_code, 2),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn resources_resolve_paths_and_overview() {
        let stub = make_stub();
        let runner = stub.runner();

        let resolved = resources::get_worktrees_resolved(&runner)
            .await
            .expect("resolved worktrees");

        let feature = resolved
            .iter()
            .find(|wt| wt.name == "feature/awesome")
            .expect("feature worktree");
        assert_eq!(feature.absolute_path, "/repo/feature-branch");

        let main = resolved.iter().find(|wt| wt.is_main).expect("main");
        assert_eq!(main.absolute_path, "/repo/main");

        let overview = resources::get_overview(
            &runner,
            &SecurityPolicy {
                allow_hooks: true,
                allow_branch_delete: false,
            },
        )
        .await
        .expect("overview");

        assert_eq!(overview.repo_root, "/repo/main");
        assert_eq!(overview.wtp_version.as_deref(), Some("wtp 1.2.3"));
        assert!(overview.security.allow_hooks);
        assert!(!overview.security.allow_branch_delete);
    }

    #[tokio::test]
    async fn resources_group_worktrees_by_branch_prefix() {
        let stub = make_stub();
        let grouped = resources::get_worktrees_by_branch_prefix(&stub.runner())
            .await
            .expect("grouped worktrees");

        assert_eq!(grouped.get("feature").map(Vec::len), Some(1));
        assert_eq!(grouped.get("bugfix").map(Vec::len), Some(1));
        assert_eq!(grouped.get("main").map(Vec::len), Some(1));
    }

    #[tokio::test]
    async fn resources_lookup_by_name() {
        let stub = make_stub();
        let by_path = resources::get_worktree_by_name(&stub.runner(), "feature-branch")
            .await
            .expect("lookup worktree by path");
        assert!(by_path.is_some());
        assert_eq!(by_path.as_ref().unwrap().name, "feature/awesome");

        let by_branch = resources::get_worktree_by_name(&stub.runner(), "feature/awesome")
            .await
            .expect("lookup worktree by branch");
        assert!(by_branch.is_some());
        assert_eq!(by_branch.unwrap().name, "feature/awesome");
    }
}
