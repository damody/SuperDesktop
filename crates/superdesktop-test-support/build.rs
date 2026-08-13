use std::{env, fs, path::PathBuf, process::Command};

fn write_minimal_icon(path: &PathBuf) {
    let icon: [u8; 70] = [
        0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 32, 0, 48, 0, 0, 0, 22, 0, 0, 0, 40, 0, 0, 0, 1, 0, 0,
        0, 2, 0, 0, 0, 1, 0, 32, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 215, 120, 0, 255, 0, 0, 0, 0,
    ];
    fs::write(path, icon).expect("generated icon must be writable");
}

fn main() {
    if env::var("CARGO_CFG_WINDOWS").is_err() {
        return;
    }

    println!("cargo:rerun-if-changed=resources/superdesktop-test-support.rc");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set"));
    let icon = out_dir.join("superdesktop-test-support.ico");
    write_minimal_icon(&icon);
    let resource_script = out_dir.join("superdesktop-test-support.generated.rc");
    let icon_path = icon.display().to_string().replace('\\', "/");
    fs::write(
        &resource_script,
        format!(
            "{}\n201 ICON \"{}\"\n",
            include_str!("resources/superdesktop-test-support.rc"),
            icon_path
        ),
    )
    .expect("generated resource script must be writable");
    let output = out_dir.join("superdesktop-test-support.res");
    let status = Command::new("llvm-rc.exe")
        .arg("/fo")
        .arg(&output)
        .arg(&resource_script)
        .status()
        .expect("llvm-rc.exe must be installed to compile Windows product resources");
    assert!(
        status.success(),
        "llvm-rc.exe failed to compile test-support resources"
    );
    println!("cargo:rustc-link-arg={}", output.display());
}
