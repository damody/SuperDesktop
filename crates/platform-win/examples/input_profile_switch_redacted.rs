use std::time::Duration;

use platform_win::common::system_status::{input_status, request_input_profile};

fn main() -> Result<(), String> {
    let before = input_status()?;
    let original_id = before.active_profile_id.clone();
    let Some(alternate_id) = before
        .profiles
        .iter()
        .find(|profile| profile.id != original_id)
        .map(|profile| profile.id.clone())
    else {
        println!(
            "{{\"schema\":\"superdesktop-input-switch-redacted/v1\",\"result\":\"not_applicable\",\"profile_count\":{},\"identities_redacted\":true}}",
            before.profiles.len()
        );
        return Ok(());
    };

    let switched = request_input_profile(&alternate_id, Duration::from_secs(2));
    let switched_observed = switched
        .as_ref()
        .is_ok_and(|status| status.active_profile_id == alternate_id);
    let restored = request_input_profile(&original_id, Duration::from_secs(2));
    let restored_observed = restored
        .as_ref()
        .is_ok_and(|status| status.active_profile_id == original_id);
    if !switched_observed || !restored_observed {
        return Err(format!(
            "controlled input switch failed (switched={switched_observed}, restored={restored_observed})"
        ));
    }
    println!(
        "{{\"schema\":\"superdesktop-input-switch-redacted/v1\",\"result\":\"passed\",\"profile_count\":{},\"switched_observed\":true,\"original_restored\":true,\"identities_redacted\":true}}",
        before.profiles.len()
    );
    Ok(())
}
