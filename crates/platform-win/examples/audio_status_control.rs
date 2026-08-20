use std::process::ExitCode;

use platform_win::common::system_status::{
    audio_status, set_mute_and_observe, set_volume_and_observe,
};

fn print_status() -> Result<(), String> {
    let status = audio_status()?;
    println!(
        "{{\"volume_percent\":{},\"muted\":{}}}",
        status.volume_percent, status.muted
    );
    Ok(())
}

fn run() -> Result<(), String> {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("snapshot") if arguments.next().is_none() => print_status(),
        Some("restore") => {
            let volume = arguments
                .next()
                .ok_or("missing volume")?
                .parse::<u8>()
                .map_err(|error| format!("invalid volume: {error}"))?;
            let muted = arguments
                .next()
                .ok_or("missing mute state")?
                .parse::<bool>()
                .map_err(|error| format!("invalid mute state: {error}"))?;
            if arguments.next().is_some() {
                return Err("unexpected argument".into());
            }
            set_volume_and_observe(volume)?;
            set_mute_and_observe(muted)?;
            print_status()
        }
        _ => Err("usage: audio_status_control <snapshot|restore VOLUME MUTED>".into()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("audio status control error: {error}");
            ExitCode::FAILURE
        }
    }
}
