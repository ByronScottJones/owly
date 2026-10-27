# Changelog

Every unit of work, newest first, one entry per branch. An entry holds only what was known when the work was done: git has the commits, pull request and merge time (`cargo xtask changelog` prints them, in UTC, for each entry's branch), and `cargo xtask changelog --check` fails if an entry holds one of them or has no timestamp. All timestamps are ISO 8601 UTC, the time the entry was written or last updated.

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
