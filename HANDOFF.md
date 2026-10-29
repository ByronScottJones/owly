# Handoff

What is left, and where to resume. Open items only; finished work is in [CHANGELOG.md](CHANGELOG.md), and what the project must do is in [ENGINEERING.md](ENGINEERING.md).

## 1. Status

- Updated: 2026-10-29T15:55:00Z
- In flight: none.
- Verified on 2026-10-27 on the Ubuntu 26.04 VM (no Wi-Fi card), before GitHub's CI could run again: `just ci`, `gui-check`, `gui-a11y`, `hwsim`, `gui-e2e`, `gui-build`, `just fuzz 60`, and the `owly` and `owly-gui` `.deb`s installed with `apt`. The VM runs `kernel.apparmor_restrict_unprivileged_userns=0` until it reboots.
- GitHub's CI is running again and passed on `main` on 2026-10-29: `ci.yml` (`check`, `gui`, `fuzz`, `hwsim`) and `packages.yml` (Nix, AUR, `.deb`, `.rpm`, the window's `.deb`, and the install checks on Ubuntu 22.04 and 24.04, Debian 12 and 13, and Fedora 44). Both workflows now run only when started by hand (Actions tab, or `ghx workflow run ci.yml`), to save the allocation, so nothing guards `main` automatically: run `just ci` and `just gui-check gui-a11y gui-e2e` on the VM before a merge, and start both workflows before a release.

## 2. Resume here

1. On a computer with a real card and an iPhone: `just install`, then `just gui-dev` (or `owly gui`), and walk through H-2.
2. On the VM, from the checkout (`~/owly`, synced from the Mac): `just gui-check gui-a11y gui-e2e deny`.

## 3. In progress

None.

## 4. Outstanding work

| ID | Item | Why | Done when | Owner | Blocked by |
|---|---|---|---|---|---|
| H-2 | Try the window on a real card and phone | Nothing has run against a real `owlyd` with polkit, or a real iPhone | Turn On takes the radio; the phone is listed; a file and a link go each way; closing the window gives Wi-Fi back (OWLY-3 to OWLY-6, and the leaving rule in implementation.md) | Byron | A supported card |
| H-3 | Contacts Only | The selector lists it but refuses it until an Apple identity exists (docs/implementation.md, "Identity") | Choosing it works with an imported identity, and a test covers it | Byron | Identity import for `owly-airdrop` |
| H-5 | Drag and drop, and the native picker | The native picker is covered by `gui-e2e` (Add Files, typed path, send). Drop is not: under Xvfb the window receives no drag event from a test source that works against a plain GTK target and a bare WebKitGTK view; cause unknown | Dropping files on a person, and Add Folder, send them, on GNOME and KDE; if the drop does not work, find why the window gets no drag event | Byron | A graphical session on the VM or another computer |
| H-8 | Review the accepted advisory | `glib` (RUSTSEC-2024-0429, via Tauri's GTK 3 bindings) is ignored in `deny.toml` with a review date (reviewed 2026-10-27: Tauri 2.12.5 and 3.0.0-alpha.5 still use gtk-rs 0.18, so nothing to move to yet) | Reviewed, or Tauri's bindings moved, before 2027-01-27 | Byron | |
| H-9 | Who may loosen OWLY-3 to OWLY-6 | docs/implementation.md is the authority for the window's requirements (decided 2026-10-29); the file does not say who approves a change to them, or which rules are expected to change | The owner has said which rules, if any, need loosening and who approves; ENGINEERING.md and implementation.md say so | Byron | |

## 5. Decisions needed

- **`just gui-e2e` in CI?** It clicks fixed coordinates in a 960x600 window, so font or theme differences between runners could break it. Recommendation: keep it a local check until the `gui` job has run on GitHub a few times, then add it as its own job so it can't hide the others.
- **CHANGELOG.md history.** One backfilled entry stands for everything before the window. Recommendation: leave it, since `git log --merges` has the rest.

## 6. Known issues, risks and unverified areas

- Opening links (OWLY-7) is not exercised end to end: the confirmation dialog the backend shows before links are turned on, and `xdg-open` starting the browser, need a graphical session (with H-5 and H-2).
- `npm audit` reports 2 low advisories (`deepmerge`, via `@sveltejs/vite-plugin-svelte`) with no fix yet; recheck when the plugin updates.
- The page has not been checked against a screen reader, only against axe-core.
- The popup of the visibility menu is drawn by GTK, not the page, so it does not follow the page's accent colour.
- The `.deb` depends on `libwebkit2gtk-4.1-0`, which Debian 12 and Ubuntu 22.04 and newer have. CI installs it with `apt` on Debian 12 and 13 and Ubuntu 22.04 and 24.04 and checks that `/usr/bin/owly-gui` has no missing libraries; the window has not been launched on them.
- `just hwsim` loads and unloads `mac80211_hwsim` and needs root.

## 7. Environment and access notes

- The test VM is an Ubuntu 26.04 arm64 guest in UTM, reached by SSH with the owner's key; the user has passwordless sudo there. The Mac cannot build `owly-radio` (netlink), so builds and tests run on the VM, synced with rsync excluding `target`, `.git`, `node_modules`, `ui/dist` and `.claude` (a `.claude` directory under `crates/` breaks the workspace's `crates/*` glob).
- Network access to the VM is blocked inside Claude Code's sandbox; those commands need it lifted.
- GitHub work goes through `ghx` (see `agents/git.md`); the repositories are private. No credentials are recorded here.
- A local Qwen3.8 27B is available through lmstudio-rs-mcp (`agents/lmstudio-subagents.md`). On 2026-10-24 its `run_subagent` calls timed out after 60 s (the server's idle limit) even for a one-directory task, though `health_check` passed; it may need a smaller or non-thinking model for tool loops.

## 8. Cleanup debt

- On the VM: `~/uictl-src` (a clone of uictl, reviewed and not installed), `/tmp/owly-gui-e2e`, and packages installed for testing (`just`, `hostapd`, `xdotool`, `xvfb`, WebKitGTK development packages).
