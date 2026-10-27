//! Taking the radio from owlyd for as long as a session runs, as
//! `owly radio up` does for the CLI.

use std::fmt;

use futures_util::StreamExt;
use owlyd::api::{self, LinkInfo, RadioProxy};

use crate::wifi::{self, Connection};

/// Why the radio couldn't be had, by what the caller can do about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// owlyd isn't running or isn't on the bus.
    Unreachable,
    /// polkit refused (`io.github.t4t5.Owly.acquire`).
    NotAuthorized,
    /// Another client holds the radio.
    Busy,
    /// Try again later.
    NotNow,
    /// No card that can do AWDL.
    NoRadio,
    Other,
}

#[derive(Debug, Clone)]
pub struct RadioError {
    pub kind: Failure,
    pub message: String,
}

impl fmt::Display for RadioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RadioError {}

impl RadioError {
    fn from_zbus(e: &zbus::Error) -> Self {
        if api::unreachable(e) {
            return RadioError {
                kind: Failure::Unreachable,
                message: format!(
                    "owlyd can't be reached ({e}). `systemctl enable --now owlyd` starts it."
                ),
            };
        }
        let (kind, message) = match api::api_error(e) {
            Some(("NotAuthorized", m)) => (Failure::NotAuthorized, m),
            Some(("Busy", m)) => (Failure::Busy, m),
            Some(("NotNow", m)) => (Failure::NotNow, m),
            Some(("NoRadio", m)) => (Failure::NoRadio, m),
            Some((_, m)) => (Failure::Other, m),
            None => (Failure::Other, e.to_string()),
        };
        RadioError { kind, message }
    }
}

/// The radio, leased to this process. Give it back with [`Held::release`];
/// owlyd also ends the lease when this process leaves the bus.
pub struct Held {
    radio: RadioProxy<'static>,
    lease: u64,
    link: LinkInfo,
    wifi: Vec<Connection>,
}

impl Held {
    /// Brings the radio up and leases it for as long as this process wants.
    /// `bus` is the bus owlyd is on, when it isn't the system bus.
    pub async fn acquire(bus: Option<&str>) -> Result<Held, RadioError> {
        let conn = api::connect(bus)
            .await
            .map_err(|e| RadioError::from_zbus(&e))?;
        let radio = api::radio(&conn)
            .await
            .map_err(|e| RadioError::from_zbus(&e))?;
        let wifi = wifi::active();
        let (lease, link) = radio
            .acquire(0)
            .await
            .map_err(|e| RadioError::from_zbus(&e))?;
        let link = LinkInfo::from_dict(&link).ok_or(RadioError {
            kind: Failure::Other,
            message: "owlyd gave a lease with no link".into(),
        })?;
        Ok(Held {
            radio,
            lease,
            link,
            wifi,
        })
    }

    pub fn link(&self) -> &LinkInfo {
        &self.link
    }

    /// A future that resolves, with its reason, when this lease ends other
    /// than by [`Held::release`]: the link was lost, or owlyd stopped. It
    /// doesn't borrow `self`, so a task can hold it.
    pub fn watch(&self) -> impl Future<Output = String> + Send + 'static {
        let radio = self.radio.clone();
        let lease = self.lease;
        async move {
            let Ok(mut states) = radio.receive_radio_state().await else {
                return "owlyd can't be watched".into();
            };
            while let Some(signal) = states.next().await {
                let Ok(args) = signal.args() else { continue };
                if args.lease == lease && args.state == "idle" {
                    return args.reason;
                }
            }
            "owlyd went away".into()
        }
    }

    /// Gives the radio back, and then Wi-Fi as this user (see [`wifi`]).
    pub async fn release(self) -> Result<(), RadioError> {
        self.radio
            .release(self.lease)
            .await
            .map_err(|e| RadioError::from_zbus(&e))?;
        wifi::restore(self.wifi).await;
        Ok(())
    }

    /// Wi-Fi back as this user, once [`Held::watch`] says the lease is over.
    pub async fn restore_wifi(self) {
        wifi::restore(self.wifi).await;
    }
}

/// owlyd's radio state: `idle`, `starting`, `up` or `stopping`.
pub async fn state(bus: Option<&str>) -> Result<String, RadioError> {
    let conn = api::connect(bus)
        .await
        .map_err(|e| RadioError::from_zbus(&e))?;
    let radio = api::radio(&conn)
        .await
        .map_err(|e| RadioError::from_zbus(&e))?;
    radio
        .current_state()
        .await
        .map_err(|e| RadioError::from_zbus(&e))
}
