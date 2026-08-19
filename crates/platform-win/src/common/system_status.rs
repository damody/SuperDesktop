// SAFETY: COM calls are scoped by `ComApartment`, returned COM interfaces own their lifetimes,
// Win32 output buffers are initialized and length-checked before conversion, and every HWND/HKL
// is treated as a copied opaque value and validated before a message or query is issued.

use std::{
    ffi::c_void,
    thread,
    time::{Duration, Instant},
    time::{SystemTime, UNIX_EPOCH},
};

use shell_provider_protocol::{
    AudioStatus, ClockCalendarStatus, InputProfile, InputStatus, MAX_TEXT_BYTES, MAX_WIFI_NETWORKS,
    NetworkStatus, PowerStatus, StatusAvailability, SystemStatusSnapshot, WifiNetwork, WifiStatus,
};
use windows::Win32::{
    Foundation::{HANDLE, LPARAM, RPC_E_CHANGED_MODE, WPARAM},
    Globalization::{GetUserDefaultLocaleName, LCIDToLocaleName},
    Media::Audio::{
        Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
    },
    NetworkManagement::WiFi::{
        WLAN_AVAILABLE_NETWORK, WLAN_AVAILABLE_NETWORK_CONNECTED,
        WLAN_AVAILABLE_NETWORK_HAS_PROFILE, WLAN_AVAILABLE_NETWORK_LIST,
        WLAN_CONNECTION_PARAMETERS, WLAN_INTERFACE_INFO_LIST, WlanCloseHandle, WlanConnect,
        WlanDisconnect, WlanEnumInterfaces, WlanFreeMemory, WlanGetAvailableNetworkList,
        WlanOpenHandle, WlanScan, dot11_BSS_type_any, wlan_connection_mode_profile,
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
use windows::core::{GUID, PCWSTR};

const LOCALE_NAME_CAPACITY: usize = 85;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcessId() -> u32;
    fn ProcessIdToSessionId(process_id: u32, session_id: *mut u32) -> i32;
}

struct ComApartment(bool);

const MAX_WIFI_INTERFACES: usize = 64;
const MAX_NATIVE_WIFI_NETWORKS: usize = MAX_WIFI_NETWORKS * 4;

struct WlanClient {
    handle: HANDLE,
}

impl WlanClient {
    fn open() -> Result<Self, String> {
        let mut negotiated = 0;
        let mut handle = HANDLE::default();
        let status = unsafe { WlanOpenHandle(2, None, &mut negotiated, &mut handle) };
        if status != 0 {
            return Err(format!("WLAN service unavailable ({status})"));
        }
        Ok(Self { handle })
    }

    fn interfaces(&self) -> Result<Vec<WlanInterface>, String> {
        let mut raw = std::ptr::null_mut::<WLAN_INTERFACE_INFO_LIST>();
        let status = unsafe { WlanEnumInterfaces(self.handle, None, &mut raw) };
        if status != 0 || raw.is_null() {
            return Err(format!("WLAN interface enumeration failed ({status})"));
        }
        let memory = WlanMemory(raw);
        let count = unsafe { (*memory.0).dwNumberOfItems as usize }.min(MAX_WIFI_INTERFACES);
        let entries =
            unsafe { std::slice::from_raw_parts((*memory.0).InterfaceInfo.as_ptr(), count) };
        Ok(entries
            .iter()
            .map(|entry| WlanInterface {
                guid: entry.InterfaceGuid,
                id: wifi_interface_id(&entry.InterfaceGuid),
            })
            .collect())
    }

    fn available_networks(&self, interface: &WlanInterface) -> Result<Vec<WifiNetwork>, String> {
        let mut raw = std::ptr::null_mut::<WLAN_AVAILABLE_NETWORK_LIST>();
        let status =
            unsafe { WlanGetAvailableNetworkList(self.handle, &interface.guid, 0, None, &mut raw) };
        if status != 0 || raw.is_null() {
            return Err(format!("WLAN network enumeration failed ({status})"));
        }
        let memory = WlanMemory(raw);
        let count = unsafe { (*memory.0).dwNumberOfItems as usize }.min(MAX_NATIVE_WIFI_NETWORKS);
        let entries = unsafe { std::slice::from_raw_parts((*memory.0).Network.as_ptr(), count) };
        Ok(entries
            .iter()
            .filter_map(|entry| wifi_network_from_native(&interface.id, entry))
            .collect())
    }
}

impl Drop for WlanClient {
    fn drop(&mut self) {
        let _ = unsafe { WlanCloseHandle(self.handle, None) };
    }
}

struct WlanMemory<T>(*mut T);

impl<T> Drop for WlanMemory<T> {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { WlanFreeMemory(self.0.cast()) };
        }
    }
}

#[derive(Clone, Debug)]
struct WlanInterface {
    guid: GUID,
    id: String,
}

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
        wifi: wifi_status().unwrap_or_else(|reason| StatusAvailability::Unavailable { reason }),
    })
}

