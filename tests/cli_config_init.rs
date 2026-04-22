#![cfg(unix)]

use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn config_init_creates_yaml_with_detected_designer_sources() {
    let dir = tempdir().expect("tempdir");
    let main = dir.path().join("src").join("configuration");
    let ext = dir.path().join("extensions").join("sales");
    fs::create_dir_all(&main).expect("main");
    fs::create_dir_all(&ext).expect("ext");
    fs::write(main.join("Configuration.xml"), "<Configuration/>").expect("main xml");
    fs::write(
        ext.join("Configuration.xml"),
        "<Configuration><ConfigurationExtensionPurpose kind=\"Customization\">Customization</ConfigurationExtensionPurpose></Configuration>",
    )
    .expect("ext xml");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .current_dir(dir.path())
        .args(["config", "init"])
        .output()
        .expect("run command");

    assert!(output.status.success());
    let config = fs::read_to_string(dir.path().join("v8project.yaml")).expect("config");
    assert!(config.contains("format: DESIGNER"));
    assert!(config.contains("workPath: 'build'"));
    assert!(config.contains("infobase:"));
    assert!(config.contains("  connection: 'File=build/ib'"));
    assert!(config.contains("path: 'src/configuration'"));
    assert!(config.contains("type: EXTENSION"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Config written"));
}

#[test]
fn config_init_uses_json_envelope_and_config_path_override() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("Configuration.xml"), "<Configuration/>").expect("xml");
    let config_path = dir.path().join("custom.yaml");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .current_dir(dir.path())
        .args([
            "--config",
            &config_path.display().to_string(),
            "--output",
            "json",
            "config",
            "init",
            "--connection",
            "File=/tmp/test-ib",
        ])
        .output()
        .expect("run command");

    assert!(output.status.success());
    assert!(config_path.exists());
    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "config init");
    assert_eq!(payload["data"]["source_sets"][0]["path"], ".");
    assert_eq!(payload["data"]["source_sets"][0]["type"], "CONFIGURATION");
    let config = fs::read_to_string(config_path).expect("config");
    assert!(config.contains("infobase:"));
    assert!(config.contains("  connection: 'File=/tmp/test-ib'"));
}

#[test]
fn config_init_detects_edt_extension_by_marker_contents() {
    let dir = tempdir().expect("tempdir");
    let config_project = dir.path().join("workspace").join("cfg-project");
    let extension_project = dir.path().join("workspace").join("addon-project");
    fs::create_dir_all(config_project.join("metadata")).expect("config metadata");
    fs::create_dir_all(extension_project.join("metadata")).expect("ext metadata");
    fs::write(
        config_project.join(".project"),
        "<projectDescription><name>configuration</name></projectDescription>",
    )
    .expect("config project");
    fs::write(
        config_project.join("metadata").join("Configuration.xml"),
        "<Configuration/>",
    )
    .expect("config xml");
    fs::write(
        extension_project.join(".project"),
        "<projectDescription><name>sales</name></projectDescription>",
    )
    .expect("ext project");
    fs::write(
        extension_project.join("metadata").join("Configuration.xml"),
        "<Configuration><ConfigurationExtensionPurpose kind=\"Customization\">Customization</ConfigurationExtensionPurpose></Configuration>",
    )
    .expect("ext xml");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .current_dir(dir.path())
        .args(["config", "init"])
        .output()
        .expect("run command");

    assert!(output.status.success());
    let config = fs::read_to_string(dir.path().join("v8project.yaml")).expect("config");
    assert!(config.contains("format: EDT"));
    assert!(config.contains("path: 'workspace/cfg-project'"));
    assert!(config.contains("path: 'workspace/addon-project'"));
    assert!(config.contains("type: EXTENSION"));
}

#[test]
fn config_init_refuses_to_overwrite_without_force() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("v8project.yaml"), "existing").expect("existing");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .current_dir(dir.path())
        .args(["config", "init"])
        .output()
        .expect("run command");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
}

#[test]
fn config_init_detects_designer_external_aggregate_source_set() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("Configuration.xml"), "<Configuration/>").expect("config xml");
    fs::create_dir_all(dir.path().join("tools")).expect("tools dir");
    fs::write(
        dir.path().join("tools").join("alpha.xml"),
        "<ExternalDataProcessor><Properties><Name>Alpha</Name></Properties></ExternalDataProcessor>",
    )
    .expect("alpha xml");
    fs::write(
        dir.path().join("tools").join("beta.xml"),
        "<MetaDataObject><ExternalDataProcessor><Properties><Name>Beta</Name></Properties></ExternalDataProcessor></MetaDataObject>",
    )
    .expect("beta xml");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .current_dir(dir.path())
        .args(["config", "init", "--format", "designer"])
        .output()
        .expect("run command");

    assert!(output.status.success());
    let config = fs::read_to_string(dir.path().join("v8project.yaml")).expect("config");
    assert!(config.contains("type: EXTERNAL_DATA_PROCESSORS"));
    assert!(config.contains("path: 'tools'"));
}

