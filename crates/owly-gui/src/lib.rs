//! The Owly window: a Tauri shell over `owly-session`. The frontend, in
//! `ui/`, only presents; everything that touches the radio, the network or
//! the disk is here.

mod commands;
mod error;
mod model;
mod state;
mod validate;

use std::path::PathBuf;

use tauri::Manager;

use crate::state::{AppState, Config};

const DEFAULT_IFACE: &str = "awdl0";

fn config(app: &tauri::App) -> Result<Config, Box<dyn std::error::Error>> {
    let iface = std::env::var("OWLY_IFACE").unwrap_or_else(|_| DEFAULT_IFACE.into());
    validate::interface(&iface)?;
    let receive_dir = match std::env::var_os("OWLY_RECEIVE_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => app.path().download_dir()?.join("Owly"),
    };
    Ok(Config {
        iface,
        name: owly_session::link::host_name()?,
        bus: std::env::var("OWLY_BUS").ok(),
        receive_dir,
    })
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();
    owly_session::link::install_crypto();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config = config(app)?;
            app.manage(AppState::new(config));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::acquire_radio,
            commands::release_radio,
            commands::doctor,
            commands::start_browse,
            commands::stop_browse,
            commands::send,
            commands::set_visibility,
            commands::respond,
        ])
        .run(tauri::generate_context!())
        .expect("the Owly window failed to start");
}
