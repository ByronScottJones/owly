//! Finding receivers: mDNS, and `/Discover` at the radio's AWDL peers.

use std::io;
use std::net::SocketAddrV6;
use std::time::Duration;

use owly_airdrop::driver::mdns::Mdns;
use owly_airdrop::driver::sender::Sender;
use owly_airdrop::proto::ble::Advert;
use owly_airdrop::proto::mdns::{BrowseEvent, RECEIVER_FLAGS, SUPPORTS_DVZIP};
use owly_airdrop::proto::sender;

use crate::Notes;
use crate::bluez::{BluezAdvertiser, Registration};
use crate::ids::{AIRDROP_PORT, random_tag};
use crate::link::{Interface, Lease};

/// A receiver that answered `/Discover` with a name.
#[derive(Debug, Clone)]
pub struct Listed {
    pub instance: String,
    pub name: String,
    pub addr: SocketAddrV6,
    pub flags: u32,
}

/// BLE-1: the advert that makes Apple receivers turn on AirDrop. Without
/// Bluetooth, sending still works to receivers that are already listening.
pub async fn advertise(skip: bool, notes: &Notes) -> Option<Registration> {
    if skip {
        return None;
    }
    let result = async {
        BluezAdvertiser::new()
            .await?
            .advertise(Advert::anonymous(random_tag()?))
            .await
    }
    .await;
    match result {
        Ok(guard) => Some(guard),
        Err(e) => {
            notes.say(format!("not advertising over Bluetooth: {e}"));
            None
        }
    }
}

/// `/Discover` at one address until it answers or `limit` runs out. An
/// iPhone opens its port only once our BLE advert has woken it: in legacy's
/// run that worked, 14.8 s after the advert started. Until then it refuses
/// the connection, or doesn't answer at all.
pub async fn knock(
    sender: &Sender,
    addr: SocketAddrV6,
    limit: Duration,
    notes: &Notes,
) -> io::Result<Option<sender::Receiver>> {
    let deadline = tokio::time::Instant::now() + limit;
    let mut said = false;
    loop {
        let attempt = tokio::time::timeout_at(deadline, sender.discover(addr)).await;
        match attempt {
            Ok(Ok(answer)) => return Ok(answer),
            Ok(Err(e)) if tokio::time::Instant::now() < deadline => {
                if !said {
                    notes.say(format!("waiting for the receiver to open its port ({e})"));
                    said = true;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("the receiver didn't open its port within {limit:?}"),
                ));
            }
        }
    }
}

/// The receiver `to` names, or the first found, by mDNS and, on an owlyd
/// lease, by `/Discover` at the radio's AWDL peers (DISC-5): an iPhone's
/// mDNS doesn't reach `awdl0`, and a peer can turn up at any time before
/// the deadline. Whichever finds it first wins.
pub async fn find_receiver(
    iface: &Interface,
    sender: &Sender,
    lease: Option<&Lease>,
    to: Option<&str>,
    limit: Duration,
    notes: &Notes,
) -> io::Result<Listed> {
    let deadline = tokio::time::Instant::now() + limit;
    let by_mdns = async {
        let mut chosen = None;
        browse_receivers(
            iface,
            sender,
            limit,
            |r| {
                let wanted = to.is_none_or(|t| t == r.name || t == r.instance);
                if wanted {
                    chosen = Some(r.clone());
                }
                wanted
            },
            notes,
        )
        .await?;
        chosen.ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "no such receiver found"))
    };
    let Some(lease) = lease else {
        return by_mdns.await;
    };
    let by_peers = knock_peers(iface, sender, lease, to, deadline, notes);
    tokio::pin!(by_mdns, by_peers);
    let (mut mdns_done, mut peers_failed) = (false, None);
    loop {
        tokio::select! {
            r = &mut by_mdns, if !mdns_done => match r {
                Ok(found) => return Ok(found),
                Err(e) => {
                    if e.kind() != io::ErrorKind::TimedOut {
                        notes.say(format!("not browsing mDNS: {e}"));
                    }
                    mdns_done = true;
                }
            },
            r = &mut by_peers, if peers_failed.is_none() => match r {
                Ok(found) => return Ok(found),
                Err(e) => peers_failed = Some(e),
            },
        }
        // Both gave up: the AWDL peers' error says more about why a phone
        // wasn't found.
        if mdns_done && let Some(e) = peers_failed {
            return Err(e);
        }
    }
}

