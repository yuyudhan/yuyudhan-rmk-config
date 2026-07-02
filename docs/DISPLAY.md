# OLED Display Layout — yuyudhan-1

Both halves use an SSD1306 128×32 OLED over I²C.

---

## LEFT half (central) — `src/status.rs` → `StatusRenderer`

### Layout — two 16 px bands

```
y=0  ┌────────────────────────────────────────────────────────────────┐
     │  LAYER NAME (FONT_9X15_BOLD, left)          BATTERY (right)   │  ← top band (y 0–15)
y=16 ├────────────────────────────────────────────────────────────────┤
     │  S  C  A  G  (FONT_9X15_BOLD, x=0/12/24/36)  BT+WPM (right)  │  ← bottom band (y 16–31)
y=32 └────────────────────────────────────────────────────────────────┘
```

### Field map

| Region | Source field | Font | Position |
|--------|-------------|------|----------|
| Layer name | `ctx.layer` → `LAYER_NAMES[n]` | `FONT_9X15_BOLD` | x=0, y=0, `Baseline::Top` |
| Battery % | `*ctx.battery` | `FONT_9X15_BOLD` | right-aligned, y=0; advance 9 px/char |
| Modifier S | `ctx.modifiers.left_shift() \|\| right_shift()` | `FONT_9X15_BOLD` | x=0, y=17 |
| Modifier C | `ctx.modifiers.left_ctrl() \|\| right_ctrl()` | `FONT_9X15_BOLD` | x=12, y=17 |
| Modifier A | `ctx.modifiers.left_alt() \|\| right_alt()` | `FONT_9X15_BOLD` | x=24, y=17 |
| Modifier G | `ctx.modifiers.left_gui() \|\| right_gui()` | `FONT_9X15_BOLD` | x=36, y=17 |
| BT profile + WPM | `ctx.ble_status` + `ctx.wpm` | `FONT_6X10` | right-aligned, y=19; advance 6 px/char |

**Active modifier**: a 10×15 px filled rectangle at `(cell_x, 16)` is drawn first; the letter is then rendered in `BinaryColor::Off` on top, giving an inverted (white-box, black-letter) highlight.

### Layer name table

| Index | Name | Source |
|-------|------|--------|
| 0 | BASE | layer 0, `config/keyboard.toml` |
| 1 | NAV | layer 1 |
| 2 | NUM | layer 2 |
| 3 | MEDIA | layer 3 |
| 4 | SYM | layer 4 |
| 5 | FUN | layer 5 |
| 6 | MOUSE | layer 6 |
| 7 | DISPOFF | layer 7 — both displays blank (ctx.layer == 7) |
| ≥8 | L{n} | fallback, unlikely |

> **Keep in sync:** `LAYER_NAMES` lives in `src/layer_names.rs`, shared by both halves (`StatusRenderer` and `TrishulRenderer`) — rmk has no runtime layer-name API. If `config/keyboard.toml` layer order changes, update that one constant in lockstep.

### BT profile display (bottom-right)

The **profile number** `P0`–`P3` is **always shown** in `FONT_9X15_BOLD` at a fixed
x=110 position — visible in every state, including when testing over USB.
A small `FONT_6X10` state label appears to its left only when the state is non-obvious:

| Rendered | Meaning |
|----------|---------|
| `P0` … `P3` | `BleState::Connected` — BLE active on profile n (label omitted; normal state) |
| `~ P0` … `~ P3` | `BleState::Advertising` — searching for profile n's host |
| `USB P0` … `USB P3` | `BleState::Inactive` — wired USB mode; profile n memorised and ready |

Profile is 0-indexed, matching the `User0`–`User3` profile-select keys in the MEDIA layer.

### Pixel mockups

**BASE layer, no mods, USB mode (profile 0 memorised), 100% battery, WPM 0:**
```
┌──────────────────────────────────┐  128 px wide
│BASE                         100% │  y 0–15  FONT_9X15_BOLD
│S  C  A  G    W0      USB   P0    │  y 16–31 mods+profile=9X15_BOLD, wpm+state=6X10
└──────────────────────────────────┘  32 px tall
```

**NAV layer (Space held), Shift+Ctrl active, BLE connected on profile 0, 85%, WPM 120:**
```
┌──────────────────────────────────┐
│NAV                           85% │
│[S][C] A  G   W120          P0    │  ← no state label when BLE connected
└──────────────────────────────────┘
```

**MEDIA layer, GUI active, BLE advertising on profile 2, 30%, WPM 0:**
```
┌──────────────────────────────────┐
│MEDIA                         30% │
│ S   C   A  [G]  W0     ~   P2    │  ← "~" = searching for profile 2's host
└──────────────────────────────────┘
```

**Sleeping (any state):**
```
┌──────────────────────────────────┐
│                                  │  ← display cleared, nothing drawn
└──────────────────────────────────┘
```

---

## RIGHT half (peripheral) — `src/trishul.rs` → `TrishulRenderer`

### Orientation

The SSD1306 is mounted **portrait** — the short (32 px) edge faces the user; the long
(128 px) edge points away. `rotation = 90` in `config/keyboard.toml` swaps the DrawTarget
to 32 wide × 128 tall; y = 0 is the far end, y = 127 is closest to the user.
If content renders upside-down on hardware, change `rotation = 90` → `rotation = 270`.

### Band map (portrait, y = 0 at far end)

