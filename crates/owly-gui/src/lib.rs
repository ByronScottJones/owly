//! The Owly window: a Tauri shell over `owly-session`. The frontend, in
//! `ui/`, only presents; everything that touches the radio, the network or
//! the disk is here.
//!
//! Trust boundaries: the webview is our own static page under a strict CSP,
//! but its calls are still checked as untrusted (docs: tauri.md §5). It
//! names a receiver only by an id from the last scan, and files only by a
//! selection the backend made; what peers send is made fit to show
//! ([`model::display_text`]) before the page sees it.

mod commands;
mod error;
mod model;
mod state;
mod validate;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{Manager, RunEvent, WebviewEvent};

use crate::state::{AppState, Config};

const DEFAULT_IFACE: &str = "awdl0";

fn config(app: &tauri::App) -> Result<Config, Box<dyn std::error::Error>> {
    let iface = std::env::var("OWLY_IFACE").unwrap_or_else(|_| DEFAULT_IFACE.into());
    validate::interface(&iface)?;
    let receive_dir = match std::env::var_os("OWLY_RECEIVE_DIR") {
        Some(dir) => validate::directory(PathBuf::from(dir))?,
        None => app.path().download_dir()?.join("Owly"),
    };
    // A bus other than the system's is for tests; a release build never
    // takes the radio from one the environment names.
    let bus = if cfg!(debug_assertions) {
        std::env::var("OWLY_BUS").ok()
    } else {
        None
    };
    Ok(Config {
        iface,
        name: owly_session::link::host_name()?,
        bus,
        receive_dir,
    })
}

/// The window may show its own page and nothing else: a page that navigated
/// away would be a page we don't control, with the same IPC.
fn may_navigate(url: &tauri::Url) -> bool {
    match url.scheme() {
        "tauri" => true,
        "http" | "https" => {
            let host = url.host_str().unwrap_or_default();
            host == "tauri.localhost"
                || (cfg!(debug_assertions) && host == "localhost" && url.port() == Some(1420))
        }
        _ => false,
    }
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();
    owly_session::link::install_crypto();
    let guard = tauri::plugin::Builder::<tauri::Wry, ()>::new("navigation-guard")
        .on_navigation(|_, url| may_navigate(url))
        .build();
    let app = tauri::Builder::default()
        // First, as the plugin requires: a second launch (a second `owly gui`
        // or a desktop-entry click) raises this window instead of starting
        // another that would fight it for the radio and port 8771.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A compositor may refuse to raise a window; the user then
            // finds it themselves, so these failures are only logged.
            if let Some(window) = app.get_webview_window("main")
                && let Err(e) = window.unminimize().and_then(|()| window.set_focus())
            {
                tracing::debug!("could not raise the running window: {e}");
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(guard)
        .on_webview_event(|webview, event| {
            if let WebviewEvent::DragDrop(drag) = event {
                commands::on_drag_drop(webview.app_handle(), drag);
            }
        })
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
            commands::choose_items,
            commands::discard_selection,
            commands::send_selection,
            commands::send_link,
            commands::set_visibility,
            commands::respond,
        ])
        .build(tauri::generate_context!())
        .expect("the Owly window failed to start");

    // Give the radio and Wi-Fi back before leaving.
    let leaving = AtomicBool::new(false);
    app.run(move |handle, event| {
        if let RunEvent::ExitRequested { api, .. } = event
            && !leaving.swap(true, Ordering::SeqCst)
        {
            api.prevent_exit();
            let handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                commands::shutdown(&handle).await;
                handle.exit(0);
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> tauri::Url {
        text.parse().expect("a URL")
    }

    #[test]
    fn only_its_own_page_may_load() {
        assert!(may_navigate(&url("tauri://localhost/index.html")));
        assert!(may_navigate(&url("http://tauri.localhost/")));
        assert!(!may_navigate(&url("https://example.org/")));
        assert!(!may_navigate(&url("http://localhost:8080/")));
        assert!(!may_navigate(&url("file:///etc/passwd")));
        assert!(!may_navigate(&url("javascript:alert(1)")));
    }

    /// The window's configuration, read as the build reads it.
    fn json(path: &str) -> serde_json::Value {
        let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        let text = std::fs::read_to_string(&full).expect("the file exists");
        serde_json::from_str(&text).expect("it parses")
    }

    #[test]
    fn the_csp_allows_no_inline_or_eval_script() {
        let conf = json("tauri.conf.json");
        let csp = conf["app"]["security"]["csp"].as_str().expect("a CSP");
        // Whole sources, so `http://ipc.localhost` isn't mistaken for `http:`.
        let sources: Vec<&str> = csp
            .split(|c: char| c == ';' || c.is_whitespace())
            .filter(|t| !t.is_empty())
            .collect();
        for banned in [
            "'unsafe-inline'",
            "'unsafe-eval'",
            "http:",
            "https:",
            "*",
            "data:",
        ] {
            assert!(!sources.contains(&banned), "the CSP allows {banned}");
        }
        assert!(csp.contains("default-src 'none'"));
        assert!(csp.contains("object-src 'none'"));
        assert_eq!(conf["app"]["security"]["freezePrototype"], true);
    }

    #[test]
    fn the_window_can_only_listen_and_call_the_listed_commands() {
        let caps = json("capabilities/default.json");
        let given: Vec<&str> = caps["permissions"]
            .as_array()
            .expect("permissions")
            .iter()
            .map(|p| p.as_str().expect("a plain permission"))
            .collect();
        for p in &given {
            assert!(
                p.starts_with("allow-") || p.starts_with("core:event:allow-"),
                "{p} gives the window more than its commands and events"
            );
        }
        assert_eq!(caps["windows"], serde_json::json!(["main"]));
        // No dialog, file-system or shell access: the backend owns those.
        assert!(
            !given
                .iter()
                .any(|p| p.contains("dialog") || p.contains("fs") || p.contains("shell"))
        );
    }

    #[test]
    fn every_command_in_the_manifest_has_a_permission() {
        let build = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs"),
        )
        .expect("build.rs");
        let caps = json("capabilities/default.json");
        let given: Vec<&str> = caps["permissions"]
            .as_array()
            .expect("permissions")
            .iter()
            .filter_map(|p| p.as_str())
            .collect();
        let commands: Vec<&str> = build
            .lines()
            .filter_map(|l| l.trim().strip_prefix('"')?.strip_suffix("\","))
            .collect();
        assert_eq!(commands.len(), 12);
        for command in commands {
            let permission = format!("allow-{}", command.replace('_', "-"));
            assert!(given.contains(&permission.as_str()), "missing {permission}");
        }
    }
}
