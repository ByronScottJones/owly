use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use owly_airdrop::driver::receiver::Request;
use owly_session::discover::Listed;
use owly_session::radio::Held;
use tauri::async_runtime::JoinHandle;
use tokio::sync::Mutex;

use crate::model::Visibility;

/// Fixed at startup.
pub struct Config {
    /// The interface AirDrop runs on, `awdl0` unless `OWLY_IFACE` says.
    pub iface: String,
    /// The name others see.
    pub name: String,
    /// The bus owlyd is on, when it isn't the system bus (`OWLY_BUS`).
    pub bus: Option<String>,
    /// Where received files go. Only the backend chooses it.
    pub receive_dir: PathBuf,
}

/// A task that ends when this is dropped.
pub struct Task(JoinHandle<()>);

impl Task {
    pub fn new(handle: JoinHandle<()>) -> Self {
        Task(handle)
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[derive(Default)]
pub struct Inner {
    pub radio: Option<Held>,
    pub radio_watch: Option<Task>,
    /// The receivers the last scan found: what `send` may name.
    pub peers: Vec<Listed>,
    pub browse: Option<Task>,
    pub receiver: Option<Task>,
    pub visibility: Visibility,
    /// Requests waiting for the user. Dropping one declines it.
    pub pending: HashMap<u64, Request>,
}

pub struct AppState {
    pub config: Config,
    pub inner: Mutex<Inner>,
    transfers: AtomicU64,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        AppState {
            config,
            inner: Mutex::new(Inner::default()),
            transfers: AtomicU64::new(1),
        }
    }

    pub fn next_transfer(&self) -> u64 {
        self.transfers.fetch_add(1, Ordering::Relaxed)
    }
}
