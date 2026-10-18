// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

fn main() {
    chrono_snap_lib::run()
}
