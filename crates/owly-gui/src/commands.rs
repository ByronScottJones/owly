//! The IPC commands, and the events the backend emits (see `ui/src/lib/api.ts`).
//!
//! | Event | Payload |
//! |---|---|
//! | `peers` | `Peer[]`: everyone the last scan found |
//! | `incoming-request` | `IncomingRequest` |
//! | `request-withdrawn` | the request's id |
//! | `files-received`, `urls-received`, `receive-failed` | `Received`, `ReceivedUrls`, `ReceiveFailed` |
//! | `transfer` | `TransferUpdate` |
//! | `status-changed` | nothing: call `get_status` |
//! | `note` | a line of what the session is doing |

use std::time::Duration;

use owly_airdrop::driver::receiver::ReceiverEvent;
use owly_airdrop::driver::sender::{Event, Payload};
use owly_airdrop::proto::sender::Outcome;
use owly_session::discover::{self, Listed};
use owly_session::link::{self, Interface, Lease};
use owly_session::radio::{self, Held};
use owly_session::receive::{self, Settings};
use owly_session::{Notes, RECEIVE_PORT, new_sender};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::AppError;
use crate::model::{
    IncomingRequest, Peer, RadioStatus, ReceiveFailed, Received, ReceivedUrls, Status,
    TransferState, TransferUpdate, Visibility,
};
use crate::state::{AppState, Inner, Task};
use crate::validate;

/// How long each scan listens, and the pause before the next.
const SCAN: Duration = Duration::from_secs(8);
const SCAN_PAUSE: Duration = Duration::from_secs(2);

/// The TXT flags an Everyone-mode receiver announces, as `owly receive`'s
/// default does (MDNS-4).
const RECEIVER_FLAGS: u32 = 0x089;

fn emit<P: Serialize + Clone>(app: &AppHandle, event: &str, payload: P) {
    if let Err(e) = app.emit(event, payload) {
        tracing::warn!("couldn't emit {event}: {e}");
    }
}

fn notes(app: &AppHandle) -> Notes {
    let app = app.clone();
    Notes::new(move |line| emit(&app, "note", line.to_owned()))
}

async fn build_status(state: &AppState, inner: &Inner) -> Status {
    let radio_state = match &inner.radio {
        Some(_) => "up".to_owned(),
        None => radio::state(state.config.bus.as_deref())
            .await
            .unwrap_or_else(|_| "unavailable".into()),
    };
    Status {
        name: state.config.name.clone(),
        interface: state.config.iface.clone(),
        visibility: inner.visibility,
        radio: RadioStatus {
            state: radio_state,
            held: inner.radio.is_some(),
        },
        receive_dir: state.config.receive_dir.to_string_lossy().into_owned(),
        contacts_only_available: false,
        browsing: inner.browse.is_some(),
    }
}

#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> Result<Status, AppError> {
    let inner = state.inner.lock().await;
    Ok(build_status(&state, &inner).await)
}

