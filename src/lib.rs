#![allow(clippy::uninlined_format_args)]
#![cfg_attr(feature = "simd", feature(portable_simd))]

pub mod apu;
pub mod cheats;
pub mod color;
pub mod consts;
pub mod cpu;
pub mod data;
pub mod devices;
pub mod diag;
pub mod dma;
pub mod gb;
pub mod gen;
pub mod info;
pub mod inst;
pub mod licensee;
pub mod macros;
pub mod mmu;
pub mod pad;
pub mod ppu;
pub mod rom;
pub mod serial;
pub mod state;
pub mod test;
pub mod timer;
pub mod util;

#[cfg(feature = "python")]
pub mod py;
