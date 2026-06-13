// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `--version` / `--help` from the terminal: respond and exit without opening the window.
    // Used by, among others, the `test` block of the Homebrew formula.
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("ghss {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--help" | "-h" => {
                println!("ghss {} — GitHub Settings Sync", env!("CARGO_PKG_VERSION"));
                println!("Desktop app: run it without arguments to open the interface.");
                return;
            }
            _ => {}
        }
    }
    ghss_lib::run()
}
