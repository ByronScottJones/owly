# Handoff

What is left, and where to resume. Open items only; finished work is in [CHANGELOG.md](CHANGELOG.md), and what the project must do is in [ENGINEERING.md](ENGINEERING.md).

## 1. Status

- Updated: 2026-10-27T19:30:00Z
- In flight: none.
- Verified on 2026-10-27 on the Ubuntu 26.04 VM (no Wi-Fi card), in place of GitHub's CI (its credits ran out): `just ci`, `gui-check`, `gui-a11y`, `hwsim`, `gui-e2e`, `gui-build`, `just fuzz 60`, and the `owly` and `owly-gui` `.deb`s installed with `apt`. The VM runs `kernel.apparmor_restrict_unprivileged_userns=0` until it reboots.
- GitHub's CI has not run the new `gui` and `gui-deb` jobs or the install checks on Ubuntu 22.04 and 24.04, Debian 12 and 13 and Fedora (H-7).

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
| H-4 | Live list of people | The scan refreshes the list every ~10 s, not as devices appear | A device appearing or leaving shows within ~2 s | Claude | |
| H-5 | Drag and drop, and the native picker | Not exercised: Xvfb has no desktop to drop from | Dropping files on a person, and Add Files and Add Folder, send them, on GNOME and KDE | Byron | A graphical session on the VM or another computer |
| H-7 | The new CI jobs on GitHub | Their steps ran on the VM; the jobs themselves, and the 22.04 build and the other releases' installs, have not | `ci.yml` `gui`, `linux-packages.yml` `gui-deb` and `packages.yml` pass on GitHub | Byron | GitHub Actions credits |
| H-8 | Review the accepted advisory | `glib` (RUSTSEC-2024-0429, via Tauri's GTK 3 bindings) is ignored in `deny.toml` with a review date | Reviewed, or Tauri's bindings moved, before 2027-01-24 | Byron | |
| H-9 | A first-run check of ENGINEERING.md | It was written by the people who know the code | A reviewer given only ENGINEERING.md lists what they would have to guess, and the gaps are filled | Byron | |

## 5. Decisions needed

- **`just gui-e2e` in CI?** It clicks fixed coordinates in a 960x600 window, so font or theme differences between runners could break it. Recommendation: keep it a local check until the `gui` job has run on GitHub a few times (it cannot now, for want of credits), then add it as its own job so it can't hide the others.
- **CHANGELOG.md history.** One backfilled entry stands for everything before the window. Recommendation: leave it, since `git log --merges` has the rest.

## 6. Known issues, risks and unverified areas

- The page has not been checked against a screen reader, only against axe-core.
- The popup of the visibility menu is drawn by GTK, not the page, so it does not follow the page's accent colour.
- The `.deb` depends on `libwebkit2gtk-4.1-0`, which Debian 12 and Ubuntu 22.04 and newer have. It was checked only by `dpkg -I` and on the VM, not installed on those releases.
- `just hwsim` loads and unloads `mac80211_hwsim` and needs root.

## 7. Environment and access notes

- The test VM is an Ubuntu 26.04 arm64 guest in UTM, reached by SSH with the owner's key; the user has passwordless sudo there. The Mac cannot build `owly-radio` (netlink), so builds and tests run on the VM, synced with rsync excluding `target`, `.git`, `node_modules`, `ui/dist` and `.claude` (a `.claude` directory under `crates/` breaks the workspace's `crates/*` glob).
- Network access to the VM is blocked inside Claude Code's sandbox; those commands need it lifted.
- GitHub work goes through `ghx` (see `agents/git.md`); the repositories are private. No credentials are recorded here.
- A local Qwen3.8 27B is available through lmstudio-rs-mcp (`agents/lmstudio-subagents.md`). On 2026-10-24 its `run_subagent` calls timed out after 60 s (the server's idle limit) even for a one-directory task, though `health_check` passed; it may need a smaller or non-thinking model for tool loops.

## 8. Cleanup debt

- On the VM: `~/uictl-src` (a clone of uictl, reviewed and not installed), `/tmp/owly-gui-e2e`, and packages installed for testing (`just`, `hostapd`, `xdotool`, `xvfb`, WebKitGTK development packages).
