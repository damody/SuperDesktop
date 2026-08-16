use std::io::Write;
use std::process::{Command, Stdio};

use shell_provider_protocol::{NotificationHostResponse, NotificationMutation};

#[test]
fn host_handles_lifecycle_malformed_input_and_eof() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_notification-area-host"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    writeln!(
        input,
        "{}",
        serde_json::to_string(&NotificationMutation::RegisterClient {
            client_id: "client".into()
        })
        .unwrap()
    )
    .unwrap();
    writeln!(input, "bad-json").unwrap();
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let responses: Vec<NotificationHostResponse> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(matches!(
        responses[0],
        NotificationHostResponse::Accepted { changed: true, .. }
    ));
    assert!(matches!(
        responses[1],
        NotificationHostResponse::Rejected(_)
    ));
}
