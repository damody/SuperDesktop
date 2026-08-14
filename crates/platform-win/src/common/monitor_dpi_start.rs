//! Read-only monitor/DPI and ExplorerPatcher Start-host capability probes.
//!
//! Real monitor snapshots and virtual topology fixtures are deliberately separate:
//! a fixture can validate event semantics but can never claim a physical mixed-DPI
//! display was present. Start probing is read-only and returns unavailable unless a
//! future, independently verified integration contract authorizes an invocation.

use std::{mem::size_of, thread, time::Duration};

use windows::{
    Win32::{
        Foundation::LPARAM,
        Graphics::Gdi::{
            EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
        },
        UI::WindowsAndMessaging::{FindWindowW, GetClassNameW},
    },
    core::{BOOL, PCWSTR, w},
};

const MONITORINFOF_PRIMARY: u32 = 1;
const MDT_EFFECTIVE_DPI: u32 = 0;

#[link(name = "Shcore")]
unsafe extern "system" {
    fn GetDpiForMonitor(monitor: HMONITOR, dpi_type: u32, dpi_x: *mut u32, dpi_y: *mut u32) -> i32;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorRecord {
    pub device_name: String,
    pub primary: bool,
    pub bounds: ScreenRect,
    pub work_area: ScreenRect,
    pub dpi_x: u32,
    pub dpi_y: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotOrigin {
    RealProfile,
    VirtualFixture,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorSnapshot {
    pub origin: SnapshotOrigin,
    pub monitors: Vec<MonitorRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TopologyEvent {
    Added {
        device_name: String,
    },
    Removed {
        device_name: String,
    },
    PrimaryChanged {
        device_name: String,
    },
    DpiChanged {
        device_name: String,
        dpi_x: u32,
        dpi_y: u32,
    },
}

/// Pure, explicitly virtual adapter used only for topology-event coverage.
pub struct VirtualTopologyAdapter {
    monitors: Vec<MonitorRecord>,
}
impl VirtualTopologyAdapter {
    pub fn new(monitors: Vec<MonitorRecord>) -> Result<Self, &'static str> {
        validate(&monitors)?;
        Ok(Self { monitors })
    }
    pub fn snapshot(&self) -> MonitorSnapshot {
        MonitorSnapshot {
            origin: SnapshotOrigin::VirtualFixture,
            monitors: self.monitors.clone(),
        }
    }
    pub fn add(&mut self, monitor: MonitorRecord) -> Result<TopologyEvent, &'static str> {
        if self
            .monitors
            .iter()
            .any(|m| m.device_name == monitor.device_name)
        {
            return Err("virtual-monitor-duplicate");
        };
        self.monitors.push(monitor.clone());
        validate(&self.monitors)?;
        Ok(TopologyEvent::Added {
            device_name: monitor.device_name,
        })
    }
    pub fn remove(&mut self, device_name: &str) -> Result<TopologyEvent, &'static str> {
        let index = self
            .monitors
            .iter()
            .position(|m| m.device_name == device_name)
            .ok_or("virtual-monitor-missing")?;
        if self.monitors[index].primary {
            return Err("virtual-primary-remove-refused");
        };
        self.monitors.remove(index);
        Ok(TopologyEvent::Removed {
            device_name: device_name.to_owned(),
        })
    }
    pub fn set_primary(&mut self, device_name: &str) -> Result<TopologyEvent, &'static str> {
        let index = self
            .monitors
            .iter()
            .position(|m| m.device_name == device_name)
            .ok_or("virtual-monitor-missing")?;
        for monitor in &mut self.monitors {
            monitor.primary = false;
        }
        self.monitors[index].primary = true;
        Ok(TopologyEvent::PrimaryChanged {
            device_name: device_name.to_owned(),
        })
    }
    pub fn set_dpi(
        &mut self,
        device_name: &str,
        dpi_x: u32,
        dpi_y: u32,
    ) -> Result<TopologyEvent, &'static str> {
        if dpi_x == 0 || dpi_y == 0 {
            return Err("virtual-dpi-zero");
        };
        let monitor = self
            .monitors
            .iter_mut()
            .find(|m| m.device_name == device_name)
            .ok_or("virtual-monitor-missing")?;
        monitor.dpi_x = dpi_x;
        monitor.dpi_y = dpi_y;
        Ok(TopologyEvent::DpiChanged {
            device_name: device_name.to_owned(),
            dpi_x,
            dpi_y,
        })
    }
}

