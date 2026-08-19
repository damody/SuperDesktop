//! Preview-only restricted guardian lease transport.
//!
//! The parent writes a sealed claim once to an inherited read-only pipe.  The
//! child receives no authority-bearing identity in argv: it verifies the claim
//! against the inherited parent process handle, then waits only for that parent.

use std::{
    ffi::c_void,
    mem::size_of,
    ptr::null_mut,
    time::{Duration, Instant},
};

use super::native_window::{ResourceSnapshot, resource_snapshot};

type RawHandle = isize;
const INVALID_HANDLE: RawHandle = -1;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const SYNCHRONIZE: u32 = 0x0010_0000;
const HANDLE_FLAG_INHERIT: u32 = 1;
const EXTENDED_STARTUPINFO_PRESENT: u32 = 0x0008_0000;
const PROC_THREAD_ATTRIBUTE_HANDLE_LIST: usize = 0x0002_0002;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_TIMEOUT: u32 = 258;
const WAIT_FAILED: u32 = 0xffff_ffff;
const FILE_TYPE_PIPE: u32 = 3;
const FILE_READ_ATTRIBUTES: u32 = 0x80;
const FILE_SHARE_ALL: u32 = 7;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const INVALID_FILE_ATTRIBUTES: u32 = 0xffff_ffff;
const ERROR_BROKEN_PIPE: u32 = 109;
const ERROR_ACCESS_DENIED: u32 = 5;

