//! Read-only, fail-closed profile/session admission probe.
//!
//! It neither creates a window nor invokes AppBar, Shell Hook, Explorer, or a
//! work-area mutation API. Each unsafe operation documents its local invariant.

use std::{
    ffi::c_void,
    mem::{align_of, size_of},
    process::ExitCode,
};

use platform_win::common::admission::{AdmissionInputs as Inputs, classify};

use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Security::{
        GetLengthSid, GetTokenInformation, PSID, TOKEN_QUERY, TokenGroups, TokenSessionId,
        TokenUser,
    },
    System::Threading::{
        GetCurrentProcess, GetCurrentProcessId, OpenProcess, OpenProcessToken,
        PROCESS_QUERY_LIMITED_INFORMATION,
    },
    UI::WindowsAndMessaging::{
        GetShellWindow, GetSystemMetrics, GetWindowThreadProcessId, SM_CLEANBOOT,
    },
};

const WTS_CONNECT_STATE: u32 = 8;
#[cfg(test)]
const WTS_ACTIVE: i32 = 0;
const UOI_NAME: i32 = 2;
const SE_GROUP_LOGON_ID: u32 = 0xc000_0000;
const SE_GROUP_ENABLED: u32 = 0x0000_0004;
const SE_GROUP_USE_FOR_DENY_ONLY: u32 = 0x0000_0010;

