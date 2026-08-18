use std::{
    ffi::c_void,
    thread,
    time::{Duration, Instant},
    time::{SystemTime, UNIX_EPOCH},
};

use shell_provider_protocol::{
    AudioStatus, ClockCalendarStatus, InputProfile, InputStatus, NetworkStatus, PowerStatus,
    StatusAvailability, SystemStatusSnapshot,
};
use windows::Win32::{
    Foundation::{LPARAM, RPC_E_CHANGED_MODE, WPARAM},
    Globalization::{GetUserDefaultLocaleName, LCIDToLocaleName},
    Media::Audio::{
        Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
    },
    Networking::NetworkListManager::{
        INetworkListManager, NLM_ENUM_NETWORK_CONNECTED, NetworkListManager,
    },
    System::{
        Com::{
            CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
            CoUninitialize,
        },
        Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
        Time::{DYNAMIC_TIME_ZONE_INFORMATION, GetDynamicTimeZoneInformation},
    },
    UI::{
        Input::KeyboardAndMouse::{GetKeyboardLayout, GetKeyboardLayoutList, HKL},
        WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId, PostMessageW, WM_INPUTLANGCHANGEREQUEST,
        },
    },
};

const LOCALE_NAME_CAPACITY: usize = 85;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcessId() -> u32;
    fn ProcessIdToSessionId(process_id: u32, session_id: *mut u32) -> i32;
}

struct ComApartment(bool);

impl ComApartment {
    fn enter() -> Result<Self, String> {
        let status = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if status.is_ok() {
            Ok(Self(true))
        } else if status == RPC_E_CHANGED_MODE {
            Ok(Self(false))
        } else {
            Err(format!("COM initialization failed: {status:?}"))
        }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

pub fn audio_status() -> Result<AudioStatus, String> {
    let _apartment = ComApartment::enter()?;
    let (endpoint, endpoint_id) = default_audio_endpoint()?;
    let volume_percent = unsafe { endpoint.GetMasterVolumeLevelScalar() }
        .map_err(|error| format!("default audio volume read failed: {error}"))?;
    let muted = unsafe { endpoint.GetMute() }
        .map_err(|error| format!("default audio mute read failed: {error}"))?
        .as_bool();
    Ok(AudioStatus {
        endpoint_id,
        volume_percent: (volume_percent.clamp(0.0, 1.0) * 100.0).round() as u8,
        muted,
    })
}

pub fn set_volume_and_observe(volume_percent: u8) -> Result<AudioStatus, String> {
    if volume_percent > 100 {
        return Err("volume percent is out of range".into());
    }
    let _apartment = ComApartment::enter()?;
    let (endpoint, _) = default_audio_endpoint()?;
    unsafe {
        endpoint.SetMasterVolumeLevelScalar(f32::from(volume_percent) / 100.0, std::ptr::null())
    }
    .map_err(|error| format!("default audio volume write failed: {error}"))?;
    let observed = audio_status()?;
    if observed.volume_percent.abs_diff(volume_percent) > 1 {
        return Err("requested audio volume was not observed".into());
    }
    Ok(observed)
}

pub fn set_mute_and_observe(muted: bool) -> Result<AudioStatus, String> {
    let _apartment = ComApartment::enter()?;
    let (endpoint, _) = default_audio_endpoint()?;
    unsafe { endpoint.SetMute(muted, std::ptr::null()) }
        .map_err(|error| format!("default audio mute write failed: {error}"))?;
    let observed = audio_status()?;
    if observed.muted != muted {
        return Err("requested audio mute state was not observed".into());
    }
    Ok(observed)
}

pub fn network_status() -> Result<NetworkStatus, String> {
    let _apartment = ComApartment::enter()?;
    let manager: INetworkListManager =
        unsafe { CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL) }
            .map_err(|error| format!("Network List Manager is unavailable: {error}"))?;
    let connected = unsafe { manager.IsConnected() }
        .map_err(|error| format!("network connected-state read failed: {error}"))?
        .as_bool();
    let internet = unsafe { manager.IsConnectedToInternet() }
        .map_err(|error| format!("network internet-state read failed: {error}"))?
        .as_bool();
    let display_name = if connected {
        connected_network_name(&manager).unwrap_or_else(|| "Connected network".into())
    } else {
        "Disconnected".into()
    };
    Ok(NetworkStatus {
        connected,
        internet,
        display_name,
    })
}

pub fn power_status() -> Result<StatusAvailability<PowerStatus>, String> {
    let mut native = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut native) }
        .map_err(|error| format!("system power status read failed: {error}"))?;
    if native.BatteryFlag & 128 != 0 {
        return Ok(StatusAvailability::NotPresent);
    }
    let battery_percent = (native.BatteryLifePercent <= 100).then_some(native.BatteryLifePercent);
    Ok(StatusAvailability::Available(PowerStatus {
        ac_online: native.ACLineStatus == 1,
        charging: native.BatteryFlag & 8 != 0,
        battery_percent,
    }))
}

