# Bluetooth profiles — model, key combos, recovery

How the yuyudhan-1 handles BLE host profiles, what the OLED tells you, and the
exact procedures when a computer will not connect. Firmware: RMK b9820492
(vendored at `vendor/rmk/`, see `vendor/rmk/YUYUDHAN_PATCH.md`).

## Profile model

- **4 profiles** (`ble_profiles_num = 4` in `config/keyboard.toml`), P0–P3.
  A profile stores one host bond (encryption keys) in flash on the left half.
- **One BLE identity.** All profiles advertise the same device
  ("yuyudhan-1", one address). Hosts always initiate reconnection — the
  keyboard only advertises and waits.
- **Whitelist advertising (local patch).** A profile WITH a bond advertises
  filtered: only that profile's computer can see or connect to the keyboard.
  A profile WITHOUT a bond advertises open, so any computer can pair.
  Consequence: to move a profile to a new computer you MUST clear it first
  (ESC+O) — the keyboard is invisible to strangers while a bond exists.
- Switching profiles disconnects the current host, loads the target
  profile's keys, and re-advertises. The target computer reconnects on its
  own — typically 1–2 s.

## Key combos (MEDIA layer = hold left-thumb ESC)

| Combo       | Action                                        |
|-------------|-----------------------------------------------|
| ESC + M     | Switch to profile 0                           |
| ESC + Comma | Switch to profile 1                           |
| ESC + Dot   | Switch to profile 2                           |
| ESC + Slash | Switch to profile 3                           |
| ESC + O     | Clear ACTIVE profile's bond → pairing mode    |
| ESC + N     | Toggle output USB ↔ BLE (persisted)           |

## OLED indicators (left half, bottom-right)

| Display  | Meaning                                                  |
|----------|----------------------------------------------------------|
| `P0`     | Connected + encrypted on profile 0                       |
| `~ P0`   | Advertising / waiting for profile 0's host               |
| `USB P0` | Output routed to USB cable; profile 0 armed for BLE      |

## Recovery procedures

### Output goes nowhere while connected

Unplug the USB cable, or press ESC+N until the `USB` marker disappears —
preferred transport defaults to USB whenever a cable is attached.

### A computer will not auto-connect on its profile

Stale bonds must be cleared on BOTH sides; clearing one side leaves a
mismatch that fails silently.

1. On that computer: Bluetooth settings → remove/forget EVERY
   "yuyudhan-1" entry.
2. On the keyboard: switch to the profile (e.g. ESC+M for P0), then
   ESC+O to clear its bond. OLED shows `~ P#` (open pairing mode).
3. On the computer: scan and pair "yuyudhan-1". OLED flips to `P#`.
4. Verify the OTHER profile still reconnects (e.g. ESC+Comma ≤5 s).

### Still failing after a clean re-pair

Turn Bluetooth OFF on the other computer and retry. If it now works, the
whitelist patch is not active on the flashed firmware (reflash from a fresh
`just build both`). If it still fails alone, that computer's BT stack is
stuck — macOS: `sudo pkill bluetoothd`; Windows: disable/enable the
Bluetooth adapter; Linux: `bluetoothctl remove <addr>` +
`systemctl restart bluetooth` — then re-pair.

## History

- 2026-07-02: P0's computer never auto-connected (OLED stuck `~ P0`) while
  P1 reconnected in 1–2 s — stock RMK advertises unfiltered, so a co-located
  bonded host can steal the connection meant for the selected profile.
  Fixed by the whitelist-advertising vendor patch (ZMK model) + a clean
  both-sides re-pair of P0.
