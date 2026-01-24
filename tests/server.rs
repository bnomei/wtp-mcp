#![cfg(unix)]

use rmcp::model::{
    CallToolRequestParams, NumberOrString, ReadResourceRequestParams, ResourceContents,
};
use rmcp::service::{RequestContext, RoleServer, RunningService, serve_directly};
use rmcp::{ErrorData as McpError, ServerHandler};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::io::duplex;

use wtp_mcp_rs::config::{Config, SecurityPolicy, WtpConfig};
use wtp_mcp_rs::resources;
use wtp_mcp_rs::server::WtpServer;

struct TestHarness {
    _dir: TempDir,
    server: WtpServer,
    peer: rmcp::Peer<RoleServer>,
    _running: RunningService<RoleServer, WtpServer>,
    wtp_path: PathBuf,
}

impl TestHarness {
    fn new() -> Self {
        let dir = TempDir::new().expect("temp dir");
        let repo_root = dir.path().join("repo");
        fs::create_dir_all(&repo_root).expect("repo dir");

        let wtp_path = dir.path().join("wtp");
        fs::write(&wtp_path, stub_script()).expect("write stub");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&wtp_path).expect("metadata").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&wtp_path, perms).expect("chmod");
        }
        let sanity = std::process::Command::new(&wtp_path)
            .arg("list")
            .output()
            .expect("run stub");
        assert!(
            sanity.status.success(),
            "stub list failed: status={:?} stderr={}",
            sanity.status.code(),
            String::from_utf8_lossy(&sanity.stderr)
        );

        let config = Config {
            repo_root: Some(repo_root),
            wtp: WtpConfig {
                path: Some(wtp_path.clone()),
            },
            security: SecurityPolicy {
                allow_hooks: false,
                allow_branch_delete: false,
            },
        };

        let mut server = WtpServer::new(config);
        server.wtp_path = wtp_path.clone();
        let (_client_io, server_io) = duplex(64);
        let running = serve_directly(server.clone(), server_io, None);
        let peer = running.peer().clone();

        Self {
            _dir: dir,
            server,
            peer,
            _running: running,
            wtp_path,
        }
    }

    fn context(&self) -> RequestContext<RoleServer> {
        RequestContext {
            peer: self.peer.clone(),
            ct: Default::default(),
            id: NumberOrString::Number(1),
            meta: Default::default(),
            extensions: Default::default(),
        }
    }
}

fn stub_script() -> String {
    "#!/bin/sh\n\
cmd=\"$1\"\n\
if [ \"$#\" -gt 0 ]; then\n\
  shift\n\
fi\n\
case \"$cmd\" in\n\
  list)\n\
    echo \"/repo/main main abc123 (main worktree)\"\n\
    echo \"feature-branch feature/awesome def456\"\n\
    echo \"/abs/bugfix bugfix/123 aaa111\"\n\
    ;;\n\
  cd)\n\
    if [ -z \"$1\" ]; then\n\
      echo \"/repo/main\"\n\
      exit 0\n\
    fi\n\
    case \"$1\" in\n\
      feature-branch)\n\
        echo \"/repo/feature-branch\"\n\
        ;;\n\
      feature/awesome)\n\
        echo \"/repo/feature-branch\"\n\
        ;;\n\
      *)\n\
        exit 1\n\
        ;;\n\
    esac\n\
    ;;\n\
  --version)\n\
    echo \"wtp 1.2.3\"\n\
    ;;\n\
  hook)\n\
    echo \"hook-$1\"\n\
    ;;\n\
  shell-init)\n\
    echo \"init-$1\"\n\
    ;;\n\
  init)\n\
    echo \"/repo/.wtp.yml\"\n\
    ;;\n\
  add|remove)\n\
    exit 0\n\
    ;;\n\
  *)\n\
    echo \"unknown command: $cmd\" >&2\n\
    exit 1\n\
    ;;\n\
esac\n"
        .to_string()
}

fn extract_text(contents: &[ResourceContents]) -> String {
    match contents.first() {
        Some(ResourceContents::TextResourceContents { text, .. }) => text.clone(),
        other => panic!("unexpected resource contents: {other:?}"),
    }
}

