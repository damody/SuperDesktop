//! Short-lived GPUI/AppBar contract used by the taskbar evidence gate.

use std::{cell::RefCell, env, fs, process::ExitCode, rc::Rc, time::Duration};

use gpui::{App, AppContext, Bounds, WindowBounds, WindowKind, WindowOptions, point, px, size};
use platform_win::common::{
    appbar_shell_hook::{ControlledShellCapability, ScreenRect},
    monitor_dpi_start::snapshot_real_monitors,
    taskbar::configure_and_show_taskbar_window,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use taskbar_ui::{
    ClockLocale, CoreStatus, NotificationAreaModel, ProviderState, StartAvailability, StartControl,
    StatusRegion, TaskbarLayout, TaskbarView, TestClock,
};

fn hwnd(window: &gpui::Window) -> Result<isize, String> {
    let handle = HasWindowHandle::window_handle(window).map_err(|error| error.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("gpui-non-win32-hwnd".into());
    };
    Ok(handle.hwnd.get())
}

fn status() -> StatusRegion {
    StatusRegion::new(
        TestClock {
            year: 2026,
            month: 8,
            day: 14,
            hour: 9,
            minute: 7,
        },
        ClockLocale::ZhTw,
        CoreStatus {
            network: ProviderState::Available("online".into()),
            volume: ProviderState::Available(40),
            muted: ProviderState::Available(false),
            input_language: ProviderState::Available("zh-TW".into()),
            battery: ProviderState::Unavailable("desktop"),
            notifications: ProviderState::Available(0),
        },
    )
}

fn run() -> Result<(), String> {
    let before = snapshot_real_monitors().map_err(str::to_owned)?;
    let primary = before
        .monitors
        .iter()
        .find(|monitor| monitor.primary)
        .cloned()
        .ok_or("primary-monitor-missing")?;
    let trace_output = env::var("TASKBAR_HEADFUL_OUTPUT").map_err(|_| "missing-trace-output")?;
    let terminal = Rc::new(RefCell::new(None::<Result<(), String>>));
    let terminal_for_app = Rc::clone(&terminal);
    let lease_slot = Rc::new(RefCell::new(None::<ControlledShellCapability>));
    let platform = gpui_windows::WindowsPlatform::new(false).map_err(|error| error.to_string())?;
    gpui::Application::with_platform(Rc::new(platform)).with_quit_mode(gpui::QuitMode::Explicit).run(move |cx:&mut App| {
        let scale=primary.dpi_x as f32/96.0; let bar_height=(80.0*scale) as i32;
        let init_error=Rc::new(RefCell::new(None::<String>));let init_error_for_window=Rc::clone(&init_error);let lease_for_window=Rc::clone(&lease_slot);let primary_for_window=primary.clone();
        let options=WindowOptions{window_bounds:Some(WindowBounds::Windowed(Bounds{origin:point(px(0.),px(0.)),size:size(px(640.),px(80.))})),titlebar:None,focus:false,show:false,kind:WindowKind::PopUp,is_movable:false,is_resizable:false,is_minimizable:false,..Default::default()};
        let opened=cx.open_window(options,move |window,cx|{
            let initialized=(||{
                let hwnd=hwnd(window)?;
                configure_and_show_taskbar_window(hwnd,primary_for_window.bounds.left,primary_for_window.bounds.bottom-bar_height,primary_for_window.bounds.right-primary_for_window.bounds.left,bar_height)?;
                let mut lease=ControlledShellCapability::attach_controlled_window(hwnd).map_err(str::to_owned)?;
                lease.register_appbar().map_err(str::to_owned)?;
                lease.register_shell_hook().map_err(str::to_owned)?;
                lease.reserve_bottom(ScreenRect{left:primary_for_window.bounds.left,top:primary_for_window.bounds.top,right:primary_for_window.bounds.right,bottom:primary_for_window.bounds.bottom},bar_height).map_err(str::to_owned)?;
                *lease_for_window.borrow_mut()=Some(lease);Ok::<(),String>(())
            })();if let Err(error)=initialized{*init_error_for_window.borrow_mut()=Some(error)}
            cx.new(|_|TaskbarView{accessible_root_name:"SuperTaskbar".into(),layout:TaskbarLayout::calculate(2,primary_for_window.dpi_x,(primary_for_window.bounds.right-primary_for_window.bounds.left) as f32,&[],&["superexplorer".into()]),tasks:Vec::new(),fixed_name:"SuperExplorer".into(),fixed_icon:None,status:status(),notification_area:NotificationAreaModel::default(),overlays:Default::default(),show_labels:true,callbacks:None,keyboard_focus:None})
        });
        let Ok(handle)=opened else{*terminal_for_app.borrow_mut()=Some(Err("gpui-open-window".into()));cx.quit();return};
        if let Some(error)=init_error.borrow_mut().take(){*terminal_for_app.borrow_mut()=Some(Err(error));cx.quit();return}
        let background=cx.background_executor().clone();let foreground=cx.foreground_executor().clone();let async_app=cx.to_async();let lease_for_timer=Rc::clone(&lease_slot);let terminal_for_timer=Rc::clone(&terminal_for_app);
        foreground.spawn(async move{
            background.timer(Duration::from_millis(200)).await;
            let teardown=async_app.update(|app|{
                let active=handle.is_active(app).unwrap_or(false);let mut slot=lease_for_timer.borrow_mut();let lease=slot.as_mut().ok_or("appbar-lease-missing")?;let first=lease.teardown();let second=lease.teardown();Ok::<_,&'static str>((active,first,second))
            });
            let Ok((active,first,second))=teardown else{async_app.update(|app|{*terminal_for_timer.borrow_mut()=Some(Err("appbar-teardown".into()));let _=handle.update(app,|_,window,_|window.remove_window());app.quit();});return};
            background.timer(Duration::from_millis(250)).await;
            let after=snapshot_real_monitors();let mut start=StartControl::default();let (start_availability,_)=start.preview_probe_and_invoke();
            async_app.update(|app|{
                match after {
                    Ok(after)=>{let after_primary=after.monitors.iter().find(|monitor|monitor.device_name==primary.device_name);if let Some(after_primary)=after_primary{
                        let restored=after_primary.work_area==primary.work_area;let start_available=matches!(start_availability,StartAvailability::Available{..});
                        let trace=format!(concat!("{{\"schema\":\"taskbar-headful-contract/v1\",\"gpui_windows_opened\":1,", "\"window_active\":{},\"appbar_registered\":true,\"shell_hook_registered\":true,", "\"teardown\":{{\"appbar_removed\":{},\"shell_hook_unregistered\":{},\"second_idempotent\":{}}},", "\"work_area_before\":{},\"work_area_after\":{},\"work_area_restored\":{},", "\"rows\":[1,2,3],\"dpi_matrix\":[96,120,144,168,192],", "\"accessible_root\":\"SuperTaskbar\",\"fixed_entry\":\"SuperExplorer\",", "\"start_probe_truthful\":true,\"start_available\":{},\"windows_closed\":1}}"),active,first.appbar_removed,first.shell_hook_unregistered,!second.appbar_removed&&!second.shell_hook_unregistered,primary.work_area.bottom,after_primary.work_area.bottom,restored,start_available);
                        match fs::write(&trace_output,trace){Ok(())=>*terminal_for_timer.borrow_mut()=Some(Ok(())),Err(error)=>*terminal_for_timer.borrow_mut()=Some(Err(format!("trace-write:{error}")))}
                    }else{*terminal_for_timer.borrow_mut()=Some(Err("primary-monitor-disappeared".into()))}},Err(error)=>*terminal_for_timer.borrow_mut()=Some(Err(error.into()))
                }
                let _=handle.update(app,|_,window,_|window.remove_window());app.quit();
            });
        }).detach();
    });
    terminal
        .borrow_mut()
        .take()
        .ok_or_else(|| "gpui-run-without-terminal".to_string())?
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            println!("{{\"admitted\":false,\"error\":\"{error}\"}}");
            ExitCode::from(2)
        }
    }
}
