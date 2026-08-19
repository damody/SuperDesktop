use std::time::{Duration, Instant};

fn main() {
    let hotkey = match platform_win::common::shell_hotkey::WinEHotkey::start() {
        Ok(hotkey) => hotkey,
        Err(error) => {
            eprintln!("Win+E hook unavailable: {error}");
            std::process::exit(1);
        }
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if hotkey.take_requested() {
            println!("win_e_requested=true");
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    eprintln!("Win+E was not observed within 5 seconds");
    std::process::exit(2);
}
