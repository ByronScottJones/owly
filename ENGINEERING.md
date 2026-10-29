# Owly: engineering specification

- Status: current for the window (`feature/tauri-gui`); the protocol, radio and daemon parts point to their own documents.
- Last reviewed against the code: 2026-10-24T22:20:00Z
- Base branch: `main`

This is the primary specification. When the code, [README.md](README.md) or [AGENTS.md](AGENTS.md) disagrees with it, that is a defect to settle in the same change. [HANDOFF.md](HANDOFF.md) lists what is left and [CHANGELOG.md](CHANGELOG.md) what has been done. The AirDrop protocol itself is specified, without any implementation, in [docs/SPEC.md](docs/SPEC.md); what Owly does where the spec leaves a choice, and its workarounds, are in [docs/implementation.md](docs/implementation.md) with `OWLY-n` IDs; the conformance tests are in [docs/conformance.md](docs/conformance.md). This file does not repeat them. It covers the project as built and, in full, the part added by the window (`owly-session` and `owly-gui`).

## 1. Overview

Owly lets a Linux computer whose Wi-Fi card is not Apple's Broadcom chip send and receive files and links with iPhones, iPads and Macs over AirDrop. It runs AWDL (Apple's peer-to-peer Wi-Fi) on the card, and speaks AirDrop over it.

Audience: people who use it, and engineers who change it.

Scope:

- A radio service (`owlyd`) that brings `awdl0` up and down for one client at a time.
- The AirDrop protocol as a library (`owly-airdrop`) and AWDL as another (`owly-awdl`).
- The `owly` command line: `doctor`, `radio`, `receive`, `send`, `browse`, `test-card`, `capture` and `gui`.
- `omdrop-owl`, the hooks for the omdrop desktop plugin, which continues the numbering of the legacy omdrop-owl 0.3.0.
- **The window** (`owly-gui`): the same send, receive and discoverability as the CLI, in a desktop window laid out like AirDrop in Finder, plus a Doctor tab. Linux only.

Non-goals: Contacts Only (it needs an Apple identity; see docs/implementation.md, "Identity"), Windows or macOS clients of the window, opening received links or files, AirDrop to devices that are not Apple's, and accounts of any kind.

## 2. Requirements

Protocol and radio requirements have the IDs of docs/SPEC.md and docs/implementation.md (`ASK-`, `AWDL-`, `MDNS-`, `OWLY-1` and `OWLY-2`, and so on). `cargo xtask coverage` fails when an in-scope MUST has no test that cites it (`// spec: ID`).

The window adds these (docs/implementation.md, "The window", is the normative text):

| ID | Requirement |
|---|---|
| OWLY-3 | The page loads only the app's bundled files: a CSP from `default-src 'none'` with no inline or `eval` script and no remote or `data:` source, and no navigation to any other page. |
| OWLY-4 | The page may only listen for events and call the app's own commands, each allowed by name; no dialog, file-system, shell, window or webview permission; every command needs a permission of its own. |
| OWLY-5 | The page never names a path or an address: files go by the id of a backend-held selection (at most 8), people by the id of the last scan. The backend resolves paths to existing files or folders, and accepts only absolute receive folders, `http` and `https` links of at most 2048 characters, and interface names of the usual form. |
| OWLY-6 | Text from the other device is shown and never acted on: controls replaced, bidirectional-override and zero-width characters dropped, lengths bounded, at most 100 files or links listed per request with a count of the rest. |

Further requirements of the window, each with the test or check that holds it:

| ID | Requirement | Held by |
|---|---|---|
| GUI-1 | The window MUST follow the system's light or dark theme, with no manual toggle, and honour reduced motion. | `ui/src/app.css`; the a11y scan runs both themes |
| GUI-2 | Every state of the page MUST pass axe-core (WCAG 2 A and AA rules) in light and dark. | `just gui-a11y` |
| GUI-3 | A second launch MUST raise the running window and exit, not start another. | `just gui-e2e` setup; manual under Xvfb |
| GUI-4 | Closing the window MUST end its lease on the radio and restore Wi-Fi as the user. | Not exercised end to end (HANDOFF H-2) |
| GUI-5 | The first scan, and the receiver's mDNS announcements, MUST NOT wait for Bluetooth. | `just gui-e2e` fails without it |
| GUI-6 | Receiving MUST announce over mDNS for as long as the receiver runs. | `just gui-e2e` fails without it |
| GUI-7 | Contacts Only MUST be listed, disabled with the reason shown, and refused by the backend with an `unavailable` error. | `VisibilityFooter.svelte`; `model.test.ts` (the refusal is shown); `commands::set_visibility` |
| GUI-8 | At most 32 incoming requests wait at once; more are declined unseen. A request nobody answers is declined by the receiver after 15 s (ASK-2). | `state::MAX_PENDING` |
| GUI-9 | The receive folder MUST be created `0700`. | `commands::start_receiving` |

