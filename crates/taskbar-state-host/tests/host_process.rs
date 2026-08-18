use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use shell_provider_protocol::{TaskbarStateHostRequest, TaskbarStateHostResponse};

#[test]
fn host_returns_health_snapshot_and_clean_shutdown() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_taskbar-state-host"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    for request in [
        TaskbarStateHostRequest::Health,
        TaskbarStateHostRequest::Snapshot,
        TaskbarStateHostRequest::Shutdown,
    ] {
        serde_json::to_writer(&mut input, &request).unwrap();
        input.write_all(b"\n").unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        let response: TaskbarStateHostResponse = serde_json::from_str(&line).unwrap();
        assert!(matches!(
            (request, response),
            (
                TaskbarStateHostRequest::Health,
                TaskbarStateHostResponse::Health { .. }
            ) | (
                TaskbarStateHostRequest::Snapshot,
                TaskbarStateHostResponse::Snapshot(_)
            ) | (
                TaskbarStateHostRequest::Shutdown,
                TaskbarStateHostResponse::Shutdown
            )
        ));
    }
    assert!(child.wait().unwrap().success());
}

#[test]
fn ordinary_itaskbarlist3_calls_publish_progress_snapshot() {
    let mut host = Command::new(env!("CARGO_BIN_EXE_taskbar-state-host"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = host.stdin.take().unwrap();
    let mut output = BufReader::new(host.stdout.take().unwrap());
    serde_json::to_writer(&mut input, &TaskbarStateHostRequest::Health).unwrap();
    input.write_all(b"\n").unwrap();
    input.flush().unwrap();
    let mut health = String::new();
    output.read_line(&mut health).unwrap();
    assert!(matches!(
        serde_json::from_str::<TaskbarStateHostResponse>(&health).unwrap(),
        TaskbarStateHostResponse::Health { .. }
    ));

    let mut fixture = Command::new(env!("CARGO_BIN_EXE_taskbar-progress-fixture"))
        .args(["normal", "42", "--local-server"])
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(500));
    serde_json::to_writer(&mut input, &TaskbarStateHostRequest::Snapshot).unwrap();
    input.write_all(b"\n").unwrap();
    input.flush().unwrap();
    let mut line = String::new();
    output.read_line(&mut line).unwrap();
    let TaskbarStateHostResponse::Snapshot(snapshot) =
        serde_json::from_str::<TaskbarStateHostResponse>(&line).unwrap()
    else {
        panic!("expected snapshot")
    };
    assert_eq!(snapshot.windows.len(), 1);
    assert_eq!(snapshot.windows[0].progress.completed, 42);
    assert_eq!(snapshot.windows[0].progress.total, 100);

    let _ = fixture.kill();
    let _ = fixture.wait();
    serde_json::to_writer(&mut input, &TaskbarStateHostRequest::Shutdown).unwrap();
    input.write_all(b"\n").unwrap();
    input.flush().unwrap();
    assert!(host.wait().unwrap().success());
}
