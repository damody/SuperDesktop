//! Session-scoped kernel owner mutex with live process/token/file fencing.

use std::{
    ffi::c_void,
    mem::size_of,
    ptr::null_mut,
    sync::atomic::{AtomicBool, Ordering},
};

use super::guardian_lease::{FileIdentity, canonical_file_identity};

type RawHandle = isize;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_ABANDONED: u32 = 0x80;
const WAIT_TIMEOUT: u32 = 258;
const TOKEN_QUERY: u32 = 0x0008;
const TOKEN_USER_CLASS: u32 = 1;
const TOKEN_STATISTICS_CLASS: u32 = 10;
static PROCESS_OWNS_SESSION: AtomicBool = AtomicBool::new(false);

#[repr(C)]
#[derive(Clone, Copy)]
struct FileTime {
    low: u32,
    high: u32,
}
#[repr(C)]
struct SidAndAttributes {
    sid: *mut c_void,
    attributes: u32,
}
#[repr(C)]
struct TokenUser {
    user: SidAndAttributes,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Luid {
    low: u32,
    high: i32,
}
#[repr(C)]
struct TokenStatistics {
    token_id: Luid,
    authentication_id: Luid,
    expiration_time: i64,
    token_type: u32,
    impersonation_level: u32,
    dynamic_charged: u32,
    dynamic_available: u32,
    group_count: u32,
    privilege_count: u32,
    modified_id: Luid,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(attrs: *mut c_void, initial_owner: i32, name: *const u16) -> RawHandle;
    fn WaitForSingleObject(handle: RawHandle, milliseconds: u32) -> u32;
    fn ReleaseMutex(handle: RawHandle) -> i32;
    fn CloseHandle(handle: RawHandle) -> i32;
    fn GetCurrentProcess() -> RawHandle;
    fn GetCurrentProcessId() -> u32;
    fn ProcessIdToSessionId(pid: u32, session: *mut u32) -> i32;
    fn GetProcessTimes(
        handle: RawHandle,
        creation: *mut FileTime,
        exit: *mut FileTime,
        kernel: *mut FileTime,
        user: *mut FileTime,
    ) -> i32;
    fn QueryFullProcessImageNameW(
        handle: RawHandle,
        flags: u32,
        path: *mut u16,
        length: *mut u32,
    ) -> i32;
}
#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(process: RawHandle, access: u32, token: *mut RawHandle) -> i32;
    fn GetTokenInformation(
        token: RawHandle,
        class: u32,
        info: *mut c_void,
        length: u32,
        returned: *mut u32,
    ) -> i32;
    fn GetLengthSid(sid: *mut c_void) -> u32;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessOwnerIdentity {
    pub pid: u32,
    pub creation_time: u64,
    pub session_id: u32,
    pub user_sid_hex: String,
    pub authentication_id: u64,
    pub executable: String,
    pub file: FileIdentity,
}

pub struct SessionOwnerMutex {
    handle: RawHandle,
    identity: ProcessOwnerIdentity,
    released: bool,
}

impl SessionOwnerMutex {
    pub fn acquire() -> Result<Self, &'static str> {
        if PROCESS_OWNS_SESSION
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("already-owned");
        }
        let reset = || PROCESS_OWNS_SESSION.store(false, Ordering::Release);
        let identity = current_identity().inspect_err(|_| reset())?;
        let name = format!("Local\\SuperDesktop.Owner.Session.{}", identity.session_id)
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // SAFETY: name is terminated and default security uses the current token DACL.
        let handle = unsafe { CreateMutexW(null_mut(), 0, name.as_ptr()) };
        if handle == 0 {
            reset();
            return Err("owner-mutex-create");
        }
        // SAFETY: live owned mutex handle and non-blocking acquisition.
        match unsafe { WaitForSingleObject(handle, 0) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => Ok(Self {
                handle,
                identity,
                released: false,
            }),
            WAIT_TIMEOUT => {
                // SAFETY: this path owns the opened handle but not the mutex.
                let _ = unsafe { CloseHandle(handle) };
                reset();
                Err("already-owned")
            }
            _ => {
                // SAFETY: this path owns the opened handle but not the mutex.
                let _ = unsafe { CloseHandle(handle) };
                reset();
                Err("owner-mutex-wait")
            }
        }
    }

    pub fn identity(&self) -> &ProcessOwnerIdentity {
        &self.identity
    }

    pub fn revalidate(&self) -> Result<(), &'static str> {
        if self.released || current_identity()? != self.identity {
            Err("owner-identity-drift")
        } else {
            Ok(())
        }
    }

    pub fn release(mut self) -> Result<(), &'static str> {
        self.revalidate()?;
        // SAFETY: this instance owns the mutex acquisition exactly once.
        if unsafe { ReleaseMutex(self.handle) } == 0 {
            return Err("owner-mutex-release");
        }
        self.released = true;
        PROCESS_OWNS_SESSION.store(false, Ordering::Release);
        Ok(())
    }
}

