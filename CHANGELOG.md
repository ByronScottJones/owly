# Changelog

Every unit of work, newest first, one entry per branch. An entry holds only what was known when the work was done: git has the commits, pull request and merge time (`cargo xtask changelog` prints them, in UTC, for each entry's branch), and `cargo xtask changelog --check` fails if an entry holds one of them or has no timestamp. All timestamps are ISO 8601 UTC, the time the entry was written or last updated.

---

### docs/rust-version-policy: when to choose and change the Rust version

- Branch: `docs/rust-version-policy`, base `main`
- Written: 2026-10-27T22:15:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner, who set the policy); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - `agents/rust.md` gains a "Rust Version" section: new projects use the latest stable unless a dependency conflicts; existing projects move to the latest stable when a dependency, fix or feature needs it, or when `rust-version` is over six months behind; how to upgrade. The six-month limit is the implementer's addition to the owner's rule, to be adjusted.
  - The same section is to go into the global `~/.agents/rust.md`, which the owner edits; the two files already differ in line wrapping.
  - Not applied to Owly: its `rust-version` is 1.88, which this policy would now call due for a bump. That is its own change.

### docs/glib-advisory-review: the accepted `glib` advisory, reviewed

- Branch: `docs/glib-advisory-review`, base `main`
- Written: 2026-10-27T21:45:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner and reviewer); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - Reviewed RUSTSEC-2024-0429 (`glib` 0.18.5, via Tauri's GTK 3 bindings; fixed in `glib` 0.20). Owly is on `tauri` 2.11.6 and `gtk` 0.18.2. The latest 2.x, 2.12.5 (needs Rust 1.90; the project's `rust-version` is 1.88), still depends on `gtk` ^0.18, and 3.0.0-alpha.5 keeps it as an optional dependency beside a new optional `gtk4`. Nothing to move to, so the ignore stays; `deny.toml` and HANDOFF.md now say so, with the next review on 2027-01-27.
  - Checked from the crates.io and RustSec pages; no build changed.

### docs/engineering-gaps: what a cold read of ENGINEERING.md had to guess

- Branch: `docs/engineering-gaps`, base `main`
- Written: 2026-10-27T21:30:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner and reviewer); Claude, claude-sonnet-5-5 (Claude Code, implementer and the reviewer subagent)
- What it did:
  - A subagent given only ENGINEERING.md was asked what it would have to guess to build and run, check, add a command, change discovery and release. Each gap it named was checked against the repo before anything was written. Added to ENGINEERING.md: the prerequisites and how to build and try the CLI; what the vectors, the hardware table, the portability checks, `Held`, burst and the H-n IDs are; the steps to add a command, and the test that counts them; where the release version lives and what `release.yml` does with it, including the `AUR_SSH_KEY` secret; how the watch is tested without a radio.
  - Not gaps, so unchanged: the permission name (`allow-<command>`) and the OWLY-n IDs were already defined, and "12 commands" is right (the table lists `start_browse` and `stop_browse` on one row).
  - Not done: a second cold read after the additions; it needs a reviewer who hasn't seen them.

---
### feature/live-people: the list of people updates as devices come and go

- Branch: `feature/live-people`, base `fix/cli-no-bluez-wait` (the `fix/cli-no-bluez-wait` entry's change is in it)
- Written: 2026-10-27T20:30:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner and reviewer); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - The window scanned for 8 s, showed the result, paused 2 s and scanned again, so a device appeared up to ~10 s late and an mDNS goodbye was ignored. `discover::watch` runs for as long as the window browses and calls back with the whole list at each change: an mDNS answer to `/Discover` adds a device, a withdrawn or expired announcement removes it, and the radio's AWDL peers are read every 0.5 s (added when they answer, removed when the radio stops hearing them). `discover::scan` is now `watch` stopped after its limit. The window sends the page each list as it changes.
  - Along the way the mDNS loop no longer spins if its event channel closes, which `browse_receivers` still does (not changed, used only by the CLI).
  - Test: `owly-session`'s veth test starts the watch before the phone exists and checks the phone is listed once it starts (3 ms in the run) and unlisted once its receiver is dropped (about 1 s).
  - Verified on the VM: fmt, clippy, `just ci`.
  - Not checked: a real iPhone's AWDL peer coming and going (H-2), and the page itself repainting on each event (`gui-e2e`, below).

---
### fix/cli-no-bluez-wait: the CLI no longer waits for BlueZ

- Branch: `fix/cli-no-bluez-wait`, base `main`
- Written: 2026-10-27T19:30:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner and reviewer); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - `owly send` and `owly browse` awaited `discover::advertise` before looking for anyone, and with no BlueZ on the bus D-Bus activation timed out after 25 s. `discover::advertise_in_background` now sets the advert up on its own task and stops it when dropped; both commands and the window use it, so the window's own copy of that task is gone. (`owly receive` never advertised.)
  - Verified on the VM, which has no Bluetooth: `owly browse --seconds 2` takes 2.0 s; `just ci` passes. The two userns-dependent veth suites needed `kernel.apparmor_restrict_unprivileged_userns=0` set again after the VM's reboot.
  - Not covered by a test: it needs a D-Bus without BlueZ.

---
### fix/mdns-reencode-limit: a fuzz crash in the mDNS target

