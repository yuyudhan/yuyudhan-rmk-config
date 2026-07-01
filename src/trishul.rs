//! Custom peripheral (right-half) OLED renderer for a 128×32 SSD1306.
//!
//! Layout — Trishul logo centred (x 52–75, full height), flanked by:
//!   • Left zone  (x 0):  active LAYER NAME when linked, "----" when the split
//!                        link is down (a real name doubles as the link-up cue).
//!   • Right zone (x≥92): this half's battery %, right-aligned.
//!   • Bottom-left (rows 24–31): inverted "CAPS" badge while Caps Lock is on.
//!   • Flank columns (x=48 / x=79): spark marks that orbit the Trishul on each
//!                        right-hand keypress (event-driven; no idle redraws).
//!
//! Layer names come from `src/layer_names.rs` (shared with the left half).
//! Fonts: FONT_9X15 (layer/battery), FONT_5X8 (CAPS badge). Top-baseline anchored.

use core::fmt::Write as _;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::{FONT_5X8, FONT_9X15};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};
use rmk::display::{DisplayRenderer, RenderContext};
use rmk::heapless::String;
use rmk::types::battery::BatteryStatus;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use crate::layer_names::{DISPLAY_OFF_LAYER, LAYER_NAMES};

/// Trishul, 24 px wide × 32 px tall, SSD1306 page format (4 pages × 24 cols).
/// Bytes taken verbatim from ZMK custom_status_screen.c `raw_trishul[4*24]`.
const TRISHUL: [u8; 96] = [
    // Page 0 (rows 0-7): prong tips
    0, 112, 240, 192, 128, 0, 0, 0, 0, 0, 254, 255, 255, 254, 0, 0, 0, 0, 0, 128, 192, 240, 112, 0,
    // Page 1 (rows 8-15): prongs merge into shaft
    0, 0, 0, 1, 3, 7, 14, 28, 56, 112, 255, 255, 255, 255, 112, 56, 28, 14, 7, 3, 1, 0, 0, 0,
    // Page 2 (rows 16-23): shaft
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    // Page 3 (rows 24-31): shaft + base
    0, 0, 0, 0, 0, 0, 0, 0, 192, 224, 255, 255, 255, 255, 224, 192, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// Twinkle points flanking the Trishul, ordered to orbit clockwise.
/// Left column x=48 (clears layer text ≤x44 and Trishul at x52);
/// right column x=79 (clears Trishul end x75 and battery ≥x92).
const SPARK_POINTS: [(i32, i32); 8] =
    [(79, 4), (79, 12), (79, 20), (79, 28), (48, 28), (48, 20), (48, 12), (48, 4)];

/// Right-half status renderer for a 128×32 SSD1306.
#[derive(Default)]
pub struct TrishulRenderer {
    /// Animation frame — advances one step per right-hand keypress render.
    frame: u8,
}

impl DisplayRenderer<BinaryColor> for TrishulRenderer {
    fn render<D: DrawTarget<Color = BinaryColor>>(&mut self, ctx: &RenderContext, display: &mut D) {
        display.clear(BinaryColor::Off).ok();
        if ctx.sleeping {
            return;
        }
        if ctx.layer == DISPLAY_OFF_LAYER {
            return;
        }

        // Trishul centred: (128 - 24) / 2 = 52, full 32-px height.
        draw_page_format_frame(display, &TRISHUL, 24, 52, 0);

        let style = MonoTextStyle::new(&FONT_9X15, BinaryColor::On);

        // Left zone: active layer name when linked; "----" when the split link
        // is down (a real name doubles as the link-up cue).
        // FONT_9X15: MEDIA/MOUSE = 5 chars × 9 px = x 0–44, clear of logo at x 52.
        let mut layer_buf: String<8> = String::new();
        let left_str: &str = if !ctx.central_connected {
            "----"
        } else if (ctx.layer as usize) < LAYER_NAMES.len() {
            LAYER_NAMES[ctx.layer as usize]
        } else {
            let _ = write!(layer_buf, "L{}", ctx.layer);
            &layer_buf
        };
        Text::with_baseline(left_str, Point::new(0, 9), style, Baseline::Top)
            .draw(display)
            .ok();

        // Right column: this half's battery %.
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
        // Right-align: FONT_9X15 advance = 9 px/char.
        // "100%" = 4 chars → x = 128 - 36 = 92, clear of logo end at x 76.
        let x = 128 - (bat.len() as i32) * 9;
        Text::with_baseline(&bat, Point::new(x, 9), style, Baseline::Top)
            .draw(display)
            .ok();

        // CAPS badge — inverted box, bottom-left, only while Caps Lock is active.
        if ctx.caps_lock {
            Rectangle::new(Point::new(0, 24), Size::new(22, 8))
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)
                .ok();
            let inv = MonoTextStyle::new(&FONT_5X8, BinaryColor::Off);
            Text::with_baseline("CAPS", Point::new(1, 24), inv, Baseline::Top)
                .draw(display)
                .ok();
        }

        // Spark: fires only on a fresh right-hand keypress render; count scales
        // with typing speed; rotates by frame. No render_interval -> zero idle cost.
        if ctx.central_connected && ctx.key_press_latch {
            let count = 2 + (ctx.wpm / 40).min(6) as usize; // 2..=8
            for k in 0..count {
                let (sx, sy) = SPARK_POINTS[(self.frame as usize + k) % SPARK_POINTS.len()];
                for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
                    Pixel(Point::new(sx + dx, sy + dy), BinaryColor::On).draw(display).ok();
                }
            }
            self.frame = self.frame.wrapping_add(1);
        }
    }
}

/// Draw an SSD1306 page-format frame onto an embedded-graphics target.
/// Copied from rmk `display/renderers/logo.rs` (private there).
fn draw_page_format_frame<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    data: &[u8],
    cols: usize,
    offset_x: i32,
    offset_y: i32,
) {
    let pages = data.len() / cols;
    for page in 0..pages {
        for col in 0..cols {
            let byte = data[page * cols + col];
            if byte == 0 {
                continue;
            }
            for bit in 0..8u32 {
                if byte & (1 << bit) != 0 {
                    let x = col as i32 + offset_x;
                    let y = page as i32 * 8 + bit as i32 + offset_y;
                    Pixel(Point::new(x, y), BinaryColor::On).draw(display).ok();
                }
            }
        }
    }
}
