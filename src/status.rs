//! Custom central (left-half) OLED renderer — 128×32 SSD1306.
//!
//! Two 16 px bands:
//!
//!  ┌──────────────────────────────────────┐
//!  │  LAYER NAME                  BAT%    │  y 0–15  FONT_9X15_BOLD
//!  │  S  C  A  G     W##  [USB]   P#      │  y 16–31
//!  └──────────────────────────────────────┘
//!
//! Bottom-right "P#" — the active BLE profile number — is always rendered in
//! FONT_9X15_BOLD at a fixed x=110 regardless of connection state.  It is
//! ALWAYS visible so the user can always tell which profile is selected:
//!
//!   P0 … P3     — BLE connected on that profile   (state indicator absent)
//!   ~ P0 … P3   — BLE advertising/searching       (tiny "~" left of P#)
//!   USB P0 … P3 — wired USB mode, profile ready   (tiny "USB" left of P#)
//!
//! This makes the profile number visible even when flashing/testing over USB.
//!
//! Layer names live in `src/layer_names.rs` (shared with the right half) and must
//! match `config/keyboard.toml` [[layer]] order. Update that module in lockstep.
//! See docs/DISPLAY.md for pixel mockups and field-map tables.

use core::fmt::Write as _;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_9X15_BOLD};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use rmk::display::{DisplayRenderer, RenderContext};
use rmk::heapless::String;
use rmk::types::battery::BatteryStatus;
use rmk::types::ble::BleState;
use crate::layer_names::{DISPLAY_OFF_LAYER, LAYER_NAMES};

/// FONT_9X15_BOLD: character advance = 9 px.
const ADV_BIG: i32 = 9;
/// FONT_6X10: character advance = 6 px.
const ADV_SM: i32 = 6;

/// x position where the profile "P#" label starts (right edge = 128).
/// "P3" = 2 chars × 9 px = 18 px → starts at 110.  Always fixed so the eye knows where to look.
const PROFILE_X: i32 = 110;

/// Right edge for the state label ("USB" / "~"), 3 px left of PROFILE_X.
const STATE_RIGHT: i32 = PROFILE_X - 3;

/// Left-half status renderer for a 128×32 SSD1306.
#[derive(Default)]
pub struct StatusRenderer;

