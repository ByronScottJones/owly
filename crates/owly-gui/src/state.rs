use std::collections::{BTreeMap, HashMap};
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
    /// What the user picked or dropped, until it's sent or discarded.
    pub selections: BTreeMap<u64, Vec<PathBuf>>,
}

/// More requests than this waiting at once are declined unseen, so a
/// sender can't fill the window with prompts.
pub const MAX_PENDING: usize = 32;
/// Selections kept; the oldest goes when another comes.
pub const MAX_SELECTIONS: usize = 8;

impl Inner {
    /// Keeps `paths` as a new selection, and returns its id.
    pub fn select(&mut self, id: u64, paths: Vec<PathBuf>) {
        self.selections.insert(id, paths);
        while self.selections.len() > MAX_SELECTIONS {
            self.selections.pop_first();
        }
    }
}

pub struct AppState {
    pub config: Config,
    pub inner: Mutex<Inner>,
    /// Held while the radio is being taken or given back, which can wait on
    /// a polkit prompt: `inner` stays free for `get_status` meanwhile.
    pub radio_gate: Mutex<()>,
    transfers: AtomicU64,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        AppState {
            config,
            inner: Mutex::new(Inner::default()),
            radio_gate: Mutex::new(()),
            transfers: AtomicU64::new(1),
        }
    }

    /// A fresh id for a transfer or a selection.
    pub fn next_transfer(&self) -> u64 {
        self.transfers.fetch_add(1, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_oldest_selection_goes_first() {
        let mut inner = Inner::default();
        for id in 1..=(MAX_SELECTIONS as u64 + 2) {
            inner.select(id, vec![PathBuf::from("/a")]);
        }
        assert_eq!(inner.selections.len(), MAX_SELECTIONS);
        assert!(!inner.selections.contains_key(&1));
        assert!(!inner.selections.contains_key(&2));
        assert!(inner.selections.contains_key(&(MAX_SELECTIONS as u64 + 2)));
    }
}
