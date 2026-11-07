# Bluetooth profiles — model, key combos, recovery

How the yuyudhan-1 handles BLE host profiles, what the OLED tells you, and the
exact procedures when a computer will not connect. Firmware: stock upstream
RMK, pinned to the `rmk-v0.9.0` tag in `Cargo.toml`. No local patches.

## Profile model

- **4 profiles** (`ble_profiles_num = 4` in `config/keyboard.toml`), P0–P3.
  A profile stores one host bond (encryption keys) in flash on the left half.
- **One BLE identity.** All profiles advertise the same device
  ("yuyudhan-1", one address). Hosts always initiate reconnection — the
  keyboard only advertises and waits.
- **Advertising is unfiltered (stock RMK).** Any bonded computer in range
  can connect while the keyboard advertises. If it is not the active
  profile's computer, RMK drops it and advertises again. So a second bonded
  computer nearby can delay the switch or "steal" it (upstream issue
  [#901](https://github.com/rmk-rs/rmk/issues/901), still open). Workaround:
  turn Bluetooth off on the computer you are leaving, or keep it out of range.
- **What a switch does:** drop the current host, wait for the link to close,
  advertise every 200 ms, the target computer notices and connects,
  encryption from the stored bond, then the OLED marker clears. Steps 1–2 and
  the 200 ms interval are on the keyboard side and are fixed in RMK (no
  `keyboard.toml` setting); noticing the advert and connecting is the
  computer's Bluetooth stack. Typically 1–3 s.
- **USB wins when cabled.** With a data cable attached and the preferred
  output set to USB (the default), keys go over the cable even while a BLE
  profile is connected. Toggle with ESC+N.

## Key combos (MEDIA layer = hold left-thumb ESC)

| Combo       | Key     | Action                                        |
|-------------|---------|-----------------------------------------------|
| ESC + L     | `User0` | Switch to profile 0                           |
| ESC + '     | `User1` | Switch to profile 1                           |
| ESC + .     | `User2` | Switch to profile 2                           |
| ESC + /     | `User3` | Switch to profile 3                           |
| ESC + O     | `User6` | Clear ACTIVE profile's bond → pairing mode    |
| ESC + N     | `User7` | Toggle output USB ↔ BLE (persisted)           |

Holding a profile key (ESC + L / ' / . / /) for 5 seconds forgets that
profile's bond and re-pairs it (RMK v0.9.0). Tap it; do not hold it.

## OLED indicators (left half)

| Display  | Meaning                                                  |
|----------|----------------------------------------------------------|
| `P0`     | Connected + encrypted on profile 0                       |
| `~ P0`   | Waiting for profile 0's computer to connect              |

## Recovery procedures

### Output goes nowhere while connected

Unplug the USB cable, or press ESC+N — the preferred output defaults to USB
whenever a cable is attached.

### A computer will not auto-connect on its profile

Stale bonds must be cleared on BOTH sides; clearing one side leaves a
mismatch that fails silently.

1. On that computer: Bluetooth settings → remove/forget EVERY
   "yuyudhan-1" entry.
2. On the keyboard: switch to the profile (e.g. ESC+L for P0), then
   ESC+O to clear its bond. OLED shows `~ P#` (open pairing mode).
3. On the computer: scan and pair "yuyudhan-1". The `~` marker clears.
4. Verify the OTHER profile still reconnects (e.g. ESC+' ≤5 s).

### Still failing after a clean re-pair

Turn Bluetooth OFF on the other computer and retry. If it now works, the
other computer was taking the connection (#901). If it still fails alone,
that computer's BT stack is stuck — macOS: `sudo pkill bluetoothd`;
Windows: disable/enable the Bluetooth adapter; Linux:
`bluetoothctl remove <addr>` + `systemctl restart bluetooth` — then re-pair.

## History

- 2026-07-02: P0's computer never auto-connected (OLED stuck `~ P0`) while
  P1 reconnected in 1–2 s — stock RMK advertises unfiltered, so a co-located
  bonded host can steal the connection meant for the selected profile.
  Reported upstream as #901; a local whitelist patch was used for a while and
  then dropped (2026-07-24). Upstream fix work: PR #1131 (open) and the
  maintainer's multi-connection PR #1319 (open).
- 2026-11-07: moved to RMK v0.9.0. Its HID report IDs changed, so every
  computer had to forget and re-pair the keyboard once.
