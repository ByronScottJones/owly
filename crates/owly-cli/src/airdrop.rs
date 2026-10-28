//! AirDrop on its own, without omdrop: `owly receive`, `owly send` and
//! `owly browse`, owly-airdrop's receiver and sender on one interface.
//!
//! The interface is any one with an IPv6 link-local address: the `awdl0`
//! of an owlyd lease (`owly radio up`) with an iPhone, or the veth harness
//! between two of these. On the `awdl0` of an owlyd lease, the receiver
//! announces in the bursts the radio asks for (MDNS-7), and the sender
//! tries the radio's AWDL peers beside mDNS (DISC-5), both as owlyd's D-Bus
//! API says. The sessions themselves are owly-session's.

use std::io;
use std::net::SocketAddrV6;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use owly_airdrop::driver::receiver::ReceiverEvent;
use owly_airdrop::driver::sender::{Event, Payload};
use owly_airdrop::proto::mdns::{self, RECEIVER_FLAGS, SUPPORTS_DVZIP};
use owly_airdrop::proto::messages::Offer;
use owly_airdrop::proto::sender::Outcome;
use owly_session::discover::{self, Listed};
use owly_session::link::{self, Interface, Lease};
use owly_session::receive::{self, Settings};
use owly_session::{AIRDROP_PORT, Notes, RECEIVE_PORT, new_sender, parse_mac};
use tokio::io::AsyncBufReadExt;

/// Where and as whom: the flags every AirDrop command takes.
#[derive(clap::Args)]
pub struct On {
    /// The interface to use, such as awdl0.
    #[arg(long, short, default_value = "awdl0")]
    iface: String,
    /// The name others see (default: the host name).
    #[arg(long)]
    name: Option<String>,
}

#[derive(clap::Args)]
pub struct ReceiveArgs {
    #[command(flatten)]
    on: On,
    /// Where received files go.
    #[arg(long, default_value = ".")]
    dir: PathBuf,
    /// The firewall has to let it in on the interface (NET-4).
    #[arg(long, default_value_t = RECEIVE_PORT)]
    port: u16,
    /// Accept every request without asking (ASK-3).
    #[arg(long)]
    accept: bool,
    /// Don't announce over mDNS, for when another responder does.
    #[arg(long)]
    no_mdns: bool,
    /// The TXT flags to announce (MDNS-4), decimal or 0x-prefixed hex,
    /// to compare what Apple senders do with each (MDNS-5).
    #[arg(long, default_value = "0x089", value_parser = parse_flags)]
    flags: u32,
}

#[derive(clap::Args)]
pub struct BrowseArgs {
    #[command(flatten)]
    on: On,
    /// How long to browse.
    #[arg(long, default_value_t = 10)]
    seconds: u64,
    /// Don't send the BLE advert that wakes Apple receivers (BLE-1).
    #[arg(long)]
    no_ble: bool,
}

#[derive(clap::Args)]
pub struct SendArgs {
    #[command(flatten)]
    on: On,
    /// The receiver: its name, service ID or AWDL address. The first one
    /// found if left out.
    #[arg(long)]
    to: Option<String>,
    /// Skip mDNS: try the receiver at this AWDL address on port 8770
    /// (DISC-5).
    #[arg(long, conflicts_with = "to")]
    mac: Option<String>,
    /// Send a link instead of files.
    #[arg(long)]
    url: Vec<String>,
    /// How long to look for the receiver.
    #[arg(long, default_value_t = 30)]
    seconds: u64,
    /// Don't send the BLE advert that wakes Apple receivers (BLE-1).
    #[arg(long)]
    no_ble: bool,
    files: Vec<PathBuf>,
}

impl On {
    fn resolve(&self) -> io::Result<(Interface, String)> {
        link::resolve(&self.iface, self.name.as_deref())
    }
}

/// `owly receive`.
pub async fn receive(bus: Option<&str>, args: ReceiveArgs) -> Result<ExitCode, String> {
    let notes = Notes::stderr();
    let (iface, name) = args.on.resolve().map_err(|e| e.to_string())?;
    let announce = match Lease::find(bus, &iface.name).await {
        Ok(lease) => lease.link.announce,
        Err(e) => {
            notes.say(format!("not announcing as the radio asks: {e}"));
            None
        }
    };
    run_receiver(&iface, name, link::burst(announce, &notes), args)
        .await
        .map_err(|e| e.to_string())
}

/// `owly browse`.
pub async fn browse(args: BrowseArgs) -> Result<ExitCode, String> {
    async {
        let notes = Notes::stderr();
        let (iface, name) = args.on.resolve()?;
        let _advert = discover::advertise_in_background(args.no_ble, &notes);
        let sender = new_sender(name)?;
        discover::browse_receivers(
            &iface,
            &sender,
            Duration::from_secs(args.seconds),
            |r| {
                println!("{:<14} {:?} ({})", r.instance, r.name, r.addr);
                false
            },
            &notes,
        )
        .await?;
        Ok(ExitCode::SUCCESS)
    }
    .await
    .map_err(|e: io::Error| e.to_string())
}

/// `owly send`.
pub async fn send(bus: Option<&str>, args: SendArgs) -> Result<ExitCode, String> {
    send_inner(bus, args).await.map_err(|e| e.to_string())
}

