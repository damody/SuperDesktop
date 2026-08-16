use std::io::Write;
use std::process::{Command, Stdio};

use shell_provider_protocol::{
    CURRENT_PROTOCOL, Envelope, ProviderRequest, ProviderResponse, ResponseBody, TerminalKind,
};

#[test]
fn process_handshakes_rejects_invalid_input_and_stops_on_eof() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_shell-provider-host"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let request = Envelope {
        protocol: CURRENT_PROTOCOL,
        request_id: "handshake".into(),
        correlation_id: "integration".into(),
        deadline_unix_ms: None,
        payload: ProviderRequest::Handshake,
    };
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{}", serde_json::to_string(&request).unwrap()).unwrap();
    writeln!(stdin, "not-json").unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let responses: Vec<ProviderResponse> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses.len(), 2);
    assert!(matches!(responses[0].body, ResponseBody::Handshake(_)));
    assert_eq!(responses[1].terminal, TerminalKind::InvalidRequest);
}
