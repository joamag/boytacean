//! General information about the crate and the emulator.

use crate::{
    gen::{NAME, VERSION},
    util::capitalize,
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Obtains the name of the emulator.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn name() -> String {
    capitalize(NAME)
}

/// Obtains the version of the emulator.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn version() -> String {
    String::from(VERSION)
}

/// Obtains the system this emulator is emulating..
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn system() -> String {
    String::from("Game Boy")
}
