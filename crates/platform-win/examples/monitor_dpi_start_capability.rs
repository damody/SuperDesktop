//! Read-only monitor/DPI and ExplorerPatcher Start-host capability runner.
//!
//! This example creates no HWND, AppBar, Shell Hook, or process. Its virtual
//! topology portion is an in-memory fixture, explicitly distinct from the real
//! profile snapshot collected before and after it.

use platform_win::common::{
    monitor_dpi_start::{
        MonitorRecord, ScreenRect, StartHostProbe, TopologyEvent, VirtualTopologyAdapter,
        probe_start_host_read_only, snapshot_real_monitors, start_probe_from_observation,
    },
    native_window::{ResourceSnapshot, resource_snapshot},
};

const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

#[link(name = "user32")]
unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(value: isize) -> i32;
    fn GetThreadDpiAwarenessContext() -> isize;
    fn AreDpiAwarenessContextsEqual(first: isize, second: isize) -> i32;
}

fn rect(value: &ScreenRect) -> String {
    format!(
        "{{\"left\":{},\"top\":{},\"right\":{},\"bottom\":{}}}",
        value.left, value.top, value.right, value.bottom
    )
}
fn monitor(value: &MonitorRecord) -> String {
    format!(
        "{{\"device_name\":\"{}\",\"primary\":{},\"bounds\":{},\"work_area\":{},\"dpi_x\":{},\"dpi_y\":{}}}",
        value.device_name.replace('\\', "\\\\"),
        value.primary,
        rect(&value.bounds),
        rect(&value.work_area),
        value.dpi_x,
        value.dpi_y
    )
}
fn resources(value: ResourceSnapshot) -> String {
    format!(
        "{{\"process_handles\":{},\"user_objects\":{},\"gdi_objects\":{}}}",
        value.process_handles, value.user_objects, value.gdi_objects
    )
}
fn fixture_monitor(name: &str, primary: bool, dpi: u32) -> MonitorRecord {
    MonitorRecord {
        device_name: name.into(),
        primary,
        bounds: ScreenRect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        },
        work_area: ScreenRect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        },
        dpi_x: dpi,
        dpi_y: dpi,
    }
}
fn event(event: TopologyEvent) -> String {
    match event {
        TopologyEvent::Added { device_name } => {
            format!("{{\"kind\":\"added\",\"device_name\":\"{device_name}\"}}")
        }
        TopologyEvent::Removed { device_name } => {
            format!("{{\"kind\":\"removed\",\"device_name\":\"{device_name}\"}}")
        }
        TopologyEvent::PrimaryChanged { device_name } => {
            format!("{{\"kind\":\"primary-changed\",\"device_name\":\"{device_name}\"}}")
        }
        TopologyEvent::DpiChanged {
            device_name,
            dpi_x,
            dpi_y,
        } => format!(
            "{{\"kind\":\"dpi-changed\",\"device_name\":\"{device_name}\",\"dpi_x\":{dpi_x},\"dpi_y\":{dpi_y}}}"
        ),
    }
}

fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('\"', "\\\""))
}

fn optional_string(value: Option<String>) -> String {
    value
        .map(|value| quoted(&value))
        .unwrap_or_else(|| "null".into())
}

fn enable_per_monitor_v2() -> Result<(), String> {
    // SAFETY: this read-only runner is a dedicated process. Its DPI awareness is
    // established before any monitor or geometry API is called.
    if unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) } == 0 {
        return Err("set-process-dpi-awareness-context-per-monitor-v2".into());
    }
    // SAFETY: retrieves only this thread's current awareness context.
    let thread_context = unsafe { GetThreadDpiAwarenessContext() };
    // SAFETY: compares opaque awareness context values without ownership transfer.
    if unsafe {
        AreDpiAwarenessContextsEqual(thread_context, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
    } == 0
    {
        return Err("thread-dpi-awareness-is-not-per-monitor-v2".into());
    }
    Ok(())
}

