# Owly

https://github.com/user-attachments/assets/b195ce4c-e078-49be-bc13-44a72b6dc189

AirDrop between iPhones, Macs and Linux computers whose Wi-Fi isn't Apple's Broadcom chip. Owly runs AWDL, the Wi-Fi link AirDrop uses, in userspace on an ordinary card, and lends it out through `owlyd`, a small system service.

- **On Omarchy**, it's the radio under [Omdrop](https://github.com/netmojo-systems/omdrop-plugin): install `omdrop-owl`.
- **Elsewhere**, `owly` sends and receives by itself: `owly send` and `owly receive`.

See [supported hardware](#supported-hardware). `owly doctor` says what your card can do.

## Install

### On Omarchy, with Omdrop

From the [AUR](https://aur.archlinux.org/packages/omdrop-owl), with [Omdrop](https://github.com/netmojo-systems/omdrop-plugin#install) if it isn't installed yet:

```sh
yay -S omdrop-owl
omarchy plugin add https://github.com/netmojo-systems/omdrop-plugin.git
```

Then turn Omdrop on from its icon in the bar. `omdrop-owl` is Omdrop's radio backend, and pulls in `owly` for the radio. It works with both of Omdrop's versions, so there's nothing to choose: where omdrop-core, Omdrop's Rust version, is installed, Omdrop runs it on the hooks `omdrop-owl` installs in `/usr/lib/omdrop/radio/`. Otherwise Omdrop's Python version runs the radio helper and sender in `/usr/lib/omdrop/`, as it did with `omdrop-owl` 0.3.0.

Upgrading from `omdrop-owl` 0.3.0 or older replaces OWL with Owly, with the same Omdrop. Settings in `/etc/omdrop/` are carried over to `/etc/owly/radio.toml` unless that file already exists (see [Configuration](#configuration)).

### On Arch, without Omdrop

```sh
yay -S owly
```

### On NixOS (flake)

```nix
inputs.owly.url = "github:t4t5/omdrop-owl";
```

```nix
imports = [ inputs.owly.nixosModules.default ];
services.owly.enable = true;
# Optional: /etc/owly/radio.toml (see Configuration)
services.owly.settings = { allow_untested = true; };
```

The module installs `owly` and `owlyd` with its D-Bus and polkit files, keeps NetworkManager off Owly's interfaces and udev from changing `awdl0`'s MAC address, opens TCP 8771 (`owly receive`) on `awdl0`, and stops NixOS restarting wpa_supplicant whenever a session adds or removes an interface. The `services.omdrop-owl` options of 0.3.0 and older still work, with a warning, and carry over to `services.owly`.

### On Debian, Ubuntu or Fedora

Download the `.deb` or `.rpm` from the newest `owly-v…` release on the [releases page](https://github.com/t4t5/omdrop-owl/releases), then:

```sh
sudo apt install ./owly_*_amd64.deb     # Debian 12, Ubuntu 22.04 and newer
sudo dnf install ./owly-*.x86_64.rpm    # Fedora 44 and newer
```

They're built for x86_64. There's no apt or dnf repository yet: to upgrade, install a newer release's package the same way.

### On other distributions, from source

To build it from source you need Rust 1.88 or newer. Then:

```sh
cargo build --release --locked -p owlyd -p owly-cli
sudo packaging/install.sh owly
sudo systemctl daemon-reload && sudo nmcli general reload conf
```

`packaging/install.sh` is the install step every package runs too. It honours `DESTDIR` and `PREFIX` for packagers, and `packaging/install.sh --list owly` prints what it installs. At runtime, owlyd uses NetworkManager (`nmcli`), `ip` and `ping`, and `hostapd` on cards without ad-hoc mode.

On Debian, Ubuntu or Fedora on another architecture, `packaging/package.sh deb` or `packaging/package.sh rpm` builds the package from source instead. It also needs `dpkg-dev` or `rpm-build`.

The packages built from [`v0.3.0`](https://github.com/t4t5/omdrop-owl/tree/v0.3.0) for Debian, Fedora and NixOS (`omdrop-owl` 0.3.0) still work with the omdrop plugin's Python version. They aren't developed further.

## Usage

### With Omdrop

Turn Omdrop on from the bar, or use the `omdrop` CLI (`omdrop on 10m`, `omdrop send <device> <file>`). Omdrop asks Owly for the radio whenever it's visible or sending.

### Without Omdrop

Take the radio, which brings `awdl0` up, and keep it until Ctrl-C:

```sh
owly radio up
```

Then, in another terminal:

```sh
owly receive --dir ~/Downloads      # visible as this computer's name, asks before accepting
owly browse                         # nearby AirDrop receivers
owly send --to "Jane's iPhone" photo.jpg
owly send --url https://example.com
```

`owly receive` listens on TCP 8771 on `awdl0`, which the firewall has to allow. With UFW: `sudo ufw allow in on awdl0 to any port 8771 proto tcp`.

A phone set to "Contacts Only" doesn't see `owly receive`: set it to "Everyone for 10 Minutes". Omdrop, with your Apple identity, can do Contacts Only.

### Checking the card

```sh
owly doctor
```

says which mode the card runs in, what's in the way (an AP on the wrong channel, a missing `hostapd`), and what's known to go wrong on it. `owly radio up` gives the radio back on Ctrl-C. In exclusive mode Wi-Fi is down while the radio is up.

## Configuration

`/etc/owly/radio.toml`, all optional:

```toml
station = "wlan0"        # the Wi-Fi interface to use; else the first station
allow_untested = false   # run on a card the quirks table doesn't mark tested
reconnect_after = true   # overrides the driver's reconnect_after_monitor
country = "DE"           # exclusive modes: the regulatory domain to ask for
channels = [6]           # exclusive modes: overrides the driver's channel order
aggregation = true       # overrides the driver's rx_ampdu: accept the phone's Block Ack session
tx_rate = "mcs7"         # the data frame rate
hostapd = "/usr/sbin/hostapd" # for a hidden AP, on cards with no ad-hoc mode; else found on PATH
tx_amsdu = true          # send unicast as A-MSDUs where the peer takes them
```

What Owly does on each Wi-Fi driver comes from [`crates/owly-radio/drivers.toml`](crates/owly-radio/drivers.toml). `/etc/owly/drivers.toml`, in the same format, is read over it, so you can try settings for a new card without rebuilding.

## Supported hardware

**Status:** ✅ both directions work, ⚠️ both have worked but not reliably ([known issues](#known-issues)), ❌ tried and fails, ❓ untested\
**Modes:** *Shared* keeps Wi-Fi connected; *Exclusive* and *Active monitor* turn it off while the radio is up ([why](#modes))

<!-- hardware-table:start -->
<!-- Generated by `cargo xtask hardware` from hardware/ and crates/owly-radio/drivers.toml: edit those, not this table. -->

| Card | Driver | Status | Tested on |
|---|---|---|---|
| MediaTek MT7925 | `mt7925e` | ✅ Shared | Framework 13, `14c3:0717`, Omarchy 4.0.4 (kernel 7.2.5) |
| MediaTek MT7612U (USB) | `mt76x2u` | ✅ Exclusive | `0e8d:7612` (kernel 7.2.9) |
| Realtek RTL8822BE | `rtw88_8822be` | ✅ Exclusive; shared untested | Crosshair VII Hero, `10ec:b822`, NixOS 26.11 (kernel 7.2.9) |
| Realtek RTL8852AE | `rtw89_8852ae` | ✅ Shared (opt-in) | ThinkPad P14s Gen 2a, `10ec:8852`, Ubuntu 26.04.1 (kernel 7.0) |
| Realtek RTL8852BE | `rtw89_8852be` | ✅ Shared (opt-in) | ASUS PRIME B550M-A WIFI II, `10ec:b852`, CachyOS (kernel 7.2.9) |
| Realtek RTL8852CE | `rtw89_8852ce` | ✅ Shared (opt-in) | Legion Slim 5 16AHP9, `10ec:c852`, Omarchy 4.0.4 (kernel 7.2.5) |
| Realtek RTL8852BU (USB) | `rtw89_8852bu` | ✅ Exclusive (opt-in), as a second card | USB adapter, `35bc:0108`, NixOS 26.05 (kernel 7.2.9) |
| Intel 9560 | `iwlwifi` | ✅ Exclusive | Lenovo 81SY, `8086:a370`, Omarchy 4.0.0 (kernel 7.2.8) |
| Intel AX200 | `iwlwifi` | ✅ Exclusive | ThinkPad X1 Yoga, `8086:2723`, NixOS 26.05 (kernel 6.19.8) |
| Intel AX201 | `iwlwifi` | ✅ Exclusive | • Ubuntu 24.04 (kernel 6.8)<br>• CachyOS (kernel 7.2.9)<br>• Intel Client Systems NUC10i7FNK, `8086:02f0`, NixOS 26.05 (kernel 6.18.55) |
| Intel AX211 | `iwlwifi` | ✅ Exclusive | • ThinkPad X1, `8086:51f1`, Ubuntu 26.04 (kernel 7.0)<br>• XPS 9320, `8086:51f0`, Arch (kernel 7.2.3)<br>• Legion Pro 7 16IRX8H, `8086:7a70`, Omarchy 4.0.4 (kernel 7.2.3)<br>• TUXEDO InfinityBook Max 15 Gen10 (Arrow Lake), `8086:7740`, Kubuntu 26.04.1 (kernel 7.0) |
| Intel BE (Panther Lake) | `iwlwifi` | ✅ Exclusive | XPS 16, `8086:e440`, Arch (kernel 7.2) |
| Intel BE200 | `iwlwifi` | ✅ Exclusive | GEEKOM IT13 Max, `8086:272b`, Omarchy 4.0.4 (kernel 7.2.5) |
| MediaTek MT7921 | `mt7921e` | ⚠️ Exclusive, opt-in; shared fails ([#14](https://github.com/t4t5/omdrop-owl/issues/14)) | `14c3:7961`, CachyOS (kernel 7.2.9) |
| MediaTek MT7922 (RZ616) | `mt7921e` | ⚠️ Exclusive, opt-in; shared fails ([#14](https://github.com/t4t5/omdrop-owl/issues/14)) | Minisforum MS-A2, `14c3:0616`, NixOS 26.05 (kernels 6.18 and 7.2.5) |
| MediaTek MT7927 | `mt7925e` | ❌ Shared: never heard; active monitor (opt-in) sends, but receiving times out ([#44](https://github.com/t4t5/omdrop-owl/issues/44)) | TP-Link Archer TBE550E, `14c3:7927`, CachyOS (kernel 7.2.9) |
| Realtek RTL8821CE | `rtw88_8821ce` | ❌ Shared: OWL syncs but no transfer, Wi-Fi often lost after ([#60](https://github.com/t4t5/omdrop-owl/issues/60)); exclusive receives nothing | HP laptop, `10ec:c821`, Omarchy (kernel 7.2.5) |
| Qualcomm WCN6855 (QCNFA765) | `ath11k_pci` | ❌ Can't create the monitor interface both modes need ([#45](https://github.com/t4t5/omdrop-owl/issues/45)) | ThinkPad, `17cb:1103`, CachyOS (kernel 7.2.9) |
| Qualcomm WCN7850 (FastConnect 7800) | `ath12k_wifi7_pci` | ❌ Shared: receives nothing from the phone ([#40](https://github.com/t4t5/omdrop-owl/issues/40)); exclusive untested | `17cb:1107`, Fedora 44 (kernel 7.2.9) |
| Intel Dual Band Wireless-AC 7265 | `iwlwifi` | ❌ Exclusive: firmware asserts on every ad-hoc start ([#16](https://github.com/t4t5/omdrop-owl/issues/16)) | Zenbook UX305CA, `8086:095a`, Omarchy 4.0.4 (kernel 7.2.5) |
| Intel Wireless-AC 8260 | `iwlwifi` | ❌ Exclusive: firmware resets on channel 44; no transfer on 6 | NUC6i7KYK, `8086:24f3`, Home Assistant OS (kernel 6.12) |
| Intel 9260 | `iwlwifi` | ❓ Exclusive untested | — |
| Intel AX210 | `iwlwifi` | ❓ Exclusive untested | — |

<!-- hardware-table:end -->

One card name can cover chips that behave differently, so a report from a machine, PCI ID (`lspci -nn`) or kernel not listed here helps too.

### Testing your card

```sh
owly test-card
```

walks you through a photo each way with an iPhone (through Omdrop where it's installed, otherwise `owly send` and `owly receive`), watches Wi-Fi for five minutes after the radio is given back, and writes a JSON report. The report holds no router MAC. Open a pull request adding it to [`hardware/reports/`](hardware/README.md), or attach it to an issue.

A card with no entry in `drivers.toml` gets the `*` entry, which runs only with `allow_untested = true`. Cards that can inject frames from a monitor interface while connected to Wi-Fi are the likely candidates for shared mode. Most mac80211 drivers can, but how they behave around it varies, which is what `drivers.toml` records. To try other settings, copy the `*` entry to `/etc/owly/drivers.toml` under your driver's name (`basename $(readlink /sys/class/net/<wifi>/device/driver)`) and change it there.

## Modes

**Shared.** Wi-Fi stays connected, and AWDL shares the AP's channel, which has to be **6, 44 or 149**. Owly injects frames through a monitor interface with the station's MAC, and advertises only the phone's slots on that channel. The station ACKs the phone's frames.

**Exclusive.** For cards that can't inject while connected, such as every Intel card tested. Wi-Fi is down while the radio is up, and comes back when it's given back. An ad-hoc cell on AWDL's BSSID takes the station's place to ACK the phone's frames, or a hidden AP (hostapd) on cards with no ad-hoc mode. It tries channels 44, 149 and 6 in that order, of those the card may transmit on, since an iPhone gives AWDL more slots on 5 GHz. Intel cards go straight to 6, since none has held a 5 GHz cell.

**Active monitor.** For cards with neither, such as the MediaTek MT7612U and MT7921: an active monitor carrying the station's MAC ACKs in place of the cell.

A USB adapter can run exclusive mode as a second card while the built-in one stays connected: set `station` to the adapter in `/etc/owly/radio.toml` and disconnect it (`nmcli device disconnect <adapter>`).

## Known issues

`owly doctor` lists what's known to go wrong on your card. Beyond that:

- **Shared mode needs your AP on channel 6, 44 or 149.** On any other channel, the radio won't come up, and `owly doctor` says so.
- **The firewall can block the receiver**, so the phone never sees this computer. Allow TCP 8771 on `awdl0` (Omdrop: `omdrop firewall install`).
- **Exclusive mode is slow on channel 6**, where Intel cards and others that refuse a 5 GHz cell run, since AWDL there gets few of the iPhone's slots: a 2.4 MB photo takes 10–17 s on an AX201.

## How it works

`owlyd` is a system service, started by D-Bus on the first call, that owns the Wi-Fi radio. It sets the card up for the mode `drivers.toml` and the probe choose, runs AWDL on it, and lends the resulting `awdl0` out as a lease over D-Bus (`io.github.t4t5.Owly.Radio1`), which polkit lets the user at the seat take. When the lease ends, or the client holding it exits or crashes, owlyd gives Wi-Fi back, and a journal lets it do the same after a crash of its own. It holds `CAP_NET_ADMIN` and `CAP_NET_RAW` only and parses nothing above AWDL. AirDrop itself (Omdrop's receiver, or `owly receive`) runs as the user, over ordinary IPv6 sockets on `awdl0`.

AWDL comes from `owly-awdl`, Owly's own implementation. It replaces [jedbillyb's OWL fork](https://github.com/jedbillyb/owl) and the [patches](https://github.com/t4t5/omdrop-owl/tree/v0.3.0/patches) `omdrop-owl` 0.3.0 and older carried, and does what they were made to do.

The protocol layers are Rust crates other projects can build on: `owly-awdl` (sans-IO, `no_std` AWDL), `owly-airdrop` (AirDrop, with a sans-IO core), both with a driver on tokio behind a feature, and `owly-radio` (the radio on Linux, behind its backend trait). [docs/SPEC.md](docs/SPEC.md) says what they implement on the wire.

A new kind of radio plugs in at one of two places. One that only has to carry 802.11 frames, such as an ESP32 over USB, implements the `Air` trait of `owly-awdl`'s driver (its `driver` feature) and gets AWDL from it. One that comes up some other way, or does AWDL itself, implements `owly-radio`'s `Backend` trait, which owlyd serves.

## Development

```sh
just check     # formatting and lints, before each commit
just test      # tests
just ci        # everything CI checks: lints, tests, portability, spec coverage, vectors, the hardware table
just hwsim     # each radio mode on mac80211_hwsim (root)
just           # everything else
```

To try it with a phone, run `just radio` in one terminal (it installs owlyd first when it's missing or out of date), then `just receive` or `just send` in another. With no arguments, `just send` sends a test photo, `tests/photo.png`.

The AirDrop tests run sender and receiver in a user network namespace (`unshare -rn`), so they need no root, but do need unprivileged user namespaces. `just install` installs a development build the way the packages lay it out, and `just uninstall` removes it. `packaging/arch/*/PKGBUILD` build packages from the checkout (`makepkg -si` there), and `packaging/package.sh deb` or `rpm` builds owly's `.deb` or `.rpm`.

## License

Owly is MIT. The `omdrop-owl` package also carries [omdrop-awdl's tools](crates/omdrop-owl/omdrop-awdl/README.md), unmodified, which are GPL-2.0-only.