pub fn wifi_status() -> Result<StatusAvailability<WifiStatus>, String> {
    let client = WlanClient::open()?;
    let interfaces = client.interfaces()?;
    if interfaces.is_empty() {
        return Ok(StatusAvailability::NotPresent);
    }
    let mut networks = Vec::new();
    let mut failures = Vec::new();
    for interface in &interfaces {
        match client.available_networks(interface) {
            Ok(items) => networks.extend(items),
            Err(error) => failures.push(error),
        }
    }
    if networks.is_empty() && failures.len() == interfaces.len() {
        return Err("all WLAN interfaces rejected network enumeration".into());
    }
    Ok(StatusAvailability::Available(WifiStatus {
        enabled: true,
        networks: reduce_wifi_networks(networks),
    }))
}

pub fn refresh_wifi() -> Result<(), String> {
    let client = WlanClient::open()?;
    let interfaces = client.interfaces()?;
    if interfaces.is_empty() {
        return Err("no WLAN interface is present".into());
    }
    let accepted = interfaces
        .iter()
        .filter(|interface| unsafe {
            WlanScan(client.handle, &interface.guid, None, None, None) == 0
        })
        .count();
    if accepted == 0 {
        Err("every WLAN interface rejected scan".into())
    } else {
        Ok(())
    }
}

pub fn connect_wifi_profile(interface_id: &str, profile_name: &str) -> Result<(), String> {
    if interface_id.trim().is_empty() || profile_name.trim().is_empty() {
        return Err("WLAN interface and profile identity are required".into());
    }
    if interface_id.len() > MAX_TEXT_BYTES || profile_name.len() > MAX_TEXT_BYTES {
        return Err("WLAN interface or profile identity exceeds the text limit".into());
    }
    let client = WlanClient::open()?;
    let interface = client
        .interfaces()?
        .into_iter()
        .find(|interface| interface.id == interface_id)
        .ok_or_else(|| "WLAN interface identity is stale".to_owned())?;
    let admitted = client
        .available_networks(&interface)?
        .into_iter()
        .any(|network| {
            network.connectable && network.profile_name.as_deref() == Some(profile_name)
        });
    if !admitted {
        return Err("saved WLAN profile is not currently connectable".into());
    }
    let profile_wide = profile_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let parameters = WLAN_CONNECTION_PARAMETERS {
        wlanConnectionMode: wlan_connection_mode_profile,
        strProfile: PCWSTR(profile_wide.as_ptr()),
        pDot11Ssid: std::ptr::null_mut(),
        pDesiredBssidList: std::ptr::null_mut(),
        dot11BssType: dot11_BSS_type_any,
        dwFlags: 0,
    };
    let status = unsafe { WlanConnect(client.handle, &interface.guid, &parameters, None) };
    if status == 0 {
        Ok(())
    } else {
        Err(format!("WLAN connect request rejected ({status})"))
    }
}

pub fn disconnect_wifi(interface_id: &str) -> Result<(), String> {
    if interface_id.trim().is_empty() {
        return Err("WLAN interface identity is required".into());
    }
    if interface_id.len() > MAX_TEXT_BYTES {
        return Err("WLAN interface identity exceeds the text limit".into());
    }
    let client = WlanClient::open()?;
    let interface = client
        .interfaces()?
        .into_iter()
        .find(|interface| interface.id == interface_id)
        .ok_or_else(|| "WLAN interface identity is stale".to_owned())?;
    let connected = client
        .available_networks(&interface)?
        .iter()
        .any(|network| network.connected);
    if !connected {
        return Err("WLAN interface is not connected".into());
    }
    let status = unsafe { WlanDisconnect(client.handle, &interface.guid, None) };
    if status == 0 {
        Ok(())
    } else {
        Err(format!("WLAN disconnect request rejected ({status})"))
    }
}

fn wifi_interface_id(guid: &GUID) -> String {
    format!("{guid:?}")
}

fn wifi_network_from_native(
    interface_id: &str,
    native: &WLAN_AVAILABLE_NETWORK,
) -> Option<WifiNetwork> {
    let ssid_length = usize::try_from(native.dot11Ssid.uSSIDLength)
        .ok()?
        .min(native.dot11Ssid.ucSSID.len());
    if ssid_length == 0 {
        return None;
    }
    let ssid = String::from_utf8_lossy(&native.dot11Ssid.ucSSID[..ssid_length])
        .trim()
        .to_owned();
    if ssid.is_empty() {
        return None;
    }
    let has_profile = native.dwFlags & WLAN_AVAILABLE_NETWORK_HAS_PROFILE != 0;
    let profile_name = has_profile
        .then(|| utf16_nul_terminated(&native.strProfileName))
        .flatten()
        .filter(|profile| !profile.trim().is_empty());
    Some(WifiNetwork {
        interface_id: interface_id.to_owned(),
        ssid,
        profile_name,
        signal_quality: native.wlanSignalQuality.min(100) as u8,
        secure: native.bSecurityEnabled.as_bool(),
        connected: native.dwFlags & WLAN_AVAILABLE_NETWORK_CONNECTED != 0,
        connectable: native.bNetworkConnectable.as_bool(),
    })
}

