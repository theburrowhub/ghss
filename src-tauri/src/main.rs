// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `--version` / `--help` por terminal: responde y sale sin abrir la ventana.
    // Lo usa, entre otros, el bloque `test` de la fórmula de Homebrew.
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("ghss {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--help" | "-h" => {
                println!("ghss {} — GitHub Settings Sync", env!("CARGO_PKG_VERSION"));
                println!("App de escritorio: ejecútala sin argumentos para abrir la interfaz.");
                return;
            }
            _ => {}
        }
    }
    ghss_lib::run()
}
