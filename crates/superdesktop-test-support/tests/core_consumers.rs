use settings_store::{ExecutionPreference, RuntimeMode, SettingsV1};
use shell_core::{
    BridgeLaunchRequest, BridgeLaunchSource, CorrelationId, Generation, RequestId, ShellCommand,
    ShellEffect, ShellEvent, reduce,
};
use superdesktop_test_support::{FakeEffectAdapter, ShellFixtureBuilder};

#[test]
fn desktop_consumer_contract_compiles() {
    let state = ShellFixtureBuilder::new()
        .monitor("monitor:desktop", 120, true)
        .desktop_item("item:folder", true)
        .build();
    let transition = reduce(&state, &ShellEvent::DesktopOverflow);
    assert!(matches!(
        transition.effects[0],
        ShellEffect::RequestDesktopSnapshot { .. }
    ));
}

#[test]
fn taskbar_consumer_contract_compiles() {
    let fixture = ShellFixtureBuilder::new().window("window:task", "app:task", 1, true);
    let windows = fixture.windows();
    let mut state = fixture.build();
    let requested = reduce(&state, &ShellEvent::WindowOverflow);
    let (request_id, generation) = match requested.effects[0] {
        ShellEffect::RequestWindowSnapshot {
            request_id,
            generation,
        } => (request_id, generation),
        _ => panic!("window refresh expected"),
    };
    state = reduce(
        &requested.state,
        &ShellEvent::WindowsChanged {
            request_id,
            generation,
            windows,
        },
    )
    .state;
    assert_eq!(state.applications.len(), 1);
}

#[test]
fn bridge_consumer_contract_compiles() {
    let request = BridgeLaunchRequest::default_location(
        RequestId(8),
        CorrelationId(9),
        BridgeLaunchSource::TaskbarFixedEntry,
    );
    let transition = reduce(
        &ShellFixtureBuilder::new().build(),
        &ShellEvent::Command(ShellCommand::LaunchBridge(request)),
    );
    let mut adapter = FakeEffectAdapter::default();
    adapter.apply(transition.effects);
    assert_eq!(adapter.effects().len(), 1);
}

#[test]
fn lifecycle_consumer_contract_compiles() {
    let mut settings = SettingsV1::default();
    settings.execution_preference = ExecutionPreference::Shell;
    assert_eq!(settings.effective_mode(false), RuntimeMode::Preview);
    let state = ShellFixtureBuilder::new()
        .generation(Generation(4).get())
        .build();
    let transition = reduce(
        &state,
        &ShellEvent::Command(ShellCommand::StartShell {
            explicit_opt_in: true,
        }),
    );
    assert_eq!(
        transition.effects,
        vec![ShellEffect::ProbeShellPrerequisites]
    );
}
