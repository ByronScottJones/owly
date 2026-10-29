#!/bin/bash
# The real owly-gui, end to end, with no radio: in a network namespace of its
# own, the window runs on a veth pair's `awdl0` against a fake owlyd (on a
# private bus), and a CLI "phone" on `peer0` sends to it and receives from it.
# xdotool clicks through the window under Xvfb; the checks are on what the
# phone and the receive folder see. Screenshots of each step land in
# $OUT/shots.
#
#   just gui-e2e
#
# Needs sudo (for `unshare --net` and the veth), xvfb, xdotool, python3-gi,
# gir1.2-gtk-3.0, iproute2 and dbus-daemon, and debug builds of owlyd, owly
# and owly-gui (with tauri/custom-protocol, so it loads its bundled UI: an
# OWLY_BUS override works only in debug builds). The coordinates below are
# for the 960x600 window at the top left of a 1100x700 display.
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT=${OUT:-/tmp/owly-gui-e2e}
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}/debug

if [ "${1:-}" != --inner ]; then
  # Everything below runs as root in a fresh network namespace, killing what
  # it started with the namespace's last process; the programs under test
  # drop to this user.
  exec sudo unshare --net --mount-proc --fork --kill-child -- \
    env RUN_AS="$USER" ROOT="$ROOT" OUT="$OUT" TARGET="$TARGET" bash "$0" --inner
fi

as() { sudo -u "$RUN_AS" env "$@"; }
owly() { as "$TARGET/owly" "$@"; }
shot() { python3 "$ROOT/tests/gui-e2e/grab.py" "$OUT/shots/$1.png"; }
click() { xdotool mousemove "$1" "$2" click 1; }
failures=0
check() { # check DESCRIPTION COMMAND...
  local what=$1; shift
  if "$@" > /dev/null 2>&1; then echo "ok   $what"; else echo "FAIL $what"; failures=$((failures + 1)); fi
}

rm -rf "$OUT"; mkdir -p "$OUT"/{recv,phone-recv,shots,radio/owly-fake}
echo 86:08:04:8a:2b:f4 > "$OUT/radio/owly-fake/peers"
chown -R "$RUN_AS" "$OUT"

# awdl0 is this computer, peer0 the phone.
ip link set lo up
ip link add awdl0 type veth peer name peer0
for n in awdl0 peer0; do echo 0 > "/proc/sys/net/ipv6/conf/$n/accept_dad"; ip link set "$n" up; done
sleep 1

as dbus-daemon --session --nofork --nopidfile --address="unix:path=$OUT/bus" > /dev/null 2>&1 &
sleep 1
as "$TARGET/owlyd" --fake "$OUT/radio" --bus "unix:path=$OUT/bus" > "$OUT/owlyd.log" 2>&1 &
export DISPLAY=:77
Xvfb :77 -screen 0 1100x700x24 > /dev/null 2>&1 &
sleep 2
# The window has a session bus of its own, for its single-instance guard.
as DISPLAY=:77 WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_BACKEND=x11 \
  OWLY_BUS="unix:path=$OUT/bus" OWLY_IFACE=awdl0 OWLY_RECEIVE_DIR="$OUT/recv" \
  dbus-run-session -- "$TARGET/owly-gui" > "$OUT/gui.log" 2>&1 &
sleep 7
shot 01-off

# Turn AirDrop on, which leases the radio from the fake owlyd, then choose Everyone.
click 580 316; sleep 4; shot 02-on
click 557 521; sleep 1; click 476 556; sleep 3; shot 03-everyone

# The phone sends a file; the prompt is accepted by a click.
echo "hello from a phone" > "$OUT/hello.txt"; chmod 644 "$OUT/hello.txt"
owly send --iface peer0 --name "Test Phone" --to "$(hostname)" --no-ble --seconds 20 "$OUT/hello.txt" > "$OUT/phone-send.log" 2>&1 &
sleep 7; shot 04-file-prompt; click 657 361; sleep 4; shot 05-file-received
check "the file arrives in the receive folder" grep -qx "hello from a phone" "$OUT/recv/hello.txt"
check "the phone is told it was delivered" grep -qx delivered "$OUT/phone-send.log"

# The same for a link, which is shown and never opened (SEC-4).
owly send --iface peer0 --name "Test Phone" --to "$(hostname)" --no-ble --seconds 20 --url https://example.org/owl > "$OUT/phone-url.log" 2>&1 &
sleep 7; shot 06-link-prompt; click 657 361; sleep 4; shot 07-link-received
check "the phone is told the link was delivered" grep -qx delivered "$OUT/phone-url.log"

# The phone now receives; the window lists it (a scan takes up to ~10 s), and
# sends it a link from the Send dialog.
owly receive --iface peer0 --name "Test Phone" --dir "$OUT/phone-recv" --accept > "$OUT/phone-recv.log" 2>&1 &
sleep 26; shot 08-people
click 298 210; sleep 2; shot 09-send-dialog
click 348 250; sleep 1
click 480 331; sleep 1; xdotool type --delay 40 "https://example.org/from-gui"; sleep 1; shot 10-link-typed
click 655 378; sleep 6; shot 11-sent
check "the phone receives the window's link" grep -q "https://example.org/from-gui" "$OUT/phone-recv.log"

# A file chosen with the system file chooser (OWLY-5: the backend opens the
# chooser and keeps the paths; the page sees only names). Ctrl+L types a path.
echo "chosen in the picker" > "$OUT/chosen.txt"; chmod 644 "$OUT/chosen.txt"
click 298 210; sleep 2
click 321 289; sleep 3; shot 12-picker
xdotool key ctrl+l; sleep 1; xdotool type --delay 30 "$OUT/chosen.txt"; sleep 1; xdotool key Return; sleep 2; shot 13-picked
click 655 387; sleep 6; shot 14-picked-sent
check "a file chosen in the picker reaches the phone" grep -qx "chosen in the picker" "$OUT/phone-recv/chosen.txt"

echo "screenshots: $OUT/shots"
[ "$failures" -eq 0 ] && echo "all checks passed" || { echo "$failures failed"; exit 1; }
