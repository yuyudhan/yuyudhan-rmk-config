# Bluetooth profiles — model, key combos, recovery

How the yuyudhan-1 handles BLE host profiles, what the OLED tells you, and the
exact procedures when a computer will not connect. Firmware: RMK branch
`yuyudhan/per-profile-address` (main `6c89c4b` + one local patch, upstream
issue #901), pinned by rev in `Cargo.toml` from the local clone `../rmk`.
It builds only on this Linux machine until that branch is pushed to a fork.

Any change to the RMK rev or its features changes RMK's storage schema hash,
so the first boot erases all storage on both halves (every bond and Vial
keymap edit). Every computer must then forget the keyboard and pair again.

## Profile model

- **4 profiles** (`ble_profiles_num = 4` in `config/keyboard.toml`), P0–P3.
  A profile stores one host bond (encryption keys) in flash on the left half.
- **One address per profile.** Every profile advertises "yuyudhan-1" from its
  own static random address (`ext_adv`). P0 keeps the original address; P1–P3
  use the same address with byte 3 raised by the profile number. A computer
  only sees the keyboard while its own profile is active, so a second bonded
  computer nearby can no longer take the connection (#901). A connection that
  still lands on the previous profile's address is dropped.
- **What a switch does:** drop the current host, advertise from the new
  profile's address every 30 ms for 30 s (then every 200 ms), the target
  computer notices and connects, encryption from the stored bond, then the
  OLED marker clears. The same 30 s fast window runs after a wake.
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

Turn Bluetooth OFF on the other computer and retry. With per-profile
addresses another computer should not take the connection; if it now works,
report it (#901 regression). If it still fails alone,
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
- 2026-11-08: moved from the v0.9.0 tag to RMK main `6c89c4b` (battery
  settings moved to `[split.*.battery]` tables; no other config change).
- 2026-11-08: per-profile addresses (local RMK patch). Upgrading from v0.9.0
  erased all storage on both halves on first boot (storage schema hash
  mismatch), so every computer, P0 included, had to forget and re-pair, and
  Vial keymap edits had to be redone.