async fn call_tool(
    harness: &TestHarness,
    name: &str,
    args: Option<serde_json::Value>,
) -> Result<rmcp::model::CallToolResult, McpError> {
    assert_eq!(harness.server.wtp_path, harness.wtp_path);
    let arguments = args.and_then(|value| value.as_object().cloned());
    let params = CallToolRequestParams {
        meta: None,
        name: name.to_string().into(),
        arguments,
        task: None,
    };

    harness.server.call_tool(params, harness.context()).await
}

#[tokio::test]
async fn list_tools_contains_expected_names() {
    let harness = TestHarness::new();
    let result = harness
        .server
        .list_tools(None, harness.context())
        .await
        .expect("list tools");

    let names: Vec<String> = result
        .tools
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();

    assert!(names.contains(&"list-worktrees".to_string()));
    assert!(names.contains(&"add-worktree".to_string()));
    assert!(names.contains(&"remove-worktree".to_string()));
    assert!(names.contains(&"init-config".to_string()));
    assert!(names.contains(&"get-worktree-path".to_string()));
    assert!(names.contains(&"shell-hook".to_string()));
    assert!(names.contains(&"shell-init".to_string()));
}

#[tokio::test]
async fn get_info_includes_capabilities() {
    let harness = TestHarness::new();
    let info = harness.server.get_info();
    let instructions = info.instructions.expect("instructions");
    assert!(instructions.contains("WTP MCP Server"));
    let capabilities = info.capabilities;
    assert!(capabilities.tools.is_some());
    assert!(capabilities.resources.is_some());
}

#[tokio::test]
async fn tools_return_structured_payloads() {
    let harness = TestHarness::new();
    let runner = wtp_mcp_rs::wtp::WtpRunner::new(
        harness.server.wtp_path.clone(),
        harness.server.repo_root.clone(),
    );
    let sanity = runner.run(&["list"]).await.expect("runner list");
    assert!(
        sanity.exit_code == 0,
        "runner list failed: exit={} stdout={} stderr={}",
        sanity.exit_code,
        sanity.stdout,
        sanity.stderr
    );

    let list_result = call_tool(&harness, "list-worktrees", None)
        .await
        .expect("list-worktrees");
    let list_payload = list_result
        .structured_content
        .expect("structured list output");
    let worktrees = list_payload
        .get("worktrees")
        .and_then(|value| value.as_array())
        .expect("worktrees array");
    assert_eq!(worktrees.len(), 3);

    let add_result = call_tool(
        &harness,
        "add-worktree",
        Some(json!({"new_branch": "feature/awesome", "from": "main"})),
    )
    .await
    .expect("add-worktree");
    let add_payload = add_result.structured_content.expect("add output");
    assert_eq!(
        add_payload.get("name").and_then(|v| v.as_str()),
        Some("feature/awesome")
    );
    assert_eq!(
        add_payload.get("path").and_then(|v| v.as_str()),
        Some("/repo/feature-branch")
    );

    let remove_result = call_tool(
        &harness,
        "remove-worktree",
        Some(json!({"name": "feature/awesome"})),
    )
    .await
    .expect("remove-worktree");
    let remove_payload = remove_result.structured_content.expect("remove output");
    assert_eq!(
        remove_payload.get("removed").and_then(|v| v.as_str()),
        Some("feature/awesome")
    );

    let init_result = call_tool(&harness, "init-config", None)
        .await
        .expect("init-config");
    let init_payload = init_result.structured_content.expect("init output");
    assert_eq!(
        init_payload.get("path").and_then(|v| v.as_str()),
        Some("/repo/.wtp.yml")
    );

    let path_result = call_tool(
        &harness,
        "get-worktree-path",
        Some(json!({"name": "feature/awesome"})),
    )
    .await
    .expect("get-worktree-path");
    let path_payload = path_result.structured_content.expect("path output");
    assert_eq!(
        path_payload.get("path").and_then(|v| v.as_str()),
        Some("/repo/feature-branch")
    );

    let hook_result = call_tool(&harness, "shell-hook", Some(json!({"shell": "bash"})))
        .await
        .expect("shell-hook");
    let hook_payload = hook_result.structured_content.expect("hook output");
    assert_eq!(
        hook_payload.get("shell").and_then(|v| v.as_str()),
        Some("bash")
    );
    assert!(
        hook_payload
            .get("script")
            .and_then(|v| v.as_str())
            .expect("script")
            .contains("hook-bash")
    );

    let init_shell_result = call_tool(&harness, "shell-init", Some(json!({"shell": "bash"})))
        .await
        .expect("shell-init");
    let init_shell_payload = init_shell_result
        .structured_content
        .expect("shell-init output");
    assert!(
        init_shell_payload
            .get("script")
            .and_then(|v| v.as_str())
            .expect("script")
            .contains("init-bash")
    );

}

