//! The calls the GUI makes, end to end on a veth pair and a fake radio: the
//! radio leased from `owlyd` (served in this process on a private bus, with
//! its fake `awdl0`), then `awdl0` (this computer) and `peer0` (a phone)
//! sending to and receiving from each other, with the phone found, and lost, by `watch`.
//!
//! Like the veth harness of owly-airdrop (docs/conformance.md), the test
//! re-runs itself under `unshare --user --map-root-user --net`, so it needs
//! no root, but does need unprivileged user namespaces. Needs `ip` and
//! `dbus-daemon`.

use std::net::SocketAddrV6;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use owly_airdrop::driver::receiver::ReceiverEvent;
use owly_airdrop::driver::sender::Payload;
use owly_airdrop::proto::mdns::RECEIVER_FLAGS;
use owly_airdrop::proto::sender::Outcome;
use owly_radio::fake::{self, Fake};
use owly_session::link::{Interface, Lease};
use owly_session::radio::Held;
use owly_session::receive::{self, Running, Settings};
use owly_session::{Notes, RECEIVE_PORT, discover, new_sender};
use owlyd::service::{self, Options};

const ENV: &str = "OWLY_SESSION_NETNS";
const TEST: &str = "files_and_links_cross_a_veth_with_a_leased_radio";
const PHONE: &str = "Test Phone";

fn ip(args: &[&str]) {
    let status = Command::new("ip").args(args).status().expect("running ip");
    assert!(status.success(), "ip {args:?} failed");
}

