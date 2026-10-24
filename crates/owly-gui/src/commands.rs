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
//! | `drag-over`, `drag-leave` | `DragPoint`: files are being dragged over the window |
//! | `files-dropped` | `Dropped`: files were dropped; name them by `selection.id` |
//!
//! The frontend never names a path: what is sent is a selection the backend
//! made from the native picker or the window's drop event.

use std::time::Duration;

use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;

use owly_airdrop::driver::receiver::ReceiverEvent;
use owly_airdrop::driver::sender::{Event, Payload};
use owly_airdrop::proto::sender::Outcome;
use owly_session::discover::{self, Listed};
use owly_session::link::{self, Interface, Lease};
use owly_session::radio::{self, Held};
use owly_session::receive::{self, Settings};
use owly_session::{Notes, RECEIVE_PORT, new_sender};
use serde::Serialize;
use tauri::{AppHandle, DragDropEvent, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::error::AppError;
use crate::model::{
    DragPoint, Dropped, IncomingRequest, Peer, RadioStatus, ReceiveFailed, Received, ReceivedUrls,
    SelectionInfo, Status, TransferState, TransferUpdate, Visibility, display_text,
};
use crate::state::{AppState, Inner, MAX_PENDING, Task};
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
    // Some notes carry what a peer called itself (SEC-4).
    Notes::new(move |line| emit(&app, "note", display_text(line, 500)))
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
    let _gate = state.radio_gate.lock().await;
    if state.inner.lock().await.radio.is_some() {
        return Ok(());
    }
    let held = Held::acquire(state.config.bus.as_deref()).await?;
    let ended = held.watch();
    let mut inner = state.inner.lock().await;
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
    let _gate = state.radio_gate.lock().await;
    let held = take_radio(&state).await;
    if let Some(held) = held {
        held.release().await?;
    }
    emit(&app, "status-changed", ());
    Ok(())
}

/// Ends the sessions and takes the lease out of the state, to give back.
async fn take_radio(state: &AppState) -> Option<Held> {
    let mut inner = state.inner.lock().await;
    stop_sessions(&mut inner);
    inner.radio_watch = None;
    inner.radio.take()
}

/// Gives the radio back, and Wi-Fi as this user, as the window closes:
/// owlyd ends a lease whose owner leaves the bus, but only the user's own
/// session can bring a keyring-held Wi-Fi password back (owly_session::wifi).
pub async fn shutdown(app: &AppHandle) {
    let state = app.state::<AppState>();
    let _gate = state.radio_gate.lock().await;
    let Some(held) = take_radio(&state).await else {
        return;
    };
    match tokio::time::timeout(Duration::from_secs(10), held.release()).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("couldn't give the radio back: {e}"),
        Err(_) => tracing::warn!("giving the radio back took too long"),
    }
}

/// Ends browsing and receiving, which need the link the radio gave.
fn stop_sessions(inner: &mut Inner) {
    inner.browse = None;
    inner.receiver = None;
    inner.pending.clear();
    inner.peers.clear();
    inner.selections.clear();
    inner.visibility = Visibility::Off;
}

