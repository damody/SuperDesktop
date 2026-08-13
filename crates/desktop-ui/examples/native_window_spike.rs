//! Headful preview of one GPUI-owned Win32 HWND and one borrowed subclass.
//!
//! The composition root never calls a Win32 API.  It takes the raw handle from
//! the live `gpui::Window`, then hands the numeric identity to `platform-win`.
//! The bridge is the only code allowed to send the three controlled native test
//! messages, and it never creates or destroys a window.

use std::{cell::RefCell, env, fs, process::ExitCode, rc::Rc, time::Duration};

use gpui::{
    App, AppContext, Context, Entity, IntoElement, Render, Styled, Subscription, Window,
    WindowBounds, WindowOptions, div, px, size,
};
use platform_win::common::native_window::{
    BorrowedHwnd, BridgeTrace, FatalReason, OwnedWindowEvent, ResourceSnapshot, SubclassLease,
    resource_snapshot,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

const RESOURCE_DEADLINE_TICKS: usize = 40;
const RESOURCE_POLL_MS: u64 = 50;
const PROCESS_HANDLE_DELTA_MAX: i64 = 2;
const USER_OBJECT_DELTA_MAX: i64 = 2;
const GDI_OBJECT_DELTA_MAX: i64 = 2;

#[derive(Clone)]
struct TraceBindings {
    successor_contract_sha256: String,
    admission_trace_sha256: String,
    binary_sha256: String,
}

struct SpikeView {
    lease: Option<SubclassLease>,
    close_subscription: Option<Subscription>,
}

impl SpikeView {
    fn on_gpui_window_closed(&mut self) -> Result<(), String> {
        self.lease
            .as_ref()
            .ok_or_else(|| "root-lease-missing".to_string())?
            .on_window_closed()
            .map_err(str::to_owned)
    }

    fn try_finalize(&mut self) -> Result<Option<BridgeTrace>, String> {
        let resources_after = resource_snapshot().map_err(str::to_owned)?;
        let lease = self
            .lease
            .as_ref()
            .ok_or_else(|| "root-lease-missing".to_string())?;
        match lease.terminal_trace(resources_after) {
            Ok(trace)
                if resources_within_threshold(trace.resources_before, trace.resources_after) =>
            {
                self.close_subscription.take();
                self.lease.take();
                Ok(Some(trace))
            }
            Ok(_) | Err("terminal-signals-incomplete") => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
}

impl Render for SpikeView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

fn resources_within_threshold(before: ResourceSnapshot, after: ResourceSnapshot) -> bool {
    let delta = |after: u32, before: u32| i64::from(after) - i64::from(before);
    delta(after.process_handles, before.process_handles) <= PROCESS_HANDLE_DELTA_MAX
        && delta(after.user_objects, before.user_objects) <= USER_OBJECT_DELTA_MAX
        && delta(after.gdi_objects, before.gdi_objects) <= GDI_OBJECT_DELTA_MAX
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn json_strings(events: &[&str]) -> String {
    events
        .iter()
        .map(|event| format!("\"{}\"", json_escape(event)))
        .collect::<Vec<_>>()
        .join(",")
}

fn json_owned_events(events: &[OwnedWindowEvent]) -> String {
    events
        .iter()
        .map(|event| match event {
            OwnedWindowEvent::DpiChanged {
                x,
                y,
                suggested_left,
                suggested_top,
                suggested_right,
                suggested_bottom,
            } => format!(
                "{{\"kind\":\"dpi-changed\",\"x\":{x},\"y\":{y},\"suggested_rect\":{{\"left\":{suggested_left},\"top\":{suggested_top},\"right\":{suggested_right},\"bottom\":{suggested_bottom}}}}}"
            ),
            OwnedWindowEvent::DisplayChanged {
                bits_per_pixel,
                width,
                height,
            } => format!(
                "{{\"kind\":\"display-changed\",\"bits_per_pixel\":{bits_per_pixel},\"width\":{width},\"height\":{height}}}"
            ),
            OwnedWindowEvent::ActivationChanged { state } => {
                format!("{{\"kind\":\"activation\",\"state\":{state}}}")
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn json_resources(snapshot: ResourceSnapshot) -> String {
    format!(
        "{{\"process_handles\":{},\"user_objects\":{},\"gdi_objects\":{}}}",
        snapshot.process_handles, snapshot.user_objects, snapshot.gdi_objects
    )
}

fn trace_json(trace: BridgeTrace, bindings: &TraceBindings) -> String {
    let lifecycle = trace
        .lifecycle
        .iter()
        .map(|(state, sequence)| format!("{{\"state\":\"{state}\",\"sequence\":{sequence}}}"))
        .collect::<Vec<_>>()
        .join(",");
    let fatal = match trace.fatal_callback {
        None => "null",
        Some(FatalReason::CallbackPanic) => "\"callback-panic\"",
        Some(FatalReason::CallbackError) => "\"callback-error\"",
        Some(FatalReason::RemoveSubclassFailed) => "\"remove-subclass-failed\"",
    };
    format!(
        concat!(
            "{{\"schema\":\"native-window-trace/v2\",\"gpui_window_opened\":true,",
            "\"hwnd\":{},\"owner_pid\":{},\"owner_thread\":{},\"session_id\":{},",
            "\"gpui_window_id\":{},\"generation\":{},\"lifecycle\":[{}],",
            "\"owned_events\":[{}],\"adapter_events\":[{}],",
            "\"callbacks_before_close\":{},\"callbacks_after_close\":{},",
            "\"late_event_rejected\":{},\"wm_ncdestroy_observed\":{},",
            "\"on_window_closed_observed\":{},\"callback_state_outstanding\":{},",
            "\"fatal_callback\":{},\"resources_before\":{},\"resources_after\":{},",
            "\"resource_deadline\":{{\"poll_interval_ms\":{},\"max_ticks\":{}}},",
            "\"resource_thresholds\":{{\"process_handle_delta_max\":{},\"user_object_delta_max\":{},\"gdi_object_delta_max\":{}}},",
            "\"raw_message_contract\":{{\"dpi_rect_valid_for_send\":true,\"display_parameters_valid\":true,\"activation_parameters_valid\":true}},",
            "\"input_contract\":{{\"successor_contract_sha256\":\"{}\",\"admission_trace_sha256\":\"{}\",\"binary_sha256\":\"{}\"}},",
            "\"appbar_or_hook_mutations\":false,\"bridge_created_hwnd\":false,\"bridge_destroyed_hwnd\":false,\"preview_only\":true}}"
        ),
        trace.hwnd,
        trace.owner_pid,
        trace.owner_thread,
        trace.session_id,
        trace.gpui_window_id,
        trace.generation,
        lifecycle,
        json_owned_events(&trace.owned_events),
        json_strings(&trace.adapter_events),
        trace.callbacks_before_close,
        trace.callbacks_after_close,
        trace.late_event_rejected,
        trace.wm_ncdestroy_observed,
        trace.on_window_closed_observed,
        trace.callback_state_outstanding,
        fatal,
        json_resources(trace.resources_before),
        json_resources(trace.resources_after),
        RESOURCE_POLL_MS,
        RESOURCE_DEADLINE_TICKS,
        PROCESS_HANDLE_DELTA_MAX,
        USER_OBJECT_DELTA_MAX,
        GDI_OBJECT_DELTA_MAX,
        bindings.successor_contract_sha256,
        bindings.admission_trace_sha256,
        bindings.binary_sha256,
    )
}

fn live_win32_hwnd(window: &Window) -> Result<isize, String> {
    let borrowed = HasWindowHandle::window_handle(window).map_err(|error| error.to_string())?;
    match borrowed.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as isize),
        _ => Err("gpui-non-win32-hwnd".to_string()),
    }
}

fn required_sha256_env(name: &str) -> Result<String, String> {
    let value = env::var(name).map_err(|_| format!("missing-required-binding:{name}"))?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("invalid-required-binding:{name}"));
    }
    Ok(value.to_ascii_uppercase())
}

fn run() -> Result<String, String> {
    let bindings = TraceBindings {
        successor_contract_sha256: required_sha256_env(
            "NATIVE_WINDOW_SPIKE_SUCCESSOR_CONTRACT_SHA256",
        )?,
        admission_trace_sha256: required_sha256_env("NATIVE_WINDOW_SPIKE_ADMISSION_TRACE_SHA256")?,
        binary_sha256: required_sha256_env("NATIVE_WINDOW_SPIKE_BINARY_SHA256")?,
    };
    let trace_output = env::var("NATIVE_WINDOW_SPIKE_OUTPUT").ok();
    let result = Rc::new(RefCell::new(None::<Result<String, String>>));
    let result_for_app = Rc::clone(&result);
    let platform = gpui_windows::WindowsPlatform::new(false).map_err(|error| error.to_string())?;
    gpui::Application::with_platform(Rc::new(platform))
        .with_quit_mode(gpui::QuitMode::Explicit)
        .run(move |cx: &mut App| {
            let root_slot = Rc::new(RefCell::new(None::<Entity<SpikeView>>));
            let init_error = Rc::new(RefCell::new(None::<String>));
            let root_slot_for_window = Rc::clone(&root_slot);
            let init_error_for_window = Rc::clone(&init_error);
            let bounds = gpui::Bounds::centered(None, size(px(320.), px(180.)), cx);
            let opened = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| {
                    let lease = (|| {
                        let hwnd = live_win32_hwnd(&*window)?;
                        let gpui_window_id =
                            gpui::Window::window_handle(&*window).window_id().as_u64();
                        let identity =
                            BorrowedHwnd::verify_current_process(hwnd, gpui_window_id, 1)
                                .map_err(str::to_owned)?;
                        let lease = SubclassLease::attach(identity).map_err(str::to_owned)?;
                        // The bridge owns all Win32 interaction. It sends valid synchronous
                        // raw DPI/display/activation messages and lets GPUI own dispatch.
                        lease.send_test_raw_messages().map_err(str::to_owned)?;
                        Ok::<SubclassLease, String>(lease)
                    })();
                    let entity = match lease {
                        Ok(lease) => cx.new(|_| SpikeView {
                            lease: Some(lease),
                            close_subscription: None,
                        }),
                        Err(error) => {
                            *init_error_for_window.borrow_mut() = Some(error);
                            cx.new(|_| SpikeView {
                                lease: None,
                                close_subscription: None,
                            })
                        }
                    };
                    *root_slot_for_window.borrow_mut() = Some(entity.clone());
                    entity
                },
            );
            let (Ok(handle), Some(root)) = (opened, root_slot.borrow_mut().take()) else {
                *result_for_app.borrow_mut() = Some(Err(init_error
                    .borrow_mut()
                    .take()
                    .unwrap_or_else(|| "gpui-open-window".to_string())));
                cx.quit();
                return;
            };
            if let Some(error) = init_error.borrow_mut().take() {
                *result_for_app.borrow_mut() = Some(Err(error));
                cx.quit();
                return;
            }

            let expected_window_id = handle.window_id();
            let weak_root = root.downgrade();
            let result_for_close = Rc::clone(&result_for_app);
            let subscription = cx.on_window_closed(move |app, closed_id| {
                if closed_id == expected_window_id {
                    if let Some(root) = weak_root.upgrade() {
                        if let Err(error) = root.update(app, |view, _| view.on_gpui_window_closed())
                        {
                            *result_for_close.borrow_mut() =
                                Some(Err(format!("on-window-closed:{error}")));
                            app.quit();
                        }
                    } else {
                        *result_for_close.borrow_mut() =
                            Some(Err("on-window-closed:root-lease-unavailable".to_string()));
                        app.quit();
                    }
                }
            });
            root.update(cx, |view, _| view.close_subscription = Some(subscription));

            // Warm GPUI's executors before freezing the resource baseline. Their
            // first timer creates process handles that are not window leaks.
            let close_root = root.clone();
            let close_result = Rc::clone(&result_for_app);
            let close_background = cx.background_executor().clone();
            let close_foreground = cx.foreground_executor().clone();
            let close_app = cx.to_async();
            close_foreground
                .spawn(async move {
                    close_background.timer(Duration::from_millis(100)).await;
                    close_app.update(|app| {
                        let close_ready = close_root.update(app, |view, _| {
                            let lease = view.lease.as_mut().ok_or("root-lease-missing")?;
                            lease.rebaseline_resources()?;
                            lease.begin_closing()?;
                            match lease.reject_late_adapter_event() {
                                Err("retired-subclass-generation") => {}
                                Err(error) => return Err(error),
                                Ok(()) => return Err("late-adapter-event-was-not-rejected"),
                            }
                            Ok::<(), &'static str>(())
                        });
                        if let Err(error) = close_ready {
                            *close_result.borrow_mut() =
                                Some(Err(format!("close-initiation:{error}")));
                            app.quit();
                            return;
                        }
                        if let Err(error) = handle.update(app, |_, window, _| {
                            window.remove_window();
                            Ok::<(), &'static str>(())
                        }) {
                            *close_result.borrow_mut() =
                                Some(Err(format!("close-initiation:{error}")));
                            app.quit();
                        }
                    });
                })
                .detach();

            let poll_root = root.clone();
            let poll_result = Rc::clone(&result_for_app);
            let poll_bindings = bindings.clone();
            let poll_trace_output = trace_output.clone();
            let background = cx.background_executor().clone();
            let foreground = cx.foreground_executor().clone();
            let async_app = cx.to_async();
            foreground
                .spawn(async move {
                    for _ in 0..RESOURCE_DEADLINE_TICKS {
                        background
                            .timer(Duration::from_millis(RESOURCE_POLL_MS))
                            .await;
                        let complete = async_app.update(|app| {
                            match poll_root.update(app, |view, _| view.try_finalize()) {
                                Ok(Some(trace)) => {
                                    let json = trace_json(trace, &poll_bindings);
                                    if let Some(path) = &poll_trace_output
                                        && let Err(error) = fs::write(path, &json)
                                    {
                                        *poll_result.borrow_mut() =
                                            Some(Err(format!("trace-write:{error}")));
                                        return true;
                                    }
                                    *poll_result.borrow_mut() = Some(Ok(json));
                                    true
                                }
                                Ok(None) => false,
                                Err(error) => {
                                    *poll_result.borrow_mut() = Some(Err(error));
                                    true
                                }
                            }
                        });
                        if complete {
                            async_app.update(|app| app.quit());
                            return;
                        }
                    }
                    async_app.update(|app| {
                        *poll_result.borrow_mut() =
                            Some(Err("native-terminal-or-resource-deadline".to_string()));
                        app.quit();
                    });
                })
                .detach();
        });
    result
        .borrow_mut()
        .take()
        .ok_or_else(|| "gpui-run-returned-without-terminal-result".to_string())?
}

fn main() -> ExitCode {
    match run() {
        Ok(trace) => {
            if let Ok(path) = env::var("NATIVE_WINDOW_SPIKE_OUTPUT") {
                if let Err(error) = fs::write(path, &trace) {
                    println!(
                        "{{\"admitted\":false,\"error\":\"trace-write:{}\"}}",
                        json_escape(&error.to_string())
                    );
                    return ExitCode::from(2);
                }
            }
            println!("{trace}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!(
                "{{\"admitted\":false,\"error\":\"{}\"}}",
                json_escape(&error)
            );
            ExitCode::from(2)
        }
    }
}
