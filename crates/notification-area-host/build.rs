#![allow(clippy::disallowed_methods, reason = "build scripts are exempt")]

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=resources/windows/notification-area-host.rc");
        println!("cargo:rerun-if-changed=resources/windows/notification-area-host.manifest.xml");
        embed_resource::compile(
            "resources/windows/notification-area-host.rc",
            embed_resource::NONE,
        )
        .manifest_required()
        .expect("notification-area-host manifest must compile");
    }
}