#[repr(C)]
#[derive(Clone, Copy)]
struct SidAndAttributes {
    sid: PSID,
    attributes: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct TokenUserRaw {
    user: SidAndAttributes,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct TokenGroupsRaw {
    count: u32,
    groups: [SidAndAttributes; 1],
}

struct TokenBuffer {
    words: Vec<usize>,
    byte_len: usize,
}

struct OwnedHandle(HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: this guard owns the successful Win32 handle and releases it once.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}
impl TokenBuffer {
    fn new(bytes: usize) -> Result<Self, Error> {
        let words = bytes
            .checked_add(size_of::<usize>() - 1)
            .ok_or(Error::TokenBuffer)?
            / size_of::<usize>();
        Ok(Self {
            words: vec![0; words],
            byte_len: bytes,
        })
    }
    fn capacity(&self) -> usize {
        self.words.len() * size_of::<usize>()
    }
    fn ptr(&mut self) -> *mut c_void {
        self.words.as_mut_ptr().cast()
    }
    fn contains(&self, pointer: *const u8, length: usize) -> bool {
        let start = self.words.as_ptr() as usize;
        let end = start.checked_add(self.byte_len);
        let address = pointer as usize;
        address >= start
            && address
                .checked_add(length)
                .is_some_and(|value| end.is_some_and(|limit| value <= limit))
    }
    fn read<T: Copy>(&self, offset: usize) -> Result<T, Error> {
        let address = (self.words.as_ptr() as usize)
            .checked_add(offset)
            .ok_or(Error::TokenBuffer)?;
        if offset
            .checked_add(size_of::<T>())
            .is_none_or(|end| end > self.byte_len)
            || address % align_of::<T>() != 0
        {
            return Err(Error::TokenBuffer);
        } /* SAFETY: usize backing guarantees alignment and checked range guarantees complete initialized record. */
        Ok(unsafe { (self.words.as_ptr().cast::<u8>().add(offset).cast::<T>()).read() })
    }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn ProcessIdToSessionId(process_id: u32, session_id: *mut u32) -> i32;
    fn GetWindowsDirectoryW(buffer: *mut u16, size: u32) -> u32;
    fn QueryFullProcessImageNameW(
        process: HANDLE,
        flags: u32,
        buffer: *mut u16,
        size: *mut u32,
    ) -> i32;
    fn LocalFree(memory: *mut c_void) -> *mut c_void;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn ConvertSidToStringSidW(sid: PSID, text: *mut *mut u16) -> i32;
}

#[link(name = "wtsapi32")]
unsafe extern "system" {
    fn WTSQuerySessionInformationW(
        server: *mut c_void,
        session_id: u32,
        info_class: u32,
        buffer: *mut *mut c_void,
        bytes_returned: *mut u32,
    ) -> i32;
    fn WTSFreeMemory(memory: *mut c_void);
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetProcessWindowStation() -> *mut c_void;
    fn GetUserObjectInformationW(
        object: *mut c_void,
        index: i32,
        buffer: *mut c_void,
        length: u32,
        needed: *mut u32,
    ) -> i32;
}

#[derive(Clone, Copy, Debug)]
enum Error {
    OpenToken,
    TokenSession,
    TokenUser,
    TokenGroups,
    TokenBuffer,
    TokenIdentity,
    ProcessSession,
    WtsState,
    WindowStation,
}

impl Error {
    const fn name(&self) -> &'static str {
        match self {
            Self::OpenToken => "open-process-token",
            Self::TokenSession => "token-session-id",
            Self::TokenUser => "token-user",
            Self::TokenGroups => "token-groups",
            Self::TokenBuffer => "token-buffer",
            Self::TokenIdentity => "token-identity",
            Self::ProcessSession => "process-session-id",
            Self::WtsState => "wts-connect-state",
            Self::WindowStation => "window-station",
        }
    }
}

fn sid_text(buffer: &TokenBuffer, sid: PSID) -> Result<String, Error> {
    if sid.0.is_null()
        || !(sid.0 as usize).is_multiple_of(align_of::<u32>())
        || !buffer.contains(sid.0.cast(), 8)
    {
        return Err(Error::TokenIdentity);
    }
    let sid_bytes = sid.0.cast::<u8>();
    // A SID has an 8-byte fixed header; byte 1 is the SubAuthorityCount, then
    // that many 32-bit subauthorities. Validate this untrusted token-buffer
    // layout before allowing the Win32 helper to inspect it.
    // SAFETY: the checked fixed header makes the SubAuthorityCount byte readable.
    let subauthority_count = unsafe { *sid_bytes.add(1) } as usize;
    let structural_length = 8_usize
        .checked_add(
            subauthority_count
                .checked_mul(size_of::<u32>())
                .ok_or(Error::TokenIdentity)?,
        )
        .ok_or(Error::TokenIdentity)?;
    if !buffer.contains(sid_bytes, structural_length) {
        return Err(Error::TokenIdentity);
    }
    // SAFETY: SID begins inside an API-returned token buffer; GetLengthSid reads its layout.
    let length = unsafe { GetLengthSid(sid) } as usize;
    if length != structural_length || !buffer.contains(sid.0.cast(), length) {
        return Err(Error::TokenIdentity);
    }
    let mut text = std::ptr::null_mut();
    // SAFETY: sid belongs to a live TokenUser/TokenGroups buffer; Win32 allocates
    // `text`, which is converted before being released exactly once with LocalFree.
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 || text.is_null() {
        return Err(Error::TokenIdentity);
    }
    let mut length = 0;
    // SAFETY: ConvertSidToStringSidW returns a null-terminated UTF-16 sequence.
    while unsafe { *text.add(length) } != 0 {
        length += 1;
    }
    // SAFETY: `text..text+length` contains initialized UTF-16 units.
    let result = String::from_utf16(unsafe { std::slice::from_raw_parts(text, length) })
        .map_err(|_| Error::TokenIdentity);
    // SAFETY: ConvertSidToStringSidW documented LocalAlloc ownership for text.
    unsafe { LocalFree(text.cast()) };
    result
}

fn token_bytes(
    token: HANDLE,
    class: windows::Win32::Security::TOKEN_INFORMATION_CLASS,
) -> Result<TokenBuffer, Error> {
    let error = if class == TokenUser {
        Error::TokenUser
    } else {
        Error::TokenGroups
    };
    let mut needed = 0_u32;
    // SAFETY: null/zero is the documented size query and only reads token metadata.
    let _ = unsafe { GetTokenInformation(token, class, None, 0, &mut needed) };
    if needed == 0 {
        return Err(error);
    }
    let mut bytes = TokenBuffer::new(needed as usize)?;
    let capacity = bytes.capacity();
    let mut returned = 0_u32;
    // SAFETY: aligned owned storage has at least requested capacity; API only writes it.
    unsafe {
        GetTokenInformation(
            token,
            class,
            Some(bytes.ptr()),
            capacity as u32,
            &mut returned,
        )
    }
    .map_err(|_| error)?;
    if returned == 0 || returned as usize > capacity {
        return Err(error);
    }
    bytes.byte_len = returned as usize;
    Ok(bytes)
}

fn token_details(token: HANDLE) -> Result<(u32, String, bool, String), Error> {
    let mut session = 0_u32;
    let mut returned = 0_u32;
    // SAFETY: session and returned are valid writable out-pointers of correct size.
    unsafe {
        GetTokenInformation(
            token,
            TokenSessionId,
            Some((&mut session as *mut u32).cast()),
            size_of::<u32>() as u32,
            &mut returned,
        )
    }
    .map_err(|_| Error::TokenSession)?;
    if returned < size_of::<u32>() as u32 {
        return Err(Error::TokenSession);
    }
    let user = token_bytes(token, TokenUser)?;
    let user_record: TokenUserRaw = user.read(0).map_err(|_| Error::TokenUser)?;
    let user_sid = sid_text(&user, user_record.user.sid)?;
    let groups = token_bytes(token, TokenGroups)?;
    let header: TokenGroupsRaw = groups.read(0).map_err(|_| Error::TokenGroups)?;
    let entries = (header.count as usize)
        .checked_mul(size_of::<SidAndAttributes>())
        .ok_or(Error::TokenGroups)?;
    let first_group = std::mem::offset_of!(TokenGroupsRaw, groups);
    let minimum = first_group.checked_add(entries).ok_or(Error::TokenGroups)?;
    if groups.byte_len < minimum {
        return Err(Error::TokenGroups);
    }
    let mut interactive = false;
    let mut logon_sid = None;
    for index in 0..header.count as usize {
        let offset = first_group
            .checked_add(
                index
                    .checked_mul(size_of::<SidAndAttributes>())
                    .ok_or(Error::TokenGroups)?,
            )
            .ok_or(Error::TokenGroups)?;
        let group: SidAndAttributes = groups.read(offset).map_err(|_| Error::TokenGroups)?;
        let sid = sid_text(&groups, group.sid)?;
        let enabled_identity_group = group.attributes & SE_GROUP_ENABLED != 0
            && group.attributes & SE_GROUP_USE_FOR_DENY_ONLY == 0;
        interactive |= sid == "S-1-5-4" && enabled_identity_group;
        if group.attributes & SE_GROUP_LOGON_ID == SE_GROUP_LOGON_ID
            && enabled_identity_group
            && logon_sid.replace(sid).is_some()
        {
            return Err(Error::TokenGroups);
        }
    }
    Ok((
        session,
        user_sid,
        interactive,
        logon_sid.ok_or(Error::TokenIdentity)?,
    ))
}

fn token_identity() -> Result<(u32, String, bool, String), Error> {
    let mut token = HANDLE::default();
    // SAFETY: current-process pseudo-handle + TOKEN_QUERY grants only read access;
    // a successful real token handle is closed exactly once below.
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
        .map_err(|_| Error::OpenToken)?;
    let result = token_details(token);
    // SAFETY: OpenProcessToken returned this owned handle; close exactly once.
    unsafe { CloseHandle(token) }.map_err(|_| Error::OpenToken)?;
    result
}

fn is_system_explorer_path(image: &str, windows_directory: &str) -> bool {
    image.eq_ignore_ascii_case(&format!(
        "{}\\explorer.exe",
        windows_directory.trim_end_matches('\\')
    ))
}

fn shell_owner_identity(session: u32) -> Result<(String, String), Error> {
    // SAFETY: queries the OS-designated shell window; it neither creates nor mutates a HWND.
    let shell = unsafe { GetShellWindow() };
    if shell.is_invalid() {
        return Err(Error::TokenIdentity);
    }
    let mut pid = 0_u32;
    // SAFETY: valid shell HWND and writable PID output; this is a metadata query.
    unsafe { GetWindowThreadProcessId(shell, Some(&mut pid)) };
    if pid == 0 {
        return Err(Error::TokenIdentity);
    }
    let mut shell_session = 0_u32;
    // SAFETY: shell PID and writable session out-pointer are valid; query only.
    if unsafe { ProcessIdToSessionId(pid, &mut shell_session) } == 0 || shell_session != session {
        return Err(Error::TokenIdentity);
    }
    // SAFETY: read-only query process handle, closed exactly once below.
    let process = OwnedHandle(
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
            .map_err(|_| Error::TokenIdentity)?,
    );
    let mut windows_directory = vec![0_u16; 32768];
    // SAFETY: supplied UTF-16 buffer is valid writable storage for Windows directory.
    let windows_length = unsafe {
        GetWindowsDirectoryW(
            windows_directory.as_mut_ptr(),
            windows_directory.len() as u32,
        )
    } as usize;
    if windows_length == 0 || windows_length >= windows_directory.len() {
        // SAFETY: OpenProcess returned this owned handle exactly once.
        return Err(Error::TokenIdentity);
    }
    let windows_directory = match String::from_utf16(&windows_directory[..windows_length]) {
        Ok(value) => value,
        Err(_) => {
            // SAFETY: OpenProcess returned this owned handle exactly once.
            return Err(Error::TokenIdentity);
        }
    };
    let mut image = vec![0_u16; 32768];
    let mut image_length = image.len() as u32;
    // SAFETY: query-only process handle and writable UTF-16 buffer/count are valid.
    let image_ok =
        unsafe { QueryFullProcessImageNameW(process.0, 0, image.as_mut_ptr(), &mut image_length) }
            != 0;
    let verified_image = image_ok
        && (image_length as usize) < image.len()
        && String::from_utf16(&image[..image_length as usize])
            .ok()
            .is_some_and(|value| is_system_explorer_path(&value, &windows_directory));
    if !verified_image {
        // SAFETY: OpenProcess returned this owned handle exactly once.
        return Err(Error::TokenIdentity);
    }
    let mut token = HANDLE::default();
    // SAFETY: requests query-only token metadata; result handle is closed below.
    let opened = unsafe { OpenProcessToken(process.0, TOKEN_QUERY, &mut token) };
    let result = if opened.is_ok() {
        token_details(token).and_then(|(token_session, user_sid, _, logon_sid)| {
            (token_session == session)
                .then_some((user_sid, logon_sid))
                .ok_or(Error::TokenIdentity)
        })
    } else {
        Err(Error::TokenIdentity)
    };
    let _token = opened.is_ok().then_some(OwnedHandle(token));
    result
}

fn process_session() -> Result<u32, Error> {
    let mut session = 0_u32;
    // SAFETY: this PID and out-pointer are valid; the API only queries session metadata.
    (unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) } != 0)
        .then_some(session)
        .ok_or(Error::ProcessSession)
}

