# yuyudhan-1 — RMK firmware

RMK (Rust) firmware for a **Corne split keyboard** (42-key hardware, 36-key layout) on
two **nice!nano v2** controllers (nRF52840), wireless over BLE. This is a port of the
`yuyudhan-1` ZMK keymap to [RMK](https://github.com/HaoboGu/rmk), with the entire keymap
expressed in a single [`config/keyboard.toml`](config/keyboard.toml).

- **Left half = central** — USB/BLE to the host, holds the keymap, runs Vial.
- **Right half = peripheral** — BLE to the central only.

One profile, one keymap. Everything lives in `config/keyboard.toml`; there is no second variant.

## Hardware

- **MCU:** nice!nano v2 (nRF52840), ×2
- **Shield:** Corne (split, 6×3 + 3 thumb per half)
- **Display:** SSD1306 128×32 OLED on the central (I²C, P0.17 SDA / P0.20 SCL)
- **Battery:** reported over BLE via the nRF internal VDDH divider
- **Bluetooth:** 4 profiles
- **Live remap:** Vial

## Layers

7-layer Miryoku-style layout. Layers activate on a thumb **hold**; tap sends the key.

| Layer | Thumb (hold) | Contents |
|-------|--------------|----------|
| 0 BASE | — | QWERTY + GACS home-row mods (A/S/D/F = GUI/Alt/Ctrl/Shift, mirrored J/K/L/') |
| 1 NAV | Space | vim arrows (HJKL), Home/End/PgUp/PgDn, clipboard (⌘Z/X/C/V, ⌘⇧Z) |
| 2 NUM | Backspace | columnar numpad + brackets/symbols on the left hand |
| 3 MEDIA | Escape | volume, brightness, media transport, Bluetooth (BT0–3 / clear / USB-BLE toggle), lock/logout |
| 4 SYM | Enter | programmer symbols on the left hand |
| 5 FUN | Delete | F1–F12 + PrintScreen/ScrollLock/Pause |
| 6 MOUSE | Tab | pointer move, scroll wheel, mouse buttons |

**Home-row mods** use the `HRM` morse profile (permissive-hold + unilateral-tap + flow-tap,
200 ms hold, 120 ms prior-idle). **Thumb layer-taps** use the `TL` profile (permissive-hold,
no flow-tap, 200 ms).
Edit these in the `[behavior.morse.profiles]` table of `config/keyboard.toml` — every
setting in that file carries a comment explaining what it does and how it changes
real-world typing feel, so the TOML itself is the reference documentation.

### MEDIA-layer Bluetooth keys

RMK exposes BLE control as `User` keycodes (4 profiles configured via `ble_profiles_num = 4`):

| Key in keymap | Action | Vial label |
|---------------|--------|------------|
| `User0`–`User3` | select BT profile 0–3 | BT0–BT3 |
| `User6` | clear bond for current profile | Clear BT |
| `User7` | toggle output USB ↔ BLE | Switch Output |

## Build

Requires [`just`](https://github.com/casey/just), a Rust toolchain, and (one-time) the
embedded tools. `just setup` installs everything it needs.

```sh
just                  # list recipes
just setup            # one-time toolchain: target, llvm-tools, flip-link, cargo-make, cargo-binutils, cargo-hex-to-uf2
just build both       # build BOTH halves -> rmk-central.uf2 + rmk-peripheral.uf2
just build left       # build only the LEFT half (central) UF2
just build right      # build only the RIGHT half (peripheral) UF2
just check both       # fast compile-only sanity check (left | right | both)
just docs svg         # render config/keyboard.toml -> yuyudhan-1_keymap.svg
just docs html        # render config/keyboard.toml -> yuyudhan-1-viewer.html
just expand left      # macro-expand the keymap to confirm it compiled in (needs cargo-expand)
just clean            # wipe Rust cache + build/ staging (keeps local firmware/ archive)
```

`just build <target>` runs the `cargo make` chain (compile → objcopy → hex → uf2), which writes all
intermediates into a gitignored `build/` staging dir — never the repo root. It then moves the
finished `.uf2`(s) into a local `firmware/<datetime>/` directory (gitignored), alongside a snapshot
of the config and keymap visuals. `just flash` pulls the newest matching `.uf2` from there
automatically; `just clean` wipes `build/` and the Rust cache but keeps the `firmware/` archive.

## Flash

The nice!nano's Adafruit bootloader accepts drag-and-drop `.uf2` — no probe needed.

1. Double-tap the reset button on a half → it mounts as `/Volumes/NICENANO`.
2. Flash the matching half:
   ```sh
   just flash left       # left half  (rmk-central.uf2)
   just flash right      # right half (rmk-peripheral.uf2)
   ```
   (Or drag the `.uf2` onto the `NICENANO` drive manually.)
3. Repeat for the other half. The board reboots and unmounts automatically.

**Remove the USB cable after flashing** — RMK forces USB output mode while cabled.

Reflash the central after any keymap change. The peripheral only needs reflashing when
firmware (not just the keymap) changes.

## Editing the keymap

- **Static:** edit the `[[layer]]` `keys` blocks in `config/keyboard.toml`, then `just build both` and reflash the central.
- **Live:** open [vial.rocks](https://vial.rocks) over USB, hold the two left thumb keys, click **Unlock**, and remap in the browser.