#[repr(C)]
struct SecurityAttributes {
    length: u32,
    security_descriptor: *mut c_void,
    inherit_handle: i32,
}
#[repr(C)]
struct StartupInfoW {
    cb: u32,
    reserved: *mut u16,
    desktop: *mut u16,
    title: *mut u16,
    x: u32,
    y: u32,
    x_size: u32,
    y_size: u32,
    x_count: u32,
    y_count: u32,
    fill: u32,
    flags: u32,
    show: u16,
    reserved2_count: u16,
    reserved2: *mut u8,
    std_input: RawHandle,
    std_output: RawHandle,
    std_error: RawHandle,
}
#[repr(C)]
struct StartupInfoExW {
    startup: StartupInfoW,
    attribute_list: *mut c_void,
}
#[repr(C)]
struct ProcessInformation {
    process: RawHandle,
    thread: RawHandle,
    process_id: u32,
    thread_id: u32,
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct FileTime {
    low: u32,
    high: u32,
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct ByHandleFileInformation {
    attrs: u32,
    creation: FileTime,
    access: FileTime,
    write: FileTime,
    volume: u32,
    size_high: u32,
    size_low: u32,
    links: u32,
    file_index_high: u32,
    file_index_low: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> RawHandle;
    fn CloseHandle(handle: RawHandle) -> i32;
    fn GetCurrentProcessId() -> u32;
    fn GetProcessId(handle: RawHandle) -> u32;
    fn GetProcessTimes(
        handle: RawHandle,
        creation: *mut FileTime,
        exit: *mut FileTime,
        kernel: *mut FileTime,
        user: *mut FileTime,
    ) -> i32;
    fn ProcessIdToSessionId(pid: u32, session: *mut u32) -> i32;
    fn QueryFullProcessImageNameW(
        handle: RawHandle,
        flags: u32,
        path: *mut u16,
        length: *mut u32,
    ) -> i32;
    fn CreatePipe(
        read: *mut RawHandle,
        write: *mut RawHandle,
        attrs: *mut SecurityAttributes,
        size: u32,
    ) -> i32;
    fn SetHandleInformation(handle: RawHandle, mask: u32, flags: u32) -> i32;
    fn InitializeProcThreadAttributeList(
        list: *mut c_void,
        count: u32,
        flags: u32,
        size: *mut usize,
    ) -> i32;
    fn UpdateProcThreadAttribute(
        list: *mut c_void,
        flags: u32,
        attribute: usize,
        value: *mut c_void,
        size: usize,
        previous: *mut c_void,
        returned: *mut usize,
    ) -> i32;
    fn DeleteProcThreadAttributeList(list: *mut c_void);
    fn CreateProcessW(
        app: *const u16,
        command: *mut u16,
        process_attrs: *mut SecurityAttributes,
        thread_attrs: *mut SecurityAttributes,
        inherit: i32,
        flags: u32,
        environment: *mut c_void,
        directory: *const u16,
        startup: *mut StartupInfoW,
        info: *mut ProcessInformation,
    ) -> i32;
    fn ReadFile(
        handle: RawHandle,
        buffer: *mut u8,
        len: u32,
        read: *mut u32,
        overlapped: *mut c_void,
    ) -> i32;
    fn WriteFile(
        handle: RawHandle,
        buffer: *const u8,
        len: u32,
        written: *mut u32,
        overlapped: *mut c_void,
    ) -> i32;
    fn WaitForSingleObject(handle: RawHandle, milliseconds: u32) -> u32;
    fn GetExitCodeProcess(handle: RawHandle, code: *mut u32) -> i32;
    fn GetLastError() -> u32;
    fn GetFileType(handle: RawHandle) -> u32;
    fn GetFileAttributesW(path: *const u16) -> u32;
    fn CreateFileW(
        path: *const u16,
        access: u32,
        share: u32,
        attrs: *mut SecurityAttributes,
        creation: u32,
        flags: u32,
        template: RawHandle,
    ) -> RawHandle;
    fn GetFileInformationByHandle(handle: RawHandle, info: *mut ByHandleFileInformation) -> i32;
    fn GetFullPathNameW(
        path: *const u16,
        size: u32,
        out: *mut u16,
        file_part: *mut *mut u16,
    ) -> u32;
    fn GetProcessHandleCount(process: RawHandle, count: *mut u32) -> i32;
}
#[link(name = "bcrypt")]
unsafe extern "system" {
    fn BCryptGenRandom(algorithm: RawHandle, buffer: *mut u8, len: u32, flags: u32) -> i32;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileIdentity {
    pub volume_serial: u32,
    pub file_index: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeaseIdentity {
    pub pid: u32,
    pub creation_time: u64,
    pub session_id: u32,
    pub nonce: String,
    pub executable: String,
    pub file: FileIdentity,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaseReject {
    ForgedPid,
    StaleCreationTime,
    WrongSession,
    WrongExecutable,
    ExecutableReparseRefused,
    FileIdentityMismatch,
    BadNonce,
    DuplicateClaim,
    DuplicateInheritedRole,
    InvalidParentProcessHandle,
    InsufficientProcessRights,
    InvalidChannelDirection,
    ChannelNotOneShot,
    WaitTimeout,
    WaitFailed(u32),
    UnexpectedInheritedHandle,
}

/// Stateful production claim validator. A sealed nonce claim is consumable
/// exactly once, even when the same identity is presented again.
#[derive(Default)]
pub struct LeaseValidator {
    consumed: bool,
}

impl LeaseValidator {
    pub fn validate_once(
        &mut self,
        actual: &LeaseIdentity,
        claim: &LeaseIdentity,
    ) -> Result<(), LeaseReject> {
        if self.consumed {
            return Err(LeaseReject::DuplicateClaim);
        }
        validate_identity(actual, claim)?;
        self.consumed = true;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandleCounts {
    pub before: u32,
    pub after: u32,
}
pub struct ParentLease {
    pub identity: LeaseIdentity,
    pub child_pid: u32,
    pub explicit_allowlist_count: u32,
    pub parent_handles: HandleCounts,
    parent: OwnedHandle,
    child_process: OwnedHandle,
    child_thread: OwnedHandle,
}
impl ParentLease {
    pub fn close_owned_handles(mut self) -> Result<u32, &'static str> {
        self.child_thread.close()?;
        self.child_process.close()?;
        self.parent.close()?;
        Ok(3)
    }
}
struct OwnedHandle(Option<RawHandle>);
impl OwnedHandle {
    fn new(raw: RawHandle) -> Result<Self, &'static str> {
        if raw == 0 || raw == INVALID_HANDLE {
            Err("invalid-handle")
        } else {
            Ok(Self(Some(raw)))
        }
    }
    fn raw(&self) -> RawHandle {
        self.0.unwrap_or(INVALID_HANDLE)
    }
    fn close(&mut self) -> Result<(), &'static str> {
        let raw = self.0.take().ok_or("handle-already-closed")?;
        // SAFETY: `raw` is the unique live handle removed from this owner.
        if unsafe { CloseHandle(raw) } == 0 {
            Err("close-handle")
        } else {
            Ok(())
        }
    }
}
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if let Some(raw) = self.0.take() {
            // SAFETY: Drop owns the last unclosed raw handle in this guard.
            let _ = unsafe { CloseHandle(raw) };
        }
    }
}
fn handle_count() -> Result<u32, &'static str> {
    let mut count = 0;
    // SAFETY: -1 is the documented current-process pseudo-handle and `count` is writable.
    if unsafe { GetProcessHandleCount(-1, &mut count) } == 0 {
        Err("get-process-handle-count")
    } else {
        Ok(count)
    }
}
fn u64_time(value: FileTime) -> u64 {
    ((value.high as u64) << 32) | value.low as u64
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn canonical_file(path: &str) -> Result<(String, FileIdentity), LeaseReject> {
    let source = wide(path);
    let mut full = vec![0u16; 32768];
    // SAFETY: `source` is terminated and `full` is writable for its reported capacity.
    let length = unsafe {
        GetFullPathNameW(
            source.as_ptr(),
            full.len() as u32,
            full.as_mut_ptr(),
            null_mut(),
        )
    };
    if length == 0 || length as usize >= full.len() {
        return Err(LeaseReject::WrongExecutable);
    };
    let canonical = String::from_utf16_lossy(&full[..length as usize]);
    let canonical_wide = wide(&canonical);
    // SAFETY: `canonical_wide` is a live null-terminated UTF-16 path.
    let attrs = unsafe { GetFileAttributesW(canonical_wide.as_ptr()) };
    if attrs == INVALID_FILE_ATTRIBUTES {
        return Err(LeaseReject::WrongExecutable);
    };
    if attrs & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(LeaseReject::ExecutableReparseRefused);
    };
    // SAFETY: all pointers are null or valid terminated path pointers; returned handle is owned.
    let file = OwnedHandle::new(unsafe {
        CreateFileW(
            canonical_wide.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_ALL,
            null_mut(),
            OPEN_EXISTING,
            0,
            0,
        )
    })
    .map_err(|_| LeaseReject::WrongExecutable)?;
    let mut info = ByHandleFileInformation::default();
    // SAFETY: `file` is live and `info` is writable for the complete native record.
    if unsafe { GetFileInformationByHandle(file.raw(), &mut info) } == 0 {
        return Err(LeaseReject::WrongExecutable);
    };
    Ok((
        canonical,
        FileIdentity {
            volume_serial: info.volume,
            file_index: ((info.file_index_high as u64) << 32) | info.file_index_low as u64,
        },
    ))
}

pub fn canonical_file_identity(path: &str) -> Result<(String, FileIdentity), LeaseReject> {
    canonical_file(path)
}
fn process_identity(handle: RawHandle, nonce: String) -> Result<LeaseIdentity, LeaseReject> {
    // SAFETY: caller supplies the candidate process handle; failure is converted to typed reject.
    let pid = unsafe { GetProcessId(handle) };
    if pid == 0 {
        // SAFETY: read immediately after the failed Win32 call on this thread.
        return Err(if unsafe { GetLastError() } == ERROR_ACCESS_DENIED {
            LeaseReject::InsufficientProcessRights
        } else {
            LeaseReject::InvalidParentProcessHandle
        });
    };
    let (mut creation, mut exit, mut kernel, mut user) = (
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
    );
    // SAFETY: all four FILETIME outputs are valid writable locals for the candidate handle.
    if unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        // SAFETY: read immediately after the failed GetProcessTimes call.
        return Err(if unsafe { GetLastError() } == ERROR_ACCESS_DENIED {
            LeaseReject::InsufficientProcessRights
        } else {
            LeaseReject::InvalidParentProcessHandle
        });
    };
    let mut session = 0;
    // SAFETY: `pid` came from the handle and `session` is writable.
    if unsafe { ProcessIdToSessionId(pid, &mut session) } == 0 {
        return Err(LeaseReject::InvalidParentProcessHandle);
    };
    let mut path = vec![0u16; 32768];
    let mut length = path.len() as u32;
    // SAFETY: `path` and its in/out length describe a writable bounded UTF-16 buffer.
    if unsafe { QueryFullProcessImageNameW(handle, 0, path.as_mut_ptr(), &mut length) } == 0 {
        return Err(LeaseReject::InvalidParentProcessHandle);
    };
    let (executable, file) = canonical_file(&String::from_utf16_lossy(&path[..length as usize]))?;
    Ok(LeaseIdentity {
        pid,
        creation_time: u64_time(creation),
        session_id: session,
        nonce,
        executable,
        file,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NegativeFixtureResult {
    pub case: &'static str,
    pub rejection: LeaseReject,
}

/// Runs deterministic attacks through the same validators used by the child.
/// The access-rights case also uses a real synchronization-only process handle.
pub fn production_negative_fixtures() -> Result<Vec<NegativeFixtureResult>, &'static str> {
    let baseline = LeaseIdentity {
        pid: 7,
        creation_time: 8,
        session_id: 9,
        nonce: "0123456789abcdef0123456789abcdef".into(),
        executable: "C:\\guardian-parent.exe".into(),
        file: FileIdentity {
            volume_serial: 10,
            file_index: 11,
        },
    };
    let mut results = Vec::new();
    let mut claim = baseline.clone();
    claim.pid += 1;
    results.push(NegativeFixtureResult {
        case: "forged-pid",
        rejection: validate_identity(&baseline, &claim).unwrap_err(),
    });
    claim = baseline.clone();
    claim.creation_time += 1;
    results.push(NegativeFixtureResult {
        case: "stale-creation-time",
        rejection: validate_identity(&baseline, &claim).unwrap_err(),
    });
    claim = baseline.clone();
    claim.session_id += 1;
    results.push(NegativeFixtureResult {
        case: "wrong-session",
        rejection: validate_identity(&baseline, &claim).unwrap_err(),
    });
    claim = baseline.clone();
    claim.executable = "C:\\wrong.exe".into();
    results.push(NegativeFixtureResult {
        case: "wrong-executable",
        rejection: validate_identity(&baseline, &claim).unwrap_err(),
    });
    claim = baseline.clone();
    claim.file.file_index += 1;
    results.push(NegativeFixtureResult {
        case: "wrong-file-identity",
        rejection: validate_identity(&baseline, &claim).unwrap_err(),
    });
    claim = baseline.clone();
    claim.nonce = "forged".into();
    results.push(NegativeFixtureResult {
        case: "forged-nonce",
        rejection: validate_identity(&baseline, &claim).unwrap_err(),
    });
    let mut once = LeaseValidator::default();
    once.validate_once(&baseline, &baseline)
        .map_err(|_| "initial-claim-rejected")?;
    results.push(NegativeFixtureResult {
        case: "duplicate-claim",
        rejection: once.validate_once(&baseline, &baseline).unwrap_err(),
    });
    results.push(NegativeFixtureResult {
        case: "unexpected-handle",
        rejection: validate_explicit_handle_set(&[1, 2, 3]).unwrap_err(),
    });

    // SAFETY: requests a real handle to the current process with deliberately
    // insufficient query rights; the RAII wrapper closes it on every path.
    let rights = OwnedHandle::new(unsafe { OpenProcess(SYNCHRONIZE, 0, GetCurrentProcessId()) })?;
    results.push(NegativeFixtureResult {
        case: "insufficient-process-rights",
        rejection: process_identity(rights.raw(), baseline.nonce.clone()).unwrap_err(),
    });
    Ok(results)
}

fn nonce() -> Result<String, &'static str> {
    let mut bytes = [0u8; 16];
    // SAFETY: system-preferred RNG accepts a null algorithm and writes exactly 16 bytes.
    if unsafe { BCryptGenRandom(0, bytes.as_mut_ptr(), 16, 2) } != 0 {
        return Err("bcrypt-gen-random");
    };
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
pub fn validate_identity(actual: &LeaseIdentity, claim: &LeaseIdentity) -> Result<(), LeaseReject> {
    if claim.nonce.len() != 32 || !claim.nonce.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(LeaseReject::BadNonce);
    };
    if actual.pid != claim.pid {
        return Err(LeaseReject::ForgedPid);
    };
    if actual.creation_time != claim.creation_time {
        return Err(LeaseReject::StaleCreationTime);
    };
    if actual.session_id != claim.session_id {
        return Err(LeaseReject::WrongSession);
    };
    if actual.executable != claim.executable {
        return Err(LeaseReject::WrongExecutable);
    };
    if actual.file != claim.file {
        return Err(LeaseReject::FileIdentityMismatch);
    };
    Ok(())
}
pub fn validate_inherited_roles(parent: RawHandle, channel: RawHandle) -> Result<(), LeaseReject> {
    if parent == channel {
        return Err(LeaseReject::DuplicateInheritedRole);
    };
    // SAFETY: the candidate handle is queried only; invalid/type mismatch becomes rejection.
    if unsafe { GetFileType(channel) } != FILE_TYPE_PIPE {
        return Err(LeaseReject::InvalidChannelDirection);
    };
    let (mut c, mut e, mut k, mut u) = (
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
    );
    // SAFETY: candidate parent is queried with writable FILETIME outputs; failures reject.
    if unsafe { GetProcessId(parent) } == 0
        || unsafe { GetProcessTimes(parent, &mut c, &mut e, &mut k, &mut u) } == 0
    {
        return Err(LeaseReject::InvalidParentProcessHandle);
    };
    // SAFETY: zero-timeout wait is read-only and the candidate remains live for this call.
    match unsafe { WaitForSingleObject(parent, 0) } {
        WAIT_TIMEOUT => Ok(()),
        WAIT_OBJECT_0 => Err(LeaseReject::StaleCreationTime),
        // SAFETY: read immediately after WAIT_FAILED on this thread.
        WAIT_FAILED => Err(LeaseReject::WaitFailed(unsafe { GetLastError() })),
        _ => Err(LeaseReject::InvalidParentProcessHandle),
    }
}

pub fn validate_explicit_handle_set(handles: &[RawHandle]) -> Result<(), LeaseReject> {
    if handles.len() != 2 {
        return Err(LeaseReject::UnexpectedInheritedHandle);
    }
    if handles[0] == handles[1] {
        return Err(LeaseReject::DuplicateInheritedRole);
    }
    Ok(())
}
fn encode_claim(claim: &LeaseIdentity) -> Vec<u8> {
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
        claim.pid,
        claim.creation_time,
        claim.session_id,
        claim.nonce,
        claim.file.volume_serial,
        claim.file.file_index,
        claim.executable
    )
    .into_bytes()
}
fn decode_claim(bytes: &[u8]) -> Result<LeaseIdentity, LeaseReject> {
    let value = std::str::from_utf8(bytes).map_err(|_| LeaseReject::BadNonce)?;
    let mut fields = value.splitn(7, '\t');
    let pid = fields
        .next()
        .ok_or(LeaseReject::BadNonce)?
        .parse()
        .map_err(|_| LeaseReject::BadNonce)?;
    let creation_time = fields
        .next()
        .ok_or(LeaseReject::BadNonce)?
        .parse()
        .map_err(|_| LeaseReject::BadNonce)?;
    let session_id = fields
        .next()
        .ok_or(LeaseReject::BadNonce)?
        .parse()
        .map_err(|_| LeaseReject::BadNonce)?;
    let nonce = fields.next().ok_or(LeaseReject::BadNonce)?.to_owned();
    let volume_serial = fields
        .next()
        .ok_or(LeaseReject::BadNonce)?
        .parse()
        .map_err(|_| LeaseReject::BadNonce)?;
    let file_index = fields
        .next()
        .ok_or(LeaseReject::BadNonce)?
        .parse()
        .map_err(|_| LeaseReject::BadNonce)?;
    let executable = fields.next().ok_or(LeaseReject::BadNonce)?.to_owned();
    Ok(LeaseIdentity {
        pid,
        creation_time,
        session_id,
        nonce,
        executable,
        file: FileIdentity {
            volume_serial,
            file_index,
        },
    })
}
fn write_once(handle: RawHandle, claim: &LeaseIdentity) -> Result<(), &'static str> {
    let body = encode_claim(claim);
    let length = (body.len() as u32).to_le_bytes();
    let (mut n, mut offset) = (0, 0);
    // SAFETY: synchronous pipe handle, readable four-byte input, and writable count are valid.
    if unsafe { WriteFile(handle, length.as_ptr(), 4, &mut n, null_mut()) } == 0 || n != 4 {
        return Err("claim-header-write");
    };
    while offset < body.len() {
        n = 0;
        // SAFETY: body slice is live for the synchronous write and `n` is writable.
        if unsafe {
            WriteFile(
                handle,
                body[offset..].as_ptr(),
                (body.len() - offset) as u32,
                &mut n,
                null_mut(),
            )
        } == 0
            || n == 0
        {
            return Err("claim-body-write");
        };
        offset += n as usize;
    }
    Ok(())
}
fn read_exact(handle: RawHandle, mut out: &mut [u8]) -> Result<(), LeaseReject> {
    while !out.is_empty() {
        let mut n = 0;
        // SAFETY: remaining output slice is writable for this synchronous pipe read.
        if unsafe {
            ReadFile(
                handle,
                out.as_mut_ptr(),
                out.len() as u32,
                &mut n,
                null_mut(),
            )
        } == 0
            || n == 0
        {
            return Err(LeaseReject::ChannelNotOneShot);
        };
        let (_, rest) = out.split_at_mut(n as usize);
        out = rest;
    }
    Ok(())
}
fn read_once_claim(handle: RawHandle) -> Result<LeaseIdentity, LeaseReject> {
    let mut length = [0u8; 4];
    read_exact(handle, &mut length)?;
    let length = u32::from_le_bytes(length) as usize;
    if length == 0 || length > 65536 {
        return Err(LeaseReject::ChannelNotOneShot);
    };
    let mut body = vec![0; length];
    read_exact(handle, &mut body)?;
    let mut extra = 0;
    let mut n = 0;
    // SAFETY: the inherited read end is synchronous and owned by this child.
    // EOF is valid only when its sole writer was closed (ERROR_BROKEN_PIPE).
    // SAFETY: one-byte probe buffer and count are writable for this synchronous EOF check.
    let eof = unsafe { ReadFile(handle, &mut extra, 1, &mut n, null_mut()) };
    // SAFETY: GetLastError is consumed only on the failed ReadFile path above.
    if eof != 0 || n != 0 || unsafe { GetLastError() } != ERROR_BROKEN_PIPE {
        return Err(LeaseReject::ChannelNotOneShot);
    };
    decode_claim(&body)
}
fn valid_acknowledgement(bytes: &[u8], nonce: &str) -> bool {
    bytes == format!("guardian-lease-accepted:{nonce}").as_bytes()
}
pub fn spawn_restricted_child(
    executable: &str,
    terminal_path: &str,
) -> Result<ParentLease, &'static str> {
    let before = handle_count()?;
    // SAFETY: current PID is valid; the returned inheritable handle is immediately owned.
    let parent = OwnedHandle::new(unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE,
            1,
            GetCurrentProcessId(),
        )
    })?;
    let identity = process_identity(parent.raw(), nonce()?).map_err(|_| "parent-identity")?;
    let mut attrs = SecurityAttributes {
        length: size_of::<SecurityAttributes>() as u32,
        security_descriptor: null_mut(),
        inherit_handle: 1,
    };
    let (mut read, mut write) = (0, 0);
    // SAFETY: both handle outputs and SECURITY_ATTRIBUTES are initialized writable locals.
    if unsafe { CreatePipe(&mut read, &mut write, &mut attrs, 0) } == 0 {
        return Err("create-pipe");
    };
    let mut channel_read = OwnedHandle::new(read)?;
    let mut channel_write = OwnedHandle::new(write)?;
    // SAFETY: write end is live and only its inheritance flag is cleared.
    if unsafe { SetHandleInformation(channel_write.raw(), HANDLE_FLAG_INHERIT, 0) } == 0 {
        return Err("clear-write-inherit");
    };
    let mut bytes = 0;
    // SAFETY: null list is the documented size query and `bytes` is writable.
    let _ = unsafe { InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut bytes) };
    if bytes == 0 {
        return Err("attribute-list-size");
    };
    let mut list = vec![0u8; bytes];
    let ptr = list.as_mut_ptr().cast();
    // SAFETY: `list` owns at least the size reported by the immediately preceding query.
    if unsafe { InitializeProcThreadAttributeList(ptr, 1, 0, &mut bytes) } == 0 {
        return Err("attribute-list-init");
    };
    let mut allowed = [parent.raw(), channel_read.raw()];
    validate_explicit_handle_set(&allowed).map_err(|_| "invalid-explicit-handle-set")?;
    // SAFETY: initialized attribute list receives the exact live two-handle array.
    if unsafe {
        UpdateProcThreadAttribute(
            ptr,
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            allowed.as_mut_ptr().cast(),
            size_of_val(&allowed),
            null_mut(),
            null_mut(),
        )
    } == 0
    {
        // SAFETY: `ptr` is an initialized list and is deleted exactly once on this path.
        unsafe { DeleteProcThreadAttributeList(ptr) };
        return Err("attribute-list-allowlist");
    };
    let command = format!(
        "\"{executable}\" --guardian-child --lease-handle {} --channel-handle {} --terminal-path \"{}\"",
        parent.raw(),
        channel_read.raw(),
        terminal_path
    );
    let mut command = wide(&command);
    let app = wide(executable);
    let mut startup = StartupInfoExW {
        startup: StartupInfoW {
            cb: size_of::<StartupInfoExW>() as u32,
            reserved: null_mut(),
            desktop: null_mut(),
            title: null_mut(),
            x: 0,
            y: 0,
            x_size: 0,
            y_size: 0,
            x_count: 0,
            y_count: 0,
            fill: 0,
            flags: 0,
            show: 0,
            reserved2_count: 0,
            reserved2: null_mut(),
            std_input: 0,
            std_output: 0,
            std_error: 0,
        },
        attribute_list: ptr,
    };
    let mut info = ProcessInformation {
        process: 0,
        thread: 0,
        process_id: 0,
        thread_id: 0,
    };
    // SAFETY: all strings/buffers remain live through synchronous CreateProcessW; explicit
    // inheritance is restricted by the initialized attribute list.
    let ok = unsafe {
        CreateProcessW(
            app.as_ptr(),
            command.as_mut_ptr(),
            null_mut(),
            null_mut(),
            1,
            EXTENDED_STARTUPINFO_PRESENT,
            null_mut(),
            null_mut(),
            &mut startup.startup,
            &mut info,
        )
    };
    // SAFETY: CreateProcessW no longer references the initialized list; delete exactly once.
    unsafe { DeleteProcThreadAttributeList(ptr) };
    if ok == 0 {
        return Err("create-restricted-child");
    };
    let process = OwnedHandle::new(info.process)?;
    let thread = OwnedHandle::new(info.thread)?;
    channel_read.close()?;
    write_once(channel_write.raw(), &identity)?;
    channel_write.close()?;
    let acknowledgement = format!("{terminal_path}.accepted");
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut acknowledged = false;
    while Instant::now() < deadline {
        if std::fs::read(&acknowledgement)
            .is_ok_and(|bytes| valid_acknowledgement(&bytes, &identity.nonce))
        {
            acknowledged = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    if !acknowledged {
        return Err("child-acceptance-timeout");
    }
    let after = handle_count()?;
    Ok(ParentLease {
        identity,
        child_pid: info.process_id,
        explicit_allowlist_count: 2,
        parent_handles: HandleCounts { before, after },
        parent,
        child_process: process,
        child_thread: thread,
    })
}
pub fn child_accept_and_wait(
    parent_raw: RawHandle,
    channel_raw: RawHandle,
    terminal_path: &str,
    deadline_ms: u32,
) -> Result<HandleCounts, LeaseReject> {
    let expected = std::env::current_exe().map_err(|_| LeaseReject::WrongExecutable)?;
    child_accept_and_wait_expected(
        parent_raw,
        channel_raw,
        terminal_path,
        deadline_ms,
        &expected.to_string_lossy(),
    )
}

pub fn child_accept_and_wait_expected(
    parent_raw: RawHandle,
    channel_raw: RawHandle,
    terminal_path: &str,
    deadline_ms: u32,
    expected_parent_executable: &str,
) -> Result<HandleCounts, LeaseReject> {
    let counts = child_accept_and_wait_expected_deferred_terminal(
        parent_raw,
        channel_raw,
        terminal_path,
        deadline_ms,
        expected_parent_executable,
    )?;
    let terminal = format!(
        "{{\"schema\":\"guardian-terminal/v3\",\"parent_terminal_observed\":true,\"unique_success_terminal_count\":1,\"child_handles_before\":{},\"child_handles_after\":{},\"released_inherited_handles\":2,\"explicit_allowlist_exact\":true,\"verified_roles\":[\"parent-wait-handle\",\"one-shot-read-channel\"]}}\n",
        counts.before, counts.after
    );
    std::fs::write(terminal_path, terminal).map_err(|_| LeaseReject::UnexpectedInheritedHandle)?;
    Ok(counts)
}

/// Validates the restricted authority channel and waits for the exact parent,
/// but deliberately leaves the success terminal to the caller. Production
/// recovery uses this so success cannot be published before Explorer recovery.
pub fn child_accept_and_wait_expected_deferred_terminal(
    parent_raw: RawHandle,
    channel_raw: RawHandle,
    terminal_path: &str,
    deadline_ms: u32,
    expected_parent_executable: &str,
) -> Result<HandleCounts, LeaseReject> {
    let before = handle_count().map_err(|_| LeaseReject::UnexpectedInheritedHandle)?;
    if parent_raw == 0 || parent_raw == INVALID_HANDLE {
        return Err(LeaseReject::InvalidParentProcessHandle);
    }
    if channel_raw == 0 || channel_raw == INVALID_HANDLE || parent_raw == channel_raw {
        return Err(LeaseReject::DuplicateInheritedRole);
    }
    // SAFETY: immediately assumes ownership of the two handles emitted by the
    // explicit STARTUPINFOEX allowlist; Drop closes either handle on every reject.
    let mut parent =
        OwnedHandle::new(parent_raw).map_err(|_| LeaseReject::InvalidParentProcessHandle)?;
    let mut channel =
        OwnedHandle::new(channel_raw).map_err(|_| LeaseReject::InvalidChannelDirection)?;
    validate_explicit_handle_set(&[parent.raw(), channel.raw()])?;
    validate_inherited_roles(parent.raw(), channel.raw())?;
    let claim = read_once_claim(channel.raw())?;
    channel
        .close()
        .map_err(|_| LeaseReject::UnexpectedInheritedHandle)?;
    // No nonce is accepted from argv; it is sealed and single-read from the pipe.
    let actual = process_identity(parent.raw(), claim.nonce.clone())?;
    LeaseValidator::default().validate_once(&actual, &claim)?;
    let (expected_executable, expected_file) = canonical_file(expected_parent_executable)?;
    if actual.executable != expected_executable {
        return Err(LeaseReject::WrongExecutable);
    }
    if actual.file != expected_file {
        return Err(LeaseReject::FileIdentityMismatch);
    }
    std::fs::write(
        format!("{terminal_path}.accepted"),
        format!("guardian-lease-accepted:{}", actual.nonce),
    )
    .map_err(|_| LeaseReject::UnexpectedInheritedHandle)?;
    // SAFETY: validated parent process handle remains owned for the bounded wait.
    match unsafe { WaitForSingleObject(parent.raw(), deadline_ms) } {
        WAIT_OBJECT_0 => {}
        WAIT_TIMEOUT => return Err(LeaseReject::WaitTimeout),
        // SAFETY: read immediately after WAIT_FAILED on this thread.
        WAIT_FAILED => return Err(LeaseReject::WaitFailed(unsafe { GetLastError() })),
        _ => return Err(LeaseReject::InvalidParentProcessHandle),
    };
    parent
        .close()
        .map_err(|_| LeaseReject::UnexpectedInheritedHandle)?;
    let after = handle_count().map_err(|_| LeaseReject::UnexpectedInheritedHandle)?;
    Ok(HandleCounts { before, after })
}
pub fn current_resources() -> Result<ResourceSnapshot, &'static str> {
    resource_snapshot()
}