Non-functional: no root for the window (the radio is leased through owlyd's polkit-gated D-Bus API); no network access of the page; accessibility as GUI-1 and GUI-2.

## 3. External interfaces

### Command line

`owly gui` replaces the `owly` process with `owly-gui`, found beside the `owly` binary or on `PATH`. If it is missing it exits with status 2 and says to install the `owly-gui` package or run `just gui-build`. The other subcommands are in [README.md](README.md) and `owly --help`.

### Environment variables of `owly-gui`

| Variable | Default | Validation | Notes |
|---|---|---|---|
| `OWLY_IFACE` | `awdl0` | 1 to 15 characters of letters, digits, `.`, `_` and `-` | The link to receive and scan on; it must be the one owlyd leased (otherwise the scan skips the radio's AWDL peers). |
| `OWLY_RECEIVE_DIR` | `<Downloads>/Owly` | Absolute | Created with mode `0700` when receiving starts, if it does not exist. |
| `OWLY_BUS` | the system bus | any D-Bus address | Honoured in debug builds only. |
| `RUST_LOG` | `warn` | `tracing` filter | Logs go to stderr. |

### Commands (IPC)

Each is allowed by a permission named `allow-<command>` (`crates/owly-gui/build.rs` lists them, `capabilities/default.json` grants them). Errors reject with `{ kind, message }`.

| Command | Arguments | Returns | What it does |
|---|---|---|---|
| `get_status` | | `Status` | Name, interface, visibility, radio state, receive folder, whether Contacts Only is available, whether browsing. |
| `acquire_radio` | | | Leases the radio from owlyd (polkit may ask); watches for the lease ending. |
| `release_radio` | | | Gives it back and stops browsing and receiving. |
| `doctor` | | JSON | The report of `owly doctor --json`. |
| `start_browse`, `stop_browse` | | | Starts or stops the watch for people and the BLE advert. |
| `choose_items` | `selection: id or null`, `directory: bool` | `SelectionInfo` or null | The native picker; adds to a selection or makes one. Null if cancelled. |
| `discard_selection` | `selection` | | Forgets a selection. |
| `send_selection` | `peer`, `selection` | transfer id | Sends the selection's files to a person from the last scan. |
| `send_link` | `peer`, `url` | transfer id | Sends a link. |
| `set_visibility` | `mode: everyone, contacts_only, off` | | Starts or stops the receiver. Contacts Only is refused. |
| `respond` | `request`, `accept` | | Answers an incoming request. |

### Events

| Event | Payload | When |
|---|---|---|
| `peers` | `Peer[]` | After each scan, and cleared when browsing stops. |
| `incoming-request` | `IncomingRequest` | Someone asks to send. |
| `request-withdrawn` | request id | The sender left, or it timed out. |
| `files-received`, `urls-received`, `receive-failed` | `Received`, `ReceivedUrls`, `ReceiveFailed` | A request ends. |
| `transfer` | `TransferUpdate` | A send changes state: waiting, accepted, progress, delivered, declined, failed. |
| `status-changed` | none | Call `get_status`. |
| `note` | text (sanitised, at most 500 characters) | What the session is doing. |
| `drag-over`, `drag-leave` | `DragPoint` or none | Files are dragged over the window. |
| `files-dropped` | `Dropped` | Files were dropped; named by `selection.id`. |

### Exit status

`owly-gui` exits 0 on a normal close. It panics with "the Owly window failed to start" if the webview cannot be created, and a bad `OWLY_IFACE` or `OWLY_RECEIVE_DIR` stops start-up with a message on stderr.

## 4. Data

- Received files go in the receive folder under the names the other device offered, made safe by `owly-airdrop`'s sandboxed extraction (UP-*, never overwriting a file).
- The TLS identity of the receiver is `$XDG_STATE_HOME/owly` (else `~/.local/state/owly`), generated on first use and shared with the CLI.
- Links are text. Nothing is opened (SEC-4).
- Nothing is stored by the page: no cookies, no local storage.

## 5. Behaviour

- **Turn on:** `acquire_radio` leases owlyd's radio with duration 0 (until released). `start_browse` follows; `set_visibility(everyone)` starts the receiver on the leased link, announcing in the bursts the radio asks for (OWLY-1).
- **Watch:** runs while the window is on: mDNS browse of `_airdrop._tcp` on the link, plus, on a lease, `/Discover` at each AWDL peer the radio hears, read every 0.5 s (DISC-5). A device is listed when it answers `/Discover` with a name, and unlisted when its mDNS announcement is withdrawn or expires, or the radio stops hearing the peer. One entry per instance; each change sends the page the whole list. If neither source can be watched, it tries again after 5 s.
- **Send:** the page names a person by id and a selection by id; the backend looks both up, starts a transfer task and emits `transfer` updates. A selection is dropped when sent or discarded.
- **Receive:** each request is shown with its sender, model and a bounded list of items, with Decline focused first. Accept starts the transfer into the receive folder; the page then shows what arrived. Links are shown with a Copy button.
- **Leaving:** on exit the app prevents the exit once, releases the radio, restores Wi-Fi (NetworkManager connections that were active when it started), then exits.
- **Radio lost:** when owlyd ends the lease (another client, a restart), the app emits a note and `status-changed`; the page shows the problem banner.

## 6. Architecture

```text
 page (Svelte)  <-- events / commands (IPC) -->  owly-gui (Tauri)  --> owly-session --> owly-airdrop, owly-awdl
                                                                              \--> owlyd (D-Bus) --> owly-radio --> the card
```

- **owly-session** has the session calls with no interface: lease the radio, find the link, scan, receive, report. It takes a `Notes` sink for what it says, so the CLI prints and the window emits events.
- **owly-gui** owns the state of one window: the lease, the scan task, the receiver, the requests waiting, and the selections. The page holds no secrets and no paths; it renders what events say and calls commands.
- **owlyd** serves one lease of the radio at a time and ends it when the holder leaves the bus.

Data flow of a send: the page calls `choose_items` (or the window's drop event makes a selection), then `send_selection(peer, selection)`; the backend finds both, runs `owly_airdrop`'s sender against the person's address, and emits `transfer` updates, which the page shows on the person.

File map (paths from `crates/`):

| File | Purpose |
|---|---|
| `owly-session/src/lib.rs` | The library's modules, and the re-exports the CLI and the window use. |
| `owly-session/src/notes.rs` | `Notes`, the sink for what a session says: stderr for the CLI, events for the window. |
| `owly-session/src/ids.rs` | Port numbers (8770 probe, 8771 receive), random service ids, MAC parsing, and the sender. |
| `owly-session/src/link.rs` | Interface lookup, the crypto provider, host name, and owlyd's lease on a link (`Lease`), with the announce burst. |
| `owly-session/src/radio.rs` | `Held`: taking and giving back the radio through owlyd, watching for its end, and the radio's state. |
| `owly-session/src/wifi.rs` | Restoring the NetworkManager connections the radio took down, as the user. |
| `owly-session/src/discover.rs` | mDNS browse, `/Discover` at AWDL peers, the BLE advert, and `scan`, which combines them. |
| `owly-session/src/receive.rs` | The receiver: listener, mDNS announcement and the identity state. |
| `owly-session/src/doctor.rs` | The capability report, from owlyd or a local probe. |
| `owly-session/src/bluez.rs` | The AirDrop BLE advertisement over BlueZ's D-Bus API. |
| `owly-session/tests/session.rs` | The window's calls end to end on a veth pair and a fake radio. |
| `owly-gui/Cargo.toml` | The `owly_gui` library and the `owly-gui` binary. |
| `owly-gui/build.rs` | The list of commands and the permission manifest built from it. |
| `owly-gui/tauri.conf.json` | The window, the CSP, and the `.deb` bundle. |
| `owly-gui/owly-gui.desktop` | The desktop entry template for the package. |
| `owly-gui/capabilities/default.json` | The permissions the main window has. |
| `owly-gui/icons/` | The icons (an SVG and the sizes made from it). |
| `owly-gui/src/main.rs` | Starts the window. |
| `owly-gui/src/lib.rs` | Configuration from the environment, the navigation guard, the plugins, the exit handler, and the security tests. |
| `owly-gui/src/commands.rs` | The IPC commands, the events, the scan loop, the receiver, and transfers. |
| `owly-gui/src/state.rs` | The window's state: the lease, tasks, requests waiting, selections and their limits. |
| `owly-gui/src/model.rs` | What crosses the IPC boundary, and the text sanitising. |
| `owly-gui/src/validate.rs` | Checks of everything the page sends: paths, links, interface names. |
| `owly-gui/src/error.rs` | `AppError` and how it crosses as `{ kind, message }`. |
| `owly-gui/ui/index.html`, `src/main.ts` | The page's entry. |
| `owly-gui/ui/src/App.svelte` | The shell: sidebar, problem banner and dialogs. |
| `owly-gui/ui/src/app.css` | The theme tokens (light and dark) and reduced motion. |
| `owly-gui/ui/src/lib/api.ts` | The typed commands and events; the only module that calls `invoke` or `listen`. |
| `owly-gui/ui/src/lib/model.svelte.ts` | The page's state and actions, the only code that calls the backend. |
| `owly-gui/ui/src/lib/format.ts` | Initials, avatar colours, sizes, and transfer text. |
| `owly-gui/ui/src/lib/AirDropView.svelte` | The people grid, empty and off states, and the drop highlight. |
| `owly-gui/ui/src/lib/PersonCard.svelte`, `Avatar.svelte` | One person, and their round avatar. |
| `owly-gui/ui/src/lib/SendDialog.svelte` | Choosing files, a folder or a link to send. |
| `owly-gui/ui/src/lib/IncomingDialog.svelte` | The prompt for a request, with Decline first. |
| `owly-gui/ui/src/lib/Arrivals.svelte` | What was received, and received links with Copy. |
| `owly-gui/ui/src/lib/VisibilityFooter.svelte` | "Allow me to be discovered by:" and its explanation. |
| `owly-gui/ui/src/lib/DoctorPanel.svelte` | The Doctor tab. |
| `owly-gui/ui/src/lib/*.test.ts` | Vitest tests of the API, formatting and the model. |
| `owly-gui/ui/mock/` | A mock backend for development, and `a11y.py`, the axe-core scan in WebKitGTK. |
| `../tests/gui-e2e/` | The end-to-end run: `run.sh` and its screenshot helper. |

The rest of the workspace: `owly-awdl` (AWDL, sans-IO, `no_std` core), `owly-airdrop` (the protocol core and its tokio driver), `owly-radio` (the card on Linux), `owlyd` (the radio service), `owly-cli` (the `owly` command), `omdrop-owl` (the omdrop hooks), and `xtask` (repository tasks).

## 7. Security and privacy model

The window's page is a web page, and anything that runs in a webview can be attacked through its content: a peer-controlled string, a dependency, a bug. The design assumes the page is compromised and limits what that gets an attacker.

| Boundary | Untrusted side | What crosses | Defence |
|---|---|---|---|
| Page to backend | The page | Command arguments | Only 12 named commands, each needing its own permission (OWLY-4); arguments are ids and enums, never paths or addresses (OWLY-5); the backend validates them (`validate.rs`). |
| Backend to page | The other device's names, file names and links | Event payloads | Sanitised and bounded (OWLY-6); shown as text, never as HTML (Svelte escapes), never opened (SEC-4). |
| The page's content | Anything it loads | | CSP with no inline script, `eval`, remote or `data:` source; `freezePrototype`; navigation restricted to the app's own page (OWLY-3). |
| The network | Any device on the link | AirDrop requests | `owly-airdrop`'s protections (the spec's SEC-* and UP-* rules): sandboxed extraction, size limits, ASK timeouts. The window adds the cap of 32 pending requests. |
| The radio | Other users of the computer | The lease | owlyd's polkit-gated D-Bus API; the window runs as the user. |
| The files on disk | | The receive folder | Created `0700`; names are made safe and never overwrite. |

Threats and what stands in the way:

- **A malicious sender** names a file `evil‮gpj.exe`, or floods requests: bidi and control characters are removed, requests are capped and unanswered ones are declined at 15 s.
- **A compromised page** tries to send `~/.ssh` to a nearby device: it cannot name a path. It could only send what the user picked or dropped, and only to a person from the last scan.
- **A compromised page** reads files, runs programs or opens windows: no permission allows it.
- **A second instance** takes the radio or the port: a second launch raises the first (GUI-3).
- **Wi-Fi left down** after a crash or close: the window restores the connections it saw active on exit; owlyd undoes a killed client's session when its lease ends.
- **A debug override reaching production:** `OWLY_BUS` is ignored in release builds.

Privacy: the computer's name is what others see (the host name). Nothing is sent anywhere but to the device the user chose, or, when receiving in Everyone mode, to whoever asks. No telemetry, no accounts.

Accepted risks: `glib` (RUSTSEC-2024-0429, an unsound iterator in the GTK 3 bindings Tauri uses on Linux) is ignored in `deny.toml` until 2027-01-24, with a reason. The unmaintained-crate warnings for `zbus_names`, `zvariant`, `proc-macro-error` and `unic-*` are warnings, not failures.

## 8. Error handling and logging

- Backend errors cross as `{ kind, message }` (`AppError`); the page shows `message` and chooses what to offer from `kind`: `radio_unreachable` (owlyd isn't running), `not_authorized` (polkit said no), `radio_busy`, `radio_not_now`, `no_radio`, `radio` (any other radio failure), `io`, `invalid`, `unavailable` and `not_found`.
- Failed transfers are `transfer` updates with state `failed` and a reason; a declined one is `declined`.
- Radio loss and scan problems are notes (`note` events), and a problem banner shows when the radio is not usable.
- Logs go to stderr through `tracing`, `warn` by default; `RUST_LOG=owly_gui=debug,owly_airdrop=debug` for more. Nothing is logged that a sender chose, apart from at `debug`.

## 9. Technology and dependencies

Rust 2024 edition, 1.98 or newer; the workspace is `resolver = "3"`. Node 22 for the page.

| Dependency | Where | Why |
|---|---|---|
| `tauri` 2.11 | owly-gui | The window and IPC. 2.12 needs Rust 1.90, which the workspace now has; the move to it is not done, so the npm packages stay pinned to `~2.11.0`, which Tauri checks. |
| `tauri-plugin-dialog` 2 | owly-gui | The native file picker, called by the backend only; the page has no dialog permission. |
| `tauri-plugin-single-instance` 2 | owly-gui | GUI-3. |
| `thiserror` 2, `serde`, `serde_json`, `url`, `tracing`, `tokio` | owly-gui | Errors, payloads, link validation, logs, async. |
| `zbus` 5, `nix` 0.31, `futures-util`, `getrandom` | owly-session | The D-Bus lease, interface lookup, tag generation. |
| `owly-airdrop`, `owly-awdl`, `owly-radio`, `owlyd` | owly-session | The protocol, AWDL, the radio and its API. |
| `p256` 0.14, `rcgen` 0.14, `rustls` 0.23 (ring) | owly-airdrop | The receiver's self-signed identity and TLS (TLS-*). `p256` 0.14 is the first with `crypto-bigint` 0.7.7 (RUSTSEC-2026-0387). |
| `svelte` 5, `vite` 8, `typescript` 5.9, `@tauri-apps/api` ~2.11 | `ui/` | The page. |
| `vitest` 5, `svelte-check`, `prettier`, `axe-core` | `ui/` (dev) | Component and model tests, type checks, formatting, accessibility. |

Every crate's licence is on `deny.toml`'s allow-list; `cargo deny check` and `npm audit` pass (0 vulnerabilities on 2026-10-24).

## 10. Build, run, test and release

| Task | Command |
|---|---|
| Format and lints (without the window) | `just check` |
| Tests (without the window) | `just test` |
| Everything CI checks | `just ci` (adds `cargo deny`, the portability checks, `cargo xtask coverage`, the vectors, the hardware table) |
| The window's checks | `just gui-check` (svelte-check, Prettier, Vitest, build, clippy, Rust tests) |
| Accessibility | `just gui-a11y` (needs xvfb, python3-gi, python3-gi-cairo, gir1.2-webkit2-4.1) |
| Run with live reload | `just gui-dev` |
| End to end | `just gui-e2e` (sudo, xvfb, xdotool) |
| The window's `.deb` | `just gui-build` |
| Advisories | `just deny` |
| Version | `just bump <x.y.z>` (also the window's `tauri.conf.json`, the package's dependency on `owly`, and `ui/package.json`) |

The window needs WebKitGTK 4.1 and GTK 3 development packages to build. `owly-radio` builds only on Linux, so on another system everything runs on a Linux machine; this project's tests run on an Ubuntu 26.04 VM. A debug build of the window loads its bundled page only with `--features tauri/custom-protocol`; otherwise it waits for the dev server on port 1420.

CI (`.github/workflows`): `ci.yml` runs `just ci`, fuzzing, `just hwsim` and the window's `gui` job; `packages.yml` builds the AUR, `.deb`, `.rpm` and Nix packages and installs the `.deb`s (with the window's) and the `.rpm` on each release they are for; `linux-packages.yml` builds them, the window's on Ubuntu 22.04 for its glibc; `release.yml` tags, publishes and attaches them. Releasing is pushing a new `pkgver` to `main`.

## 11. Verification plan

| Requirement | Held by |
|---|---|
| OWLY-3 | `owly-gui/src/lib.rs`: `only_its_own_page_may_load`, `the_csp_allows_no_inline_or_eval_script` |
| OWLY-4 | `the_window_can_only_listen_and_call_the_listed_commands`, `every_command_in_the_manifest_has_a_permission` |
| OWLY-5 | `state.rs`: `the_oldest_selection_goes_first`; `validate.rs`: six tests |
| OWLY-6 | `model.rs`: `controls_are_replaced_and_reordering_dropped`, `long_text_is_cut_with_an_ellipsis`, `an_offer_is_bounded_and_counted` |
| GUI-1, GUI-2 | `just gui-a11y`: 9 states in 2 themes |
| GUI-3, GUI-5, GUI-6 | `just gui-e2e`; `tests/gui-e2e/run.sh` |
| The session calls | `crates/owly-session/tests/session.rs`: the lease, a scan, a file and a link each way, the release |
| The page's logic | 39 Vitest tests under `ui/src/lib` |

Acceptance of a release of the window: `just ci`, `just gui-check`, `just gui-a11y` and `just gui-e2e` pass, and the walk-through of HANDOFF H-2 on a real card and phone. Show that a check can fail before relying on it: the a11y scan was shown to fail on a deliberate violation, and `just gui-e2e` on the removal of the two fixes it exists for. Results of runs belong in CHANGELOG.md.

## 12. Decisions

- **A separate crate and a shared library (`owly-gui`, `owly-session`).** Context: the session logic was in the CLI. Choice: move it to a library both use. Rationale: one behaviour, and the window does not shell out. Rejected: wrapping the CLI. Consequence: the CLI shrank, and `owly-gui` stays out of `just check` and `just test` because it needs WebKitGTK.
- **Tauri 2 with Svelte, Vite and TypeScript.** Rationale: the team's `tauri.md` prefers Svelte, and it gives a small binary with the system webview. Rejected: GTK or Qt directly (more code for the layout), Electron (a bundled browser).
- **Follow the system theme, with no toggle.** Rationale: a desktop app should. Consequence: both themes are tested.
- **Our own icons**, not Apple's.
- **Contacts Only is listed but disabled.** Rationale: it matches Finder and says why it is unavailable; real support is a separate feature (HANDOFF H-3).
- **The backend owns paths and peers.** Rationale: OWLY-5. Rejected: validating paths sent by the page, which still lets a compromised page choose any readable file.
- **A per-command permission manifest.** Rationale: Tauri's default allows every command to every window.
- **Tauri packaged as its own `.deb`, `owly-gui`, depending on `owly`.** Rationale: the CLI package stays free of WebKitGTK. The product name is `owly-gui` so the package name does not clash with `owly`; the desktop entry's name is `Owly` through a template.
- **`just gui-e2e` is not in CI yet.** See HANDOFF, section 5.
