//! Custom peripheral (right-half) OLED renderer — 128×32 SSD1306, mounted **portrait**
//! (short/32 px edge faces the user). `rotation = 90` in keyboard.toml makes the
//! DrawTarget 32 wide × 128 tall; all coordinates below are in that portrait frame.
//!
//! Band map (portrait, y = 0 at the far end from the user):
//!
//!   rows   2-25   OM glyph (Devanagari ॐ, 30×24 px, page-format bitmap)
//!              + bindu halo pulse (circle on tick%8, 3.2 s cycle)
//!   rows  30-73   Trishul (28×48 px, page-format bitmap)
//!              + shaft energy pulses on right-hand keypress
//!   rows  80-100  Equalizer bars (5 bars, 400 ms idle swell, WPM-reactive)
//!              / CAPS inverted badge (overlay when Caps Lock on)
//!              / NO-LINK blinking badge (when split link is down)
//!   rows 104-112  Battery gauge outline + fill (blinks at <20%)
//!   rows 113-127  Battery number in FONT_9X15 (centred; no "%" — gauge provides context)
//!
//! Animation: `render_interval = 400` in keyboard.toml drives idle ticks (2.5 fps);
//! event renders while typing make bars/pulses react ~30 fps.  Zero cost while sleeping.
//! Tune: lower to 120–160 ms for smoother idle bars; raise to 800 ms to save battery.
//!
//! Rotation contingency: if content appears upside-down on hardware, change
//! `rotation = 90` → `rotation = 270` in config/keyboard.toml — layout unchanged.

use core::fmt::Write as _;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::{FONT_5X8, FONT_9X15};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use rmk::display::{DisplayRenderer, RenderContext};
use rmk::heapless::String;
use rmk::types::battery::BatteryStatus;
use crate::layer_names::DISPLAY_OFF_LAYER;

// ── Bitmaps ──────────────────────────────────────────────────────────────────

/// Devanagari Om (U+0950), 30 cols × 24 rows (3 SSD1306 pages), page format.
/// Rasterised from Kohinoor Devanagari (index 0) at 400 px via PIL MaxFilter(5)
/// + LANCZOS thumbnail to 30 px, threshold 90. Round-tripped byte-exact against
/// the ASCII preview before embedding. To regenerate:
///   python3 -c "
///     from PIL import Image,ImageDraw,ImageFont,ImageFilter
///     f=ImageFont.truetype('/System/Library/Fonts/Kohinoor.ttc',400,index=0)
///     img=Image.new('L',(1600,1600),0); d=ImageDraw.Draw(img)
///     d.text((800,800),'\u0950',font=f,fill=255,anchor='mm')
///     g=img.crop(img.getbbox()).filter(ImageFilter.MaxFilter(5))
///     g.thumbnail((30,30),Image.LANCZOS)
///     out=Image.new('L',(30,30),0); out.paste(g,((30-g.width)//2,(30-g.height)//2))
///     out=out.point(lambda p:255 if p>=90 else 0)
///     # --- then pack: data[page*30+col] = OR of (1<<(y%8)) for set pixels in page ---
///   "
const OM: [u8; 90] = [
    // page 0 (rows 0-7)
    0, 0, 128, 192, 192, 192, 192, 192, 128, 0, 0, 48, 112, 240, 224, 192, 199, 199, 199, 194, 224, 240, 112, 48, 0, 0, 0, 0, 0, 0,
    // page 1 (rows 8-15)
    0, 1, 3, 3, 193, 193, 225, 243, 255, 191, 30, 0, 0, 0, 0, 1, 1, 1, 1, 129, 224, 240, 120, 60, 28, 28, 28, 248, 248, 224,
    // page 2 (rows 16-23)
    48, 120, 240, 224, 224, 192, 192, 225, 243, 127, 63, 62, 56, 112, 112, 112, 120, 60, 30, 31, 63, 121, 112, 96, 96, 112, 112, 127, 63, 15,
];

