use shell_provider_protocol::{CURRENT_PROTOCOL, ProviderCapability, contract_manifest};

#[test]
fn shell_core_can_consume_provider_contract_without_platform_types() {
    assert_eq!(CURRENT_PROTOCOL.major, 1);
    assert_eq!(
        ProviderCapability::SearchFiles,
        ProviderCapability::SearchFiles
    );
    assert_eq!(contract_manifest()["protocol"]["major"], 1);
}
