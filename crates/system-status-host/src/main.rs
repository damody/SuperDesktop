use std::io::{self, BufRead, Write};

use shell_provider_protocol::{
    SystemStatusHostRequest, SystemStatusHostResponse, validate_frame_size,
};
use system_status_host::{SystemStatusRuntime, register_provider_callbacks};

fn main() -> io::Result<()> {
    let mut runtime = SystemStatusRuntime::default();
    let (callback_events, _callback_registration) = register_provider_callbacks();
    runtime.attach_provider_callbacks(callback_events);
    let input = io::stdin().lock();
    let mut output = io::stdout().lock();
    for frame in input.split(b'\n') {
        let frame = frame?;
        if frame.is_empty() {
            continue;
        }
        let response = if validate_frame_size(&frame).is_err() {
            SystemStatusHostResponse::Rejected("frame-too-large".into())
        } else {
            match serde_json::from_slice::<SystemStatusHostRequest>(&frame) {
                Ok(request) => runtime.apply(request),
                Err(error) => SystemStatusHostResponse::Rejected(error.to_string()),
            }
        };
        serde_json::to_writer(&mut output, &response)?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}