fn validate(monitors: &[MonitorRecord]) -> Result<(), &'static str> {
    if monitors.is_empty() {
        return Err("monitor-set-empty");
    };
    if monitors.iter().filter(|m| m.primary).count() != 1 {
        return Err("monitor-set-primary-invalid");
    };
    if monitors.iter().any(|m| {
        m.device_name.is_empty()
            || m.dpi_x == 0
            || m.dpi_y == 0
            || m.bounds.right <= m.bounds.left
            || m.bounds.bottom <= m.bounds.top
    }) {
        return Err("monitor-set-invalid");
    };
    Ok(())
}
fn rect(rect: windows::Win32::Foundation::RECT) -> ScreenRect {
    ScreenRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}

unsafe extern "system" fn enumerate_callback(
    monitor: HMONITOR,
    _: HDC,
    _: *mut windows::Win32::Foundation::RECT,
    data: LPARAM,
) -> BOOL {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || -> Result<(), &'static str> {
            // SAFETY: EnumDisplayMonitors receives this pointer from the immediate caller; it remains valid for enumeration.
            let output = unsafe { &mut *(data.0 as *mut Vec<MonitorRecord>) };
            let mut info = MONITORINFOEXW::default();
            info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
            // SAFETY: initialized MONITORINFOEXW is layout-compatible with MONITORINFO at its prefix.
            if !unsafe {
                GetMonitorInfoW(
                    monitor,
                    (&mut info as *mut MONITORINFOEXW).cast::<MONITORINFO>(),
                )
                .as_bool()
            } {
                return Err("get-monitor-info");
            }
            let end = info
                .szDevice
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(info.szDevice.len());
            let device_name =
                String::from_utf16(&info.szDevice[..end]).map_err(|_| "monitor-device-utf16")?;
            let (mut dpi_x, mut dpi_y) = (0, 0);
            // SAFETY: local writable outputs and live HMONITOR supplied by enumeration.
            if unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) } != 0
            {
                return Err("get-monitor-dpi");
            }
            output.push(MonitorRecord {
                device_name,
                primary: (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0,
                bounds: rect(info.monitorInfo.rcMonitor),
                work_area: rect(info.monitorInfo.rcWork),
                dpi_x,
                dpi_y,
            });
            Ok(())
        },
    ));
    if matches!(result, Ok(Ok(()))) {
        BOOL(1)
    } else {
        BOOL(0)
    }
}

