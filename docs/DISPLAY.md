# OLED Display Layout — yuyudhan-1

Both halves use an SSD1306 128×32 OLED over I²C, mounted **portrait** — the
short (32 px) edge faces the user.  `rotation = 90` in `config/keyboard.toml`
gives a 32 wide × 128 tall DrawTarget for each half.

---

## LEFT half (central) — `src/status.rs` → `StatusRenderer`

### Orientation

Same mounting as the right half: short (32 px) edge toward the user, long axis away.
`rotation = 90` → 32×128 portrait DrawTarget.  No `render_interval` — all redraws
are event-driven (layer/BLE/WPM/battery), so idle battery cost is zero.
If content renders upside-down, change `rotation = 90` → `rotation = 270`.

### Band map (portrait, y = 0 at far end)

```
y:    0                    32
      ┌──────────────────────┐  ← far from user
   2  │  OM glyph (30×24)    │
  25  ├──────────────────────┤
  27  │  Layer name band     │  inverted box when not BASE
  44  ├──────────────────────┤
  48  │  WPM  "W{n}"         │
  57  ├──────────────────────┤
  61  │  BLE profile "P{n}"  │
  75  ├──────────────────────┤
  77  │  BLE state (if any)  │  blank when connected
  84  ├──────────────────────┤
  87  │  Battery gauge       │
  95  ├──────────────────────┤
  98  │  Battery number      │
 112  ├──────────────────────┤
 127  └──────────────────────┘  ← nearest to user
```

### Field map

| Region | Source | Font / draw | Position (portrait) |
|--------|--------|-------------|---------------------|
| Om glyph | `OM` bitmap (static) | page-format | x=1, y=2 (30×24 px, 3 pages) |
| Layer name | `ctx.layer` → `LAYER_NAMES[n]` | `FONT_9X15_BOLD` ≤3 chars / `FONT_6X10` ≥4 chars | centred x, y=28 or y=31; inverted 32×18 box at (0,27) when not BASE |
| WPM | `ctx.wpm` | `FONT_6X10` | `"W{n}"`, centred x, y=48 |
| BLE profile | `ctx.ble_status.profile` | `FONT_9X15_BOLD` | `"P{n}"`, centred x, y=61 |
| BLE state | `ctx.ble_status.state` | `FONT_5X8` | centred x, y=77; blank when `Connected` |
| Battery gauge | `*ctx.battery` | stroke-1 rect + fill | outline (2,87) 28×9; nub (30,90) 2×3; fill (4,89) |
| Battery number | `*ctx.battery` | `FONT_9X15_BOLD` | centred x, y=98; no "%" (36 px > 32 px) |

No modifier (SCAG) display — removed; the keyboard layout does not use it.

### Layer name font rules

| Name length | Font | Max width | Fits 32 px? |
|-------------|------|-----------|-------------|
| ≤3 chars (NAV, NUM, SYM, FUN) | `FONT_9X15_BOLD` | 3 × 9 = 27 px | ✓ |
| 4 chars (BASE) | `FONT_6X10` | 4 × 6 = 24 px | ✓ |
| 5 chars (MEDIA, MOUSE) | `FONT_6X10` | 5 × 6 = 30 px | ✓ |

When the active layer is not BASE, a full-width inverted `Rectangle(0, 27, 32, 18)`
highlights the name band.  BASE is shown plain (no box) — it is the default state
and needs no visual emphasis.

### Layer name table

| Index | Name | Font on left display |
|-------|------|---------------------|
| 0 | BASE | `FONT_6X10`, plain |
| 1 | NAV | `FONT_9X15_BOLD`, inverted box |
| 2 | NUM | `FONT_9X15_BOLD`, inverted box |
| 3 | MEDIA | `FONT_6X10`, inverted box |
| 4 | SYM | `FONT_9X15_BOLD`, inverted box |
| 5 | FUN | `FONT_9X15_BOLD`, inverted box |
| 6 | MOUSE | `FONT_6X10`, inverted box |
| 7 | DISPOFF | layer 7 — both displays blank (`ctx.layer == 7`) |
| ≥8 | L{n} | fallback |

> **Keep in sync:** `LAYER_NAMES` lives in `src/layer_names.rs` and is read by
> `src/status.rs` (central binary only).  `src/trishul.rs` reads only
> `DISPLAY_OFF_LAYER` from that module.  If `config/keyboard.toml` layer order
> changes, update `layer_names.rs` in lockstep.

### BLE state display

Profile `P0`–`P3` is always shown large (centred, `FONT_9X15_BOLD`).  The state
label below it is shown only when non-obvious:

| State rendered | Meaning |
|----------------|---------|
| *(blank)* | `BleState::Connected` — BLE active on that profile (clean display) |
| `~` | `BleState::Advertising` — searching for the profile's host |
| `USB` | `BleState::Inactive` — wired USB mode; profile memorised and ready |

