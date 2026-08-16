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
fn enable_without_complete_authority_is_a_non_mutating_dry_run() {
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
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["audit"]["disposition"], "dry_run");
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
