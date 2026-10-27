// Keeps Windows from opening a console behind the window, as Tauri apps do.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    owly_gui::run();
}