fn wts_state(session: u32) -> Result<i32, Error> {
    let mut buffer = std::ptr::null_mut();
    let mut bytes = 0_u32;
    // SAFETY: null server is WTS_CURRENT_SERVER_HANDLE and both result pointers are valid.
    let ok = unsafe {
        WTSQuerySessionInformationW(
            std::ptr::null_mut(),
            session,
            WTS_CONNECT_STATE,
            &mut buffer,
            &mut bytes,
        )
    };
    let result = if ok == 0 || buffer.is_null() || bytes < size_of::<i32>() as u32 {
        Err(Error::WtsState)
    } else {
        // SAFETY: success plus size check permits an i32 read from WTS-owned memory.
        Ok(unsafe { *(buffer as *const i32) })
    };
    if !buffer.is_null() {
        // SAFETY: any non-null WTS result buffer is released exactly once, including error paths.
        unsafe { WTSFreeMemory(buffer) };
    }
    result
}

fn station_name() -> Result<String, Error> {
    // SAFETY: obtains the process-associated station without changing it.
    let station = unsafe { GetProcessWindowStation() };
    if station.is_null() {
        return Err(Error::WindowStation);
    }
    let mut bytes = 0_u32;
    // SAFETY: null/zero asks for UOI_NAME length only.
    unsafe { GetUserObjectInformationW(station, UOI_NAME, std::ptr::null_mut(), 0, &mut bytes) };
    if bytes < size_of::<u16>() as u32 {
        return Err(Error::WindowStation);
    }
    let mut utf16 = vec![0_u16; bytes.div_ceil(size_of::<u16>() as u32) as usize];
    // SAFETY: utf16 supplies `bytes` writable storage for UOI_NAME.
    if unsafe {
        GetUserObjectInformationW(
            station,
            UOI_NAME,
            utf16.as_mut_ptr().cast(),
            bytes,
            &mut bytes,
        )
    } == 0
    {
        return Err(Error::WindowStation);
    }
    let end = utf16
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(utf16.len());
    String::from_utf16(&utf16[..end]).map_err(|_| Error::WindowStation)
}