fn default_audio_endpoint() -> Result<(IAudioEndpointVolume, String), String> {
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
            .map_err(|error| format!("MMDevice enumerator is unavailable: {error}"))?;
    let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) }
        .map_err(|error| format!("default render endpoint is unavailable: {error}"))?;
    let id = unsafe { device.GetId() }
        .map_err(|error| format!("default endpoint identity read failed: {error}"))?;
    let endpoint_id = unsafe { id.to_string() }
        .map_err(|error| format!("default endpoint identity conversion failed: {error}"))?;
    unsafe { CoTaskMemFree(Some(id.0.cast())) };
    let endpoint = unsafe { device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) }
        .map_err(|error| format!("default endpoint volume interface is unavailable: {error}"))?;
    Ok((endpoint, endpoint_id))
}

fn connected_network_name(manager: &INetworkListManager) -> Option<String> {
    let networks = unsafe { manager.GetNetworks(NLM_ENUM_NETWORK_CONNECTED) }.ok()?;
    let mut item = [None];
    let mut fetched = 0u32;
    unsafe { networks.Next(&mut item, Some(&mut fetched)) }.ok()?;
    if fetched != 1 {
        return None;
    }
    unsafe { item[0].as_ref()?.GetName() }
        .ok()
        .map(|name| name.to_string())
}

pub fn input_status() -> Result<InputStatus, String> {
    let mut layouts = keyboard_layouts()?;
    let active = foreground_layout()?;
    if !layouts.iter().any(|layout| layout.0 == active.0) {
        layouts.push(active);
    }
    layouts.sort_by_key(|layout| layout.0 as usize);
    layouts.dedup_by_key(|layout| layout.0 as usize);
    let profiles = layouts
        .into_iter()
        .map(profile_from_hkl)
        .collect::<Vec<_>>();
    let active_profile_id = profile_id(active);
    if !profiles
        .iter()
        .any(|profile| profile.id == active_profile_id)
    {
        return Err("active input profile was not enumerated".into());
    }
    Ok(InputStatus {
        active_profile_id,
        profiles,
    })
}

pub fn system_status_snapshot(
    host_generation: u64,
    snapshot_generation: u64,
) -> Result<SystemStatusSnapshot, String> {
    if host_generation == 0 || snapshot_generation == 0 {
        return Err("system status generations must be non-zero".into());
    }
    Ok(SystemStatusSnapshot {
        host_generation,
        snapshot_generation,
        network: available_or_unavailable(network_status()),
        audio: available_or_unavailable(audio_status()),
        power: power_status().unwrap_or_else(|reason| StatusAvailability::Unavailable { reason }),
        clock: available_or_unavailable(clock_calendar_status()),
        input: available_or_unavailable(input_status()),
        overflowed: false,
    })
}

