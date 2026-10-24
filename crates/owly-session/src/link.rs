//! The interface a session runs on, and owlyd's lease on it.

use std::io;
use std::net::Ipv6Addr;

use owly_airdrop::proto::mdns;
use owly_awdl::MacAddr;
use owly_radio::radio::Announce;
use owlyd::api::{self, LinkInfo, RadioProxy};

use crate::Notes;

#[derive(Debug, Clone)]
pub struct Interface {
    pub name: String,
    pub index: u32,
    pub addr: Ipv6Addr,
}

impl Interface {
    /// The interface's index and IPv6 link-local address (NET-1, NET-2).
    pub fn lookup(name: &str) -> io::Result<Self> {
        let index = nix::net::if_::if_nametoindex(name)
            .map_err(|e| io::Error::other(format!("{name}: {e}")))?;
        let addr = nix::ifaddrs::getifaddrs()?
            .filter(|a| a.interface_name == name)
            .filter_map(|a| a.address?.as_sockaddr_in6().map(|s| s.ip()))
            .find(|ip| ip.segments()[0] & 0xffc0 == 0xfe80)
            .ok_or_else(|| io::Error::other(format!("{name} has no IPv6 link-local address")))?;
        Ok(Interface {
            name: name.into(),
            index,
            addr,
        })
    }
}

/// Picks rustls's crypto provider, which ring is here, and the interface
/// and the name others see: `name`, or the host name.
pub fn resolve(iface: &str, name: Option<&str>) -> io::Result<(Interface, String)> {
    // Fails only when a provider is already installed, which is as good.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let iface = Interface::lookup(iface)?;
    let name = match name {
        Some(name) => name.to_owned(),
        None => nix::unistd::gethostname()?.to_string_lossy().into_owned(),
    };
    Ok((iface, name))
}

/// owlyd's lease on the interface we're on, when there's one.
pub struct Lease {
    radio: RadioProxy<'static>,
    pub link: LinkInfo,
}

impl Lease {
    /// Why there's none when owlyd can't be reached, holds no lease, or
    /// leases another interface: on a link no Owly radio set up, such as
    /// the veth harness, or from an owlyd older than this owly.
    pub async fn find(bus: Option<&str>, iface: &str) -> Result<Lease, String> {
        let conn = api::connect(bus)
            .await
            .map_err(|e| format!("owlyd can't be reached ({e})"))?;
        let radio = api::radio(&conn).await.map_err(|e| e.to_string())?;
        let link = radio.link().await.map_err(|e| {
            format!("owlyd doesn't say which link is up ({e}); is it older than this owly?")
        })?;
        let link = LinkInfo::from_dict(&link).ok_or("the radio isn't up")?;
        if link.interface != iface {
            return Err(format!(
                "the radio is up on {}, not {iface}",
                link.interface
            ));
        }
        Ok(Lease { radio, link })
    }

    /// The AWDL peers the radio hears now.
    pub async fn peers(&self) -> Vec<MacAddr> {
        let peers = self.radio.peers().await.unwrap_or_default();
        peers
            .iter()
            .filter_map(|p| p.get("mac")?.downcast_ref::<String>().ok()?.parse().ok())
            .collect()
    }
}

/// The burst the radio asks receivers to announce in (MDNS-7).
pub fn burst(announce: Option<Announce>, notes: &Notes) -> Option<mdns::Burst> {
    let a = announce?;
    notes.say(format!(
        "announcing {} times {} ms apart, every {} s, as the radio asks",
        a.count,
        a.interval.as_millis(),
        a.pause.as_secs()
    ));
    Some(mdns::Burst {
        count: a.count,
        interval: a.interval,
        pause: a.pause,
    })
}
