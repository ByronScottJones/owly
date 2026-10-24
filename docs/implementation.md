# Implementation notes

What Owly does where [the spec](SPEC.md) leaves a choice, and what it does to work with Apple devices from radios and an OS that AirDrop wasn't built for. None of it is part of the protocol: another implementation can choose differently and still conform.

Requirements here have IDs with the prefix `OWLY` and follow the spec's conventions (§0.1). Tests cite them the same way, and `cargo xtask coverage` reads them along with the spec's.

## Choices

- **Receiver port (MDNS-1):** Owly's receiver listens on TCP `8771`, the port omdrop's firewall rule opens on `awdl0`, and publishes it in SRV. The sender's probe under DISC-5 goes to `8770`, the port Apple devices listen on.
- **TXT flags (MDNS-5):** `0x089`. DVZip isn't advertised, so Apple senders upload gzip.
- **Decline (ASK-2):** a declined or timed-out `/Ask` gets `403`, then the connection is closed.
- **AWDL values:** Owly announces AWDL version 3.4; HT capability info `0x11ce`, A-MPDU parameters `0x1b` and RX MCS mask `0xff` (one stream); and Data Path State flags `0x8f24`, with country code `X0\0`, its master channel in the social channel map, its AWDL address and 2 bytes of extended flags.
- **A-MSDU (AWDL-14):** at most 16 subframes are unpacked from a frame, and a frame with more is dropped.
- **Identity (§7):** `owly-airdrop` doesn't implement Contacts Only. On Omarchy, omdrop-core does, with an identity extracted from the user's Mac by `seemoo-lab/airdrop-keychain-extractor`, since Linux has no way to get one of its own. `omdrop-plugin/docs/identity-contract.md` says how it's stored and presented.

## Radio placement

A Linux Wi-Fi card can't hop channels the way an Apple device does, so `owly-radio` runs AWDL in one of two placements:

- **Shared:** the radio stays on the access point's channel and advertises only the slots where a peer's sequence names that channel (AWDL-12). Multicast goes in AWDL's multicast slots, where the peer is on that channel.
- **Fixed:** the radio stays on one channel and advertises it in every slot. A peer visits it for a slot or two in sixteen, so multicast goes in any slot, with the 3 TU guard (AWDL-16).

Accepting Block Ack (AWDL-15) only helps where the hardware answers A-MPDUs itself: `rx_ampdu` in `crates/owly-radio/drivers.toml` says which cards do.

## Workarounds

**OWLY-1** (receiver, SHOULD) Owly's radio doesn't carry the receiver's records in the Service Response TLV, so the receiver announces in bursts (MDNS-7). The burst depends on the placement, so the radio layer chooses it, and it SHOULD be:

| Placement | Copies | Apart | Pause between bursts |
|---|---|---|---|
| Shared | 3 | 50 ms | 3 s |
| Fixed | 64 | 16.4 ms (16 TU, one AW, for a whole 16-slot period) | 3 s |

**OWLY-2** (SHOULD) A node SHOULD send each valid peer an ICMPv6 echo request, link-local to link-local, every 3 s, which makes Apple peers learn its neighbor entry. Its replies need not reach the IP stack.

- **Finding a receiving iPhone (DISC-5):** an iPhone's mDNS doesn't reach a Linux node's `awdl0`, so Owly's sender also tries `/Discover` at each AWDL peer while it advertises over BLE, and keeps trying until the phone wakes.
- **Early SYNs (AWDL-18):** `owly-awdl` hands packets to the kernel's IP stack, whose SYN-ACK would wait on neighbor discovery, which AWDL doesn't carry. So it holds the SYN until the peer's neighbor entry exists.
- **MAC address (AWDL-2):** if udev or NetworkManager rewrites `awdl0`'s MAC address, peers connect to an address the interface doesn't have. `owly-radio` pins it for the session.
- **Firewall (NET-4):** host firewalls must let TCP `8771` in from `fe80::/10` on `awdl0`. `owly-radio`'s probe warns when ufw is on, `omdrop firewall install` adds the rule, and the NixOS module opens the port.

## The window

`owly-gui` is a window over the same session calls as `owly receive`, `send` and `browse`, in a webview that is treated as untrusted: a defect in the page, or a script that gets into it, must not be able to read files, send anything to anyone, or reach beyond the window's own commands.

**OWLY-3** (window, MUST) The page loads only the files bundled with the app. Its content security policy MUST start from `default-src 'none'` and allow no inline or `eval` script and no remote or `data:` source, and the window MUST refuse to navigate to any other page.

**OWLY-4** (window, MUST) The page's permissions are listening for events and calling the app's own commands, each one allowed by name. It MUST have no dialog, file-system, shell, window or webview permission, and every command MUST need a permission of its own.

**OWLY-5** (window, MUST) The page never names a path or an address. It sends a file by the id of a selection the backend holds (made by the native picker or a drop on the window; at most 8, the oldest going first), and a person by the id of the last scan. The backend MUST resolve a selection's paths to existing files or folders in canonical form, MUST require the receive folder to be absolute, and MUST accept only `http` and `https` links of at most 2048 characters and interface names of the usual form.

**OWLY-6** (window, MUST) Text from the other device is shown and never acted on (SEC-4). Before it goes to the page, control characters MUST be replaced, bidirectional-override and zero-width characters dropped, and its length bounded; a request lists at most 100 of its files or links, with a count of the rest.

- **Who may see this computer:** Everyone and Receiving Off. Contacts Only is listed but can't be chosen until the computer has an Apple identity (above), which Owly can't make.
- **Pending requests:** at most 32 wait for an answer at once; a request beyond that is declined unseen. A request nobody answers is declined by the receiver after 15 s (ASK-2).
- **One window:** a second launch raises the window that's running instead of starting another, which would fight it for the radio and port 8771.
- **Leaving:** closing the window ends its lease on the radio and restores Wi-Fi as the user, as `owly radio up` does on Ctrl-C.
- **Announcing and scanning:** the BLE advert (BLE-1) runs on a task of its own, since a computer with no BlueZ takes 25 s to time out activating it, and neither the first scan nor the receiver's mDNS announcements may wait for that.
- **Tests:** `just gui-check` and `just gui-a11y` (the interface and its accessibility under WebKitGTK), `owly-session`'s `tests/session.rs` (the calls the window makes, against a fake radio on a veth pair) and `just gui-e2e` (the real window, clicked through under Xvfb).

## Testing

- Fuzzing and robustness tools are never pointed at Apple devices we don't own. Current iOS and macOS `sharingd` hits a `fatalError` on an unknown HTTP path (HTTP-6), and has crashed on deeply nested XML plists and malformed HTTP framing. That takes down AirDrop, AirPlay, Handoff, Universal Clipboard and Continuity Camera on the target until `launchd` restarts it.
- [conformance.md](conformance.md) lists the conformance tests, and [open-questions.md](open-questions.md) what still needs a capture.
