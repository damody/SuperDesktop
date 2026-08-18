use std::{
    collections::BTreeMap,
    ffi::c_void,
    io::{self, BufRead, Write},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use shell_provider_protocol::{
    TaskbarAttentionState, TaskbarProgressKind, TaskbarProgressState, TaskbarStateHostRequest,
    TaskbarStateHostResponse, TaskbarStateSnapshot, TaskbarWindowIdentity, TaskbarWindowState,
};
use windows::{
    Win32::{
        Foundation::{CLASS_E_NOAGGREGATION, E_FAIL, E_INVALIDARG, HWND, RECT},
        System::{
            Com::{
                CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, CoInitializeEx, CoRegisterClassObject,
                CoRevokeClassObject, IClassFactory, IClassFactory_Impl, REGCLS_MULTIPLEUSE,
            },
            Threading::GetCurrentProcessId,
        },
        UI::{
            Controls::HIMAGELIST,
            Shell::{
                ITaskbarList_Impl, ITaskbarList2_Impl, ITaskbarList3_Impl, ITaskbarList4,
                ITaskbarList4_Impl, STPFLAG, TBPF_ERROR, TBPF_INDETERMINATE, TBPF_NOPROGRESS,
                TBPF_NORMAL, TBPF_PAUSED, TBPFLAG, THUMBBUTTON,
            },
            WindowsAndMessaging::{GetWindowThreadProcessId, HICON, IsWindow},
        },
    },
    core::{BOOL, Error, GUID, IUnknown, Interface, PCWSTR, Ref, Result, implement},
};

const CLSID_TASKBAR_LIST: GUID = GUID::from_u128(0x56fdf344_fd6d_11d0_958a_006097c9a090);

#[link(name = "kernel32")]
unsafe extern "system" {
    fn ProcessIdToSessionId(process_id: u32, session_id: *mut u32) -> i32;
}

#[derive(Default)]
struct HostState {
    host_generation: u64,
    snapshot_generation: u64,
    next_observation_generation: u64,
    windows: BTreeMap<(isize, u32), TaskbarWindowState>,
}

impl HostState {
    fn new() -> Self {
        let generation = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |value| value.as_nanos().min(u128::from(u64::MAX)) as u64)
            .max(1);
        Self {
            host_generation: generation,
            snapshot_generation: 1,
            next_observation_generation: 1,
            windows: BTreeMap::new(),
        }
    }

    fn window_mut(&mut self, hwnd: HWND) -> Result<&mut TaskbarWindowState> {
        let (process_id, session_id) = validate_window(hwnd)?;
        let key = (hwnd.0 as isize, process_id);
        if !self.windows.contains_key(&key) {
            let observation_generation = self.next_observation_generation;
            self.next_observation_generation = self
                .next_observation_generation
                .checked_add(1)
                .ok_or_else(|| Error::from(E_FAIL))?;
            self.windows.insert(
                key,
                TaskbarWindowState {
                    identity: TaskbarWindowIdentity {
                        process_id,
                        session_id,
                        hwnd_identity: hwnd.0 as i64,
                        observation_generation,
                    },
                    progress: TaskbarProgressState::none(),
                    attention: TaskbarAttentionState::none(),
                },
            );
        }
        self.windows
            .get_mut(&key)
            .ok_or_else(|| Error::from(E_FAIL))
    }

    fn changed(&mut self) {
        self.snapshot_generation = self.snapshot_generation.saturating_add(1).max(1);
    }

    fn snapshot(&self) -> TaskbarStateSnapshot {
        TaskbarStateSnapshot {
            host_generation: self.host_generation,
            snapshot_generation: self.snapshot_generation,
            windows: self.windows.values().cloned().collect(),
            overflowed: false,
        }
    }
}

