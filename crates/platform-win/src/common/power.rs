//! Explicitly confirmed interactive session power actions.

use windows::Win32::System::Shutdown::{
    EWX_LOGOFF, EWX_POWEROFF, EWX_REBOOT, ExitWindowsEx, SHTDN_REASON_FLAG_PLANNED,
    SHTDN_REASON_MAJOR_OTHER,
};
use windows::Win32::UI::WindowsAndMessaging::{
    IDYES, MB_DEFBUTTON2, MB_ICONWARNING, MB_TASKMODAL, MB_YESNO, MessageBoxW,
};
use windows::core::PCWSTR;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionPowerAction {
    SignOut,
    Restart,
    ShutDown,
}

pub fn confirm_and_execute(action: SessionPowerAction) -> Result<bool, String> {
    let (question, flags) = match action {
        SessionPowerAction::SignOut => ("Sign out of Windows now?", EWX_LOGOFF),
        SessionPowerAction::Restart => ("Restart Windows now?", EWX_REBOOT),
        SessionPowerAction::ShutDown => ("Shut down Windows now?", EWX_POWEROFF),
    };
    let question = wide(question);
    let caption = wide("SuperDesktop power action");
    // SAFETY: The strings are owned, NUL-terminated UTF-16 buffers. No owner
    // HWND is passed; MB_TASKMODAL prevents accidental interaction behind the
    // prompt and the second (No) button is the default.
    let answer = unsafe {
        MessageBoxW(
            None,
            PCWSTR(question.as_ptr()),
            PCWSTR(caption.as_ptr()),
            MB_YESNO | MB_DEFBUTTON2 | MB_ICONWARNING | MB_TASKMODAL,
        )
    };
    if answer != IDYES {
        return Ok(false);
    }
    // SAFETY: The action is a closed enum mapped to documented interactive
    // session flags and is reachable only after the explicit confirmation.
    unsafe { ExitWindowsEx(flags, SHTDN_REASON_MAJOR_OTHER | SHTDN_REASON_FLAG_PLANNED) }
        .map_err(|error| error.to_string())?;
    Ok(true)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_copy_is_nul_terminated() {
        for action in [
            SessionPowerAction::SignOut,
            SessionPowerAction::Restart,
            SessionPowerAction::ShutDown,
        ] {
            let label = format!("{action:?}");
            let value = wide(&label);
            assert_eq!(value.last(), Some(&0));
            assert_eq!(value.iter().filter(|unit| **unit == 0).count(), 1);
        }
    }
}
