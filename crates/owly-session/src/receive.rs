//! Receiving: the listener, its mDNS announcement, and the state it keeps.

use std::io;
use std::net::SocketAddrV6;
use std::path::{Path, PathBuf};

use owly_airdrop::driver::mdns::{self as driver_mdns, Mdns};
use owly_airdrop::driver::receiver::{Options, Receiver, ReceiverEvent};
use owly_airdrop::driver::tls::Identity;
use owly_airdrop::proto::mdns::{Burst, Service};
use owly_airdrop::proto::receiver::Config;
use tokio::sync::mpsc;

use crate::ids::random_service_id;
use crate::link::Interface;

/// What to receive as, and where.
pub struct Settings {
    /// The name others see.
    pub name: String,
    /// Where received files go.
    pub dir: PathBuf,
    pub port: u16,
    /// Don't announce over mDNS, for when another responder does.
    pub no_mdns: bool,
    /// The TXT flags to announce (MDNS-4).
    pub flags: u32,
    pub burst: Option<Burst>,
}

/// A receiver that's listening. Dropping it stops the announcement; the
/// listener runs until `events` is dropped.
pub struct Running {
    pub events: mpsc::Receiver<ReceiverEvent>,
    pub service_id: String,
    _registration: Option<driver_mdns::Registration>,
}

/// Owly's state directory, `$XDG_STATE_HOME/owly` or `~/.local/state/owly`.
pub fn state_dir() -> io::Result<PathBuf> {
    Ok(std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .ok_or_else(|| io::Error::other("no XDG_STATE_HOME or HOME"))?
        .join("owly"))
}

/// Binds the listener on `iface`, with the identity kept in `state`, and
/// announces it unless `settings.no_mdns`.
pub async fn start(iface: &Interface, state: &Path, settings: Settings) -> io::Result<Running> {
    let Settings {
        name,
        dir,
        port,
        no_mdns,
        flags,
        burst,
    } = settings;
    let identity = Identity::load_or_generate(state)?;
    let mut config = Config::new(name);
    config.model_name = "Linux".into();
    let addr = SocketAddrV6::new(iface.addr, port, 0, iface.index);
    let (receiver, events) = Receiver::bind(addr, &identity, Options::new(config, dir)).await?;
    tokio::spawn(receiver.run());

    let service_id = random_service_id()?;
    let registration = if no_mdns {
        None
    } else {
        let service = Service {
            instance: service_id.clone(),
            host: format!("owly-{service_id}"),
            port,
            addr: iface.addr,
            flags,
        };
        Some(
            Mdns::new(iface.index)
                .with_burst(burst)
                .register(service)
                .await?,
        )
    };
    Ok(Running {
        events,
        service_id,
        _registration: registration,
    })
}
