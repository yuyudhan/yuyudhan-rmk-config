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

### Layout — full 32 px, Trishul centred with flanking zones

```
x:   0         48  52        76  79      92      128
     ├─LAYER────┤sp├──TRISHUL─┤sp├────────┼─BAT%──┤
     └CAPS(24-31)┘                        (right-aligned)
```

The Trishul logo is 24 px wide, centred at x=52 (pixels 52–75). Everything else clears it:
- **Left zone** (x=0, y=9): active **layer name** when the split link is up; `----` when it's down (a real name doubles as the link-up cue). Max shown name "MEDIA"/"MOUSE" = 5×9 = x 0–44; logo starts x=52 → 8 px gap. `DISPOFF` (layer 7) is never drawn — that layer blanks the screen.
- **Battery %** (right-aligned, y=9): rightmost char ends at x=128; "100%" starts at x=92, logo ends at x=75 → 17 px gap.
- **CAPS badge** (rows 24–31, x 0–21): inverted 22×8 box with "CAPS" in `FONT_5X8`, drawn only while Caps Lock is on. Below the layer name (which ends at row 23) and left of the Trishul base (x≥52).
- **Spark marks** (flank columns x=48 and x=79): small 5-px plus shapes that light up on each fresh right-hand keypress and rotate by animation frame, giving an orbiting twinkle around the Trishul. Purely event-driven — the display already redraws on every `KeyboardEvent`, so there is **no `render_interval`** and no idle-battery cost. Count scales with WPM (2 at rest … 8 at ≥240 wpm).

### Field map

| Region | Source field | Font | Position |
|--------|-------------|------|----------|
| Layer name / link | `ctx.central_connected` + `ctx.layer` → `LAYER_NAMES[n]` | `FONT_9X15` | x=0, y=9, `Baseline::Top` |
| Trishul logo | constant bitmap | page-format | x=52, y=0 (full 32 px height) |
| Battery % | `*ctx.battery` | `FONT_9X15` | right-aligned, y=9; advance 9 px/char |
| CAPS badge | `ctx.caps_lock` | `FONT_5X8` (inverted) | box (0,24) size 22×8; text (1,24) |
| Spark | `ctx.key_press_latch` + `ctx.wpm`, gated on `ctx.central_connected` | pixels | flank columns x=48 / x=79, rows 4–28 |

Layer/battery text at y=9 with a 15 px tall font occupies rows 9–23, visually centred in the top 32 px.

### Pixel mockups

**Linked, NAV layer, 92% battery, idle (no spark):**
```
┌──────────────────────────────────┐
│NAV        ╔╗ ╔╗ ╔╗         92%  │  ← layer name replaces old "LINK"
│           ╚╩═╩═╩╝               │
└──────────────────────────────────┘
```

**Linked, BASE layer, typing on the right hand (sparks orbiting), Caps Lock on:**
```
┌──────────────────────────────────┐
│BASE      · ╔╗ ╔╗ ╔╗ ·      88%  │  ← "·" = spark marks in flank columns
│CAPS       ╚╩═╩═╩╝  ·             │  ← inverted CAPS badge bottom-left
└──────────────────────────────────┘
```

**Split link down, 47% battery:**
```
┌──────────────────────────────────┐
│----       ╔╗ ╔╗ ╔╗         47%  │  ← "----" = central not linked
│           ╚╩═╩═╩╝               │
└──────────────────────────────────┘
```

**Sleeping / DISPOFF layer (7):**
```
┌──────────────────────────────────┐
│                                  │  ← display cleared
└──────────────────────────────────┘
```

---

## Overlap analysis — both screens

**LEFT (status.rs):**
- Layer name max width: "MEDIA"/"MOUSE" = 5 chars × 9 px = 45 px (x 0–44). Battery min x: `128 - 4*9 = 92`. Gap = 47 px. ✓ No overlap.
- Modifiers max extent: x 0–44 (4th cell starts at 36, glyph ends at 44). Status min x: `128 - 9*6 = 74`. Gap = 29 px. ✓ No overlap.

**RIGHT (trishul.rs):**
- Layer name (or "----"): max "MEDIA"/"MOUSE" = 5 chars × 9 px = x 0–44. Logo starts x=52. Gap = 8 px. ✓
- Battery "100%": 4 chars × 9 px = 36 px → x=92 to x=128. Logo ends at x=75. Gap = 17 px. ✓
- Spark columns x=48 (plus reaches x 47–49) and x=79 (x 78–80): between layer text (≤44) and logo (≥52), and between logo (≤75) and battery (≥92). ✓
- CAPS badge rows 24–31, x 0–21: below layer text (≤ row 23), left of logo base (x≥52). ✓