/// `awdl0` and `peer0`, up, with link-local addresses and no wait for DAD.
fn veth() -> (Interface, Interface) {
    ip(&["link", "set", "lo", "up"]);
    ip(&[
        "link", "add", "awdl0", "type", "veth", "peer", "name", "peer0",
    ]);
    for (name, mac) in [
        ("awdl0", "02:00:00:00:00:0a"),
        ("peer0", "02:00:00:00:00:0b"),
    ] {
        std::fs::write(format!("/proc/sys/net/ipv6/conf/{name}/accept_dad"), "0").unwrap();
        ip(&["link", "set", name, "address", mac]);
        ip(&["link", "set", name, "up"]);
    }
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        match (Interface::lookup("awdl0"), Interface::lookup("peer0")) {
            (Ok(a), Ok(b)) => return (a, b),
            (a, b) => assert!(
                Instant::now() < end,
                "no link-local addresses: {:?} {:?}",
                a.err(),
                b.err()
            ),
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A private bus, ended with the test.
struct Bus(Child);

impl Drop for Bus {
    fn drop(&mut self) {
        self.0.kill().ok();
        self.0.wait().ok();
    }
}

fn start_bus(dir: &Path) -> (Bus, String) {
    let socket = dir.join("bus");
    let address = format!("unix:path={}", socket.display());
    let child = Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--nopidfile"])
        .arg(format!("--address={address}"))
        .stdout(Stdio::null())
        .spawn()
        .expect("dbus-daemon");
    let end = Instant::now() + Duration::from_secs(10);
    while !socket.exists() {
        assert!(Instant::now() < end, "the bus never started");
        std::thread::sleep(Duration::from_millis(20));
    }
    (Bus(child), address)
}

/// The first request is accepted; returns what arrives: files' paths, or URLs.
async fn accept_and_collect(mut running: Running) -> Result<Vec<String>, String> {
    let wait = tokio::time::timeout(Duration::from_secs(20), async {
        while let Some(event) = running.events.recv().await {
            match event {
                ReceiverEvent::Request(request) => request.accept(),
                ReceiverEvent::Received { paths, .. } => {
                    return Ok(paths
                        .iter()
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect());
                }
                ReceiverEvent::Urls { urls, .. } => return Ok(urls),
                ReceiverEvent::Failed { reason, .. } => return Err(reason),
                ReceiverEvent::Discovered { .. } | ReceiverEvent::Withdrawn { .. } => {}
            }
        }
        Err("the receiver stopped".into())
    });
    wait.await.unwrap_or_else(|_| Err("nothing arrived".into()))
}

/// The lists the watch reports, until one satisfies `done`.
async fn wait_for(
    watching: &mut std::pin::Pin<&mut impl std::future::Future<Output = ()>>,
    seen: &mut tokio::sync::mpsc::UnboundedReceiver<Vec<discover::Listed>>,
    done: impl Fn(&[discover::Listed]) -> bool,
) -> Vec<discover::Listed> {
    let wait = async {
        loop {
            tokio::select! {
                () = &mut *watching => panic!("the watch ended"),
                Some(listed) = seen.recv() => if done(&listed) { return listed },
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(10), wait)
        .await
        .expect("the list never changed as expected")
}

async fn scenario() {
    owly_session::link::install_crypto();
    let (awdl, peer) = veth();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("radio");
    std::fs::create_dir_all(root.join("owly-fake")).unwrap();
    std::fs::write(root.join("owly-fake/peers"), "86:08:04:8a:2b:f4\n").unwrap();
    let (_bus, address) = start_bus(dir.path());

    // owlyd, with a fake radio, on the private bus.
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let options = Options {
        bus: Some(address.clone()),
        polkit: false,
    };
    let daemon = tokio::spawn(service::serve(
        Fake::new(&root),
        fake::probe,
        options,
        async {
            stopped.await.ok();
        },
    ));

    // What the GUI's "turn on" does: lease the radio, then find the lease.
    let end = Instant::now() + Duration::from_secs(10);
    let held = loop {
        match Held::acquire(Some(&address)).await {
            Ok(held) => break held,
            Err(e) => assert!(Instant::now() < end, "no lease: {e}"),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(held.link().interface, "awdl0");
    let lease = Lease::find(Some(&address), "awdl0")
        .await
        .expect("the lease");

    // The watch starts before the phone is there, and lists it as soon as it
    // appears (not at the end of a scan).
    let sender = new_sender("Test Computer".into()).unwrap();
    let (changes, mut seen) = tokio::sync::mpsc::unbounded_channel();
    let notes = Notes::stderr();
    let watching = discover::watch(&awdl, &sender, Some(&lease), &notes, |listed| {
        changes.send(listed.to_vec()).ok();
    });
    tokio::pin!(watching);
    tokio::select! {
        () = &mut watching => panic!("the watch ended"),
        () = tokio::time::sleep(Duration::from_millis(500)) => {}
    }
    assert!(seen.try_recv().is_err(), "listed someone before the phone");

    let phone_state = dir.path().join("phone-state");
    let phone_dir = dir.path().join("phone-received");
    std::fs::create_dir(&phone_dir).unwrap();
    let phone = receive::start(
        &peer,
        &phone_state,
        Settings {
            name: PHONE.into(),
            dir: phone_dir.clone(),
            port: RECEIVE_PORT,
            no_mdns: false,
            flags: RECEIVER_FLAGS,
            burst: None,
        },
    )
    .await
    .expect("the phone's receiver");
    let started = Instant::now();
    let listed = wait_for(&mut watching, &mut seen, |l| {
        l.iter().any(|l| l.name == PHONE)
    })
    .await;
    eprintln!(
        "the phone was listed {:?} after it started",
        started.elapsed()
    );
    let target = listed
        .iter()
        .find(|l| l.name == PHONE)
        .expect("listed, as waited for");

    let file = dir.path().join("photo.txt");
    std::fs::write(&file, "an owl\n").unwrap();
    let files = Payload::Files(vec![file]);
    let (outcome, arrived) = tokio::join!(
        sender.send(target.addr, &files, target.flags, None),
        accept_and_collect(phone),
    );
    assert!(matches!(outcome, Ok(Outcome::Delivered)), "{outcome:?}");
    let paths = arrived.expect("the file arrives");
    assert_eq!(paths.len(), 1);
    assert_eq!(std::fs::read_to_string(&paths[0]).unwrap(), "an owl\n");
    assert!(Path::new(&paths[0]).starts_with(&phone_dir));

    // The phone sends a link to the GUI's receiver (Everyone mode).
    let own_dir = dir.path().join("received");
    std::fs::create_dir(&own_dir).unwrap();
    let own = receive::start(
        &awdl,
        &dir.path().join("own-state"),
        Settings {
            name: "Test Computer".into(),
            dir: own_dir,
            port: RECEIVE_PORT,
            no_mdns: true,
            flags: RECEIVER_FLAGS,
            burst: None,
        },
    )
    .await
    .expect("the computer's receiver");
    let phone_sender = new_sender(PHONE.into()).unwrap();
    let to = SocketAddrV6::new(awdl.addr, RECEIVE_PORT, 0, peer.index);
    let link = Payload::Urls(vec!["https://example.org/owl".into()]);
    let (outcome, arrived) = tokio::join!(
        phone_sender.send(to, &link, RECEIVER_FLAGS, None),
        accept_and_collect(own),
    );
    assert!(matches!(outcome, Ok(Outcome::Delivered)), "{outcome:?}");
    assert_eq!(
        arrived.expect("the link arrives"),
        ["https://example.org/owl"]
    );

    // The phone is gone (its receiver was dropped), and so is its entry.
    let started = Instant::now();
    wait_for(&mut watching, &mut seen, |l| {
        l.iter().all(|l| l.name != PHONE)
    })
    .await;
    eprintln!(
        "the phone was unlisted {:?} after it went",
        started.elapsed()
    );

    // Turning off gives the radio back.
    held.release().await.expect("release");
    assert!(Lease::find(Some(&address), "awdl0").await.is_err());
    stop.send(()).ok();
    daemon.await.unwrap().expect("owlyd ends cleanly");
}

#[test]
fn files_and_links_cross_a_veth_with_a_leased_radio() {
    if std::env::var_os(ENV).is_some() {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(scenario());
        return;
    }
    let status = Command::new("unshare")
        .args(["--user", "--map-root-user", "--net", "--"])
        .arg(std::env::current_exe().unwrap())
        .args([TEST, "--exact", "--nocapture", "--test-threads=1"])
        .env(ENV, "1")
        .status()
        .expect("running unshare (util-linux)");
    assert!(
        status.success(),
        "{TEST} failed in its network namespace. If unshare itself failed, \
         unprivileged user namespaces may be disabled (kernel.apparmor_restrict_unprivileged_userns)."
    );
}
