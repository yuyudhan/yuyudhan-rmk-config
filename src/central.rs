#![no_main]
#![no_std]

use rmk::macros::rmk_central;

mod status;

#[rmk_central]
mod keyboard_central {}