pub fn clock_calendar_status() -> Result<ClockCalendarStatus, String> {
    let mut locale_buffer = [0u16; LOCALE_NAME_CAPACITY];
    let locale_length = unsafe { GetUserDefaultLocaleName(&mut locale_buffer) };
    if locale_length <= 1 {
        return Err("user locale name is unavailable".into());
    }
    let locale = String::from_utf16_lossy(&locale_buffer[..locale_length as usize - 1]);
    let mut zone = DYNAMIC_TIME_ZONE_INFORMATION::default();
    let disposition = unsafe { GetDynamicTimeZoneInformation(&mut zone) };
    if disposition == u32::MAX {
        return Err("dynamic time zone information is unavailable".into());
    }
    let time_zone = utf16_nul_terminated(&zone.TimeZoneKeyName)
        .or_else(|| utf16_nul_terminated(&zone.StandardName))
        .ok_or_else(|| "time zone identity is unavailable".to_owned())?;
    let unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before the Unix epoch".to_owned())?
        .as_millis()
        .try_into()
        .map_err(|_| "system clock value is out of range".to_owned())?;
    Ok(ClockCalendarStatus {
        unix_ms,
        locale,
        time_zone,
    })
}

fn available_or_unavailable<T>(result: Result<T, String>) -> StatusAvailability<T> {
    match result {
        Ok(value) => StatusAvailability::Available(value),
        Err(reason) => StatusAvailability::Unavailable { reason },
    }
}

fn utf16_nul_terminated(value: &[u16]) -> Option<String> {
    let length = value
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(value.len());
    (length != 0).then(|| String::from_utf16_lossy(&value[..length]))
}

pub fn request_input_profile(profile_id: &str, timeout: Duration) -> Result<InputStatus, String> {
    request_input_profile_for_session(profile_id, current_session_id()?, timeout)
}

pub fn current_session_id() -> Result<u32, String> {
    let mut session_id = 0u32;
    if unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session_id) } == 0 {
        Err("interactive session identity is unavailable".into())
    } else {
        Ok(session_id)
    }
}

pub fn request_input_profile_for_session(
    profile_id: &str,
    expected_session_id: u32,
    timeout: Duration,
) -> Result<InputStatus, String> {
    if current_session_id()? != expected_session_id {
        return Err("input profile request belongs to another session".into());
    }
    let before = input_status()?;
    if !before
        .profiles
        .iter()
        .any(|profile| profile.id == profile_id)
    {
        return Err("requested input profile is not installed".into());
    }
    let layout = parse_profile_id(profile_id)?;
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return Err("foreground window is unavailable".into());
    }
    unsafe {
        PostMessageW(
            Some(hwnd),
            WM_INPUTLANGCHANGEREQUEST,
            WPARAM(0),
            LPARAM(layout.0 as isize),
        )
    }
    .map_err(|error| format!("input profile request failed: {error}"))?;

    wait_for_input_profile_observation(profile_id, timeout, input_status)
}

fn wait_for_input_profile_observation(
    profile_id: &str,
    timeout: Duration,
    mut observe: impl FnMut() -> Result<InputStatus, String>,
) -> Result<InputStatus, String> {
    let deadline = Instant::now() + timeout;
    loop {
        let observed = observe()?;
        if observed.active_profile_id == profile_id {
            return Ok(observed);
        }
        if Instant::now() >= deadline {
            return Err("input profile activation was not observed before deadline".into());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn keyboard_layouts() -> Result<Vec<HKL>, String> {
    let count = unsafe { GetKeyboardLayoutList(None) };
    if count <= 0 {
        return Err("Windows returned no installed keyboard layouts".into());
    }
    let mut layouts = vec![HKL::default(); count as usize];
    let written = unsafe { GetKeyboardLayoutList(Some(&mut layouts)) };
    if written <= 0 {
        return Err("Windows keyboard layout enumeration failed".into());
    }
    layouts.truncate(written as usize);
    Ok(layouts)
}

fn foreground_layout() -> Result<HKL, String> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return Err("foreground window is unavailable".into());
    }
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, None) };
    if thread_id == 0 {
        return Err("foreground input thread is unavailable".into());
    }
    let layout = unsafe { GetKeyboardLayout(thread_id) };
    if layout.0.is_null() {
        Err("foreground keyboard layout is unavailable".into())
    } else {
        Ok(layout)
    }
}

fn profile_from_hkl(layout: HKL) -> InputProfile {
    let raw = layout.0 as usize as u64;
    let language_id = (raw & 0xffff) as u32;
    let language_tag = locale_name(language_id).unwrap_or_else(|| format!("und-{language_id:04x}"));
    InputProfile {
        id: profile_id(layout),
        display_name: language_tag.clone(),
        language_tag,
    }
}