/// Takes the radio from owlyd (polkit decides who may), and watches for it
/// ending under us.
#[tauri::command]
pub async fn acquire_radio(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    let mut inner = state.inner.lock().await;
    if inner.radio.is_some() {
        return Ok(());
    }
    let held = Held::acquire(state.config.bus.as_deref()).await?;
    let ended = held.watch();
    inner.radio = Some(held);
    let watcher = app.clone();
    inner.radio_watch = Some(Task::new(tauri::async_runtime::spawn(async move {
        let reason = ended.await;
        let state = watcher.state::<AppState>();
        let mut inner = state.inner.lock().await;
        stop_sessions(&mut inner);
        if let Some(held) = inner.radio.take() {
            held.restore_wifi().await;
        }
        drop(inner);
        emit(&watcher, "note", format!("the radio went down: {reason}"));
        emit(&watcher, "status-changed", ());
    })));
    drop(inner);
    emit(&app, "status-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn release_radio(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    let mut inner = state.inner.lock().await;
    stop_sessions(&mut inner);
    inner.radio_watch = None;
    let held = inner.radio.take();
    drop(inner);
    if let Some(held) = held {
        held.release().await?;
    }
    emit(&app, "status-changed", ());
    Ok(())
}

/// Ends browsing and receiving, which need the link the radio gave.
fn stop_sessions(inner: &mut Inner) {
    inner.browse = None;
    inner.receiver = None;
    inner.pending.clear();
    inner.peers.clear();
    inner.visibility = Visibility::Off;
}

/// The Doctor panel's data: owlyd's capability report, as JSON text.
#[tauri::command]
pub async fn doctor(state: State<'_, AppState>) -> Result<String, AppError> {
    Ok(radio::probe(state.config.bus.as_deref()).await?)
}

#[tauri::command]
pub async fn start_browse(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    let mut inner = state.inner.lock().await;
    if inner.browse.is_some() {
        return Ok(());
    }
    let (iface, name) = link::resolve(&state.config.iface, Some(&state.config.name))?;
    let task_app = app.clone();
    inner.browse = Some(Task::new(tauri::async_runtime::spawn(async move {
        browse_loop(task_app, iface, name).await;
    })));
    drop(inner);
    emit(&app, "status-changed", ());
    Ok(())
}

async fn browse_loop(app: AppHandle, iface: Interface, name: String) {
    let notes = notes(&app);
    let state = app.state::<AppState>();
    // BLE-1: wakes Apple receivers for as long as we're looking.
    let _advert = discover::advertise(false, &notes).await;
    loop {
        let Ok(sender) = new_sender(name.clone()) else {
            notes.say("couldn't make a sender");
            return;
        };
        let lease = Lease::find(state.config.bus.as_deref(), &iface.name)
            .await
            .ok();
        let found = discover::scan(&iface, &sender, lease.as_ref(), SCAN, &notes).await;
        let peers: Vec<Peer> = found.iter().map(Peer::from).collect();
        state.inner.lock().await.peers = found;
        emit(&app, "peers", peers);
        tokio::time::sleep(SCAN_PAUSE).await;
    }
}

#[tauri::command]
pub async fn stop_browse(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    let mut inner = state.inner.lock().await;
    inner.browse = None;
    inner.peers.clear();
    drop(inner);
    emit(&app, "peers", Vec::<Peer>::new());
    emit(&app, "status-changed", ());
    Ok(())
}

/// Sends files or links to a peer the last scan found; returns the
/// transfer's id, whose progress arrives as `transfer` events.
#[tauri::command]
pub async fn send(
    app: AppHandle,
    state: State<'_, AppState>,
    peer: String,
    paths: Vec<String>,
    urls: Vec<String>,
) -> Result<u64, AppError> {
    let payload = match (paths.is_empty(), urls.is_empty()) {
        (false, true) => Payload::Files(validate::paths(&paths)?),
        (true, false) => Payload::Urls(validate::urls(&urls)?),
        _ => return Err(AppError::Invalid("send either files or links".into())),
    };
    let target: Listed = state
        .inner
        .lock()
        .await
        .peers
        .iter()
        .find(|l| l.instance == peer)
        .cloned()
        .ok_or_else(|| AppError::NotFound("that receiver is no longer nearby".into()))?;
    let id = state.next_transfer();
    let name = state.config.name.clone();
    tauri::async_runtime::spawn(async move {
        run_transfer(app, id, name, target, payload).await;
    });
    Ok(id)
}

async fn run_transfer(app: AppHandle, id: u64, name: String, target: Listed, payload: Payload) {
    let update = |state: TransferState, sent: u64, total: u64, reason: Option<String>| {
        emit(
            &app,
            "transfer",
            TransferUpdate {
                id,
                peer: target.instance.clone(),
                state,
                sent,
                total,
                reason,
            },
        );
    };
    update(TransferState::Waiting, 0, 0, None);
    let sender = match new_sender(name) {
        Ok(sender) => sender,
        Err(e) => return update(TransferState::Failed, 0, 0, Some(e.to_string())),
    };
    let (events, mut updates) = tokio::sync::mpsc::unbounded_channel();
    let outcome = {
        let send = sender.send(target.addr, &payload, target.flags, Some(events));
        tokio::pin!(send);
        loop {
            tokio::select! {
                outcome = &mut send => break outcome,
                Some(event) = updates.recv() => match event {
                    Event::Accepted => update(TransferState::Accepted, 0, 0, None),
                    Event::Progress(p) => update(TransferState::Progress, p.sent, p.total, None),
                },
            }
        }
    };
    match outcome {
        Ok(Outcome::Delivered) => update(TransferState::Delivered, 0, 0, None),
        Ok(Outcome::Declined) => update(TransferState::Declined, 0, 0, None),
        Ok(Outcome::Failed(reason)) => update(TransferState::Failed, 0, 0, Some(reason)),
        Err(e) => update(TransferState::Failed, 0, 0, Some(e.to_string())),
    }
}

/// Who may see this computer. Contacts Only needs an Apple identity, which
/// Linux has no way to make (docs/implementation.md), so it's refused.
#[tauri::command]
pub async fn set_visibility(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: Visibility,
) -> Result<(), AppError> {
    match mode {
        Visibility::ContactsOnly => {
            return Err(AppError::Unavailable(
                "Contacts Only needs an Apple identity, which this computer doesn't have".into(),
            ));
        }
        Visibility::Off => {
            let mut inner = state.inner.lock().await;
            inner.receiver = None;
            inner.pending.clear();
            inner.visibility = Visibility::Off;
        }
        Visibility::Everyone => start_receiving(&app, &state).await?,
    }
    emit(&app, "status-changed", ());
    Ok(())
}

async fn start_receiving(app: &AppHandle, state: &AppState) -> Result<(), AppError> {
    let mut inner = state.inner.lock().await;
    if inner.visibility == Visibility::Everyone {
        return Ok(());
    }
    let notes = notes(app);
    let (iface, name) = link::resolve(&state.config.iface, Some(&state.config.name))?;
    let announce = match Lease::find(state.config.bus.as_deref(), &iface.name).await {
        Ok(lease) => lease.link.announce,
        Err(e) => {
            notes.say(format!("not announcing as the radio asks: {e}"));
            None
        }
    };
    std::fs::create_dir_all(&state.config.receive_dir)?;
    let settings = Settings {
        name,
        dir: state.config.receive_dir.clone(),
        port: RECEIVE_PORT,
        no_mdns: false,
        flags: RECEIVER_FLAGS,
        burst: link::burst(announce, &notes),
    };
    let mut running = receive::start(&iface, &receive::state_dir()?, settings).await?;
    let task_app = app.clone();
    inner.receiver = Some(Task::new(tauri::async_runtime::spawn(async move {
        while let Some(event) = running.events.recv().await {
            on_receiver_event(&task_app, event).await;
        }
    })));
    inner.visibility = Visibility::Everyone;
    Ok(())
}

async fn on_receiver_event(app: &AppHandle, event: ReceiverEvent) {
    match event {
        ReceiverEvent::Discovered { .. } => {}
        ReceiverEvent::Request(request) => {
            let shown = IncomingRequest::new(request.id, &request.ask);
            app.state::<AppState>()
                .inner
                .lock()
                .await
                .pending
                .insert(request.id, request);
            emit(app, "incoming-request", shown);
        }
        ReceiverEvent::Withdrawn { id } => {
            app.state::<AppState>()
                .inner
                .lock()
                .await
                .pending
                .remove(&id);
            emit(app, "request-withdrawn", id);
        }
        // SEC-4: shown, never opened.
        ReceiverEvent::Urls { id, urls } => emit(app, "urls-received", ReceivedUrls { id, urls }),
        ReceiverEvent::Received { id, paths } => {
            let paths = paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            emit(app, "files-received", Received { id, paths });
        }
        ReceiverEvent::Failed { id, reason } => {
            emit(app, "receive-failed", ReceiveFailed { id, reason });
        }
    }
}

/// The user's answer to an `incoming-request`. A request nobody answers is
/// declined by the receiver after 15 s (ASK-2).
#[tauri::command]
pub async fn respond(
    state: State<'_, AppState>,
    request: u64,
    accept: bool,
) -> Result<(), AppError> {
    let pending = state.inner.lock().await.pending.remove(&request);
    let pending = pending.ok_or_else(|| AppError::NotFound("that request has ended".into()))?;
    if accept {
        pending.accept();
    } else {
        pending.decline();
    }
    Ok(())
}
