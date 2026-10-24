# Handoff

What is left, and where to resume. Open items only; finished work is in [CHANGELOG.md](CHANGELOG.md), and what the project must do is in [ENGINEERING.md](ENGINEERING.md).

## 1. Status

- Updated: 2026-10-24T22:20:00Z
- In flight: none. `feature/tauri-gui` completes the window (CHANGELOG.md).
- Verified on 2026-10-24 on the Ubuntu 26.04 VM (no Wi-Fi card): format, lints, unit tests, spec coverage, `cargo deny check`, the frontend checks and accessibility scan, the portability checks, `just hwsim` and `just gui-e2e`. The veth tests of `owly-airdrop` ran as root from a copy of the binary: 19 of 26 passed, and the 7 failures only read a vectors file the namespace can't reach. They have not run as a user on the VM (H-1).
- CI on GitHub has not run the new `gui` job, the `owly-gui` package job or its install check (H-7).

## 2. Resume here

1. On a computer with a real card and an iPhone: `just install`, then `just gui-dev` (or `owly gui`), and walk through H-2.
2. On the VM, from the checkout (`~/owly`, synced from the Mac): `just gui-check gui-a11y gui-e2e deny`.

## 3. In progress

None.

## 4. Outstanding work

| ID | Item | Why | Done when | Owner | Blocked by |
|---|---|---|---|---|---|
| H-1 | Allow user namespaces on the VM | `owly-airdrop`'s and `owly-session`'s veth tests re-run themselves under `unshare -rn`, which Ubuntu's AppArmor refuses | `cargo test --workspace --exclude owly-gui` passes on the VM as a user | Byron | Approval to set `kernel.apparmor_restrict_unprivileged_userns=0` (CI sets it itself) |
| H-2 | Try the window on a real card and phone | Nothing has run against a real `owlyd` with polkit, or a real iPhone | Turn On takes the radio; the phone is listed; a file and a link go each way; closing the window gives Wi-Fi back (OWLY-3 to OWLY-6, and the leaving rule in implementation.md) | Byron | A supported card |
| H-3 | Contacts Only | The selector lists it but refuses it until an Apple identity exists (docs/implementation.md, "Identity") | Choosing it works with an imported identity, and a test covers it | Byron | Identity import for `owly-airdrop` |
| H-4 | Live list of people | The scan refreshes the list every ~10 s, not as devices appear | A device appearing or leaving shows within ~2 s | Claude | |
| H-5 | Drag and drop, and the native picker | Not exercised: Xvfb has no desktop to drop from | Dropping files on a person, and Add Files and Add Folder, send them, on GNOME and KDE | Byron | A graphical session on the VM or another computer |
| H-6 | `owly send`, `browse` and `receive` wait 25 s for BlueZ | `discover::advertise` is awaited first, and with no BlueZ on the bus D-Bus activation times out after 25 s. The window's scan no longer waits; the CLI still does | The CLI starts at once with no Bluetooth, as the window does | Claude | |
| H-7 | The new CI jobs | Written and checked for syntax only | `ci.yml` `gui`, `linux-packages.yml` `gui-deb` and `packages.yml` pass on GitHub | Byron | Pushing the branch |
| H-8 | Review the accepted advisory | `glib` (RUSTSEC-2024-0429, via Tauri's GTK 3 bindings) is ignored in `deny.toml` with a review date | Reviewed, or Tauri's bindings moved, before 2027-01-24 | Byron | |
| H-9 | A first-run check of ENGINEERING.md | It was written by the people who know the code | A reviewer given only ENGINEERING.md lists what they would have to guess, and the gaps are filled | Byron | |

## 5. Decisions needed

- **`just gui-e2e` in CI?** It clicks fixed coordinates in a 960x600 window, so font or theme differences between runners could break it. Recommendation: keep it a local check until the `gui` job has run on GitHub a few times, then add it as its own job so it can't hide the others.
- **`assets/` (the Finder reference screenshots) is untracked.** Recommendation: commit it, since README and ENGINEERING.md refer to the layout.
- **CHANGELOG.md history.** One backfilled entry stands for everything before the window. Recommendation: leave it, since `git log --merges` has the rest.

## 6. Known issues, risks and unverified areas

- The page has not been checked against a screen reader, only against axe-core.
- The popup of the visibility menu is drawn by GTK, not the page, so it does not follow the page's accent colour.
- The `.deb` depends on `libwebkit2gtk-4.1-0`, which Debian 12 and Ubuntu 22.04 and newer have. It was checked only by `dpkg -I` and on the VM, not installed on those releases.
- `just hwsim` loads and unloads `mac80211_hwsim` and needs root.

## 7. Environment and access notes

- The test VM is an Ubuntu 26.04 arm64 guest in UTM, reached by SSH with the owner's key; the user has passwordless sudo there. The Mac cannot build `owly-radio` (netlink), so builds and tests run on the VM, synced with rsync excluding `target`, `.git`, `node_modules`, `ui/dist`, `assets` and `.claude` (a `.claude` directory under `crates/` breaks the workspace's `crates/*` glob).
- Network access to the VM is blocked inside Claude Code's sandbox; those commands need it lifted.
- GitHub work goes through `ghx` (see `agents/git.md`); the repositories are private. No credentials are recorded here.
- A local Qwen3.8 27B is available through lmstudio-rs-mcp (`agents/lmstudio-subagents.md`). On 2026-10-24 its `run_subagent` calls timed out after 60 s (the server's idle limit) even for a one-directory task, though `health_check` passed; it may need a smaller or non-thinking model for tool loops.

## 8. Cleanup debt

- On the VM: `~/uictl-src` (a clone of uictl, reviewed and not installed), `/tmp/owly-gui-e2e`, and packages installed for testing (`just`, `hostapd`, `xdotool`, `xvfb`, WebKitGTK development packages).
- `assets/` is untracked (see section 5).