fn reduce_wifi_networks(networks: Vec<WifiNetwork>) -> Vec<WifiNetwork> {
    let mut by_ssid = std::collections::BTreeMap::<String, WifiNetwork>::new();
    for network in networks {
        let replace = by_ssid
            .get(&network.ssid)
            .is_none_or(|current| wifi_network_rank(&network) > wifi_network_rank(current));
        if replace {
            by_ssid.insert(network.ssid.clone(), network);
        }
    }
    let mut networks = by_ssid.into_values().collect::<Vec<_>>();
    networks.sort_by(|left, right| {
        right
            .connected
            .cmp(&left.connected)
            .then_with(|| {
                right
                    .profile_name
                    .is_some()
                    .cmp(&left.profile_name.is_some())
            })
            .then_with(|| right.signal_quality.cmp(&left.signal_quality))
            .then_with(|| left.ssid.cmp(&right.ssid))
    });
    networks.truncate(MAX_WIFI_NETWORKS);
    networks
}

fn wifi_network_rank(network: &WifiNetwork) -> (bool, bool, u8, bool) {
    (
        network.connected,
        network.profile_name.is_some(),
        network.signal_quality,
        network.connectable,
    )
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

    #[test]
    fn wifi_native_decoder_bounds_lengths_and_preserves_truthful_flags() {
        let mut native = WLAN_AVAILABLE_NETWORK::default();
        native.dot11Ssid.uSSIDLength = 40;
        native.dot11Ssid.ucSSID[..4].copy_from_slice(b"wifi");
        native.strProfileName[..8].copy_from_slice(&[
            b'p' as u16,
            b'r' as u16,
            b'o' as u16,
            b'f' as u16,
            b'i' as u16,
            b'l' as u16,
            b'e' as u16,
            0,
        ]);
        native.wlanSignalQuality = 140;
        native.bSecurityEnabled = true.into();
        native.bNetworkConnectable = true.into();
        native.dwFlags = WLAN_AVAILABLE_NETWORK_CONNECTED | WLAN_AVAILABLE_NETWORK_HAS_PROFILE;
        let decoded = wifi_network_from_native("interface", &native).unwrap();
        assert!(decoded.ssid.starts_with("wifi"));
        assert_eq!(decoded.profile_name.as_deref(), Some("profile"));
        assert_eq!(decoded.signal_quality, 100);
        assert!(decoded.secure && decoded.connected && decoded.connectable);

        native.dot11Ssid.uSSIDLength = 0;
        assert!(wifi_network_from_native("interface", &native).is_none());
    }

    #[test]
    fn wifi_reducer_deduplicates_and_orders_connected_saved_strongest() {
        let network =
            |ssid: &str, interface: &str, connected: bool, saved: bool, signal_quality: u8| {
                WifiNetwork {
                    interface_id: interface.into(),
                    ssid: ssid.into(),
                    profile_name: saved.then(|| format!("profile-{ssid}")),
                    signal_quality,
                    secure: true,
                    connected,
                    connectable: true,
                }
            };
        let reduced = reduce_wifi_networks(vec![
            network("same", "weak", false, false, 20),
            network("same", "saved", false, true, 10),
            network("connected", "live", true, true, 30),
            network("strong", "strong", false, false, 90),
        ]);
        assert_eq!(reduced.len(), 3);
        assert_eq!(reduced[0].ssid, "connected");
        assert_eq!(reduced[1].ssid, "same");
        assert_eq!(reduced[1].interface_id, "saved");
        assert_eq!(reduced[2].ssid, "strong");
    }

    #[test]
    fn wifi_adapter_source_owns_handles_memory_and_exact_command_identity() {
        let source = include_str!("system_status.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "struct WlanClient",
            "WlanCloseHandle",
            "struct WlanMemory",
            "WlanFreeMemory",
            "WlanEnumInterfaces",
            "WlanGetAvailableNetworkList",
            "WlanScan",
            "WlanConnect",
            "WlanDisconnect",
            "saved WLAN profile is not currently connectable",
            "WLAN interface identity is stale",
        ] {
            assert!(production.contains(required), "missing {required}");
        }
        for forbidden in ["WlanSetProfile", "profile xml", "password"] {
            assert!(!production.contains(forbidden), "forbidden {forbidden}");
        }
    }

    #[test]
    fn wifi_commands_reject_empty_oversized_stale_and_unsaved_identity_before_mutation() {
        assert!(connect_wifi_profile("", "profile").is_err());
        assert!(connect_wifi_profile("interface", "").is_err());
        assert!(disconnect_wifi("").is_err());
        let oversized = "x".repeat(MAX_TEXT_BYTES + 1);
        assert!(connect_wifi_profile(&oversized, "profile").is_err());
        assert!(connect_wifi_profile("interface", &oversized).is_err());
        assert!(disconnect_wifi(&oversized).is_err());
        assert!(disconnect_wifi("{00000000-0000-0000-0000-000000000000}").is_err());

        if let Ok(StatusAvailability::Available(wifi)) = wifi_status()
            && let Some(network) = wifi.networks.first()
        {
            assert!(
                connect_wifi_profile(&network.interface_id, "superdesktop-unsaved-fixture")
                    .is_err()
            );
        }
    }
}