#[test]
fn config_init_rejects_external_only_autodiscovery_without_configuration() {
    let dir = tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("tools")).expect("tools dir");
    fs::write(
        dir.path().join("tools").join("alpha.xml"),
        "<ExternalDataProcessor><Properties><Name>Alpha</Name></Properties></ExternalDataProcessor>",
    )
    .expect("alpha xml");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .current_dir(dir.path())
        .args(["config", "init", "--format", "designer"])
        .output()
        .expect("run command");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("did not find a CONFIGURATION source-set"));
}

#[test]
fn config_init_auto_prefers_edt_when_designer_only_has_external_root() {
    let dir = tempdir().expect("tempdir");
    let config_project = dir.path().join("workspace").join("cfg");
    fs::create_dir_all(config_project.join("metadata")).expect("config metadata");
    fs::write(
        config_project.join(".project"),
        "<projectDescription><name>configuration</name></projectDescription>",
    )
    .expect("config project");
    fs::write(
        config_project.join("metadata").join("Configuration.xml"),
        "<Configuration/>",
    )
    .expect("config xml");
    fs::create_dir_all(dir.path().join("tools")).expect("tools dir");
    fs::write(
        dir.path().join("tools").join("alpha.xml"),
        "<ExternalDataProcessor><Properties><Name>Alpha</Name></Properties></ExternalDataProcessor>",
    )
    .expect("alpha xml");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .current_dir(dir.path())
        .args(["config", "init"])
        .output()
        .expect("run command");

    assert!(output.status.success());
    let config = fs::read_to_string(dir.path().join("v8project.yaml")).expect("config");
    assert!(config.contains("format: EDT"));
    assert!(config.contains("path: 'workspace/cfg'"));
    assert!(!config.contains("path: 'tools'"));
}

#[test]
fn config_init_keeps_nested_edt_configuration_under_external_root() {
    let dir = tempdir().expect("tempdir");
    let external_root = dir.path().join("processors");
    for name in ["alpha", "beta"] {
        let project = external_root.join(name);
        fs::create_dir_all(project.join("src")).expect("external project dir");
        fs::write(
            project.join(".project"),
            format!("<projectDescription><name>{name}</name></projectDescription>"),
        )
        .expect("external project");
        fs::write(
            project.join("src").join("root.xml"),
            format!(
                "<ExternalDataProcessor><Properties><Name>{name}</Name></Properties></ExternalDataProcessor>"
            ),
        )
        .expect("external xml");
    }
    let config_project = external_root.join("apps").join("cfg");
    fs::create_dir_all(config_project.join("metadata")).expect("config metadata");
    fs::write(
        config_project.join(".project"),
        "<projectDescription><name>configuration</name></projectDescription>",
    )
    .expect("config project");
    fs::write(
        config_project.join("metadata").join("Configuration.xml"),
        "<Configuration/>",
    )
    .expect("config xml");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .current_dir(dir.path())
        .args(["config", "init", "--format", "edt"])
        .output()
        .expect("run command");

    assert!(output.status.success());
    let config = fs::read_to_string(dir.path().join("v8project.yaml")).expect("config");
    assert!(config.contains("path: 'processors'"));
    assert!(config.contains("type: EXTERNAL_DATA_PROCESSORS"));
    assert!(config.contains("path: 'processors/apps/cfg'"));
    assert!(config.contains("type: CONFIGURATION"));
}

#[test]
fn config_init_ignores_non_edt_root_project_marker_when_nested_project_exists() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join(".project"), "<root/>").expect("root project marker");
    let config_project = dir.path().join("workspace").join("cfg");
    fs::create_dir_all(config_project.join("metadata")).expect("config metadata");
    fs::write(
        config_project.join(".project"),
        "<projectDescription><name>configuration</name></projectDescription>",
    )
    .expect("config project");
    fs::write(
        config_project.join("metadata").join("Configuration.xml"),
        "<Configuration/>",
    )
    .expect("config xml");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .current_dir(dir.path())
        .args(["config", "init", "--format", "edt"])
        .output()
        .expect("run command");

    assert!(output.status.success());
    let config = fs::read_to_string(dir.path().join("v8project.yaml")).expect("config");
    assert!(config.contains("path: 'workspace/cfg'"));
    assert!(config.contains("type: CONFIGURATION"));
}