fn start_json(start: StartHostProbe) -> String {
    let (reason, observed_taskbar_class) = match start {
        StartHostProbe::Unavailable {
            reason,
            observed_taskbar_class,
        } => (reason, observed_taskbar_class),
    };
    let missing = start_probe_from_observation(None);
    let untrusted = start_probe_from_observation(Some("Shell_TrayWnd".into()));
    let fixture = |name: &str, probe: StartHostProbe| match probe {
        StartHostProbe::Unavailable { reason, .. } => format!(
            "{{\"name\":{},\"status\":\"unavailable\",\"reason\":{}}}",
            quoted(name),
            quoted(reason)
        ),
    };
    format!(
        "{{\"status\":\"unavailable\",\"reason\":{},\"observed_taskbar_class\":{},\"host_observation\":{{\"taskbar_class\":{},\"path\":null}},\"invocation_attempted\":false,\"disposition\":\"stop\",\"fixtures\":[{},{}]}}",
        quoted(reason),
        optional_string(observed_taskbar_class.clone()),
        optional_string(observed_taskbar_class),
        fixture("host-missing", missing),
        fixture("untrusted-host", untrusted)
    )
}

fn run() -> Result<String, String> {
    enable_per_monitor_v2()?;
    // SAFETY: enable_per_monitor_v2 has established the runner's process and
    // thread context before this point, so these coordinates are not DPI virtualized.
    let resources_before = resource_snapshot().map_err(str::to_owned)?;
    let real_before = snapshot_real_monitors().map_err(str::to_owned)?;
    let real_after = snapshot_real_monitors().map_err(str::to_owned)?;
    if real_before != real_after {
        return Err("real-monitor-refresh-not-stable".into());
    }
    let mut fixture = VirtualTopologyAdapter::new(vec![fixture_monitor("VIRTUAL-A", true, 96)])
        .map_err(str::to_owned)?;
    let events = vec![
        fixture
            .add(fixture_monitor("VIRTUAL-B", false, 144))
            .map_err(str::to_owned)?,
        fixture.set_primary("VIRTUAL-B").map_err(str::to_owned)?,
        fixture
            .set_dpi("VIRTUAL-B", 192, 168)
            .map_err(str::to_owned)?,
        fixture.remove("VIRTUAL-A").map_err(str::to_owned)?,
    ];
    let start = probe_start_host_read_only();
    let resources_after = resource_snapshot().map_err(str::to_owned)?;
    if resources_before != resources_after {
        return Err("read-only-resource-drift".into());
    }
    let start_json = start_json(start);
    Ok(format!(
        "{{\"schema\":\"monitor-dpi-start-trace/v2\",\"mode\":\"read-only-preview\",\"dpi_awareness\":{{\"process_set_per_monitor_v2\":true,\"thread_is_per_monitor_v2\":true,\"geometry_virtualized\":false}},\"real_profile\":{{\"origin\":\"real-profile\",\"refresh_stable\":true,\"monitors\":[{}]}},\"virtual_fixture\":{{\"origin\":\"virtual-fixture\",\"physical_mixed_dpi_claimed\":false,\"events\":[{}]}},\"start_host\":{},\"start_invocation_attempted\":false,\"typed_disposition\":\"stop\",\"explorer_mutations\":false,\"shell_takeover\":false,\"resources_before\":{},\"resources_after\":{}}}",
        real_before
            .monitors
            .iter()
            .map(monitor)
            .collect::<Vec<_>>()
            .join(","),
        events.into_iter().map(event).collect::<Vec<_>>().join(","),
        start_json,
        resources(resources_before),
        resources(resources_after)
    ))
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(trace) => {
            println!("{trace}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            println!(
                "{{\"admitted\":false,\"error\":\"{}\"}}",
                error.replace('"', "\\\"")
            );
            std::process::ExitCode::from(2)
        }
    }
}
