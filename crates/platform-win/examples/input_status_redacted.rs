use platform_win::common::system_status::input_status;
use shell_provider_protocol::InputProfileKind;

fn main() -> Result<(), String> {
    let input = input_status()?;
    let processors = input
        .profiles
        .iter()
        .filter(|profile| profile.kind == InputProfileKind::InputProcessor)
        .count();
    let keyboards = input
        .profiles
        .iter()
        .filter(|profile| profile.kind == InputProfileKind::KeyboardLayout)
        .count();
    let legacy = input
        .profiles
        .iter()
        .filter(|profile| profile.kind == InputProfileKind::LegacyKeyboardLayout)
        .count();
    let named = input
        .profiles
        .iter()
        .filter(|profile| !profile.input_method_name.trim().is_empty())
        .count();
    println!(
        "{{\"schema\":\"superdesktop-input-redacted/v1\",\"profile_count\":{},\"input_processor_count\":{processors},\"keyboard_count\":{keyboards},\"legacy_fallback_count\":{legacy},\"authoritatively_named_count\":{named},\"active_present\":{},\"identities_redacted\":true}}",
        input.profiles.len(),
        input
            .profiles
            .iter()
            .any(|profile| profile.id == input.active_profile_id)
    );
    Ok(())
}
