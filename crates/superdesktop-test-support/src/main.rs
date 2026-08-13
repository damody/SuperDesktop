//! Dedicated binary carrying a machine-verifiable test-support identity.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod identity;

fn main() {
    let _ = (identity::APP_USER_MODEL_ID, identity::ORIGINAL_FILENAME);

    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("validate-json-schema") {
        let schema_path = args.next().expect("schema path");
        let instance_path = args.next().expect("instance path");
        let schema: serde_json::Value =
            serde_json::from_slice(&std::fs::read(schema_path).expect("schema read"))
                .expect("schema json");
        let instance: serde_json::Value =
            serde_json::from_slice(&std::fs::read(instance_path).expect("instance read"))
                .expect("instance json");
        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .should_validate_formats(true)
            .build(&schema)
            .expect("Draft 2020-12 schema");
        if let Err(error) = validator.validate(&instance) {
            eprintln!("JSON_SCHEMA_INVALID: {error}");
            std::process::exit(1);
        }
    }
}