async fn send_inner(bus: Option<&str>, args: SendArgs) -> io::Result<ExitCode> {
    let SendArgs {
        on,
        to,
        mac,
        url,
        seconds,
        no_ble,
        files,
    } = args;
    let notes = Notes::stderr();
    let (iface, name) = on.resolve()?;
    let payload = match (url.is_empty(), files.is_empty()) {
        (false, true) => Payload::Urls(url),
        (true, false) => Payload::Files(
            files
                .iter()
                .map(std::path::absolute)
                .collect::<Result<_, _>>()?,
        ),
        _ => return Err(io::Error::other("give either files or --url, not both")),
    };
    let _advert = discover::advertise_in_background(no_ble, &notes);
    let sender = new_sender(name)?;
    let target = match mac {
        Some(mac) => {
            let addr =
                SocketAddrV6::new(parse_mac(&mac)?.link_local(), AIRDROP_PORT, 0, iface.index);
            let receiver = discover::knock(&sender, addr, Duration::from_secs(seconds), &notes)
                .await?
                .ok_or_else(|| io::Error::other("the receiver at that address doesn't list us"))?;
            Listed {
                instance: mac,
                name: receiver.computer_name,
                addr,
                // UP-1b: no TXT to say, and Apple receivers take DVZip.
                flags: RECEIVER_FLAGS | SUPPORTS_DVZIP,
            }
        }
        None => {
            let lease = Lease::find(bus, &iface.name)
                .await
                .inspect_err(|e| notes.say(format!("not trying the radio's AWDL peers: {e}")))
                .ok();
            match &to {
                Some(to) => notes.say(format!("looking for {to:?} for {seconds} s...")),
                None => notes.say(format!("looking for a receiver for {seconds} s...")),
            }
            discover::find_receiver(
                &iface,
                &sender,
                lease.as_ref(),
                to.as_deref(),
                Duration::from_secs(seconds),
                &notes,
            )
            .await?
        }
    };
    notes.say(format!("sending to {:?} at {}", target.name, target.addr));
    let (events, mut updates) = tokio::sync::mpsc::unbounded_channel();
    let printer = tokio::spawn(async move {
        while let Some(event) = updates.recv().await {
            match event {
                Event::Accepted => eprintln!("accepted"),
                Event::Progress(p) => eprint!("\r{} / {} bytes", p.sent, p.total),
            }
        }
        eprintln!();
    });
    let outcome = sender
        .send(target.addr, &payload, target.flags, Some(events))
        .await?;
    let _ = printer.await;
    Ok(match outcome {
        Outcome::Delivered => {
            println!("delivered");
            ExitCode::SUCCESS
        }
        Outcome::Declined => {
            println!("declined");
            ExitCode::from(1)
        }
        Outcome::Failed(reason) => {
            println!("failed: {reason}");
            ExitCode::from(2)
        }
    })
}

fn parse_flags(text: &str) -> Result<u32, String> {
    match text.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => text.parse(),
    }
    .map_err(|e| format!("{text}: {e}"))
}

async fn run_receiver(
    iface: &Interface,
    name: String,
    burst: Option<mdns::Burst>,
    args: ReceiveArgs,
) -> io::Result<ExitCode> {
    let ReceiveArgs {
        dir,
        port,
        accept,
        no_mdns,
        flags,
        ..
    } = args;
    let settings = Settings {
        name: name.clone(),
        dir,
        port,
        no_mdns,
        flags,
        burst,
    };
    let mut running = receive::start(iface, &receive::state_dir()?, settings).await?;
    eprintln!(
        "receiving as {name:?} ({}) on [{}]:{port}, flags {flags:#x}",
        running.service_id, iface.addr
    );
    let events = &mut running.events;

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    loop {
        let event = tokio::select! {
            event = events.recv() => event,
            _ = tokio::signal::ctrl_c() => return Ok(ExitCode::SUCCESS),
        };
        let Some(event) = event else {
            return Ok(ExitCode::SUCCESS);
        };
        match event {
            ReceiverEvent::Discovered { peer, .. } => eprintln!("seen by {peer}"),
            ReceiverEvent::Request(request) => {
                // SEC-4: everything here is the sender's; {:?} escapes it.
                eprintln!(
                    "{:?} ({:?}) wants to send:",
                    request.ask.sender_computer_name, request.ask.sender_model_name
                );
                match &request.ask.offer {
                    Offer::Files(files) => {
                        for f in files {
                            eprintln!(
                                "  {:?} {} {}",
                                f.name,
                                f.file_type,
                                f.size.map(|s| format!("{s} bytes")).unwrap_or_default()
                            );
                        }
                    }
                    Offer::Urls(urls) => {
                        for u in urls {
                            eprintln!("  {u:?}");
                        }
                    }
                }
                if accept {
                    request.accept();
                    continue;
                }
                eprint!("accept? [y/N] ");
                // ASK-2: the receiver declines by itself after 15 s.
                let answer = tokio::time::timeout(Duration::from_secs(15), stdin.next_line()).await;
                match answer {
                    Ok(Ok(Some(line))) if line.trim().eq_ignore_ascii_case("y") => request.accept(),
                    Ok(_) => request.decline(),
                    Err(_) => eprintln!("\ntimed out"),
                }
            }
            ReceiverEvent::Withdrawn { .. } => eprintln!("the request was withdrawn"),
            // SEC-4: shown, never opened.
            ReceiverEvent::Urls { urls, .. } => {
                for u in urls {
                    println!("{u:?}");
                }
            }
            ReceiverEvent::Received { paths, .. } => {
                for p in paths {
                    println!("{}", p.display());
                }
            }
            ReceiverEvent::Failed { reason, .. } => eprintln!("transfer failed: {reason}"),
        }
    }
}
