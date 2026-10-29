use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use owly_airdrop::driver::receiver::Request;
use owly_session::discover::Listed;
use owly_session::radio::Held;
use tauri::async_runtime::JoinHandle;
use tokio::sync::Mutex;

use crate::error::AppError;
use crate::model::Visibility;
use crate::settings::{Fixed, Settings};
use crate::validate;

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
    /// The settings file, and what it held at start-up.
    pub settings_file: PathBuf,
    pub settings: Settings,
    /// Which of them an environment variable sets.
    pub fixed: Fixed,
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
    /// The settings in force: the file's, changed by the preferences.
    pub settings: Settings,
    /// The links of the last few receipts, as they arrived: what
    /// `open_link` may name, by receipt id and position (OWLY-5, OWLY-7).
    pub links: BTreeMap<u64, Vec<String>>,
}

/// More requests than this waiting at once are declined unseen, so a
/// sender can't fill the window with prompts.
pub const MAX_PENDING: usize = 32;
/// Selections kept; the oldest goes when another comes.
pub const MAX_SELECTIONS: usize = 8;
/// Receipts whose links are kept; the oldest goes when another comes.
pub const MAX_LINK_RECEIPTS: usize = 16;

impl Inner {
    /// Keeps `paths` as a new selection, and returns its id.
    pub fn select(&mut self, id: u64, paths: Vec<PathBuf>) {
        self.selections.insert(id, paths);
        while self.selections.len() > MAX_SELECTIONS {
            self.selections.pop_first();
        }
    }

    /// Keeps the links a receipt brought.
    pub fn keep_links(&mut self, id: u64, urls: Vec<String>) {
        self.links.insert(id, urls);
        while self.links.len() > MAX_LINK_RECEIPTS {
            self.links.pop_first();
        }
    }

    /// The link to open for the page's `id` and `index`, when links may be
    /// opened and it is still held and is an `http` or `https` link.
    pub fn link_to_open(&self, id: u64, index: usize) -> Result<String, AppError> {
        if !self.settings.window.open_links {
            return Err(AppError::Unavailable(
                "opening links is off; turn it on in the preferences".into(),
            ));
        }
        let url = self
            .links
            .get(&id)
            .and_then(|urls| urls.get(index))
            .ok_or_else(|| AppError::NotFound("that link is no longer held".into()))?;
        let mut checked = validate::urls(std::slice::from_ref(url))?;
        Ok(checked.remove(0))
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
        let inner = Inner {
            settings: config.settings.clone(),
            ..Inner::default()
        };
        AppState {
            config,
            inner: Mutex::new(inner),
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

    // spec: OWLY-5
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

    fn holding(open_links: bool) -> Inner {
        let mut inner = Inner::default();
        inner.settings.window.open_links = open_links;
        inner.keep_links(
            7,
            vec!["https://example.org/a".into(), "javascript:alert(1)".into()],
        );
        inner
    }

    // spec: OWLY-7
    #[test]
    fn a_link_opens_only_when_links_are_on() {
        assert!(holding(false).link_to_open(7, 0).is_err());
        assert_eq!(
            holding(true).link_to_open(7, 0).expect("opens"),
            "https://example.org/a"
        );
    }

    // spec: OWLY-5, OWLY-7
    #[test]
    fn a_link_is_named_by_receipt_and_position_and_must_be_http() {
        let inner = holding(true);
        assert!(inner.link_to_open(7, 1).is_err(), "not an http link");
        assert!(inner.link_to_open(7, 2).is_err(), "no such position");
        assert!(inner.link_to_open(8, 0).is_err(), "no such receipt");
    }

    // spec: OWLY-7
    #[test]
    fn the_oldest_receipt_of_links_goes_first() {
        let mut inner = Inner::default();
        for id in 1..=(MAX_LINK_RECEIPTS as u64 + 2) {
            inner.keep_links(id, vec!["https://example.org".into()]);
        }
        assert_eq!(inner.links.len(), MAX_LINK_RECEIPTS);
        assert!(!inner.links.contains_key(&1));
    }
}
