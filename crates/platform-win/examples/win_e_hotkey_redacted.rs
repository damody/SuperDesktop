use std::time::{Duration, Instant};

fn main() {
    let expected = match std::env::args().nth(1).as_deref() {
        Some("desktop") => platform_win::common::shell_hotkey::ShellHotkeyAction::ShowDesktop,
        Some("input") => platform_win::common::shell_hotkey::ShellHotkeyAction::CycleInput,
        _ => platform_win::common::shell_hotkey::ShellHotkeyAction::OpenExplorer,
    };
    let hotkey = match platform_win::common::shell_hotkey::ShellHotkeys::start() {
        Ok(hotkey) => hotkey,
        Err(error) => {
            eprintln!("Shell hotkey hook unavailable: {error}");
            std::process::exit(1);
        }
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if hotkey.take_requested() == Some(expected) {
            println!("shell_hotkey_requested={expected:?}");
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    eprintln!("Expected shell hotkey was not observed within 5 seconds");
    std::process::exit(2);
}
