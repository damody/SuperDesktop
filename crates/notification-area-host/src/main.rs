use std::{
    io::{self, BufRead, Write},
    time::{SystemTime, UNIX_EPOCH},
};

use notification_area_host::{CompatibilityAdmission, NativeCompatibilityRegistry};
use shell_provider_protocol::{
    MAX_FRAME_BYTES, NotificationHostResponse, NotificationMutation, validate_frame_size,
};

fn main() -> io::Result<()> {
    let compatibility_admission = CompatibilityAdmission::from_process_args(std::env::args());
    let mut compatibility_window = if compatibility_admission.owns_shell_identity() {
        let window =
            platform_win::common::notify_icon_compat::NotifyIconCompatibilityWindow::start()
                .map_err(io::Error::other)?;
        platform_win::common::notify_icon_compat::broadcast_taskbar_created()
            .map_err(io::Error::other)?;
        Some(window)
    } else {
        None
    };
    let mut registry = NativeCompatibilityRegistry::default();
    let input = io::stdin().lock();
    let mut output = io::stdout().lock();
    for frame in input.split(b'\n') {
        let _ = registry.reconcile_dead_clients();
        if let Some((_, queue)) = compatibility_window.as_mut()
            && let Ok(mut queue) = queue.lock()
        {
            while let Some(ingress) = queue.pop() {
                let terminal = registry.apply_ingress(ingress);
                trace_compatibility(&format!("terminal={terminal:?}"));
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
                Ok(mutation) => {
                    let response = registry.apply_mutation(mutation, unix_time_ms());
                    if let NotificationHostResponse::Snapshot(snapshot) = &response {
                        trace_compatibility(&format!("snapshot-icons={}", snapshot.icons.len()));
                    }
                    response
                }
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

fn trace_compatibility(message: &str) {
    let Some(path) = std::env::var_os("SUPERDESKTOP_NOTIFYICON_TRACE") else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{message}");
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_millis() as u64)
}