fn validate_window(hwnd: HWND) -> Result<(u32, u32)> {
    if hwnd.0.is_null() || !unsafe { IsWindow(Some(hwnd)).as_bool() } {
        return Err(Error::from(E_INVALIDARG));
    }
    let mut process_id = 0;
    if unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) } == 0 || process_id == 0 {
        return Err(Error::from(E_INVALIDARG));
    }
    let mut session_id = 0;
    let mut current_session = 0;
    if unsafe { ProcessIdToSessionId(process_id, &mut session_id) } == 0
        || unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut current_session) } == 0
        || session_id != current_session
    {
        return Err(Error::from(E_INVALIDARG));
    }
    Ok((process_id, session_id))
}

#[implement(ITaskbarList4)]
struct TaskbarListCompat {
    state: Arc<Mutex<HostState>>,
}

impl ITaskbarList_Impl for TaskbarListCompat_Impl {
    fn HrInit(&self) -> Result<()> {
        Ok(())
    }
    fn AddTab(&self, _hwnd: HWND) -> Result<()> {
        Ok(())
    }
    fn DeleteTab(&self, _hwnd: HWND) -> Result<()> {
        Ok(())
    }
    fn ActivateTab(&self, _hwnd: HWND) -> Result<()> {
        Ok(())
    }
    fn SetActiveAlt(&self, _hwnd: HWND) -> Result<()> {
        Ok(())
    }
}

impl ITaskbarList2_Impl for TaskbarListCompat_Impl {
    fn MarkFullscreenWindow(&self, _hwnd: HWND, _fullscreen: BOOL) -> Result<()> {
        Ok(())
    }
}

impl ITaskbarList3_Impl for TaskbarListCompat_Impl {
    fn SetProgressValue(&self, hwnd: HWND, completed: u64, total: u64) -> Result<()> {
        if total == 0 || completed > total {
            return Err(Error::from(E_INVALIDARG));
        }
        let mut state = self.state.lock().map_err(|_| Error::from(E_FAIL))?;
        let window = state.window_mut(hwnd)?;
        let kind = match window.progress.kind {
            TaskbarProgressKind::Paused => TaskbarProgressKind::Paused,
            TaskbarProgressKind::Error => TaskbarProgressKind::Error,
            _ => TaskbarProgressKind::Normal,
        };
        window.progress = TaskbarProgressState {
            kind,
            completed,
            total,
        };
        state.changed();
        Ok(())
    }

    fn SetProgressState(&self, hwnd: HWND, flags: TBPFLAG) -> Result<()> {
        let mut state = self.state.lock().map_err(|_| Error::from(E_FAIL))?;
        let window = state.window_mut(hwnd)?;
        window.progress = if flags == TBPF_NOPROGRESS {
            TaskbarProgressState::none()
        } else if flags == TBPF_INDETERMINATE {
            TaskbarProgressState {
                kind: TaskbarProgressKind::Indeterminate,
                completed: 0,
                total: 0,
            }
        } else {
            let kind = if flags == TBPF_NORMAL {
                TaskbarProgressKind::Normal
            } else if flags == TBPF_PAUSED {
                TaskbarProgressKind::Paused
            } else if flags == TBPF_ERROR {
                TaskbarProgressKind::Error
            } else {
                return Err(Error::from(E_INVALIDARG));
            };
            let (completed, total) = if window.progress.total == 0 {
                (0, 1)
            } else {
                (window.progress.completed, window.progress.total)
            };
            TaskbarProgressState {
                kind,
                completed,
                total,
            }
        };
        state.changed();
        Ok(())
    }