### Pixel mockups (portrait, narrow dimension shown horizontally)

**BASE layer, BLE connected on profile 0, 100% battery, WPM 0:**
```
┌────────┐  ← far from user (y=0)
│  OM ॐ  │  rows 2-25
│  BASE  │  rows 27-44 (FONT_6X10, plain — no inverted box)
│   W0   │  rows 48-57
│   P0   │  rows 61-75 (FONT_9X15_BOLD)
│        │  rows 77-84 (blank — connected)
│[======]│  rows 87-95 (gauge full)
│  100   │  rows 98-112
└────────┘  ← nearest (y=127)
```

**NAV layer, BLE advertising profile 2, 47% battery, WPM 120:**
```
┌────────┐
│  OM ॐ  │
│▓ NAV ▓ │  rows 27-44 (FONT_9X15_BOLD, inverted box — not BASE)
│  W120  │
│   P2   │
│   ~    │  rows 77-84 ("~" = searching for profile 2's host)
│[====]  │  47% fill
│   47   │
└────────┘
```

**Sleeping / DISPOFF layer (7):** display cleared, nothing drawn.

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
| Equalizer bars | `tick`, `wpm`, `central_connected` | 4 filled rects width-5 | x 2/8/14/20, baseline y=100, when connected |
| ✓ link icon | `central_connected` | `Line` stroke-1 | x=25–31, y=85–93 (always on when linked) |
| ✗ link icon | `!central_connected`, `tick%2==0` | `Line` stroke-2 | x=25–31, y=84–93 (blinking when not linked) |
| CAPS badge | `caps_lock && central_connected` | `FONT_5X8` inverted | box (2,80) 22×8 |
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
| ✓/✗ link icon | `central_connected`, `tick%2==0` for ✗ | ~800 ms blink for ✗; always-on for ✓ |

`render_interval = 400` ms in `[split.peripheral.display]` drives idle ticks (2.5 fps).
The render loop is gated on `!sleeping` (zero cost while sleeping).
Tune: lower to 120–160 ms for smoother idle bars; raise to 800 ms to save battery.

### Pixel mockups (portrait, narrow dimension shown horizontally)

**Linked, 92% battery, idle:**
```
┌────────┐  ← far from user (y=0)
│  OM ॐ  │  rows 2-25
│ TRISHUL│  rows 30-73
│ ▃▅▇▃ ✓ │  rows 80-100: 4 bars (x=2–24), ✓ icon (x=25–31)
│[======]│  rows 104-112 (gauge full)
│   92   │  rows 113-127 (number)
└────────┘  ← nearest (y=127)
```

**Linked, typing (bars dancing), Caps Lock on:**
```
┌────────┐
│  OM ॐ  │
│ TRISHUL│  + shaft pulse bulge rising
│ CAPS ✓ │  rows 80-87: CAPS badge (x=2–23), ✓ icon (x=25–31)
│ ▂▆▅▇ ✓ │  rows 88-100: bars below CAPS
│[=====] │
│   88   │
└────────┘
```

**Split link down, 47% battery:**
```
┌────────┐
│  OM ॐ  │
│ TRISHUL│
│      ✗ │  rows 80-100: bars blank; ✗ blinking at x=25–31
│[====]  │  47% fill
│   47   │
└────────┘
```

**Sleeping / DISPOFF layer (7):** display cleared, nothing drawn.

---

## Overlap analysis — both screens

**LEFT (status.rs, portrait 32×128):**
- Om x 1–30 (3 pages × 30 cols). ✓
- Layer name max: "MEDIA"/"MOUSE" 5 chars × 6 px = 30 px, centred → x = 1 to x = 30. ✓
- WPM "W200" 4 chars × 6 px = 24 px, centred → x = 4 to x = 27. ✓
- BLE profile "P0" 2 chars × 9 px = 18 px, centred → x = 7 to x = 24. ✓
- Battery number "100" 3 chars × 9 px = 27 px, centred → x = 2 to x = 28. ✓
- Inverted layer box: full-width (x 0–31), y 27–44 — no other element in that band. ✓
- Nothing exceeds x 31 / y 127. ✓

**RIGHT (trishul.rs, portrait 32×128):**
- Om x 1–30; Trishul x 2–29; bars x 2–24 (4 bars × 5 px, gap 1); icon x 25–31. Nothing exceeds x 31. ✓
- Battery number "100" = 3 chars × 9 px = 27 px, x = (32-27)/2 = 2 → ends x 28. ✓
- Shaft pulse x 13–18; Trishul shaft at x 15–16 (intentional overlap — bulge overlays shaft). ✓
- ✗/✓ icon x 25–31; bars end at x 24; gap of 1 px. ✓
- CAPS badge x 2–23, y 80–87; icon strip x 25–31 is untouched — no overlap. ✓
