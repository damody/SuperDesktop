use std::{
    io::Write,
    process::{Command, Stdio},
};

use shell_provider_protocol::{SystemStatusHostRequest, SystemStatusHostResponse};

#[test]
fn process_handshakes_snapshots_rejects_malformed_input_and_stops_on_eof() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_system-status-host"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    writeln!(
        input,
        "{}",
        serde_json::to_string(&SystemStatusHostRequest::Handshake).unwrap()
    )
    .unwrap();
    writeln!(
        input,
        "{}",
        serde_json::to_string(&SystemStatusHostRequest::Snapshot).unwrap()
    )
    .unwrap();
    writeln!(input, "bad-json").unwrap();
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let responses = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<SystemStatusHostResponse>(line).unwrap())
        .collect::<Vec<_>>();
    assert!(matches!(
        responses[0],
        SystemStatusHostResponse::Handshake { .. }
    ));
    assert!(matches!(
        responses[1],
        SystemStatusHostResponse::Snapshot(_)
    ));
    assert!(matches!(
        responses[2],
        SystemStatusHostResponse::Rejected(_)
    ));
}