fn collect() -> Result<Inputs, Error> {
    // SAFETY: SM_CLEANBOOT is a read-only system metric with no pointer input.
    let clean_boot = unsafe { GetSystemMetrics(SM_CLEANBOOT) };
    let (token_session_id, token_user_sid, interactive_group, token_logon_sid) = token_identity()?;
    let process_session_id = process_session()?;
    let (shell_owner_sid, shell_owner_logon_sid) = shell_owner_identity(process_session_id)?;
    Ok(Inputs {
        clean_boot,
        token_session_id,
        process_session_id,
        wts_state: wts_state(process_session_id)?,
        window_station: station_name()?,
        token_user_sid,
        token_logon_sid,
        interactive_group,
        logon_group: true,
        shell_owner_sid,
        shell_owner_logon_sid,
    })
}

fn main() -> ExitCode {
    let (disposition, admitted, detail) = match collect() {
        Ok(input) => {
            let (d, a) = classify(&input);
            (
                d,
                a,
                format!(
                    ",\"safe_mode_clean_boot\":{},\"token_session_id\":{},\"process_session_id\":{},\"wts_connect_state\":{},\"window_station\":{:?},\"token_user_sid\":{:?},\"token_logon_sid\":{:?},\"shell_owner_sid\":{:?},\"shell_owner_logon_sid\":{:?},\"interactive_group\":{},\"logon_group\":{}",
                    input.clean_boot,
                    input.token_session_id,
                    input.process_session_id,
                    input.wts_state,
                    input.window_station,
                    input.token_user_sid,
                    input.token_logon_sid,
                    input.shell_owner_sid,
                    input.shell_owner_logon_sid,
                    input.interactive_group,
                    input.logon_group
                ),
            )
        }
        Err(error) => (error.name(), false, String::new()),
    };
    println!(
        "{{\"probe\":\"capability-profile-read-only\",\"disposition\":\"{disposition}\",\"admitted\":{admitted},\"mutations_attempted\":false{detail}}}"
    );
    if admitted {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}

#[cfg(test)]
mod tests {
    use super::{Inputs, WTS_ACTIVE, classify};
    fn admitted() -> Inputs {
        Inputs {
            clean_boot: 0,
            token_session_id: 1,
            process_session_id: 1,
            wts_state: WTS_ACTIVE,
            window_station: "WinSta0".into(),
            token_user_sid: "S-1-5-21-1".into(),
            token_logon_sid: "S-1-5-5-1-2".into(),
            interactive_group: true,
            logon_group: true,
            shell_owner_sid: "S-1-5-21-1".into(),
            shell_owner_logon_sid: "S-1-5-5-1-2".into(),
        }
    }
    #[test]
    fn admits_supported_interactive_session() {
        assert_eq!(classify(&admitted()), ("admitted", true));
    }
    #[test]
    fn fails_closed_for_safe_mode_and_token_fixtures() {
        let mut input = admitted();
        input.clean_boot = 1;
        assert_eq!(classify(&input), ("safe-mode", false));
        let mut input = admitted();
        input.token_session_id = 2;
        assert_eq!(classify(&input), ("token-session-mismatch", false));
        let mut input = admitted();
        input.token_user_sid = "S-1-5-18".into();
        assert_eq!(
            classify(&input),
            ("service-system-or-anonymous-token", false)
        );
        let mut input = admitted();
        input.interactive_group = false;
        assert_eq!(
            classify(&input),
            ("non-interactive-or-foreign-token", false)
        );
        // Restricted-token fixture: same session/user/logon identity but the
        // INTERACTIVE group is deny-only or disabled, so it must not admit.
        let mut input = admitted();
        input.interactive_group = false;
        assert_eq!(
            classify(&input),
            ("non-interactive-or-foreign-token", false)
        );
        let mut input = admitted();
        input.shell_owner_sid = "S-1-5-21-foreign".into();
        assert_eq!(classify(&input), ("shell-owner-token-mismatch", false));
        let mut input = admitted();
        input.shell_owner_logon_sid = "S-1-5-5-foreign".into();
        assert_eq!(classify(&input), ("shell-owner-token-mismatch", false));
        let mut input = admitted();
        input.window_station = "Service-0x0-3e7$".into();
        assert_eq!(classify(&input), ("non-interactive-window-station", false));
    }

    #[test]
    fn rejects_same_named_non_system_explorer_fixture() {
        assert!(!super::is_system_explorer_path(
            r"C:\tmp\explorer.exe",
            r"C:\Windows"
        ));
        assert!(super::is_system_explorer_path(
            r"C:\Windows\explorer.exe",
            r"C:\Windows"
        ));
    }
}
