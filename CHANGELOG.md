# Changelog

Every unit of work, newest first, one entry per branch. An entry holds only what was known when the work was done: git has the commits, pull request and merge time (`cargo xtask changelog` prints them, in UTC, for each entry's branch), and `cargo xtask changelog --check` fails if an entry holds one of them or has no timestamp. All timestamps are ISO 8601 UTC, the time the entry was written or last updated.

---
### docs/h9-approvals: the owner's answers on loosening the window's rules

- Branch: `docs/h9-approvals`, base `main`
- Written: 2026-10-29T17:40:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner, who gave the approvals); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - Records who approves a change to OWLY-3 to OWLY-7 (the owner, by merging a pull request that edits docs/implementation.md, the test and the CHANGELOG) in docs/implementation.md and ENGINEERING.md. H-9 is closed.
  - Approved by the owner but not built, now H-10 and H-11 in HANDOFF: thumbnails of received images, and a "Show in folder" button for a received file with no option to open it.
  - Confirmed as built: `http` and `https` links, and the link-opening setting off by default.
  - OWLY-7 now also alerts on a punycode (`xn--`) host, and the incoming prompt, which asks whether to accept a link, shows the same alerts as the received card. The prompt's links are now `ReceivedLink`s, not plain text, so the event's shape changed for the page.
  - H-2 (a real card and phone) is deferred by the owner until a Linux computer with a supported card is set up.
### feature/open-link: an Open button for received links, and a settings file

- Branch: `feature/open-link`, base `main`
- Written: 2026-10-29T15:55:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner, who allowed the button, asked for the caution and the non-ASCII alert, and recommended the settings file and dialog); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - A received link can be opened in the browser from its card in the window, on a click, once the user has turned that on. New requirement OWLY-7 in docs/implementation.md (the authority for the window's rules), with OWLY-4 and OWLY-6 amended to point at it. The card says to check the address first; a link with any character that is not printable ASCII (counting the zero-width and reordering characters the display drops, and control characters) gets an alert naming them as code points, and its button reads "Open anyway".
  - The setting lives in a new settings file, `~/.config/owly/owly_config.toml` (TOML; `[window] open_links`), edited by a new Preferences dialog (sidebar item, or Ctrl+,) or by hand. `OWLY_OPEN_LINKS` beats the file and fixes the setting, as configuration.md orders. The file is written `0600` in a `0700` folder through a temporary file; a file that does not parse, or has an unknown key, stops the window starting and names the file.
  - Off by default. Turning it on from the page first asks in a dialog the backend shows (the page has no dialog permission and cannot answer it); Cancel changes nothing. This is why the setting is not simply a checkbox the page writes: the page is untrusted (OWLY-3 to OWLY-6).
  - The page names a link by receipt and position, never its address (OWLY-5); the backend keeps the addresses of the last 16 receipts, opens only `http` and `https` links of at most 2048 characters, with `xdg-open` and no shell.
  - Two commands (`open_link`, `set_settings`), so 14 in all; `toml` 1 is a direct dependency (already in the build).
  - Also fixed: the `urls-received` event sent the page the links unsanitised, while the incoming prompt sanitised them (OWLY-6). They are now sanitised, bounded to 100 and counted, as the prompt's are.
  - Verified on the VM, on Rust 1.99.1: `gui-check` (31 Rust tests, the page's tests, svelte-check, Prettier, clippy), `gui-a11y` (12 states in 2 themes, including the links card and the preferences), `gui-e2e`.
  - Not exercised: the confirmation dialog and the browser opening (they need a graphical session), and the incoming prompt, which does not yet mark non-ASCII links (the alert is on the received card).
### docs/engineering-gaps-2: the second cold read of ENGINEERING.md

- Branch: `docs/engineering-gaps-2`, base `main`
- Written: 2026-10-29T15:35:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner and reviewer); Claude, claude-sonnet-5-5 (Claude Code, implementer and the reviewer subagent)
- What it did:
  - A second subagent, given only ENGINEERING.md, listed what it would still guess. The facts were checked against the repo and added: the apt packages and where `just`, Rust and Node come from; that `just install` builds and that owlyd starts on demand; how to run the window; how to run `owly-session`'s veth test and what it needs; the format of a CHANGELOG entry (pointer), what "upstream" is, where `AUR_SSH_KEY` comes from; that checks run before a merge and the owner merges; where the command count appears (three places); one-line meanings of AWDL, lease, polkit, the BLE advert, Everyone and Contacts Only, hwsim, the fake radio, `ghx` and omdrop-owl.
  - The one contradiction it found is settled by the owner: docs/implementation.md, "The window", is the authority for OWLY-3 to OWLY-6, and ENGINEERING.md says so (line 7 and section 2), with OWLY-5's "canonical form" added to match. Still open: who approves a loosening of those rules (H-9), and whether `gui-e2e` is a merge requirement (HANDOFF section 5).
  - Not done: a third read. The remaining gaps are decisions, not missing facts.
### chore/aur-publish-off: no publishing to the AUR until there is a key

- Branch: `chore/aur-publish-off`, base `main`
- Written: 2026-10-29T15:41:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner, who asked for it); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - `aur-release.yml` publishes to the AUR only when the repository variable `AUR_PUBLISH` is `true`. Unset (as now), a release still tags, builds the packages and attaches the files, and the publish step is replaced by a notice. The "`AUR_SSH_KEY` isn't set" failure applies only when publishing is on. To turn it on later: add the `AUR_SSH_KEY` secret, then set the variable (`ghx variable set AUR_PUBLISH --body true`).
  - This changes nothing today: the release jobs already run only on `t4t5/owly`, not on this fork. The switch is for a release made from a repository that has no key yet.
  - Not tested: the workflow has not run (it cannot here). The YAML parses, and the conditions were read against the step outputs.

