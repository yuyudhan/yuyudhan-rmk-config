# yuyudhan-1 RMK Firmware — Design & Replication Document

This document captures the full design rationale, hardware mapping, behavioral decisions, and
step-by-step reproduction instructions for the **ZMK → RMK port** of the `yuyudhan-1` Corne
keyboard firmware. Read alongside `config/keyboard.toml` and `config/vial.json` as a living
reference for future maintainers.

---

## Overview

**yuyudhan-1** is a Miryoku-style QWERTY split keyboard built on:

- **Hardware:** Corne shield, 42-key physical layout (3 rows × 6 columns per half + 3 thumb keys
  per half), configured for a **36-key logical layout** (outer pinky columns 0 and 11 are
  physically present but left unmapped).
- **Controllers:** 2× nice!nano v2 (nRF52840), one per half.
- **Split link:** wireless BLE only — no TRRS cable.
- **Left half = central:** connects to the host over USB or BLE, holds the full keymap, runs Vial.
- **Right half = peripheral:** connects to the central over BLE; never talks to the host directly.
- **Firmware:** [RMK](https://github.com/HaoboGu/rmk) (Rust), a type-safe embedded framework
  where the entire keymap, pin map, and behavioral tuning live in a single TOML file:
  `config/keyboard.toml`.

The project is a complete port of the ZMK keymap from
`../yuyudhan-zmk-config/config/yuyudhan-1.keymap` (7 layers, home-row mods, thumb layer-taps,
media/Bluetooth control, mouse, symbols, function keys). No second variant or profile exists —
one keymap, one build.

---

## Why RMK / Why This Pinned Revision

**Why RMK:** RMK is a full Rust firmware stack. It exposes every behavioral detail (tap-hold
profiles, BLE profile count, matrix type, display renderer) as structured TOML and Rust trait
implementations, giving complete, type-checked control over the keyboard's behavior without
patching a C/Zephyr tree.

**Why the pinned revision `b98204928e9a8532064ce99add1f6c4c554e08c9`:** This git rev (merged
2026-06-29, PR #885) introduces *per-profile* `enable_flow_tap` override in the morse behavior
system. This is a hard requirement for this keymap:

- The **HRM** morse profile (home-row mods) needs `enable_flow_tap = true` so that a prior idle
  gate of 150 ms suppresses phantom modifiers during fast rolls.
- The **TL** morse profile (thumb layer-taps) needs `enable_flow_tap = false` because thumbs are
  off the home row and the idle gate would cause the thumb tap to misfire after the natural pause
  between words.

Before this commit, `enable_flow_tap` was a single global flag. The two profiles could not coexist
with different settings. No earlier release or published crate version supports this.

**`rmkit init` / published 0.8.x crate are deliberately NOT used.** The published crate on
crates.io/docs.rs was failing at build time at the time of porting, and the per-profile
`enable_flow_tap` feature was not yet published. The `rmk` dependency is pinned directly to git:

```toml
rmk = { git = "https://github.com/HaoboGu/rmk", rev = "b98204928e9a8532064ce99add1f6c4c554e08c9", features = [
    "nrf52840_ble",
    "split",
    "async_matrix",
    "adafruit_bl",
    "ssd1306",
] }
```

---

## Hardware & Pin Map

### Matrix topology

The Corne uses a `col2row` diode direction (same as the ZMK shield default `diode-direction =
"col2row"`). RMK's default `matrix_type = "normal"` corresponds to col2row scanning — no override
needed; do NOT set `row2col`.

The logical matrix is unified: 4 rows × 12 columns. Left half occupies cols 0–5 (`col_offset = 0`),
right half cols 6–11 (`col_offset = 6`).

### Central (left half) — `[split.central.matrix]`

```toml
matrix_type = "normal"
row_pins = ["P0_22", "P0_24", "P1_00", "P0_11"]
col_pins = ["P0_31", "P0_29", "P0_02", "P1_15", "P1_13", "P1_11"]
```

Corresponds to nice!nano pro_micro header pins 21, 20, 19, 18, 15, 14 on the columns (in that
order), which is the ZMK `corne` shield mapping for the left overlay.

### Peripheral (right half) — `[split.peripheral.matrix]`

```toml
matrix_type = "normal"
row_pins = ["P0_22", "P0_24", "P1_00", "P0_11"]
col_pins = ["P1_11", "P1_13", "P1_15", "P0_02", "P0_29", "P0_31"]
```

**The right-half column order is the exact reverse of the left.** This is not a mistake: the Corne
PCB is symmetrical — the right half is physically the same board as the left, flipped 180°. The
column traces therefore run in the opposite direction when viewed from the "front". The ZMK
`corne_right.overlay` has the same reversal (pin 14 → pin 21, left to right). The `col_offset = 6`
shifts these 6 columns into the right half of the 12-column logical address space.

### BLE addressing

Each half has a fixed BLE address so the central can reliably reconnect to the peripheral:

```toml
# central
ble_addr = [0x18, 0xe2, 0x21, 0x80, 0xc0, 0xc7]

# peripheral
ble_addr = [0x7e, 0xfe, 0x73, 0x9e, 0x66, 0xe3]
```

These are arbitrary-but-fixed values. Change them only if another RMK Corne is in the same room
(they must be globally unique per half pair).

---

## The 36-Key Structure

The Corne hardware has 42 switch positions (3 rows × 6 columns per half, plus 3 thumbs per half =
4 rows × 6 columns in the matrix). The outer pinky column (col 0 on the left, col 11 on the right)
is physically populated but is not used in the `yuyudhan-1` layout — matching the ZMK
`default_transform` which selects exactly 36 positions.

RMK expresses this via `matrix_map` inside `[layout]`. Only these positions are wired to logical
key slots:

```toml
[layout]
rows = 4
cols = 12
layers = 7
matrix_map = """
(0,1,L) (0,2,L) (0,3,L) (0,4,L) (0,5,L)   (0,6,R) (0,7,R) (0,8,R) (0,9,R) (0,10,R)
(1,1,L) (1,2,L) (1,3,L) (1,4,L) (1,5,L)   (1,6,R) (1,7,R) (1,8,R) (1,9,R) (1,10,R)
(2,1,L) (2,2,L) (2,3,L) (2,4,L) (2,5,L)   (2,6,R) (2,7,R) (2,8,R) (2,9,R) (2,10,R)
                (3,3,L) (3,4,L) (3,5,L)   (3,6,R) (3,7,R) (3,8,R)
"""
```

Reading the map:

- **Main rows (rows 0–2):** cols 1–5 on the left, cols 6–10 on the right. Col 0 (far-left pinky
  outer) and col 11 (far-right pinky outer) are absent → not mapped → physically inert.
- **Thumb row (row 3):** cols 3–5 on the left, cols 6–8 on the right. The 6 thumb keys across
  both halves account for the remaining 6 positions, giving 30 main + 6 thumb = **36 total**.
- **`(row,col,hand)` tags:** the `L`/`R` hand annotations are present to enable the optional
  `unilateral_tap` HRM fallback (see Tap-hold section); they do not affect normal operation.

---

## Layers

All 7 layers are defined as `[[layer]]` blocks in `config/keyboard.toml`. Layers are activated by
holding the indicated thumb key; tapping that key sends the label key instead.

| Index | Name  | Thumb hold key | Purpose |
|-------|-------|----------------|---------|
| 0 | BASE  | — (always active)  | QWERTY + GACS home-row mods (A/S/D/F = GUI/Alt/Ctrl/Shift, mirrored J/K/L/') |
| 1 | NAV   | Space (left-center) | Vim arrows (HJKL), Home/End/PgUp/PgDn, clipboard (⌘Z/X/C/V, ⌘⇧Z redo), Caps |
| 2 | NUM   | Backspace (right-center) | Columnar numpad [ 7 8 9 ] / ; 4 5 6 = / ` 1 2 3 \\ on the left hand; . 0 - on thumb |
| 3 | MEDIA | Escape (left-outer) | Volume, brightness, media transport, Bluetooth control, lock/logout |
| 4 | SYM   | Enter (right-outer) | Programmer symbols { & * ( } / : $ % ^ + / ~ ! @ # | on the left hand |
| 5 | FUN   | Delete (right-inner) | F1–F12 in numpad layout + PrintScreen / ScrollLock / Pause |
| 6 | MOUSE | Tab (left-inner) | Pointer movement (HJKL positions), scroll wheel, mouse buttons on the right thumb |

---

## Tap-Hold Behavior Mapping

### From ZMK to RMK

ZMK uses two distinct behaviors for the two categories of hold keys on this keyboard:

| ZMK behavior | Flavor | Purpose |
|---|---|---|
| `&mt` (mod-tap) | `balanced` | Home-row mods: A S D F J K L ' |
| `&lt` (layer-tap) | `balanced` | Thumb cluster: Esc/Space/Tab/Enter/Bspc/Del |

RMK's equivalent is the `morse` system with named profiles, assigned as the third argument to
`MT(key, mod, <profile>)` and `LT(layer, key, <profile>)`.

### HRM profile — home-row mods

```toml
HRM = { enable_flow_tap = false, permissive_hold = true, unilateral_tap = true, hold_timeout = "200ms", gap_timeout = "200ms" }
```

- `permissive_hold = true` → balanced/permissive-hold mode: the modifier fires as soon as
  another key is **pressed and released** while the HRM key is held, with no need to wait out
  `hold_timeout`. This makes intentional mod combos (Ctrl+L, Shift+A) fast at any typing speed.
- `enable_flow_tap = false` → the global `prior_idle_time = "150ms"` gate is NOT applied to
  HRM keys. A modifier can fire even mid-streak (previous keypress < 150 ms ago). Without this,
  at 100 wpm (inter-key ~120 ms) an HRM key could never become a modifier during normal typing.
- `unilateral_tap = true` → any HRM key adjacent to a **same-hand** key resolves as a tap,
  regardless of release order (FIFO or LIFO). This is the only same-hand-roll protection under
  `permissive_hold` (the press-time guard in `MorseMode::Normal` does not apply here).
  Consequence: same-hand mod combos require the **opposite-hand modifier** — Ctrl+C = K+C
  (right Ctrl), Ctrl+O = D+O (left Ctrl). Cross-hand combos (Ctrl+L, Shift+A) are unaffected.
- `hold_timeout = "200ms"` → governs a *pure* hold with no other key pressed (e.g. holding
  Shift alone to extend a selection); does not delay nested-key combos under permissive_hold.
- `gap_timeout = "200ms"` → RMK-specific inter-event window, set equal to hold_timeout for
  consistency.

### TL profile — thumb layer-taps

```toml
TL  = { enable_flow_tap = false, permissive_hold = true, hold_timeout = "200ms", gap_timeout = "200ms" }
```

- `permissive_hold = true` → matches ZMK `balanced` flavor: if another key is pressed **and
  released** inside the hold window, the tap-hold resolves as a hold (layer activation). This is
  the primary usage pattern for thumb layer access — "thumb down → finger presses and releases key
  → thumb up" reliably activates the layer at any typing speed.
- `enable_flow_tap = false` → thumbs are off the home row. The prior-idle gate must be disabled
  for thumbs, or the natural typing pause between words would cause the thumb tap (Space, Enter,
  etc.) to misfire as a hold. This is the feature that required pinning to rev
  `b98204928e9a8532064ce99add1f6c4c554e08c9` — per-profile override was not available before.
- `hold_timeout = "200ms"` → same 200 ms window as HRM for a consistent feel.

### Implementation detail: no morse pool consumed

`MT` and `LT` both expand to `KeyAction::TapHold` inside the RMK proc-macro (see
`action_parser.rs` L383–424 at the pinned rev). They do NOT use `KeyAction::Morse` (the tap-dance
pool). There are no `TD` or `Morse(n)` keys anywhere in `config/keyboard.toml`. Therefore
`morse_max_num` does not need to be set or increased; the default is sufficient.

---

## Multiple Bluetooth Profiles

### Configuration

```toml
[rmk]
ble_profiles_num = 4
```

This single setting is load-bearing. **It MUST be 4.** Here is why.

### How RMK maps User keycodes to BLE actions

RMK exposes BLE control as numbered `User` keycodes. With `N` BLE profiles configured, the
assignments are:

| Keycode | N=3 (default) | N=4 (this config) |
|---------|---------------|-------------------|
| `User0` | BT profile 0 | BT profile 0 |
| `User1` | BT profile 1 | BT profile 1 |
| `User2` | BT profile 2 | BT profile 2 |
| `User3` | — (undefined) | BT profile 3 |
| `User(N+2)` | `User5` = clear bond | `User6` = clear bond |
| `User(N+3)` | `User6` = USB/BLE toggle | `User7` = USB/BLE toggle |

With the default `ble_profiles_num = 3` (N=3):
- Clear-bond would be `User5`, toggle would be `User6`.
- The MEDIA layer in `config/keyboard.toml` places `User6` and `User7` at those positions.
- Those keys would silently do nothing (or wrong things) if N=3.

**With `ble_profiles_num = 4` (N=4, this config):**
- `User0`–`User3` select BT profiles 0–3.
- `User6` = `User(4+2)` = clear the bond for the currently active profile.
- `User7` = `User(4+3)` = toggle the HID output between USB and BLE.

### MEDIA-layer control surface

The relevant rows from the MEDIA layer (`config/keyboard.toml`):

```
# Top row (right hand):
WM(Q, LCtrl | LGui)  BrightnessDown  BrightnessUp  User6  WM(Q, LShift | LGui)
#                                                   ^^^^^ clear bond

# Bottom row (right hand):
User7   User0   User1   User2   User3
# ^^^^                                toggle USB/BLE
```

`User6` (clear bond) sits at the `BT_CLR` position from ZMK (top row, second from right). `User7`
(output toggle) sits at the `OUT_TOG` position from ZMK (bottom row, leftmost right). `User0`–`User3`
sit at the `BT_SEL 0`–`BT_SEL 3` positions.

### Vial labels

`config/vial.json` names these keycodes with friendly labels so Vial's picker shows them
correctly:

| User keycode | Vial `shortName` |
|---|---|
| `User0` | BT0 |
| `User1` | BT1 |
| `User2` | BT2 |
| `User3` | BT3 |
| `User6` | Clear BT |
| `User7` | Switch Output |

### How to pair a device

1. Hold Escape (left-outer thumb) to enter the MEDIA layer.
2. Tap `User0` (BT0) through `User3` (BT3) to select the desired profile slot.
3. On the host, open Bluetooth settings and pair with "yuyudhan-1".
4. To re-pair a slot (e.g. because you switched to a new device): select the profile with BT0–BT3,
   then tap `User6` (clear bond) to erase the stored pairing for that slot, then pair fresh.
5. To switch between USB and BLE output on the fly: tap `User7` (output toggle). Note that RMK
   forces USB output mode while a USB cable is plugged in — remove the cable after flashing.

### Vial unlock

```toml
[host]
unlock_keys = [[3, 3], [3, 4]]
```

Hold the two left thumb keys (electrical positions row 3 col 3 = Escape thumb, row 3 col 4 =
Space thumb) simultaneously and click "Unlock" in Vial to enable live remapping.

---

## Known Gaps vs ZMK

Neither gap blocks parity for normal typing use.

### (a) `quick-tap-ms` (175 ms same-key repeat) — not ported

ZMK's `&mt` and `&lt` both set `quick-tap-ms = <175>`. This means: if you tap a mod-tap or
layer-tap key and then hold it again within 175 ms, ZMK treats the second press as a tap and
repeats the character (e.g. tap Backspace, then hold Backspace → continuous deletion). Without
this, the second hold would re-arm the modifier or layer.

No `morse` profile field in RMK provides same-key re-press-to-repeat behavior. `enable_flow_tap` /
`prior_idle_time` serve a *different* purpose (suppressing holds after other keys). This gap means
home-row-mod letters and thumb-tap keys will not auto-repeat by re-hold.

If key-repeat on re-hold becomes important, the only RMK path is a custom Rust key processor — it
cannot be expressed in TOML.

### (b) `display_tog` custom OLED toggle — mapped to `No`

ZMK's MEDIA layer had a custom `display_tog` behavior (`compatible = "zmk,behavior-display-toggle"`)
which toggled the OLED on and off. RMK has no equivalent keycode. The position is mapped to `No`
(blank/dead key). RMK manages display lifecycle automatically: the display blanks after idle and
wakes on any keypress. Explicit toggle is not needed for normal use.

---

## Display

The central (left) half drives an SSD1306 128×32 OLED over I²C:

```toml
[split.central.display]
driver = "ssd1306"
size = "128x32"
rotation = 0
renderer = "OledRenderer"

[split.central.display.protocol.i2c]
instance = "TWISPI0"
sda = "P0_17"
scl = "P0_20"
```

`TWISPI0` is nRF52 TWI instance 0, physically wired to P0.17 (SDA) and P0.20 (SCL) — exactly the
`i2c0` / TWIM0 block used by the ZMK Corne shield (`i2c0: &i2c0` in `corne_left.overlay`). The
`ssd1306` feature in `Cargo.toml` enables the driver.

The built-in `OledRenderer` shows layer name, battery percentage, BLE connection status, WPM, and
active modifiers on the central screen.

**Peripheral display / Trishul logo — not ported.** The ZMK right-half screen displayed a custom
Trishul logo (a 24×32 px LVGL bitmap in `custom_status_screen.c`). Porting this requires
implementing a custom `DisplayRenderer` trait in Rust (wrapping an `embedded_graphics`
`ImageRaw<BinaryColor>` for the logo + dynamic battery/connection overlays) and referencing it via
`renderer = "crate::trishul::TrishulRenderer"` in a `[[split.peripheral]]` display table. This is
an optional enhancement — it is out of scope for core parity and not present in the current build.

---

## Reproduce from Scratch

These steps build the identical firmware from a clean checkout. Do not use `rmkit init` or the
published crate.

**Prerequisites:** `rustup`, `just`, `cargo-make`, `cargo-binutils`, `flip-link`, `llvm-tools`
component. Run `just setup` after step 4 to install the embedded-specific parts automatically.

### Steps

1. **Clone RMK at the pinned revision and copy the BLE split example:**

   ```sh
   git clone https://github.com/HaoboGu/rmk /tmp/rmk-src
   cd /tmp/rmk-src && git checkout b98204928e9a8532064ce99add1f6c4c554e08c9
   mkdir -p ~/code/yuyudhan/yuyudhan-rmk-config
   cp -R /tmp/rmk-src/examples/use_config/nrf52840_ble_split/. \
         ~/code/yuyudhan/yuyudhan-rmk-config/
   cd ~/code/yuyudhan/yuyudhan-rmk-config
   ```

   This gives you: `Cargo.toml`, `build.rs`, `memory.x` (Adafruit bootloader origin `0x1000`,
   correct for nice!nano), `Makefile.toml`, `.cargo/config.toml`, `src/central.rs`,
   `src/peripheral.rs`, `vial.json`. Leave `src/central.rs` and `src/peripheral.rs` unchanged —
   the `#[rmk_central]` and `#[rmk_peripheral]` macros generate everything from `keyboard.toml`.

2. **Repoint `Cargo.toml` — replace the workspace path `rmk` dep with git+rev and add features:**

   In `Cargo.toml`, find the `rmk = { path = "../../../rmk", ... }` line and replace it with:

   ```toml
   rmk = { git = "https://github.com/HaoboGu/rmk", rev = "b98204928e9a8532064ce99add1f6c4c554e08c9", features = [
       "nrf52840_ble",
       "split",
       "async_matrix",
       "adafruit_bl",
       "ssd1306",
   ] }
   ```

   Keep all `nrf-sdc`, `nrf-mpsl`, embassy, and cortex-m deps exactly as copied — they are
   version-coherent with this RMK rev. Remove the `readme = "../../README.md"` line (the file
   won't exist standalone). Keep both `[[bin]]` entries (`central` → `src/central.rs`,
   `peripheral` → `src/peripheral.rs`).

3. **Place the keymap and Vial config at their final paths:**

   ```sh
   mkdir -p config
   cp <your-keyboard.toml> config/keyboard.toml
   cp <your-vial.json>     config/vial.json
   ```

   `config/keyboard.toml` is the full 7-layer config documented in this file.
   `config/vial.json` defines the 4×12 matrix and the `customKeycodes` array (BT0–BT3,
   Clear BT, Switch Output).

4. **Update `.cargo/config.toml` to point `KEYBOARD_TOML_PATH` at `config/keyboard.toml`:**

   ```toml
   [env]
   DEFMT_LOG = "debug"
   KEYBOARD_TOML_PATH = { value = "config/keyboard.toml", relative = true }
   ```

   The RMK proc-macro reads this env var at compile time to locate the keymap.

5. **Update `build.rs` to read `config/vial.json`:**

   In `build.rs`, locate the line:

   ```rust
   let p = Path::new("vial.json");
   ```

   Change it to:

   ```rust
   let p = Path::new("config/vial.json");
   ```

   Also update the `rerun-if-changed` directive:

   ```rust
   println!("cargo:rerun-if-changed=config/vial.json");
   ```

6. **Install embedded tooling (one-time):**

   ```sh
   just setup
   # installs: thumbv7em-none-eabihf target, llvm-tools, flip-link,
   #           cargo-make, cargo-binutils, cargo-hex-to-uf2
   ```

7. **Build both halves:**

   ```sh
   just build
   # equivalent to:
   #   cargo build --release --target thumbv7em-none-eabihf --bin central
   #   cargo build --release --target thumbv7em-none-eabihf --bin peripheral
   ```

   Produces two ELF files in `target/thumbv7em-none-eabihf/release/`. Convert to UF2:

   ```sh
   cargo make uf2 --release
   # → rmk-central.uf2 + rmk-peripheral.uf2 at the repo root
   ```

8. **Flash:**

   Double-tap reset on each half → the `NICENANO` USB drive mounts. Copy the matching UF2:

   ```sh
   just flash central      # rmk-central.uf2 → left half
   just flash peripheral   # rmk-peripheral.uf2 → right half
   ```

   Or drag-and-drop manually. The board reboots automatically after the copy. **Remove the USB
   cable after flashing** — RMK forces USB output mode while cabled.

9. **Archive firmware artifacts (recommended):**

   ```sh
   mkdir -p firmware/$(date -u +%Y%m%dT%H%M%SZ)
   cp rmk-central.uf2 rmk-peripheral.uf2 firmware/$(date -u +%Y%m%dT%H%M%SZ)/
   ```

   Keep at least the most recent known-good pair so you can re-flash without a full rebuild.

### Verification

After flashing both halves:

1. **Basic typing (BASE layer):** plug the central into USB, type QWERTY — letters come through.
2. **Home-row mod:** hold `F` for > 200 ms, tap `j` → result is `J` (Shift applied). Tap `F`
   quickly → lowercase `f`.
3. **NAV layer:** hold left-center thumb (Space position) → tap `h/j/k/l` → cursor moves
   ←/↓/↑/→.
4. **Bluetooth:** hold Escape (MEDIA layer), tap BT0 → keyboard announces on BLE; pair from host.
   Tap BT1, pair a second device. Switch between them by re-entering MEDIA and tapping BT0/BT1.
5. **Output toggle:** while on MEDIA, tap output toggle (bottom-left of right hand) → switches
   between USB and BLE output.
6. **Vial live remap:** open [vial.rocks](https://vial.rocks) over USB, hold both left thumb keys,
   click Unlock → all 7 layers render and are editable without reflashing.