    fn RegisterTab(&self, _tab: HWND, _mdi: HWND) -> Result<()> {
        Ok(())
    }
    fn UnregisterTab(&self, _tab: HWND) -> Result<()> {
        Ok(())
    }
    fn SetTabOrder(&self, _tab: HWND, _before: HWND) -> Result<()> {
        Ok(())
    }
    fn SetTabActive(&self, _tab: HWND, _mdi: HWND, _reserved: u32) -> Result<()> {
        Ok(())
    }
    fn ThumbBarAddButtons(
        &self,
        _hwnd: HWND,
        _count: u32,
        _buttons: *const THUMBBUTTON,
    ) -> Result<()> {
        Ok(())
    }
    fn ThumbBarUpdateButtons(
        &self,
        _hwnd: HWND,
        _count: u32,
        _buttons: *const THUMBBUTTON,
    ) -> Result<()> {
        Ok(())
    }
    fn ThumbBarSetImageList(&self, _hwnd: HWND, _images: HIMAGELIST) -> Result<()> {
        Ok(())
    }
    fn SetOverlayIcon(&self, _hwnd: HWND, _icon: HICON, _description: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn SetThumbnailTooltip(&self, _hwnd: HWND, _tip: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn SetThumbnailClip(&self, _hwnd: HWND, _clip: *const RECT) -> Result<()> {
        Ok(())
    }
}

impl ITaskbarList4_Impl for TaskbarListCompat_Impl {
    fn SetTabProperties(&self, _tab: HWND, _flags: STPFLAG) -> Result<()> {
        Ok(())
    }
}

#[implement(IClassFactory)]
struct TaskbarClassFactory {
    state: Arc<Mutex<HostState>>,
}

impl IClassFactory_Impl for TaskbarClassFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        iid: *const GUID,
        output: *mut *mut c_void,
    ) -> Result<()> {
        if !outer.is_null() {
            return Err(Error::from(CLASS_E_NOAGGREGATION));
        }
        if iid.is_null() || output.is_null() {
            return Err(Error::from(E_INVALIDARG));
        }
        let object: ITaskbarList4 = TaskbarListCompat {
            state: Arc::clone(&self.state),
        }
        .into();
        // SAFETY: COM supplied writable output storage and a valid IID pointer.
        unsafe { object.query(iid, output).ok() }
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        Ok(())
    }
}

fn write_response(output: &mut impl Write, response: &TaskbarStateHostResponse) -> io::Result<()> {
    serde_json::to_writer(&mut *output, response)?;
    output.write_all(b"\n")?;
    output.flush()
}

fn run() -> Result<()> {
    // SAFETY: initializes COM once for this process before creating COM objects.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok()?;
    let state = Arc::new(Mutex::new(HostState::new()));
    let factory: IClassFactory = TaskbarClassFactory {
        state: Arc::clone(&state),
    }
    .into();
    // SAFETY: factory remains alive until the cookie is explicitly revoked.
    let cookie = unsafe {
        CoRegisterClassObject(
            &CLSID_TASKBAR_LIST,
            &factory,
            CLSCTX_LOCAL_SERVER,
            REGCLS_MULTIPLEUSE,
        )?
    };
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let response = match line
            .ok()
            .and_then(|line| serde_json::from_str::<TaskbarStateHostRequest>(&line).ok())
        {
            Some(TaskbarStateHostRequest::Snapshot) => state
                .lock()
                .map(|state| TaskbarStateHostResponse::Snapshot(state.snapshot()))
                .unwrap_or(TaskbarStateHostResponse::InvalidRequest),
            Some(TaskbarStateHostRequest::Health) => state
                .lock()
                .map(|state| TaskbarStateHostResponse::Health {
                    host_generation: state.host_generation,
                    provider_available: true,
                })
                .unwrap_or(TaskbarStateHostResponse::InvalidRequest),
            Some(TaskbarStateHostRequest::Shutdown) => {
                write_response(&mut stdout, &TaskbarStateHostResponse::Shutdown)
                    .map_err(|_| Error::from(E_FAIL))?;
                break;
            }
            None => TaskbarStateHostResponse::InvalidRequest,
        };
        write_response(&mut stdout, &response).map_err(|_| Error::from(E_FAIL))?;
    }
    // SAFETY: revokes exactly the successful registration owned above.
    unsafe { CoRevokeClassObject(cookie) }?;
    Ok(())
}

fn main() {
    if run().is_err() {
        std::process::exit(1);
    }
}
