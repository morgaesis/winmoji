#![cfg_attr(not(feature = "console"), windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    if let Err(error) = winmoji::app::run() {
        winmoji::app::report_fatal(&error.to_string());
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("winmoji runs on Windows");
    std::process::exit(1);
}
