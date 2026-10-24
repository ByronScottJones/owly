use std::io;

use owly_airdrop::driver::sender::Sender;
use owly_airdrop::proto::mdns;
use owly_airdrop::proto::sender;
use owly_awdl::MacAddr;

/// Apple's receivers listen here (MDNS-1), and an AWDL peer with no mDNS
/// is tried here (DISC-5).
pub const AIRDROP_PORT: u16 = 8770;

/// Where we receive: the port omdrop's firewall rule opens on `awdl0`
/// (docs/implementation.md, NET-4). A host firewall that blocks the
/// port is the first reason a phone never lists us.
pub const RECEIVE_PORT: u16 = 8771;

pub fn random_service_id() -> io::Result<String> {
    let mut bytes = [0; 6];
    getrandom::fill(&mut bytes).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(mdns::service_id(bytes))
}

/// BLE-5: a fresh tag per advert.
pub(crate) fn random_tag() -> io::Result<[u8; 3]> {
    let mut tag = [0; 3];
    getrandom::fill(&mut tag).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(tag)
}

pub fn parse_mac(text: &str) -> io::Result<MacAddr> {
    text.parse()
        .map_err(|_| io::Error::other(format!("bad MAC address {text}")))
}

/// A sender that calls itself `name`, under a fresh service ID.
pub fn new_sender(name: String) -> io::Result<Sender> {
    Sender::new(sender::Config::new(name, random_service_id()?))
}
