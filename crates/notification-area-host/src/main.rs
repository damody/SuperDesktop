use std::io::{self, BufRead, Write};

use notification_area_host::NotificationRegistry;
use shell_provider_protocol::{
    MAX_FRAME_BYTES, NotificationHostResponse, NotificationMutation, validate_frame_size,
};

fn main() -> io::Result<()> {
    let mut registry = NotificationRegistry::default();
    let input = io::stdin().lock();
    let mut output = io::stdout().lock();
    for frame in input.split(b'\n') {
        let frame = frame?;
        if frame.is_empty() {
            continue;
        }
        let response = if validate_frame_size(&frame).is_err() {
            NotificationHostResponse::Rejected("frame-too-large".into())
        } else {
            match serde_json::from_slice::<NotificationMutation>(&frame) {
                Ok(mutation) => registry.apply(mutation),
                Err(error) => NotificationHostResponse::Rejected(error.to_string()),
            }
        };
        serde_json::to_writer(&mut output, &response)?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    let _ = MAX_FRAME_BYTES;
    Ok(())
}