/// `/Discover` at each AWDL peer in turn (DISC-5) until one that `to` names
/// answers, or the deadline. The list is read again each round: peers come
/// and go, and an iPhone changes its AWDL address.
pub async fn knock_peers(
    iface: &Interface,
    sender: &Sender,
    lease: &Lease,
    to: Option<&str>,
    deadline: tokio::time::Instant,
    notes: &Notes,
) -> io::Result<Listed> {
    let mut tried = std::collections::HashSet::new();
    while tokio::time::Instant::now() < deadline {
        for mac in lease.peers().await {
            if tried.insert(mac) {
                notes.say(format!("trying AWDL peer {mac}"));
            }
            let addr = SocketAddrV6::new(mac.link_local(), AIRDROP_PORT, 0, iface.index);
            let mac = mac.to_string();
            // A peer that refuses fails at once; one that's silent mustn't
            // hold up the others for long.
            let wait = deadline
                .saturating_duration_since(tokio::time::Instant::now())
                .min(Duration::from_secs(3));
            let Ok(Ok(Some(receiver))) = tokio::time::timeout(wait, sender.discover(addr)).await
            else {
                continue;
            };
            if to.is_none_or(|t| t == receiver.computer_name || t == mac) {
                return Ok(Listed {
                    instance: mac,
                    name: receiver.computer_name,
                    addr,
                    // UP-1b: no TXT to say, and Apple receivers take DVZip.
                    flags: RECEIVER_FLAGS | SUPPORTS_DVZIP,
                });
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "no AWDL peer answered /Discover; is the receiver unlocked, with AirDrop on?",
    ))
}

/// BROWSING and DISCOVERING (§8.1): each resolved instance gets a
/// `/Discover`, and is listed only if it answers with a name (DISC-1).
/// Stops early when `on_listed` returns true.
pub async fn browse_receivers(
    iface: &Interface,
    sender: &Sender,
    limit: Duration,
    mut on_listed: impl FnMut(&Listed) -> bool,
    notes: &Notes,
) -> io::Result<()> {
    let mut browse = Mdns::new(iface.index).browse().await?;
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        let event = tokio::select! {
            event = browse.events.recv() => event,
            () = tokio::time::sleep_until(deadline) => return Ok(()),
        };
        let Some(BrowseEvent::Found(found)) = event else {
            continue;
        };
        let addr = SocketAddrV6::new(found.addr, found.port, 0, iface.index);
        match sender.discover(addr).await {
            Ok(Some(receiver)) => {
                let listed = Listed {
                    instance: found.instance,
                    name: receiver.computer_name,
                    addr,
                    flags: found.flags,
                };
                if on_listed(&listed) {
                    return Ok(());
                }
            }
            Ok(None) => notes.say(format!("{} doesn't list us", found.instance)),
            Err(e) => notes.say(format!("{}: /Discover failed: {e}", found.instance)),
        }
    }
}

/// Every receiver that answers within `limit`, by mDNS and, on an owlyd
/// lease, by `/Discover` at the radio's AWDL peers (DISC-5), one entry per
/// instance.
pub async fn scan(
    iface: &Interface,
    sender: &Sender,
    lease: Option<&Lease>,
    limit: Duration,
    notes: &Notes,
) -> Vec<Listed> {
    let found = std::sync::Mutex::new(Vec::<Listed>::new());
    let add = |listed: &Listed| {
        // A poisoned list only means another listing panicked half-way.
        let mut found = found.lock().unwrap_or_else(|e| e.into_inner());
        if !found.iter().any(|l| l.instance == listed.instance) {
            found.push(listed.clone());
        }
        false
    };
    let by_mdns = async {
        if let Err(e) = browse_receivers(iface, sender, limit, add, notes).await {
            notes.say(format!("not browsing mDNS: {e}"));
        }
    };
    let by_peers = async {
        let Some(lease) = lease else { return };
        let deadline = tokio::time::Instant::now() + limit;
        let mut seen = std::collections::HashSet::new();
        while tokio::time::Instant::now() < deadline {
            for mac in lease.peers().await {
                if !seen.insert(mac) {
                    continue;
                }
                let addr = SocketAddrV6::new(mac.link_local(), AIRDROP_PORT, 0, iface.index);
                let wait = deadline
                    .saturating_duration_since(tokio::time::Instant::now())
                    .min(Duration::from_secs(3));
                if let Ok(Ok(Some(receiver))) =
                    tokio::time::timeout(wait, sender.discover(addr)).await
                {
                    let mut found = found.lock().unwrap_or_else(|e| e.into_inner());
                    let instance = mac.to_string();
                    if !found.iter().any(|l| l.instance == instance) {
                        found.push(Listed {
                            instance,
                            name: receiver.computer_name,
                            addr,
                            // UP-1b: no TXT to say, and Apple receivers take DVZip.
                            flags: RECEIVER_FLAGS | SUPPORTS_DVZIP,
                        });
                    }
                } else {
                    // Not answering yet: an iPhone may open its port later.
                    seen.remove(&mac);
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    };
    tokio::join!(by_mdns, by_peers);
    found.into_inner().unwrap_or_else(|e| e.into_inner())
}