fn locale_name(language_id: u32) -> Option<String> {
    let mut buffer = [0u16; LOCALE_NAME_CAPACITY];
    let written = unsafe { LCIDToLocaleName(language_id, Some(&mut buffer), 0) };
    if written <= 1 {
        None
    } else {
        Some(String::from_utf16_lossy(&buffer[..written as usize - 1]))
    }
}

fn profile_id(layout: HKL) -> String {
    format!("hkl:{:016x}", layout.0 as usize as u64)
}

fn parse_profile_id(value: &str) -> Result<HKL, String> {
    let raw = value
        .strip_prefix("hkl:")
        .ok_or_else(|| "input profile identity prefix is invalid".to_owned())?;
    if raw.len() != 16 || !raw.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("input profile identity is invalid".into());
    }
    let value =
        u64::from_str_radix(raw, 16).map_err(|_| "input profile identity is invalid".to_owned())?;
    if value == 0 {
        return Err("input profile identity is null".into());
    }
    Ok(HKL(value as usize as *mut c_void))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_provider_protocol::Validate;

    #[test]
    fn real_input_profiles_are_bounded_stable_and_include_active_profile() {
        let status = input_status().unwrap();
        status.validate().unwrap();
        assert!(!status.profiles.is_empty());
        assert!(status.profiles.len() <= shell_provider_protocol::MAX_INPUT_PROFILES);
        assert!(
            status
                .profiles
                .iter()
                .any(|profile| profile.id == status.active_profile_id)
        );
    }

    #[test]
    fn invalid_missing_and_same_profile_requests_are_fail_closed_or_observed() {
        assert!(request_input_profile("invalid", Duration::from_millis(1)).is_err());
        assert!(request_input_profile("hkl:0000000000000001", Duration::from_millis(1)).is_err());
        let before = input_status().unwrap();
        let observed =
            request_input_profile(&before.active_profile_id, Duration::from_millis(500)).unwrap();
        assert_eq!(observed.active_profile_id, before.active_profile_id);
    }

    #[test]
    fn wrong_session_and_unobserved_activation_timeout_fail_closed() {
        let session = current_session_id().unwrap();
        let before = input_status().unwrap();
        assert!(
            request_input_profile_for_session(
                &before.active_profile_id,
                session.wrapping_add(1),
                Duration::from_millis(1),
            )
            .unwrap_err()
            .contains("another session")
        );
        let never_active = InputStatus {
            active_profile_id: "hkl:0000000000000001".into(),
            profiles: vec![InputProfile {
                id: "hkl:0000000000000001".into(),
                language_tag: "und".into(),
                display_name: "fixture".into(),
            }],
        };
        assert!(
            wait_for_input_profile_observation("hkl:0000000000000002", Duration::ZERO, || Ok(
                never_active.clone()
            ),)
            .unwrap_err()
            .contains("before deadline")
        );
    }

    #[test]
    fn real_audio_network_and_power_observations_are_bounded() {
        use shell_provider_protocol::Validate;

        let audio = audio_status().unwrap();
        audio.validate().unwrap();
        let network = network_status().unwrap();
        network.validate().unwrap();
        let power = power_status().unwrap();
        power.validate().unwrap();
    }

    #[test]
    fn audio_commands_validate_ranges_and_confirm_current_state_without_drift() {
        assert!(set_volume_and_observe(101).is_err());
        let before = audio_status().unwrap();
        let volume = set_volume_and_observe(before.volume_percent).unwrap();
        assert!(volume.volume_percent.abs_diff(before.volume_percent) <= 1);
        let mute = set_mute_and_observe(before.muted).unwrap();
        assert_eq!(mute.muted, before.muted);
    }

    #[test]
    fn live_clock_and_complete_snapshot_are_real_and_valid() {
        use shell_provider_protocol::Validate;

        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let clock = clock_calendar_status().unwrap();
        assert!(clock.unix_ms >= before);
        clock.validate().unwrap();
        let snapshot = system_status_snapshot(1, 1).unwrap();
        snapshot.validate().unwrap();
        assert_eq!(snapshot.host_generation, 1);
        assert_eq!(snapshot.snapshot_generation, 1);
    }
}
