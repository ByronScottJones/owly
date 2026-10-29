/// Every command the window may call. Listing them makes each one need a
/// permission in a capability file (`allow-<command>`), rather than being
/// open to any window by default.
const COMMANDS: &[&str] = &[
    "get_status",
    "acquire_radio",
    "release_radio",
    "doctor",
    "start_browse",
    "stop_browse",
    "choose_items",
    "discard_selection",
    "send_selection",
    "send_link",
    "set_visibility",
    "respond",
    "open_link",
    "set_settings",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("the Tauri build step failed");
}