---
### docs/handoff-manual-ci: HANDOFF.md says CI is manual

- Branch: `docs/handoff-manual-ci`, base `main`
- Written: 2026-10-29T15:26:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner and reviewer); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - HANDOFF.md section 1 now says `ci.yml` and `packages.yml` run only when started by hand, so the checks go on the VM before a merge. The workflow change itself is in the `chore/ci-manual-only` entry; this line was held back so the two branches did not conflict.

---
### chore/ci-manual-only: GitHub CI runs only when started by hand

- Branch: `chore/ci-manual-only`, base `main`
- Written: 2026-10-29T15:22:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner, who chose manual-only runs); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - `ci.yml` and `packages.yml` start only with `workflow_dispatch`, not on every pull request push and merge to `main`. Each run had cost about 17 min (CI) and 30 min (Packages) of runner time, twice per pull request. This is what `agents/githubactions.md` already asks for ("Manual by Default").
  - `release.yml` is unchanged: it starts on a version bump (a `PKGBUILD` change on `main`) and its release jobs run only on the upstream repository.
  - ENGINEERING.md section 10 says so; nothing now guards `main` automatically, so `just ci` and the window's checks go on the VM before a merge, and the workflows are started by hand before a release.
  - Not checked: that `workflow_dispatch` shows the two workflows in the Actions tab; that is visible once this is on `main`.
### docs/close-h7: the new CI jobs have run on GitHub

- Branch: `docs/close-h7`, base `main`
- Written: 2026-10-29T14:49:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner and reviewer); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - Closes H-7 in HANDOFF.md. With Actions credits back, GitHub's CI passed on `main` after the `cargo-deny` fix: `ci.yml` (`check`, `gui`, `fuzz`, `hwsim`) and `packages.yml` (Nix, AUR, `.deb`, `.rpm`, `gui-deb`, and the install checks on Ubuntu 22.04 and 24.04, Debian 12 and 13, and Fedora 44). The Nix job building with `rust-version = "1.98"` also confirms that choice.
  - The decision about `just gui-e2e` in CI no longer says CI cannot run it.