#[tokio::test]
async fn remove_worktree_branch_delete_blocked() {
    let harness = TestHarness::new();
    let err = call_tool(
        &harness,
        "remove-worktree",
        Some(json!({"name": "feature/awesome", "with_branch": true})),
    )
    .await
    .expect_err("expected policy error");

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_REQUEST);
}

#[tokio::test]
async fn resources_list_and_read() {
    let harness = TestHarness::new();
    assert_eq!(harness.server.wtp_path, harness.wtp_path);

    let resources_list = harness
        .server
        .list_resources(None, harness.context())
        .await
        .expect("list resources");
    let uris: Vec<String> = resources_list
        .resources
        .into_iter()
        .map(|resource| resource.uri.clone())
        .collect();
    assert!(uris.contains(&resources::URI_WORKTREES.to_string()));
    assert!(uris.contains(&resources::URI_OVERVIEW.to_string()));

    let templates = harness
        .server
        .list_resource_templates(None, harness.context())
        .await
        .expect("list resource templates");
    assert_eq!(templates.resource_templates.len(), 1);
    assert_eq!(
        templates.resource_templates[0].uri_template,
        resources::URI_WORKTREE_TEMPLATE
    );

    let worktrees = harness
        .server
        .read_resource(
            ReadResourceRequestParams {
                meta: None,
                uri: resources::URI_WORKTREES.to_string(),
            },
            harness.context(),
        )
        .await
        .expect("read worktrees");
    let worktrees_text = extract_text(&worktrees.contents);
    let parsed: serde_json::Value = serde_json::from_str(&worktrees_text).unwrap();
    assert_eq!(parsed.as_array().map(|arr| arr.len()), Some(3));

    let overview = harness
        .server
        .read_resource(
            ReadResourceRequestParams {
                meta: None,
                uri: resources::URI_OVERVIEW.to_string(),
            },
            harness.context(),
        )
        .await
        .expect("read overview");
    let overview_text = extract_text(&overview.contents);
    let parsed: serde_json::Value = serde_json::from_str(&overview_text).unwrap();
    assert_eq!(
        parsed.get("repo_root").and_then(|v| v.as_str()),
        Some("/repo/main")
    );

    let by_name = harness
        .server
        .read_resource(
            ReadResourceRequestParams {
                meta: None,
                uri: resources::URI_WORKTREE_TEMPLATE.replace("{name}", "feature/awesome"),
            },
            harness.context(),
        )
        .await
        .expect("read worktree by name");
    let by_name_text = extract_text(&by_name.contents);
    let parsed: serde_json::Value = serde_json::from_str(&by_name_text).unwrap();
    assert_eq!(
        parsed.get("name").and_then(|v| v.as_str()),
        Some("feature/awesome")
    );

    let missing_worktree = harness
        .server
        .read_resource(
            ReadResourceRequestParams {
                meta: None,
                uri: resources::URI_WORKTREE_TEMPLATE.replace("{name}", "missing"),
            },
            harness.context(),
        )
        .await
        .expect_err("missing worktree");
    assert_eq!(
        missing_worktree.code,
        rmcp::model::ErrorCode::RESOURCE_NOT_FOUND
    );

    let missing = harness
        .server
        .read_resource(
            ReadResourceRequestParams {
                meta: None,
                uri: "wtp://missing".to_string(),
            },
            harness.context(),
        )
        .await;
    assert!(missing.is_err());
}
