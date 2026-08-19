fn main() {
    match platform_win::common::explorer_recovery::trusted_explorer_shell_present() {
        Ok(present) => println!("trusted_explorer_shell_present={present}"),
        Err(error) => {
            eprintln!("explorer presence probe failed: {error}");
            std::process::exit(1);
        }
    }
}
