#![no_main]
#![no_std]

use rmk::macros::rmk_peripheral;

mod trishul;

#[rmk_peripheral(id = 0)]
mod keyboard_peripheral {}