/// The Doctor panel's data: what `owly doctor --json` prints.
#[tauri::command]
pub async fn doctor(state: State<'_, AppState>) -> Result<serde_json::Value, AppError> {
    Ok(owly_session::doctor::report(state.config.bus.as_deref()).await)
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
    // BLE-1: wakes Apple receivers for as long as we're looking. On its own
    // task: with no BlueZ on the bus, activating it takes 25 s to time out,
    // and scanning must not wait for that.
    let _advert = Task::new(tauri::async_runtime::spawn({
        let notes = notes.clone();
        async move {
            let _guard = discover::advertise(false, &notes).await;
            std::future::pending::<()>().await;
        }
    }));
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

/// Asks the user for files or a folder with the native picker, and keeps
/// them as a selection (or adds them to `selection`). `None` when they
/// cancelled.
#[tauri::command]
pub async fn choose_items(
    app: AppHandle,
    state: State<'_, AppState>,
    selection: Option<u64>,
    directory: bool,
) -> Result<Option<SelectionInfo>, AppError> {
    let picker = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let dialog = picker.dialog().file();
        if directory {
            dialog.blocking_pick_folder().map(|folder| vec![folder])
        } else {
            dialog.blocking_pick_files()
        }
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    let chosen = validate::existing(picked.into_iter().filter_map(|p| p.into_path().ok()))?;
    Ok(Some(add_to_selection(&state, selection, chosen).await))
}

async fn add_to_selection(
    state: &AppState,
    selection: Option<u64>,
    chosen: Vec<PathBuf>,
) -> SelectionInfo {
    let mut inner = state.inner.lock().await;
    let existing = selection.and_then(|id| inner.selections.get(&id).cloned());
    let id = selection
        .filter(|_| existing.is_some())
        .unwrap_or_else(|| state.next_transfer());
    let mut all = existing.unwrap_or_default();
    for path in chosen {
        if !all.contains(&path) {
            all.push(path);
        }
    }
    let info = selection_info(id, &all);
    inner.select(id, all);
    info
}

/// How many names of a selection the window lists.
const SELECTION_LISTED: usize = 20;

fn selection_info(id: u64, paths: &[PathBuf]) -> SelectionInfo {
    SelectionInfo {
        id,
        count: paths.len(),
        names: paths
            .iter()
            .take(SELECTION_LISTED)
            .map(|p| {
                let name = p.file_name().unwrap_or(p.as_os_str());
                display_text(&name.to_string_lossy(), 255)
            })
            .collect(),
        more: paths.len().saturating_sub(SELECTION_LISTED),
    }
}

/// Forgets a selection that won't be sent.
#[tauri::command]
pub async fn discard_selection(state: State<'_, AppState>, selection: u64) -> Result<(), AppError> {
    state.inner.lock().await.selections.remove(&selection);
    Ok(())
}

/// Sends a selection to a peer the last scan found; returns the transfer's
/// id, whose progress arrives as `transfer` events.
#[tauri::command]
pub async fn send_selection(
    app: AppHandle,
    state: State<'_, AppState>,
    peer: String,
    selection: u64,
) -> Result<u64, AppError> {
    let (target, paths) = {
        let mut inner = state.inner.lock().await;
        let target = find_peer(&inner, &peer)?;
        let paths = inner
            .selections
            .remove(&selection)
            .ok_or_else(|| AppError::NotFound("those files are no longer chosen".into()))?;
        (target, paths)
    };
    Ok(start_transfer(&app, &state, target, Payload::Files(paths)))
}

/// Sends a link to a peer the last scan found.
#[tauri::command]
pub async fn send_link(
    app: AppHandle,
    state: State<'_, AppState>,
    peer: String,
    url: String,
) -> Result<u64, AppError> {
    let urls = validate::urls(&[url])?;
    let target = find_peer(&*state.inner.lock().await, &peer)?;
    Ok(start_transfer(&app, &state, target, Payload::Urls(urls)))
}

fn find_peer(inner: &Inner, id: &str) -> Result<Listed, AppError> {
    inner
        .peers
        .iter()
        .find(|l| l.instance == id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("that receiver is no longer nearby".into()))
}

fn start_transfer(app: &AppHandle, state: &AppState, target: Listed, payload: Payload) -> u64 {
    let id = state.next_transfer();
    let name = state.config.name.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        run_transfer(app, id, name, target, payload).await;
    });
    id
}

/// What the window does with a drag over it: say where, so the page can
/// light up a person, and on a drop keep the files as a selection.
pub fn on_drag_drop(app: &AppHandle, event: &DragDropEvent) {
    match event {
        DragDropEvent::Enter { position, .. } | DragDropEvent::Over { position } => {
            let point = DragPoint {
                x: position.x,
                y: position.y,
            };
            emit(app, "drag-over", point);
        }
        DragDropEvent::Drop { paths, position } => {
            emit(app, "drag-leave", ());
            let (app, paths, position) = (app.clone(), paths.clone(), *position);
            tauri::async_runtime::spawn(async move {
                match validate::existing(paths) {
                    Ok(chosen) => {
                        let state = app.state::<AppState>();
                        let selection = add_to_selection(&state, None, chosen).await;
                        let dropped = Dropped {
                            selection,
                            x: position.x,
                            y: position.y,
                        };
                        emit(&app, "files-dropped", dropped);
                    }
                    Err(e) => emit(&app, "note", display_text(&e.to_string(), 500)),
                }
            });
        }
        _ => emit(app, "drag-leave", ()),
    }
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
    // Received files are the user's own: no one else needs to list them.
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&state.config.receive_dir)?;
    let settings = Settings {
        name,
        dir: state.config.receive_dir.clone(),
        port: RECEIVE_PORT,
        no_mdns: false,
        flags: RECEIVER_FLAGS,
        burst: link::burst(announce, &notes),
    };
    let running = receive::start(&iface, &receive::state_dir()?, settings).await?;
    let task_app = app.clone();
    inner.receiver = Some(Task::new(tauri::async_runtime::spawn(async move {
        // All of it, not just `events`: dropping the mDNS registration stops
        // the announcements, and the task is what keeps it for as long as
        // the receiver runs.
        let mut running = running;
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
            let state = app.state::<AppState>();
            let mut inner = state.inner.lock().await;
            if inner.pending.len() >= MAX_PENDING {
                // Too many prompts at once: decline this one unseen.
                request.decline();
                return;
            }
            inner.pending.insert(request.id, request);
            drop(inner);
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
