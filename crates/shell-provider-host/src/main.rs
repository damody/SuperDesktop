use std::io::{self, BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use shell_provider_host::Dispatcher;
use shell_provider_protocol::{
    Envelope, MAX_FRAME_BYTES, ProviderRequest, ProviderResponse, ResponseBody, TerminalKind,
    contract_manifest, validate_frame_size,
};

fn main() -> io::Result<()> {
    if std::env::args().any(|argument| argument == "--manifest") {
        println!(
            "{}",
            serde_json::to_string_pretty(&contract_manifest()).expect("manifest serializes")
        );
        return Ok(());
    }

    run(io::stdin().lock(), io::stdout().lock())
}

fn run(input: impl BufRead, mut output: impl Write) -> io::Result<()> {
    let mut dispatcher = Dispatcher::default();
    for frame in input.split(b'\n') {
        let frame = frame?;
        if frame.is_empty() {
            continue;
        }
        let response = match validate_frame_size(&frame) {
            Ok(()) => match serde_json::from_slice::<Envelope<ProviderRequest>>(&frame) {
                Ok(request) => dispatcher.dispatch(request, now_unix_ms()),
                Err(error) => invalid_response(error.to_string()),
            },
            Err(error) => invalid_response(error.to_string()),
        };
        serde_json::to_writer(&mut output, &response)?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn invalid_response(message: String) -> ProviderResponse {
    ProviderResponse {
        request_id: "invalid-frame".into(),
        correlation_id: "invalid-frame".into(),
        terminal: TerminalKind::InvalidRequest,
        body: ResponseBody::Message(message),
    }
}

const _: usize = MAX_FRAME_BYTES;