/// Trishul pointing up — three barbed prongs, outer prongs arc down into shaft,
/// crossbar, damaru diamond mid-shaft, round pommel.
/// 28 cols × 48 rows (6 pages; content rows 0-43), page format.
/// Constructed parametrically (see docs/DISPLAY.md bitmap recipe).
const TRISHUL: [u8; 168] = [
    // page 0 (rows 0-7)
    0, 16, 24, 254, 254, 24, 16, 0, 0, 0, 0, 8, 12, 255, 255, 12, 8, 0, 0, 0, 0, 16, 24, 254, 254, 24, 16, 0,
    // page 1 (rows 8-15)
    0, 0, 0, 31, 255, 224, 0, 0, 0, 0, 0, 0, 0, 255, 255, 0, 0, 0, 0, 0, 0, 0, 224, 255, 31, 0, 0, 0,
    // page 2 (rows 16-23)
    0, 0, 0, 0, 0, 1, 99, 102, 108, 108, 120, 120, 112, 255, 255, 112, 120, 120, 108, 108, 102, 99, 1, 0, 0, 0, 0, 0,
    // page 3 (rows 24-31)
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 192, 255, 255, 192, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    // page 4 (rows 32-39)
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 3, 7, 255, 255, 7, 3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    // page 5 (rows 40-47)
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 15, 15, 15, 15, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

// ── Animation tables ──────────────────────────────────────────────────────────

/// Sine-like wave for equalizer bar heights (0–10 px, 8 steps).
const WAVE: [u32; 8] = [0, 2, 5, 8, 10, 8, 5, 2];
/// Per-bar phase offsets so bars move independently.
const PHASE: [u8; 5] = [0, 5, 2, 7, 4];

// ── Renderer ──────────────────────────────────────────────────────────────────

/// Right-half OLED renderer — portrait 32×128 canvas.
#[derive(Default)]
pub struct TrishulRenderer {
    /// Animation clock — +1 per render (400 ms idle tick + event renders while typing).
    tick: u8,
    /// Rising keypress energy pulses on the Trishul shaft; 0 = free slot, else 1..=6.
    pulses: [u8; 3],
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

        // Advance animation clock every render.
        self.tick = self.tick.wrapping_add(1);

        let fill_on  = PrimitiveStyle::with_fill(BinaryColor::On);
        let fill_off = PrimitiveStyle::with_fill(BinaryColor::Off);
        let small    = MonoTextStyle::new(&FONT_5X8,  BinaryColor::On);
        let small_inv= MonoTextStyle::new(&FONT_5X8,  BinaryColor::Off);
        let big      = MonoTextStyle::new(&FONT_9X15, BinaryColor::On);

        // ── 1. Om glyph (rows 2–25) ───────────────────────────────────────────
        draw_page_format_frame(display, &OM, 30, 1, 2);

        // ── 2. Bindu halo pulse ───────────────────────────────────────────────
        // Bindu of the OM glyph is at approx (18, 3) in portrait coords.
        // A thin circle halo appears for 4 ticks then disappears for 4 (3.2 s cycle idle).
        if self.tick % 8 < 4 {
            Circle::with_center(Point::new(18, 3), 7)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)
                .ok();
        }

        // ── 3. Trishul (rows 30–77) ───────────────────────────────────────────
        // Offset x=2 centres the 28-wide bitmap in 32 px. Shaft at screen x 15–16.
        draw_page_format_frame(display, &TRISHUL, 28, 2, 30);

        // ── 4. Shaft energy pulses (keypress-driven) ──────────────────────────
        // Spawn a pulse on each fresh right-hand keypress.
        if ctx.central_connected && ctx.key_press_latch {
            for slot in self.pulses.iter_mut() {
                if *slot == 0 {
                    *slot = 1;
                    break;
                }
            }
        }
        // Draw and advance each active pulse — a 6-px wide bulge over the 2-px shaft,
        // starting at y=70 (pommel top) and rising to y=52 (prong base) over 6 steps.
        for slot in self.pulses.iter_mut() {
            if *slot > 0 {
                let y = 70 - (*slot as i32) * 3;
                Rectangle::new(Point::new(13, y), Size::new(6, 3))
                    .into_styled(fill_on)
                    .draw(display)
                    .ok();
                *slot += 1;
                if *slot > 6 {
                    *slot = 0;
                }
            }
        }

        // ── 5/6/7. Bars / CAPS / NO-LINK (rows 80–100) ───────────────────────
        if !ctx.central_connected {
            // NO-LINK blinking badge — replaces bars entirely when split link is down.
            if self.tick % 2 == 0 {
                Rectangle::new(Point::new(4, 84), Size::new(24, 20))
                    .into_styled(fill_on)
                    .draw(display)
                    .ok();
                Text::with_baseline("NO",   Point::new(11, 86), small_inv, Baseline::Top).draw(display).ok();
                Text::with_baseline("LINK", Point::new( 6, 94), small_inv, Baseline::Top).draw(display).ok();
            }
        } else {
            // Equalizer bars — 5 bars, baseline y=100, height 2–20 px.
            for i in 0usize..5 {
                let h = (2
                    + WAVE[(self.tick.wrapping_add(PHASE[i]) % 8) as usize]
                    + (ctx.wpm as u32 / 30).min(8))
                .min(20);
                Rectangle::new(
                    Point::new(2 + i as i32 * 6, 100 - h as i32),
                    Size::new(4, h),
                )
                .into_styled(fill_on)
                .draw(display)
                .ok();
            }

            // CAPS badge — inverted 22×8 box overlaid on the bar region.
            if ctx.caps_lock {
                Rectangle::new(Point::new(5, 80), Size::new(22, 8))
                    .into_styled(fill_on)
                    .draw(display)
                    .ok();
                Text::with_baseline("CAPS", Point::new(6, 80), small_inv, Baseline::Top)
                    .draw(display)
                    .ok();
            }
        }

        // ── 8. Battery gauge (rows 104–112) ──────────────────────────────────
        // Outline + nub are always visible.
        Rectangle::new(Point::new(2, 104), Size::new(28, 9))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)
            .ok();
        Rectangle::new(Point::new(30, 107), Size::new(2, 3))
            .into_styled(fill_on)
            .draw(display)
            .ok();

        let mut bat_num: String<4> = String::new();
        match *ctx.battery {
            BatteryStatus::Available { level: Some(pct), .. } => {
                // Fill: blinks at <20% (low battery warning).
                let draw_fill = pct >= 20 || self.tick % 2 == 0;
                if draw_fill {
                    let fill_w = (pct as u32 * 24 / 100).max(1);
                    Rectangle::new(Point::new(4, 106), Size::new(fill_w, 5))
                        .into_styled(fill_on)
                        .draw(display)
                        .ok();
                }
                let _ = write!(bat_num, "{}", pct);
            }
            BatteryStatus::Available { level: None, .. } => {
                // Draw a question mark inside the gauge.
                let _ = write!(bat_num, "--");
            }
            BatteryStatus::Unavailable => {
                let _ = write!(bat_num, "?");
            }
        }

        // ── 9. Battery number (rows 113–127) ─────────────────────────────────
        // Centred in 32 px; FONT_9X15 advance = 9 px/char.
        // "%" omitted — "100" (27 px) fits; "100%" (36 px) would overflow.
        if !bat_num.is_empty() {
            let x = (32 - bat_num.len() as i32 * 9) / 2;
            Text::with_baseline(&bat_num, Point::new(x.max(0), 113), big, Baseline::Top)
                .draw(display)
                .ok();
        }

        // Clear any pixels the fill/number drew outside the gauge nub region.
        // (nub at x 30-31: safe — battery number worst-case "100" ends at x 27+9=28 ✓)
        let _ = fill_off; // suppress unused-variable warning; fill_off is defensive only
    }
}

/// Draw an SSD1306 page-format bitmap onto an embedded-graphics DrawTarget.
/// `data[page * cols + col]`, bit `b` → pixel at (col + offset_x, page*8 + b + offset_y).
/// LSB (bit 0) = top row of the page.
/// Copied verbatim from rmk `display/renderers/logo.rs` (private there).
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
