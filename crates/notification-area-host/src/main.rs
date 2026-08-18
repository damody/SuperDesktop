use std::io::{self, BufRead, Write};

use notification_area_host::{CompatibilityAdmission, NativeCompatibilityRegistry};
use shell_provider_protocol::{
    MAX_FRAME_BYTES, NotificationHostResponse, NotificationMutation, validate_frame_size,
};

fn main() -> io::Result<()> {
    let compatibility_admission = CompatibilityAdmission::from_process_args(std::env::args());
    let mut compatibility_window = compatibility_admission
        .owns_shell_identity()
        .then(platform_win::common::notify_icon_compat::NotifyIconCompatibilityWindow::start)
        .and_then(Result::ok);
    let mut registry = NativeCompatibilityRegistry::default();
    let input = io::stdin().lock();
    let mut output = io::stdout().lock();
    for frame in input.split(b'\n') {
        if let Some((_, queue)) = compatibility_window.as_mut()
            && let Ok(mut queue) = queue.lock()
        {
            while let Some(ingress) = queue.pop() {
                let _ = registry.apply_ingress(ingress);
            }
        }
        let frame = frame?;
        if frame.is_empty() {
            continue;
        }
        let response = if validate_frame_size(&frame).is_err() {
            NotificationHostResponse::Rejected("frame-too-large".into())
        } else {
            match serde_json::from_slice::<NotificationMutation>(&frame) {
                Ok(mutation) => registry.registry.apply(mutation),
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
