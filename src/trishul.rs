//! Custom peripheral (right-half) OLED renderer: centred Trishul logo,
//! this half's battery %, and central-link status. Ported from the ZMK
//! custom_status_screen.c right-half screen.

use core::fmt::Write as _;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_5X8;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use rmk::display::{DisplayRenderer, RenderContext};
use rmk::heapless::String;
use rmk::types::battery::BatteryStatus;

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

/// Right-half status renderer for a 128×32 SSD1306.
#[derive(Default)]
pub struct TrishulRenderer;

impl DisplayRenderer<BinaryColor> for TrishulRenderer {
    fn render<D: DrawTarget<Color = BinaryColor>>(&mut self, ctx: &RenderContext, display: &mut D) {
        display.clear(BinaryColor::Off).ok();
        if ctx.sleeping {
            return;
        }

        // Trishul centred: (128 - 24) / 2 = 52, full 32-px height.
        draw_page_format_frame(display, &TRISHUL, 24, 52, 0);

        let style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);

        // Left column: central-link status.
        let link = if ctx.central_connected { "LINK" } else { "----" };
        Text::new(link, Point::new(2, 8), style).draw(display).ok();

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
        // Right-align (FONT_5X8: 5-px glyph + 1-px advance = 6 px/char).
        let x = 128 - (bat.len() as i32) * 6;
        Text::new(&bat, Point::new(x, 8), style).draw(display).ok();
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
