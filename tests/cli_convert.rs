#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use assert_cmd::prelude::*;
use serde_json::Value;
use tempfile::tempdir;

fn make_executable(path: &Path) {
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod");
}

fn write_script(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent");
    }
    fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("write");
    make_executable(path);
}

fn write_edt_script(path: &Path, calls_log: &Path) {
    let body = format!(
        "args=\"$*\"\n\
printf '%s\\n' \"$args\" >> \"{}\"\n\
mode=\"\"\n\
project=\"\"\n\
config_files=\"\"\n\
prev=\"\"\n\
for arg in \"$@\"; do\n\
  if [ \"$prev\" = \"-command\" ]; then mode=\"$arg\"; fi\n\
  if [ \"$prev\" = \"--project\" ]; then project=\"$arg\"; fi\n\
  if [ \"$prev\" = \"--configuration-files\" ]; then config_files=\"$arg\"; fi\n\
  prev=\"$arg\"\n\
done\n\
case \"$mode\" in\n\
  export)\n\
    mkdir -p \"$config_files\"\n\
    printf '<Configuration />\\n' > \"$config_files/Configuration.xml\"\n\
    ;;\n\
  import)\n\
    mkdir -p \"$project\"\n\
    printf '<projectDescription><name>Imported</name></projectDescription>\\n' > \"$project/.project\"\n\
    ;;\n\
esac\n\
exit 0",
        calls_log.display()
    );
    write_script(path, &body);
}

fn write_config(path: &Path, base_path: &Path, work_path: &Path, edt_path: &Path) {
    let config = format!(
        "basePath: '{}'\nworkPath: '{}'\nformat: DESIGNER\nbuilder: DESIGNER\ninfobase:\n  connection: 'File=/tmp/ib'\nsource-set:\n  - name: main\n    type: CONFIGURATION\n    path: main\ntools:\n  edt_cli:\n    path: '{}'\n    interactive-mode: false\n",
        base_path.display(),
        work_path.display(),
        edt_path.display(),
    );
    fs::write(path, config).expect("config");
}

fn write_live_workspace_lock(work_path: &Path, command: &str) {
    let canonical_work = fs::canonicalize(work_path).expect("canonical work");
    let lock_owner = "integration-test-lock-owner";
    let started_at = chrono::Utc::now().to_rfc3339();

    fs::write(
        canonical_work.join(".v8-runner.workspace.lock"),
        serde_json::json!({
            "tool": "v8-runner",
            "pid": std::process::id(),
            "owner_id": lock_owner,
            "created_at": started_at,
        })
        .to_string(),
    )
    .expect("workspace lock");
    fs::write(
        canonical_work.join(".v8-runner.workspace.lock.json"),
        serde_json::json!({
            "pid": std::process::id(),
            "lock_owner": lock_owner,
            "command": command,
            "started_at": started_at,
            "canonical_work_path": canonical_work,
        })
        .to_string(),
    )
    .expect("workspace lock sidecar");
}

fn setup_project() -> (
    tempfile::TempDir,
    PathBuf,
    PathBuf,
    PathBuf,
    PathBuf,
    PathBuf,
) {
    let dir = tempdir().expect("tempdir");
    let base_path = dir.path().join("project");
    let work_path = dir.path().join("work");
    let config_path = dir.path().join("v8project.yaml");
    let edt_cli_path = dir.path().join("edt").join("1cedtcli");
    let calls_log = dir.path().join("edt-calls.log");

    fs::create_dir_all(base_path.join("main")).expect("base main");
    fs::create_dir_all(&work_path).expect("work");
    write_edt_script(&edt_cli_path, &calls_log);
    write_config(&config_path, &base_path, &work_path, &edt_cli_path);

    (
        dir,
        config_path,
        base_path,
        work_path,
        edt_cli_path,
        calls_log,
    )
}