```
y:    0                    32
      ┌──────────────────────┐  ← far from user
   2  │  OM glyph (30×24)    │
  25  ├──────────────────────┤
  26  │  (gap)               │
  30  ├──────────────────────┤
      │  Trishul (28×48)     │
  77  ├──────────────────────┤
  78  │  (gap)               │
  80  ├──────────────────────┤
      │  Equalizer bars (5)  │
      │  or CAPS badge       │
 100  │  or NO-LINK blink    │
 104  ├──────────────────────┤
      │  Battery gauge       │
 112  ├──────────────────────┤
      │  Battery number      │
 127  └──────────────────────┘  ← nearest to user
```

### Field map

| Region | Source | Font / draw | Position (portrait) |
|--------|--------|-------------|---------------------|
| Om glyph | `OM` bitmap | page-format | x=1, y=2 (30×24 px, 3 pages) |
| Bindu halo | `tick%8 < 4` | `Circle` stroke-1 | centre (18,3), r=7 |
| Trishul | `TRISHUL` bitmap | page-format | x=2, y=30 (28×48 px, 6 pages) |
| Shaft pulse | `key_press_latch` + `central_connected` | filled rect 6×3 | x=13, y=70–52 rising, 6 steps |
| Equalizer bars | `tick`, `wpm`, `central_connected` | 5 filled rects 4 wide | x 2/8/14/20/26, baseline y=100 |
| CAPS badge | `caps_lock && central_connected` | `FONT_5X8` inverted | box (5,80) 22×8 |
| NO-LINK | `!central_connected`, `tick%2==0` | `FONT_5X8` inverted | box (4,84) 24×20 |
| Battery gauge | `*ctx.battery` | stroke-1 rect + fill | outline (2,104) 28×9; nub (30,107) 2×3 |
| Battery number | `*ctx.battery` | `FONT_9X15` | centred x=(32-w)/2, y=113 |

### Bitmaps

**OM (30×24)** — rasterised from `Kohinoor.ttc` (index 0), U+0950, 400 px render,
`ImageFilter.MaxFilter(5)`, LANCZOS thumbnail to 30 px, threshold 90. Verified by
round-trip ASCII preview. Regeneration script in the `OM` constant doc comment in
`src/trishul.rs`. Hardware gate: if ॐ reads poorly on the physical panel, tweak the
threshold (60–110) and re-embed; if still muddy, remove the `OM` + bindu blocks and
move the Trishul to y=8.

**TRISHUL (28×48)** — constructed parametrically via Python `hspan` calls (three barbed
arrowheads, inward quarter-ellipse curves, crossbar, shaft, damaru diamond, pommel).
Regeneration: rerun the `hspan` block from the planning session and re-pack.

### Animation

| Effect | Trigger | Rate |
|--------|---------|------|
| Bindu halo on/off | `tick % 8 < 4` | ~3.2 s cycle idle |
| Equalizer bars swell | `WAVE[tick+PHASE[i]]` + WPM | 400 ms idle; ~30 fps typing |
| Shaft energy pulse | `key_press_latch` | per keypress, 6-step decay |
| Low-battery blink | `pct < 20 && tick%2==0` | ~800 ms |
| NO-LINK blink | `!central_connected && tick%2==0` | ~800 ms |

`render_interval = 400` ms in `[split.peripheral.display]` drives idle ticks (2.5 fps).
The render loop is gated on `!sleeping` (zero cost while sleeping).
Tune: lower to 120–160 ms for smoother idle bars; raise to 800 ms to save battery.

### Pixel mockups (portrait, narrow dimension shown horizontally)

**Linked, 92% battery, idle:**
```
┌────────┐  ← far from user (y=0)
│  OM ॐ  │  rows 2-25
│ TRISHUL│  rows 30-73
│ ▃▅▇▅▃  │  rows 80-100 (equalizer)
│[======]│  rows 104-112 (gauge full)
│   92   │  rows 113-127 (number)
└────────┘  ← nearest (y=127)
```

**Linked, typing (bars dancing), Caps Lock on:**
```
┌────────┐
│  OM ॐ  │
│ TRISHUL│  + shaft pulse bulge rising
│ CAPS   │  rows 80-88 (CAPS badge overlaid)
│ ▂▆▅▇▂  │  rows below CAPS
│[=====] │
│   88   │
└────────┘
```

**Split link down, 47% battery:**
```
┌────────┐
│  OM ॐ  │
│ TRISHUL│
│ NO     │  blinking inverted badge
│ LINK   │
│[====]  │  47% fill
│   47   │
└────────┘
```

**Sleeping / DISPOFF layer (7):** display cleared, nothing drawn.

---

## Overlap analysis — both screens

**LEFT (status.rs):**
- Layer name max width: "MEDIA"/"MOUSE" = 5 chars × 9 px = 45 px (x 0–44). Battery min x: `128 - 4*9 = 92`. Gap = 47 px. ✓ No overlap.
- Modifiers max extent: x 0–44 (4th cell starts at 36, glyph ends at 44). Status min x: `128 - 9*6 = 74`. Gap = 29 px. ✓ No overlap.

**RIGHT (trishul.rs, portrait 32×128):**
- Om x 1–30; Trishul x 2–29; bars x 2–29; gauge x 2–31 (nub 30–31). Nothing exceeds x 31. ✓
- Battery number "100" = 3 chars × 9 px = 27 px, x = (32-27)/2 = 2 → ends x 28. ✓
- Shaft pulse x 13–18; Trishul shaft at x 15–16 (intentional overlap — bulge overlays shaft). ✓
- NO-LINK badge x 4–27, y 84–103; replaces bars entirely — no coexistence needed. ✓
- CAPS badge x 5–26, y 80–87; drawn over bars (inverted box, bars underneath are obscured deliberately). ✓
