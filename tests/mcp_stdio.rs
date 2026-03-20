#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use assert_cmd::cargo::cargo_bin;
use rmcp::{
    model::CallToolRequestParams,
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
};
use serde_json::{json, Value};
use tempfile::tempdir;

fn write_config(path: &Path, base_path: &Path, work_path: &Path, platform_path: &Path) {
    let config = format!(
        "basePath: '{}'\nworkPath: '{}'\nformat: DESIGNER\nbuilder: DESIGNER\nconnection: 'File=/tmp/ib'\nsource-set:\n  - name: main\n    purpose: CONFIGURATION\n    path: .\ntools:\n  platform:\n    path: '{}'\n",
        base_path.display(),
        work_path.display(),
        platform_path.display(),
    );
    fs::write(path, config).expect("config");
}

fn setup_project() -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let base_path = dir.path().join("project");
    let work_path = dir.path().join("work");
    let platform_path = dir.path().join("platform");
    let config_path = dir.path().join("application.yaml");

    fs::create_dir_all(&base_path).expect("base");
    fs::create_dir_all(&work_path).expect("work");
    fs::create_dir_all(&platform_path).expect("platform");
    write_config(&config_path, &base_path, &work_path, &platform_path);

    (dir, config_path)
}

#[test]
fn mcp_missing_config_reports_error_on_stderr() {
    let output = std::process::Command::new(cargo_bin("v8-test-runner"))
        .args([
            "--config",
            "/definitely/missing/application.yaml",
            "mcp",
            "serve",
            "stdio",
        ])
        .output()
        .expect("run command");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("config"));
    assert!(stderr.contains("not found"));
}

#[tokio::test]
async fn mcp_stdio_exposes_expected_tools_and_capabilities() {
    let (_dir, config_path) = setup_project();
    let (transport, _stderr) = TokioChildProcess::builder(
        tokio::process::Command::new(cargo_bin("v8-test-runner")).configure(|cmd| {
            cmd.arg("--config")
                .arg(config_path.as_os_str())
                .arg("mcp")
                .arg("serve")
                .arg("stdio");
        }),
    )
    .stderr(Stdio::piped())
    .spawn()
    .expect("spawn stdio transport");

    let client = ().serve(transport).await.expect("connect rmcp client");
    let info = serde_json::to_value(client.peer().peer_info().expect("peer info")).expect("info");
    let tools = client.list_all_tools().await.expect("list tools");

    let mut names: Vec<String> = tools.iter().map(|tool| tool.name.to_string()).collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "build_project",
            "check_syntax_designer_config",
            "check_syntax_designer_modules",
            "check_syntax_edt",
            "dump_config",
            "launch_app",
            "run_all_tests",
            "run_module_tests",
        ]
    );
    assert!(info["capabilities"]["tools"].is_object());
    assert!(info["capabilities"]["resources"].is_null());
    assert!(info["capabilities"]["prompts"].is_null());

    let launch_schema = tools
        .iter()
        .find(|tool| tool.name == "launch_app")
        .map(|tool| serde_json::to_value(&tool.input_schema).expect("launch schema"))
        .expect("launch tool");
    assert_eq!(launch_schema["properties"]["utilityType"]["type"], "string");

    let module_schema = tools
        .iter()
        .find(|tool| tool.name == "run_module_tests")
        .map(|tool| serde_json::to_value(&tool.input_schema).expect("module schema"))
        .expect("module tool");
    assert!(module_schema["required"]
        .as_array()
        .expect("required")
        .iter()
        .any(|value| value == "moduleName"));

    client.cancel().await.expect("cancel client");
}

#[tokio::test]
async fn mcp_stdio_returns_structured_business_failure() {
    let (_dir, config_path) = setup_project();
    let transport = TokioChildProcess::new(
        tokio::process::Command::new(cargo_bin("v8-test-runner")).configure(|cmd| {
            cmd.arg("--config")
                .arg(config_path.as_os_str())
                .arg("mcp")
                .arg("serve")
                .arg("stdio");
        }),
    )
    .expect("spawn stdio transport");

    let client = ().serve(transport).await.expect("connect rmcp client");
    let response = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("run_module_tests").with_arguments(
                serde_json::from_value(json!({ "moduleName": "   " })).expect("arguments"),
            ),
        )
        .await
        .expect("call tool");

    assert_eq!(response.is_error, Some(true));
    let payload: Value = response.structured_content.expect("structured payload");
    assert_eq!(payload["status"], "business_failure");
    assert_eq!(payload["error"]["code"], "invalid_argument");
    assert_eq!(payload["response"]["success"], false);

    client.cancel().await.expect("cancel client");
}