---
### fix/ci-install-cargo-deny: `cargo deny` missing from the CI runner

- Branch: `fix/ci-install-cargo-deny`, base `main`
- Written: 2026-10-29T14:15:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner and reviewer); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - `just ci` runs `cargo deny check` (added with the window), but the `check` job installed only `just`, so on GitHub it failed with "no such command: `deny`". It had not run there before: the window's branch was verified on the VM while Actions credits were out. `ci.yml` now installs `cargo-deny` beside `just`. The first run on `main` after the Rust 1.98 bump failed this way, with `gui`, `hwsim`, `fuzz` and the Packages workflow passing.
  - Not checked until it runs on GitHub: whether anything after `cargo deny` in `just ci` also needs a tool the runner lacks (the VM has them all).
### feature/tauri-2.12: Tauri 2.11 to 2.12

- Branch: `feature/tauri-2.12`, base `main`
- Written: 2026-10-29T13:47:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner and reviewer); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - `tauri` 2.11.6 to 2.12.5 (needs Rust 1.90), with `tauri-build` 2.7.4, `tauri-plugin-dialog` 2.8.4, `tauri-plugin-single-instance` 2.5.5, `wry` 0.57.1 and the rest of what they pull in (`Cargo.lock`); `@tauri-apps/api` and `@tauri-apps/cli` to `~2.12.0`, since Tauri checks that their minor version matches the crate's. No source or configuration changed.
  - `crypto-bigint` 0.7.7 to 0.7.10 (and `ctutils` 0.4.5 to 0.4.7) in the same branch: 0.7.7 was yanked upstream after the last run, and `cargo deny` fails on a yanked crate. It is `p256`'s dependency, not Tauri's.
  - `glib` stays at 0.18.5: Tauri 2.12 still uses gtk-rs 0.18, so the accepted advisory (RUSTSEC-2024-0429) stands.
  - Verified on the VM, on Rust 1.99.1: `just ci` (with `cargo deny`), `gui-check`, `gui-a11y` and `gui-e2e`.
  - `npm audit` now reports 2 low advisories (`deepmerge`, via `@sveltejs/vite-plugin-svelte`, no fix available). They are not from this change (the plugin is unchanged); ENGINEERING.md section 9 and HANDOFF.md section 6 say so.
  - Not checked: the window on a real card and phone (H-2), the `.deb` build and installs on GitHub.

---
### chore/rust-1.98: the minimum Rust version, 1.88 to 1.98

- Branch: `chore/rust-1.98`, base `main`
- Written: 2026-10-29T13:02:00Z
- Worked on by: Byron Scott Jones (ByronScottJones, owner and reviewer); Claude, claude-sonnet-5-5 (Claude Code, implementer)
- What it did:
  - `rust-version` in `Cargo.toml` is 1.98, and README.md, `packaging/package.sh` and ENGINEERING.md say so. This follows the new Rust version policy (`agents/rust.md`): the workspace was ten releases behind.
  - 1.98, not the latest stable (1.99.1), because the Nix package builds with the nixpkgs that `flake.lock` pins (15 October 2026), whose default Rust is 1.98; Cargo refuses to build when `rust-version` is newer than the compiler. Move to 1.99 when `flake.lock` is updated (`nix flake update nixpkgs`, on a machine with Nix).
  - Verified on the VM with Rust 1.98.1 installed by `rustup` and chosen with `RUSTUP_TOOLCHAIN`: `just ci`, `gui-check`, `gui-a11y` and `gui-e2e`. The same four also passed on 1.99.1 before the pin was changed from 1.99. No code needed changing and no new warnings appeared.
  - Not checked: the Nix build itself (no Nix on the VM or the Mac), and the `.deb` and `.rpm` jobs on GitHub.
  - Not done: Tauri 2.12 needs Rust 1.90, which the workspace now has; the move from 2.11 is its own change (ENGINEERING.md section 9).

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