- Branch: `fix/mdns-reencode-limit`, base `main`
- Written: 2026-10-27T18:20:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner and reviewer); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - `just fuzz`, run on the VM because GitHub Actions credits ran out, found the `mdns` target crashing: a message that decodes can be heavily compressed, and `Message::encode` writes names without compression, so the re-encoded message can pass `MAX_MESSAGE` and `decode` refuses it. Owly only encodes the small messages it builds, so no code path is affected.
  - The fuzz target now checks the round trip only when the encoding fits `MAX_MESSAGE`; `encode`'s comment says so; a unit test builds a 100-record compressed message that decodes, grows past the limit when encoded, and is then refused.
  - Verified on the VM: fmt, clippy, spec coverage, the new test, and 3 minutes of the `mdns` target (12.3 million runs, no crash).
- Also run locally on the VM, in place of GitHub's CI: `just ci`, `gui-check`, `gui-a11y`, `hwsim`, `gui-e2e`, `gui-build`, the `owly` and `owly-gui` `.deb`s installed with `apt` (files, `ldd`, `desktop-file-validate`), and `just fuzz 60`. The `owly-airdrop` and `owly-session` veth tests passed as a user after the owner set `kernel.apparmor_restrict_unprivileged_userns=0` on the VM.

---
### feature/tauri-gui: a Tauri window beside the CLI

- Branch: `feature/tauri-gui`, base `main`
- Written: 2026-10-24T22:20:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner and reviewer); Claude, claude-sonnet-5-5 (Claude Code, implementer); Qwen3.8 27B via lmstudio-rs-mcp (asked for a file map under `read_only`; its runs timed out after 60 s even on a small task, so none of its output was used and Claude wrote the documentation)
- What it did:
  - `owly-session` (new library): the AirDrop session calls that were in the CLI (radio lease, interface lookup, receiver, scan, Doctor report, Wi-Fi restore, BlueZ advert), shared by the CLI and the window. The CLI shrank to arguments and printing.
  - `owly-gui` (new Tauri 2 app, Linux only) with a Svelte 5, Vite and TypeScript page in `ui/`: the people grid, the Send dialog (files, folder, link, drop), the incoming prompt, received items, Receiving Off and Everyone (Contacts Only listed but disabled until an Apple identity exists), and a Doctor tab. It follows the system theme. Our own icons.
  - The page is untrusted: no paths or addresses over IPC (backend-held selections, ids from the last scan), a per-command permission manifest with only event listening and 12 named commands, a strict CSP with no inline or eval script, navigation locked to its own page, sanitised and bounded peer text, a cap on pending requests, a `0700` receive folder, and `OWLY_BUS` honoured in debug builds only.
  - Closing the window releases the radio and restores Wi-Fi. A second launch raises the first (single-instance).
  - `owly gui` starts it; `owly-gui` is its own `.deb`, built, install-checked and attached to the release in CI. A `gui` CI job runs the checks and an axe-core scan.
  - Dependencies: `p256` 0.14 (clears RUSTSEC-2026-0387) and `rustls-webpki` 0.103.16 (RUSTSEC-2026-0382); `cargo deny check` passes and is part of `just ci`. `glib` (RUSTSEC-2024-0429, via Tauri's GTK 3 bindings) is an accepted risk in `deny.toml` until 2027-01-24.
  - Fixed, found by the end-to-end run: the window never announced itself over mDNS (the registration was dropped on return), and its first scan waited 25 s for BlueZ to time out on computers without it.
  - Tooling: `cargo xtask changelog`, `just gui-check`, `gui-a11y`, `gui-dev`, `gui-build`, `gui-e2e` and `deny`.
- Spec and docs: `docs/implementation.md` "The window" (OWLY-3 to OWLY-6, in scope in `tests/coverage.toml`); new ENGINEERING.md, HANDOFF.md, CHANGELOG.md and `agents/`; README sections on the window.
- Verification: on an Ubuntu 26.04 arm64 VM with no Wi-Fi: `cargo fmt`, clippy with `-D warnings`, the workspace unit tests, `cargo xtask coverage`, `cargo deny check`, svelte-check, Prettier, 39 Vitest tests, the frontend build, axe-core on every state in light and dark, the portability checks (Windows, Android, no-default-features), `owly-airdrop`'s veth tests (19 of 26, run as root; the other 7 read a vectors file the namespace can't reach), `owly-session`'s `tests/session.rs`, `just hwsim`, the `.deb` build with `dpkg -I` and `desktop-file-validate`, the single-instance behaviour, and `just gui-e2e` (files and links both ways, with the fault reintroduced to show it fails). Not exercised: a real `owlyd` with polkit, a real Wi-Fi card or phone, drag and drop, closing the window to release the radio, the new CI jobs on GitHub, the `.deb` installed on another machine.
- Notes: the veth tests need `kernel.apparmor_restrict_unprivileged_userns=0` on Ubuntu; it has not been changed on the VM. Open items are in HANDOFF.md.

---
### Before feature/tauri-gui: backfilled history

- Branch: `main`, several branches and direct commits
- Backfilled history: written 2026-10-24T21:50:00Z from the repository, not itemised per branch.
- Worked on by: the repository's contributors (`git shortlog -sn`)
- What it did: the Owly workspace up to 0.4.1: `owly-awdl`, `owly-airdrop`, `owly-radio`, `owlyd`, the `owly` CLI and omdrop-owl, which replaced the legacy omdrop-owl 0.3.0 (tag `v0.3.0`); the protocol spec and its conformance tests; the AUR, `.deb`, `.rpm` and Nix packages.
- Notes: `git log --merges` lists the merged branches. Entries per branch were not reconstructed.