/// Creates a controller fixture with handle inheritance disabled.  This is the
/// only launch API the evidence controller may use; the separate lease parent
/// launch owns its explicit inherited-handle list.
pub fn launch_uninherited_fixture(
    executable: &str,
    arguments: &[String],
    deadline_ms: u32,
) -> Result<u32, &'static str> {
    let mut command = format!("\"{executable}\"");
    for argument in arguments {
        command.push(' ');
        command.push('"');
        command.push_str(&argument.replace('"', "\\\""));
        command.push('"');
    }
    let mut command = wide(&command);
    let application = wide(executable);
    let mut startup = StartupInfoW {
        cb: size_of::<StartupInfoW>() as u32,
        reserved: null_mut(),
        desktop: null_mut(),
        title: null_mut(),
        x: 0,
        y: 0,
        x_size: 0,
        y_size: 0,
        x_count: 0,
        y_count: 0,
        fill: 0,
        flags: 0,
        show: 0,
        reserved2_count: 0,
        reserved2: null_mut(),
        std_input: 0,
        std_output: 0,
        std_error: 0,
    };
    let mut information = ProcessInformation {
        process: 0,
        thread: 0,
        process_id: 0,
        thread_id: 0,
    };
    // SAFETY: bInheritHandles is FALSE and returned process/thread handles are
    // immediately owned by RAII guards in this function.
    // SAFETY: all launch buffers remain live; inheritance is explicitly disabled.
    if unsafe {
        CreateProcessW(
            application.as_ptr(),
            command.as_mut_ptr(),
            null_mut(),
            null_mut(),
            0,
            0,
            null_mut(),
            null_mut(),
            &mut startup,
            &mut information,
        )
    } == 0
    {
        return Err("create-uninherited-fixture");
    }
    let mut process = OwnedHandle::new(information.process)?;
    let mut thread = OwnedHandle::new(information.thread)?;
    thread.close()?;
    // SAFETY: owned child process handle remains live for this bounded wait.
    match unsafe { WaitForSingleObject(process.raw(), deadline_ms) } {
        WAIT_OBJECT_0 => {}
        WAIT_TIMEOUT => return Err("uninherited-fixture-timeout"),
        WAIT_FAILED => return Err("uninherited-fixture-wait-failed"),
        _ => return Err("uninherited-fixture-wait-unexpected"),
    }
    let mut code = 0;
    // SAFETY: process is live and signaled after the successful wait above.
    // SAFETY: signaled process handle is live and `code` is writable.
    if unsafe { GetExitCodeProcess(process.raw(), &mut code) } == 0 {
        return Err("uninherited-fixture-exit-code");
    }
    process.close()?;
    Ok(code)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn id() -> LeaseIdentity {
        LeaseIdentity {
            pid: 7,
            creation_time: 8,
            session_id: 9,
            nonce: "0123456789abcdef0123456789abcdef".into(),
            executable: "C:\\runner.exe".into(),
            file: FileIdentity {
                volume_serial: 1,
                file_index: 2,
            },
        }
    }
    #[test]
    fn binding_rejects_identity_and_file_changes() {
        let actual = id();
        assert!(validate_identity(&actual, &actual).is_ok());
        let mut value = actual.clone();
        value.pid = 8;
        assert_eq!(
            validate_identity(&actual, &value),
            Err(LeaseReject::ForgedPid)
        );
        value = actual.clone();
        value.creation_time = 9;
        assert_eq!(
            validate_identity(&actual, &value),
            Err(LeaseReject::StaleCreationTime)
        );
        value = actual.clone();
        value.session_id = 10;
        assert_eq!(
            validate_identity(&actual, &value),
            Err(LeaseReject::WrongSession)
        );
        value = actual.clone();
        value.executable = "C:\\other.exe".into();
        assert_eq!(
            validate_identity(&actual, &value),
            Err(LeaseReject::WrongExecutable)
        );
        value = actual.clone();
        value.file.file_index = 3;
        assert_eq!(
            validate_identity(&actual, &value),
            Err(LeaseReject::FileIdentityMismatch)
        );
    }
    #[test]
    fn claim_round_trip_is_not_argv() {
        let value = id();
        assert_eq!(decode_claim(&encode_claim(&value)), Ok(value));
    }
    #[test]
    fn acknowledgement_is_bound_to_the_sealed_pipe_nonce() {
        let value = id();
        assert!(valid_acknowledgement(
            format!("guardian-lease-accepted:{}", value.nonce).as_bytes(),
            &value.nonce
        ));
        assert!(!valid_acknowledgement(
            b"guardian-lease-accepted",
            &value.nonce
        ));
        assert!(!valid_acknowledgement(
            b"guardian-lease-accepted:forged",
            &value.nonce
        ));
    }
    #[test]
    fn duplicate_roles_are_rejected() {
        assert_eq!(
            validate_inherited_roles(5, 5),
            Err(LeaseReject::DuplicateInheritedRole)
        );
    }

    #[test]
    fn production_validator_rejects_duplicate_and_unexpected_claims() {
        let value = id();
        let mut validator = LeaseValidator::default();
        assert_eq!(validator.validate_once(&value, &value), Ok(()));
        assert_eq!(
            validator.validate_once(&value, &value),
            Err(LeaseReject::DuplicateClaim)
        );
        assert_eq!(
            validate_explicit_handle_set(&[1, 2, 3]),
            Err(LeaseReject::UnexpectedInheritedHandle)
        );
    }
}
