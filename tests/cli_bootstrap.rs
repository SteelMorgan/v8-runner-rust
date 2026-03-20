#![cfg(unix)]

use assert_cmd::prelude::*;
use serde_json::Value;

#[test]
fn missing_config_in_text_mode_returns_validation_error_on_stderr() {
    let output = std::process::Command::cargo_bin("v8-test-runner")
        .expect("binary")
        .args(["--config", "/definitely/missing/application.yaml", "build"])
        .output()
        .expect("run command");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("config file not found"));
}

#[test]
fn missing_config_in_json_mode_keeps_error_envelope_shape() {
    let output = std::process::Command::cargo_bin("v8-test-runner")
        .expect("binary")
        .args([
            "--config",
            "/definitely/missing/application.yaml",
            "--output",
            "json",
            "build",
        ])
        .output()
        .expect("run command");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));

    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["command"], "error");
    assert_eq!(payload["duration_ms"], 0);
    assert_eq!(
        payload["data"]["message"],
        "config file not found: /definitely/missing/application.yaml"
    );
}
