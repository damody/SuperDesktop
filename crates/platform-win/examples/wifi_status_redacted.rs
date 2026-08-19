use platform_win::common::system_status::{network_status, refresh_wifi};
use shell_provider_protocol::StatusAvailability;

fn main() -> Result<(), String> {
    let network = network_status()?;
    let (availability, enabled, network_count, connected_count, saved_count, secure_count) =
        match network.wifi {
            StatusAvailability::Available(wifi) => {
                let connected = wifi
                    .networks
                    .iter()
                    .filter(|network| network.connected)
                    .count();
                let saved = wifi
                    .networks
                    .iter()
                    .filter(|network| network.profile_name.is_some())
                    .count();
                let secure = wifi
                    .networks
                    .iter()
                    .filter(|network| network.secure)
                    .count();
                (
                    "available",
                    wifi.enabled,
                    wifi.networks.len(),
                    connected,
                    saved,
                    secure,
                )
            }
            StatusAvailability::NotPresent => ("not_present", false, 0, 0, 0, 0),
            StatusAvailability::Unavailable { .. } => ("unavailable", false, 0, 0, 0, 0),
        };
    let refresh = if refresh_wifi().is_ok() {
        "accepted"
    } else {
        "unavailable"
    };
    println!(
        "{{\"schema\":\"superdesktop-wifi-redacted/v1\",\"wifi_availability\":\"{availability}\",\"enabled\":{enabled},\"network_count\":{network_count},\"connected_count\":{connected_count},\"saved_count\":{saved_count},\"secure_count\":{secure_count},\"refresh\":\"{refresh}\",\"identities_redacted\":true}}"
    );
    Ok(())
}