#[test]
fn convert_edt_to_designer_json_success() {
    let (_dir, config_path, base_path, _work_path, _edt_cli_path, calls_log) = setup_project();
    let edt_source = base_path.join("edt-source");
    let designer_target = base_path.join("designer-target");
    fs::create_dir_all(&edt_source).expect("edt source");
    fs::write(
        edt_source.join(".project"),
        "<projectDescription><name>DemoEdt</name></projectDescription>\n",
    )
    .expect("project marker");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .args([
            "--config",
            &config_path.display().to_string(),
            "--output",
            "json",
            "convert",
            "edt-to-designer",
            "--source",
            &edt_source.display().to_string(),
            "--target",
            &designer_target.display().to_string(),
        ])
        .output()
        .expect("run convert");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["data"]["direction"], "EDT_TO_DESIGNER");
    assert!(designer_target.join("Configuration.xml").exists());

    let calls = fs::read_to_string(calls_log).expect("calls");
    assert!(calls.contains("export"));
    assert!(calls.contains("--project"));
    assert!(calls.contains(edt_source.display().to_string().as_str()));
}

#[test]
fn convert_designer_to_edt_text_success_replaces_target_and_passes_options() {
    let (_dir, config_path, base_path, _work_path, _edt_cli_path, calls_log) = setup_project();
    let designer_source = base_path.join("designer-source");
    let edt_target = base_path.join("edt-target");
    fs::create_dir_all(&designer_source).expect("designer source");
    fs::write(designer_source.join("Configuration.xml"), "<Configuration />\n").expect("xml");
    fs::create_dir_all(&edt_target).expect("existing target");
    fs::write(edt_target.join("stale.txt"), "stale").expect("stale");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .args([
            "--config",
            &config_path.display().to_string(),
            "--no-color",
            "convert",
            "designer-to-edt",
            "--source",
            &designer_source.display().to_string(),
            "--target",
            &edt_target.display().to_string(),
            "--version",
            "8.3.24",
            "--base-project-name",
            "BaseProject",
            "--build",
        ])
        .output()
        .expect("run convert");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("● Convert completed successfully"));
    assert!(stdout.contains("direction: designer-to-edt"));
    assert!(stdout.contains(edt_target.display().to_string().as_str()));
    assert!(stdout.contains("workspace:"));

    assert!(edt_target.join(".project").exists());
    assert!(!edt_target.join("stale.txt").exists());

    let calls = fs::read_to_string(calls_log).expect("calls");
    assert!(calls.contains("import"));
    assert!(calls.contains("--configuration-files"));
    assert!(calls.contains(designer_source.display().to_string().as_str()));
    assert!(calls.contains("--version"));
    assert!(calls.contains("8.3.24"));
    assert!(calls.contains("--base-project-name"));
    assert!(calls.contains("BaseProject"));
    assert!(calls.contains("--build"));
    assert!(calls.contains("true"));
}

#[test]
fn convert_validation_runs_before_workspace_lock() {
    let (_dir, config_path, base_path, work_path, _edt_cli_path, _calls_log) = setup_project();
    let invalid_source = base_path.join("invalid-source");
    let designer_target = base_path.join("designer-target");
    fs::create_dir_all(&invalid_source).expect("invalid source");
    write_live_workspace_lock(&work_path, "convert");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .args([
            "--config",
            &config_path.display().to_string(),
            "--no-color",
            "convert",
            "edt-to-designer",
            "--source",
            &invalid_source.display().to_string(),
            "--target",
            &designer_target.display().to_string(),
        ])
        .output()
        .expect("run convert");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("EDT source path must contain '.project'"),
        "stderr:\n{stderr}"
    );
    assert!(!stderr.contains("cannot start convert"));
}

#[test]
fn convert_workspace_lock_conflict_uses_runtime_error() {
    let (_dir, config_path, base_path, work_path, _edt_cli_path, _calls_log) = setup_project();
    let edt_source = base_path.join("edt-source");
    let designer_target = base_path.join("designer-target");
    fs::create_dir_all(&edt_source).expect("edt source");
    fs::write(
        edt_source.join(".project"),
        "<projectDescription><name>DemoEdt</name></projectDescription>\n",
    )
    .expect("project marker");
    write_live_workspace_lock(&work_path, "convert");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .args([
            "--config",
            &config_path.display().to_string(),
            "--no-color",
            "convert",
            "edt-to-designer",
            "--source",
            &edt_source.display().to_string(),
            "--target",
            &designer_target.display().to_string(),
        ])
        .output()
        .expect("run convert");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ERROR: runtime error: cannot start convert"),
        "stderr:\n{stderr}"
    );
}
