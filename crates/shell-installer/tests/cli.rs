use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_record() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "superdesktop-installer-cli-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn invalid_product_identity_is_rejected_without_metadata_or_registry_authority() {
    let executable = std::env::current_exe().unwrap();
    let record = unique_record();
    let output = Command::new(env!("CARGO_BIN_EXE_shell-installer"))
        .args([
            "enable",
            "--app",
            executable.to_str().unwrap(),
            "--guardian",
            executable.to_str().unwrap(),
            "--rollback-record",
            record.to_str().unwrap(),
            "--apply",
            "--explicit-opt-in",
            "--confirm-plan",
            "deliberately-wrong",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["disposition"], "failed");
    assert!(
        value["error"]
            .as_str()
            .unwrap()
            .contains("PreflightRejected")
    );
    assert!(!record.exists());
}

#[test]
fn malformed_invocation_is_machine_readable_and_does_not_create_metadata() {
    let record = unique_record();
    let output = Command::new(env!("CARGO_BIN_EXE_shell-installer"))
        .args([
            "enable",
            "--rollback-record",
            record.to_str().unwrap(),
            "--bogus",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();
    assert!(!record.exists());
    let _ = fs::remove_file(record);
}