impl DisplayRenderer<BinaryColor> for StatusRenderer {
    fn render<D: DrawTarget<Color = BinaryColor>>(&mut self, ctx: &RenderContext, display: &mut D) {
        display.clear(BinaryColor::Off).ok();
        if ctx.sleeping {
            return;
        }
        if ctx.layer == DISPLAY_OFF_LAYER {
            return;
        }

        let big     = MonoTextStyle::new(&FONT_9X15_BOLD, BinaryColor::On);
        let big_inv = MonoTextStyle::new(&FONT_9X15_BOLD, BinaryColor::Off);
        let small   = MonoTextStyle::new(&FONT_6X10,      BinaryColor::On);
        let fill_on = PrimitiveStyle::with_fill(BinaryColor::On);

        // ── TOP BAND (y 0–15) ────────────────────────────────────────────────
        // Left:  active layer name in large bold font.
        // Right: battery percentage, right-aligned.

        // Layer name — left-aligned at (0, 0).
        let mut layer_buf: String<8> = String::new();
        let layer_str: &str = if (ctx.layer as usize) < LAYER_NAMES.len() {
            LAYER_NAMES[ctx.layer as usize]
        } else {
            let _ = write!(layer_buf, "L{}", ctx.layer);
            &layer_buf
        };
        Text::with_baseline(layer_str, Point::new(0, 0), big, Baseline::Top)
            .draw(display)
            .ok();

        // Battery % — right-aligned, same band.
        // Clamped so it never overwrites the layer name (shortest gap ≥ 0 enforced by .max()).
        let mut bat: String<8> = String::new();
        match *ctx.battery {
            BatteryStatus::Available { level: Some(pct), .. } => {
                let _ = write!(bat, "{}%", pct);
            }
            BatteryStatus::Available { level: None, .. } => {
                let _ = write!(bat, "--");
            }
            BatteryStatus::Unavailable => {
                let _ = write!(bat, "?");
            }
        }
        let bat_x = (128 - bat.len() as i32 * ADV_BIG).max(64);
        Text::with_baseline(&bat, Point::new(bat_x, 0), big, Baseline::Top)
            .draw(display)
            .ok();

        // ── BOTTOM BAND (y 16–31) ────────────────────────────────────────────
        //
        // Fixed pixel layout (128 px wide):
        //
        //  x:  0   12  24  36       52          89–107  110–127
        //      [S] [C] [A] [G]   W{wpm}   [USB|~]   P{n}
        //       ←— FONT_9X15_BOLD —→  ←FONT_6X10→  ←6X10→  ←9X15_BOLD→
        //
        // Clearances (worst case):
        //   Mods end    : x=44  (cell 3 at x=36, width=9 → 36+9-1=44)
        //   WPM "W255"  : x=52…75  (4 chars × 6 px)
        //   State "USB" : x=89…106  (3 chars × 6 px, right-aligned to STATE_RIGHT=107)
        //   Profile "P3": x=110…127 (2 chars × 9 px, fixed PROFILE_X=110)
        //   Gaps: mods→wpm 7 px, wpm→state 13 px, state→profile 3 px.  All clear.

        // Modifier cells — S C A G at pitch 12.
        // Active cell: white fill box, black letter.  Inactive: plain white letter.
        let m = ctx.modifiers;
        let mod_active = [
            m.left_shift()  || m.right_shift(),
            m.left_ctrl()   || m.right_ctrl(),
            m.left_alt()    || m.right_alt(),
            m.left_gui()    || m.right_gui(),
        ];
        for (i, &(ch, active)) in [('S', mod_active[0]),
                                    ('C', mod_active[1]),
                                    ('A', mod_active[2]),
                                    ('G', mod_active[3])].iter().enumerate() {
            let cx = i as i32 * 12;
            let mut buf: String<2> = String::new();
            let _ = write!(buf, "{}", ch);
            if active {
                Rectangle::new(Point::new(cx, 16), Size::new(10, 15))
                    .into_styled(fill_on)
                    .draw(display)
                    .ok();
                Text::with_baseline(&buf, Point::new(cx, 17), big_inv, Baseline::Top)
                    .draw(display)
                    .ok();
            } else {
                Text::with_baseline(&buf, Point::new(cx, 17), big, Baseline::Top)
                    .draw(display)
                    .ok();
            }
        }

        // WPM — small font, vertically centred in bottom band (y=19 centres 10 px in 16 px).
        let mut wpm_buf: String<8> = String::new();
        let _ = write!(wpm_buf, "W{}", ctx.wpm);
        Text::with_baseline(&wpm_buf, Point::new(52, 19), small, Baseline::Top)
            .draw(display)
            .ok();

        // BT profile — ALWAYS rendered in large bold font at fixed x=110.
        // "P0" … "P3" (always 2 chars = 18 px).  Never disappears regardless of state.
        let mut profile_buf: String<4> = String::new();
        let _ = write!(profile_buf, "P{}", ctx.ble_status.profile);
        Text::with_baseline(&profile_buf, Point::new(PROFILE_X, 17), big, Baseline::Top)
            .draw(display)
            .ok();

        // State indicator — small text to the LEFT of P#, right-aligned to STATE_RIGHT.
        // BLE connected = no indicator (normal operation; clean display).
        // Advertising   = "~"   (1 char — keyboard is searching for this profile's host).
        // USB/Inactive  = "USB" (3 chars — wired mode; profile is memorised and ready).
        let state_str: &str = match ctx.ble_status.state {
            BleState::Connected   => "",
            BleState::Advertising => "~",
            BleState::Inactive    => "USB",
        };
        if !state_str.is_empty() {
            let state_x = STATE_RIGHT - state_str.len() as i32 * ADV_SM;
            Text::with_baseline(state_str, Point::new(state_x, 19), small, Baseline::Top)
                .draw(display)
                .ok();
        }
    }
}