pub fn snapshot_real_monitors() -> Result<MonitorSnapshot, &'static str> {
    let mut monitors = Vec::new();
    // SAFETY: callback and vector pointer remain valid for this synchronous enumeration only.
    if !unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(enumerate_callback),
            LPARAM((&mut monitors as *mut Vec<MonitorRecord>) as isize),
        )
        .as_bool()
    } {
        return Err("enum-display-monitors");
    }
    validate(&monitors)?;
    monitors.sort_by(|a, b| a.device_name.cmp(&b.device_name));
    Ok(MonitorSnapshot {
        origin: SnapshotOrigin::RealProfile,
        monitors,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartHostProbe {
    Available {
        taskbar_class: String,
        host_class: String,
        host_pid: u32,
        host_executable: String,
        input_events_sent: u32,
        escape_events_sent: u32,
        foreground_changed: bool,
        restored: bool,
    },
    Unavailable {
        reason: &'static str,
        observed_taskbar_class: Option<String>,
    },
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RawKeybdInput {
    virtual_key: u16,
    scan_code: u16,
    flags: u32,
    time: u32,
    extra_info: usize,
}

#[repr(C)]
union RawInputData {
    keyboard: RawKeybdInput,
    _alignment: [u64; 4],
}

#[repr(C)]
struct RawInput {
    kind: u32,
    data: RawInputData,
}

const INPUT_KEYBOARD: u32 = 1;
const KEYEVENTF_KEYUP: u32 = 2;
const VK_LWIN: u16 = 0x5b;
const VK_ESCAPE: u16 = 0x1b;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

#[link(name = "user32")]
unsafe extern "system" {
    #[link_name = "SendInput"]
    fn raw_send_input(count: u32, inputs: *const RawInput, size: i32) -> u32;
    #[link_name = "GetForegroundWindow"]
    fn raw_get_foreground_window() -> isize;
    #[link_name = "GetWindowThreadProcessId"]
    fn raw_get_window_thread_process_id(hwnd: isize, process_id: *mut u32) -> u32;
    #[link_name = "GetClassNameW"]
    fn raw_get_class_name(hwnd: isize, class_name: *mut u16, max_count: i32) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    #[link_name = "OpenProcess"]
    fn raw_open_process(access: u32, inherit: i32, process_id: u32) -> isize;
    #[link_name = "QueryFullProcessImageNameW"]
    fn raw_query_process_image(process: isize, flags: u32, name: *mut u16, size: *mut u32) -> i32;
    #[link_name = "CloseHandle"]
    fn raw_close_handle(handle: isize) -> i32;
}

fn send_key(virtual_key: u16) -> u32 {
    let inputs = [
        RawInput {
            kind: INPUT_KEYBOARD,
            data: RawInputData {
                keyboard: RawKeybdInput {
                    virtual_key,
                    scan_code: 0,
                    flags: 0,
                    time: 0,
                    extra_info: 0,
                },
            },
        },
        RawInput {
            kind: INPUT_KEYBOARD,
            data: RawInputData {
                keyboard: RawKeybdInput {
                    virtual_key,
                    scan_code: 0,
                    flags: KEYEVENTF_KEYUP,
                    time: 0,
                    extra_info: 0,
                },
            },
        },
    ];
    // SAFETY: INPUT-compatible C layouts remain live for this synchronous call.
    unsafe {
        raw_send_input(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<RawInput>() as i32,
        )
    }
}

fn raw_class_name(hwnd: isize) -> Option<String> {
    let mut buffer = [0_u16; 256];
    // SAFETY: bounded writable local UTF-16 buffer and observed HWND only.
    let length = unsafe { raw_get_class_name(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    (length > 0).then(|| String::from_utf16_lossy(&buffer[..length as usize]))
}

fn process_identity(hwnd: isize) -> Option<(u32, String)> {
    let mut process_id = 0;
    // SAFETY: process_id is a writable local output and hwnd is observation-only.
    if unsafe { raw_get_window_thread_process_id(hwnd, &mut process_id) } == 0 || process_id == 0 {
        return None;
    }
    // SAFETY: query-only rights, no inheritance, observed PID.
    let process = unsafe { raw_open_process(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process == 0 {
        return None;
    }
    let mut buffer = [0_u16; 1024];
    let mut length = buffer.len() as u32;
    // SAFETY: query-only handle plus bounded writable UTF-16 buffer.
    let queried =
        unsafe { raw_query_process_image(process, 0, buffer.as_mut_ptr(), &mut length) != 0 };
    // SAFETY: this function exclusively owns the process handle.
    let _ = unsafe { raw_close_handle(process) };
    queried.then(|| {
        (
            process_id,
            String::from_utf16_lossy(&buffer[..length as usize]),
        )
    })
}

fn trusted_start_host(class_name: &str, executable: &str) -> bool {
    let normalized = executable.replace('/', "\\").to_ascii_lowercase();
    let executable_name = normalized.rsplit('\\').next().unwrap_or_default();
    class_name == "Windows.UI.Core.CoreWindow"
        && matches!(
            executable_name,
            "startmenuexperiencehost.exe" | "searchhost.exe"
        )
        && normalized.starts_with("c:\\windows\\systemapps\\")
}

/// Uses the supported Win32 keyboard-input contract to open Start, verifies the
/// foreground Windows host identity, and always sends Escape to restore focus.
pub fn invoke_start_host_controlled() -> StartHostProbe {
    // SAFETY: name lookup only; the taskbar HWND is not retained after this call.
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()) };
    let Ok(taskbar) = taskbar else {
        return start_probe_from_observation(None);
    };
    let mut taskbar_buffer = [0_u16; 256];
    // SAFETY: bounded local output buffer and observed HWND.
    let taskbar_length = unsafe { GetClassNameW(taskbar, &mut taskbar_buffer) };
    let taskbar_class = (taskbar_length > 0)
        .then(|| String::from_utf16_lossy(&taskbar_buffer[..taskbar_length as usize]));
    let Some(taskbar_class) = taskbar_class else {
        return start_probe_from_observation(None);
    };
    // SAFETY: retrieves a borrowed foreground HWND without mutation.
    let before = unsafe { raw_get_foreground_window() };
    let input_events_sent = send_key(VK_LWIN);
    if input_events_sent != 2 {
        let _ = send_key(VK_ESCAPE);
        return StartHostProbe::Unavailable {
            reason: "supported-start-input-failed",
            observed_taskbar_class: Some(taskbar_class),
        };
    }
    let mut observed = None;
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(50));
        // SAFETY: retrieves a borrowed foreground HWND without mutation.
        let hwnd = unsafe { raw_get_foreground_window() };
        if hwnd == 0 || hwnd == before {
            continue;
        }
        let Some(class_name) = raw_class_name(hwnd) else {
            continue;
        };
        let Some((process_id, executable)) = process_identity(hwnd) else {
            continue;
        };
        if trusted_start_host(&class_name, &executable) {
            observed = Some((hwnd, class_name, process_id, executable));
            break;
        }
    }
    let escape_events_sent = send_key(VK_ESCAPE);
    let Some((host_hwnd, host_class, host_pid, host_executable)) = observed else {
        return StartHostProbe::Unavailable {
            reason: "trusted-start-host-not-observed",
            observed_taskbar_class: Some(taskbar_class),
        };
    };
    let mut restored = false;
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(50));
        // SAFETY: retrieves a borrowed foreground HWND without mutation.
        if unsafe { raw_get_foreground_window() } != host_hwnd {
            restored = true;
            break;
        }
    }
    StartHostProbe::Available {
        taskbar_class,
        host_class,
        host_pid,
        host_executable,
        input_events_sent,
        escape_events_sent,
        foreground_changed: true,
        restored,
    }
}

/// Converts a read-only taskbar-class observation into the only permitted Start
/// capability result for this change.  Keeping this pure makes the missing-host
/// and untrusted-host dispositions testable without touching the shell.
pub fn start_probe_from_observation(observed_taskbar_class: Option<String>) -> StartHostProbe {
    match observed_taskbar_class {
        Some(observed_taskbar_class) => StartHostProbe::Unavailable {
            reason: "untrusted-start-host-invocation-refused",
            observed_taskbar_class: Some(observed_taskbar_class),
        },
        None => StartHostProbe::Unavailable {
            reason: "taskbar-host-not-found",
            observed_taskbar_class: None,
        },
    }
}

/// Performs only a read-only taskbar-class observation. It intentionally refuses
/// to synthesize a click or invoke ExplorerPatcher because no verified host/ABI
/// contract exists for this profile.
pub fn probe_start_host_read_only() -> StartHostProbe {
    // SAFETY: name lookup only; no message is sent and no HWND is retained.
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()) };
    let observed = match taskbar {
        Ok(hwnd) => {
            let mut class = [0u16; 256]; // SAFETY: bounded local output buffer, query only.
            let len = unsafe { GetClassNameW(hwnd, &mut class) };
            (len > 0).then(|| String::from_utf16_lossy(&class[..len as usize]))
        }
        Err(_) => None,
    };
    start_probe_from_observation(observed)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn monitor(name: &str, primary: bool, dpi: u32) -> MonitorRecord {
        MonitorRecord {
            device_name: name.into(),
            primary,
            bounds: ScreenRect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 100,
            },
            work_area: ScreenRect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 90,
            },
            dpi_x: dpi,
            dpi_y: dpi,
        }
    }
    #[test]
    fn virtual_topology_has_explicit_origin_and_events() {
        let mut adapter = VirtualTopologyAdapter::new(vec![monitor("A", true, 96)]).unwrap();
        assert_eq!(adapter.snapshot().origin, SnapshotOrigin::VirtualFixture);
        assert_eq!(
            adapter.add(monitor("B", false, 144)).unwrap(),
            TopologyEvent::Added {
                device_name: "B".into()
            }
        );
        assert_eq!(
            adapter.set_primary("B").unwrap(),
            TopologyEvent::PrimaryChanged {
                device_name: "B".into()
            }
        );
        assert_eq!(
            adapter.set_dpi("B", 192, 168).unwrap(),
            TopologyEvent::DpiChanged {
                device_name: "B".into(),
                dpi_x: 192,
                dpi_y: 168
            }
        );
        assert_eq!(
            adapter.remove("A").unwrap(),
            TopologyEvent::Removed {
                device_name: "A".into()
            }
        );
    }

    #[test]
    fn start_probe_fixtures_are_typed_and_never_invoke() {
        assert_eq!(
            start_probe_from_observation(None),
            StartHostProbe::Unavailable {
                reason: "taskbar-host-not-found",
                observed_taskbar_class: None,
            }
        );
        assert_eq!(
            start_probe_from_observation(Some("Shell_TrayWnd".into())),
            StartHostProbe::Unavailable {
                reason: "untrusted-start-host-invocation-refused",
                observed_taskbar_class: Some("Shell_TrayWnd".into()),
            }
        );
    }
}