impl Drop for SessionOwnerMutex {
    fn drop(&mut self) {
        if !self.released {
            // SAFETY: best-effort release only for this instance's acquisition.
            let _ = unsafe { ReleaseMutex(self.handle) };
            PROCESS_OWNS_SESSION.store(false, Ordering::Release);
        }
        // SAFETY: this instance owns this kernel handle.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

fn current_identity() -> Result<ProcessOwnerIdentity, &'static str> {
    // SAFETY: pseudo-handle is valid for synchronous read-only identity queries.
    let process = unsafe { GetCurrentProcess() };
    // SAFETY: read-only current process id.
    let pid = unsafe { GetCurrentProcessId() };
    let mut session_id = 0;
    // SAFETY: session output points to writable storage.
    if unsafe { ProcessIdToSessionId(pid, &mut session_id) } == 0 {
        return Err("owner-session");
    }
    let zero = FileTime { low: 0, high: 0 };
    let (mut creation, mut exit, mut kernel, mut user) = (zero, zero, zero, zero);
    // SAFETY: all FILETIME outputs are writable locals.
    if unsafe { GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return Err("owner-creation-time");
    }
    let creation_time = ((creation.high as u64) << 32) | creation.low as u64;
    let mut path = vec![0u16; 32_768];
    let mut length = path.len() as u32;
    // SAFETY: path buffer and in/out length are valid.
    if unsafe { QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length) } == 0 {
        return Err("owner-image");
    }
    let executable_path = String::from_utf16_lossy(&path[..length as usize]);
    let (executable, file) = canonical_file_identity(&executable_path).map_err(|_| "owner-file")?;
    let mut token = 0;
    // SAFETY: token output is writable and closed below.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return Err("owner-token");
    }
    let token_result = token_identity(token);
    // SAFETY: this function owns the opened token handle.
    let _ = unsafe { CloseHandle(token) };
    let (user_sid_hex, authentication_id) = token_result?;
    Ok(ProcessOwnerIdentity {
        pid,
        creation_time,
        session_id,
        user_sid_hex,
        authentication_id,
        executable,
        file,
    })
}

fn token_identity(token: RawHandle) -> Result<(String, u64), &'static str> {
    let user = query_token(token, TOKEN_USER_CLASS)?;
    if user.len() < size_of::<TokenUser>() {
        return Err("owner-token-user-size");
    }
    // SAFETY: GetTokenInformation returned a complete TOKEN_USER prefix in `user`.
    let token_user = unsafe { &*(user.as_ptr().cast::<TokenUser>()) };
    // SAFETY: SID pointer belongs to the live query buffer and is read-only here.
    let sid_len = unsafe { GetLengthSid(token_user.user.sid) } as usize;
    if sid_len == 0 {
        return Err("owner-token-sid");
    }
    // SAFETY: GetLengthSid reported the exact readable SID byte count.
    let sid = unsafe { std::slice::from_raw_parts(token_user.user.sid.cast::<u8>(), sid_len) };
    let stats = query_token(token, TOKEN_STATISTICS_CLASS)?;
    if stats.len() < size_of::<TokenStatistics>() {
        return Err("owner-token-statistics-size");
    }
    // SAFETY: GetTokenInformation returned a complete TOKEN_STATISTICS value.
    let stats = unsafe { &*(stats.as_ptr().cast::<TokenStatistics>()) };
    let authentication_id =
        ((stats.authentication_id.high as u32 as u64) << 32) | stats.authentication_id.low as u64;
    Ok((
        sid.iter().map(|byte| format!("{byte:02x}")).collect(),
        authentication_id,
    ))
}

fn query_token(token: RawHandle, class: u32) -> Result<Vec<u8>, &'static str> {
    let mut needed = 0;
    // SAFETY: documented size query.
    let _ = unsafe { GetTokenInformation(token, class, null_mut(), 0, &mut needed) };
    if needed == 0 {
        return Err("owner-token-info-size");
    }
    let mut value = vec![0u8; needed as usize];
    // SAFETY: value owns the returned byte capacity and returned is writable.
    if unsafe { GetTokenInformation(token, class, value.as_mut_ptr().cast(), needed, &mut needed) }
        == 0
    {
        return Err("owner-token-info");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_mutex_fences_second_owner_and_revalidates_all_identity_fields() {
        let first = SessionOwnerMutex::acquire().unwrap();
        assert_eq!(SessionOwnerMutex::acquire().err(), Some("already-owned"));
        assert!(first.revalidate().is_ok());
        assert!(!first.identity().user_sid_hex.is_empty());
        assert!(first.identity().file.file_index != 0);
        first.release().unwrap();
        SessionOwnerMutex::acquire().unwrap().release().unwrap();
    }
}
