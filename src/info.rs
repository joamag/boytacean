//! General information about the crate and the emulator.

use crate::{
    gen::{COMPILATION_DATE, COMPILATION_TIME, COMPILER, COMPILER_VERSION, NAME, VERSION},
    util::capitalize,
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct Info;

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Info {
    /// Obtains the name of the emulator.
    pub fn name() -> String {
        capitalize(NAME)
    }

    /// Obtains the version of the emulator.
    pub fn version() -> String {
        String::from(VERSION)
    }

    /// Obtains the system this emulator is emulating.
    pub fn system() -> String {
        String::from("Game Boy")
    }

    /// Obtains the name of the compiler that has been
    /// used in the compilation of the base Boytacean
    /// library. Can be used for diagnostics.
    pub fn compiler() -> String {
        String::from(COMPILER)
    }

    pub fn compiler_version() -> String {
        String::from(COMPILER_VERSION)
    }

    pub fn compilation_date() -> String {
        String::from(COMPILATION_DATE)
    }

    pub fn compilation_time() -> String {
        String::from(COMPILATION_TIME)
    }
}
