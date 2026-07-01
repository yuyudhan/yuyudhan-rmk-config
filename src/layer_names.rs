//! Layer names + display-off layer shared by both halves' OLED renderers.
//!
//! MUST match [[layer]] order in config/keyboard.toml (0-indexed).
//! If that order changes, update here in lockstep — both src/status.rs (left)
//! and src/trishul.rs (right) read these. See docs/DISPLAY.md.

/// Selecting this layer blanks the OLED (MEDIA-layer display toggle).
pub const DISPLAY_OFF_LAYER: u8 = 7;

/// Layer names — array index = layer number.
pub const LAYER_NAMES: [&str; 8] =
    ["BASE", "NAV", "NUM", "MEDIA", "SYM", "FUN", "MOUSE", "DISPOFF"];
