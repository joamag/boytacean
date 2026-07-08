//! Low-level diagnostic utilities for debugging purposes.
//!
//! Some of the implementations make use of unsafe code to store
//! a global instance of the emulator, which is going to be used
//! in panic diagnostics

#[cfg(feature = "profile")]
use std::fmt::{self, Display, Formatter};
use std::ptr::null;

use crate::gb::GameBoy;

/// Static mutable reference to the global instance of the
/// Game Boy emulator, going to be used for global diagnostics.
static mut GLOBAL_INSTANCE: *const GameBoy = null();

// Static mutable flag to enable or disable pedantic diagnostics.
#[cfg(feature = "pedantic")]
pub static mut PEDANTIC: bool = true;

/// Static mutable instance of the profile structure to be used
/// in the gathering of performance metrics of the emulator.
#[cfg(feature = "profile")]
pub static mut PROFILE: Profile = Profile::new();

/// Structure that gathers a series of performance metrics
/// (times and counters) from the emulator internals, to be
/// used for profiling purposes.
///
/// The metrics are gathered globally using the [`profile_time_gb`],
/// [`profile_count_gb`], [`profile_read_gb`] and [`profile_write_gb`]
/// macros, which are no-ops when the `profile` feature is disabled.
#[cfg(feature = "profile")]
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    /// Time spent running complete frames (in nanoseconds).
    ///
    /// This value works as the base reference for the other
    /// timing values, as it includes the complete emulation
    /// time (CPU, PPU, APU, DMA, timer and serial).
    pub frame_time: u64,

    /// Time spent rendering the background and window maps
    /// (in nanoseconds).
    pub render_bg_time: u64,

    /// Time spent rendering the objects (in nanoseconds).
    pub render_obj_time: u64,

    /// Number of frames executed.
    pub frames: u64,

    /// Number of CPU instructions executed.
    pub instructions: u64,

    /// Number of PPU lines rendered.
    pub render_lines: u64,

    /// Number of APU samples synthesized.
    pub apu_samples: u64,

    /// Number of MMU reads from the ROM area (0x0000-0x7FFF).
    pub reads_rom: u64,

    /// Number of MMU reads from the VRAM area (0x8000-0x9FFF).
    pub reads_vram: u64,

    /// Number of MMU reads from the external RAM area (0xA000-0xBFFF).
    pub reads_eram: u64,

    /// Number of MMU reads from the work RAM area (0xC000-0xFDFF).
    pub reads_wram: u64,

    /// Number of MMU reads from the OAM area (0xFE00-0xFE9F).
    pub reads_oam: u64,

    /// Number of MMU reads from the IO registers area (0xFEA0-0xFF7F, 0xFFFF).
    pub reads_io: u64,

    /// Number of MMU reads from the HRAM area (0xFF80-0xFFFE).
    pub reads_hram: u64,

    /// Number of MMU writes to the ROM area (0x0000-0x7FFF).
    pub writes_rom: u64,

    /// Number of MMU writes to the VRAM area (0x8000-0x9FFF).
    pub writes_vram: u64,

    /// Number of MMU writes to the external RAM area (0xA000-0xBFFF).
    pub writes_eram: u64,

    /// Number of MMU writes to the work RAM area (0xC000-0xFDFF).
    pub writes_wram: u64,

    /// Number of MMU writes to the OAM area (0xFE00-0xFE9F).
    pub writes_oam: u64,

    /// Number of MMU writes to the IO registers area (0xFEA0-0xFF7F, 0xFFFF).
    pub writes_io: u64,

    /// Number of MMU writes to the HRAM area (0xFF80-0xFFFE).
    pub writes_hram: u64,
}

#[cfg(feature = "profile")]
impl Profile {
    pub const fn new() -> Self {
        Self {
            frame_time: 0,
            render_bg_time: 0,
            render_obj_time: 0,
            frames: 0,
            instructions: 0,
            render_lines: 0,
            apu_samples: 0,
            reads_rom: 0,
            reads_vram: 0,
            reads_eram: 0,
            reads_wram: 0,
            reads_oam: 0,
            reads_io: 0,
            reads_hram: 0,
            writes_rom: 0,
            writes_vram: 0,
            writes_eram: 0,
            writes_wram: 0,
            writes_oam: 0,
            writes_io: 0,
            writes_hram: 0,
        }
    }

    /// Total time spent rendering both the background and window
    /// maps and the objects (in nanoseconds).
    pub fn render_time(&self) -> u64 {
        self.render_bg_time + self.render_obj_time
    }

    /// Total number of MMU read operations executed.
    pub fn total_reads(&self) -> u64 {
        self.reads_rom
            + self.reads_vram
            + self.reads_eram
            + self.reads_wram
            + self.reads_oam
            + self.reads_io
            + self.reads_hram
    }

    /// Total number of MMU write operations executed.
    pub fn total_writes(&self) -> u64 {
        self.writes_rom
            + self.writes_vram
            + self.writes_eram
            + self.writes_wram
            + self.writes_oam
            + self.writes_io
            + self.writes_hram
    }

    pub fn description(&self) -> String {
        let total = self.frame_time.max(1);
        let time_line = |name: &str, time: u64| -> String {
            format!(
                "{:<14}{:>10.3} ms ({:>5.2}%)\n",
                name,
                time as f64 / 1e6,
                time as f64 * 100.0 / total as f64
            )
        };
        String::new()
            + &time_line("Frame", self.frame_time)
            + &time_line("Render BG", self.render_bg_time)
            + &time_line("Render OBJ", self.render_obj_time)
            + &time_line("Other", self.frame_time.saturating_sub(self.render_time()))
            + &format!("Frames        {}\n", self.frames)
            + &format!("Instructions  {}\n", self.instructions)
            + &format!("Render lines  {}\n", self.render_lines)
            + &format!("APU samples   {}\n", self.apu_samples)
            + &format!(
                "Reads         {} (rom={} vram={} eram={} wram={} oam={} io={} hram={})\n",
                self.total_reads(),
                self.reads_rom,
                self.reads_vram,
                self.reads_eram,
                self.reads_wram,
                self.reads_oam,
                self.reads_io,
                self.reads_hram
            )
            + &format!(
                "Writes        {} (rom={} vram={} eram={} wram={} oam={} io={} hram={})",
                self.total_writes(),
                self.writes_rom,
                self.writes_vram,
                self.writes_eram,
                self.writes_wram,
                self.writes_oam,
                self.writes_io,
                self.writes_hram
            )
    }
}

#[cfg(feature = "profile")]
impl Default for Profile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "profile")]
impl Display for Profile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl GameBoy {
    /// Sets the current instance as the one going to be used
    /// in panic diagnostics.
    pub fn set_diag(&self) {
        self.set_global();
    }

    /// Unsets the current instance as the one going to be used
    /// in panic diagnostics.
    pub fn unset_diag(&self) {
        self.unset_global();
    }

    /// Dumps the diagnostics for the global instance of the
    /// Boytacean emulator into stdout.
    pub fn dump_diagnostics() {
        if let Some(gb) = Self::global() {
            gb.dump_diagnostics_s();
        }
    }

    /// Obtains the global instance of the Game Boy emulator
    /// ready to be used in diagnostics.
    ///
    /// If the global instance is not set, it will return `None`.
    fn global() -> Option<&'static Self> {
        unsafe {
            if GLOBAL_INSTANCE.is_null() {
                None
            } else {
                Some(&*GLOBAL_INSTANCE)
            }
        }
    }

    /// Sets the current instance as the global one going to
    /// be used in panic diagnostics.
    fn set_global(&self) {
        unsafe {
            GLOBAL_INSTANCE = self;
        }
    }

    fn unset_global(&self) {
        unsafe {
            GLOBAL_INSTANCE = null();
        }
    }

    fn dump_diagnostics_s(&self) {
        println!("Dumping Boytacean diagnostics:");
        println!("{}", self.description_debug());
    }

    /// Obtains a copy of the global profile instance with the
    /// performance metrics gathered so far.
    #[cfg(feature = "profile")]
    pub fn profile() -> Profile {
        unsafe { PROFILE }
    }

    /// Resets the global profile instance to its initial state,
    /// effectively clearing all of the gathered metrics.
    #[cfg(feature = "profile")]
    pub fn reset_profile() {
        unsafe {
            PROFILE = Profile::new();
        }
    }

    /// Dumps the performance metrics gathered by the global
    /// profile instance into stdout.
    #[cfg(feature = "profile")]
    pub fn dump_profile() {
        println!("Dumping Boytacean profile:");
        println!("{}", Self::profile());
    }
}

#[cfg(feature = "pedantic")]
#[macro_export]
macro_rules! enable_pedantic {
    () => {
        unsafe {
            $crate::diag::PEDANTIC = true;
        }
    };
}

#[cfg(not(feature = "pedantic"))]
#[macro_export]
macro_rules! enable_pedantic {
    () => {};
}

#[cfg(feature = "pedantic")]
#[macro_export]
macro_rules! disable_pedantic {
    () => {
        unsafe {
            $crate::diag::PEDANTIC = false;
        }
    };
}

#[cfg(not(feature = "pedantic"))]
#[macro_export]
macro_rules! disable_pedantic {
    () => {};
}

#[macro_export]
macro_rules! panic_gb {
    ($msg:expr) => {
        {
            $crate::gb::GameBoy::dump_diagnostics();
            panic!($msg);
        }
    };
    ($fmt:expr, $($arg:tt)*) => {
        {
            $crate::gb::GameBoy::dump_diagnostics();
            panic!($fmt, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! assert_gb {
    ($cond:expr, $fmt:expr, $($arg:tt)*) => {
        if !$cond {
            if !$cond {
                $crate::gb::GameBoy::dump_diagnostics();
                panic!($fmt, $($arg)*);
            }
        }
    };
    ($cond:expr) => {
        assert_gb!($cond, stringify!($cond));
    };
}

#[cfg(feature = "pedantic")]
#[macro_export]
macro_rules! assert_pedantic_gb {
    ($cond:expr, $fmt:expr, $($arg:tt)*) => {
        if unsafe { $crate::diag::PEDANTIC } {
            $crate::assert_gb!($cond, $fmt, $($arg)*);
        }
    };
    ($cond:expr) => {
        if unsafe { $crate::diag::PEDANTIC } {
            $crate::assert_gb!($cond);
        }
    };
}

#[cfg(not(feature = "pedantic"))]
#[macro_export]
macro_rules! assert_pedantic_gb {
    ($cond:expr, $fmt:expr, $($arg:tt)*) => {
        ()
    };
    ($cond:expr) => {
        ()
    };
}

#[cfg(feature = "profile")]
#[macro_export]
macro_rules! profile_time_gb {
    ($field:ident, $expr:expr) => {{
        let instant = std::time::Instant::now();
        let result = $expr;
        unsafe {
            $crate::diag::PROFILE.$field += instant.elapsed().as_nanos() as u64;
        }
        result
    }};
}

#[cfg(not(feature = "profile"))]
#[macro_export]
macro_rules! profile_time_gb {
    ($field:ident, $expr:expr) => {
        $expr
    };
}

#[cfg(feature = "profile")]
#[macro_export]
macro_rules! profile_count_gb {
    ($field:ident) => {
        unsafe {
            $crate::diag::PROFILE.$field += 1;
        }
    };
}

#[cfg(not(feature = "profile"))]
#[macro_export]
macro_rules! profile_count_gb {
    ($field:ident) => {
        ()
    };
}

#[cfg(feature = "profile")]
#[macro_export]
macro_rules! profile_read_gb {
    ($addr:expr) => {{
        let addr = $addr;
        unsafe {
            match addr {
                0x0000..=0x7fff => $crate::diag::PROFILE.reads_rom += 1,
                0x8000..=0x9fff => $crate::diag::PROFILE.reads_vram += 1,
                0xa000..=0xbfff => $crate::diag::PROFILE.reads_eram += 1,
                0xc000..=0xfdff => $crate::diag::PROFILE.reads_wram += 1,
                0xfe00..=0xfe9f => $crate::diag::PROFILE.reads_oam += 1,
                0xff80..=0xfffe => $crate::diag::PROFILE.reads_hram += 1,
                _ => $crate::diag::PROFILE.reads_io += 1,
            }
        }
    }};
}

#[cfg(not(feature = "profile"))]
#[macro_export]
macro_rules! profile_read_gb {
    ($addr:expr) => {
        ()
    };
}

#[cfg(feature = "profile")]
#[macro_export]
macro_rules! profile_write_gb {
    ($addr:expr) => {{
        let addr = $addr;
        unsafe {
            match addr {
                0x0000..=0x7fff => $crate::diag::PROFILE.writes_rom += 1,
                0x8000..=0x9fff => $crate::diag::PROFILE.writes_vram += 1,
                0xa000..=0xbfff => $crate::diag::PROFILE.writes_eram += 1,
                0xc000..=0xfdff => $crate::diag::PROFILE.writes_wram += 1,
                0xfe00..=0xfe9f => $crate::diag::PROFILE.writes_oam += 1,
                0xff80..=0xfffe => $crate::diag::PROFILE.writes_hram += 1,
                _ => $crate::diag::PROFILE.writes_io += 1,
            }
        }
    }};
}

#[cfg(not(feature = "profile"))]
#[macro_export]
macro_rules! profile_write_gb {
    ($addr:expr) => {
        ()
    };
}

#[cfg(all(test, feature = "profile"))]
mod tests {
    use super::Profile;
    use crate::gb::GameBoy;

    /// Tests that the render, read and write totals are computed
    /// correctly from the individual metric values.
    #[test]
    fn test_totals() {
        let mut profile = Profile::new();
        profile.render_bg_time = 2;
        profile.render_obj_time = 3;
        profile.reads_rom = 1;
        profile.reads_wram = 2;
        profile.reads_hram = 3;
        profile.writes_vram = 4;
        profile.writes_oam = 5;
        profile.writes_io = 6;

        assert_eq!(profile.render_time(), 5);
        assert_eq!(profile.total_reads(), 6);
        assert_eq!(profile.total_writes(), 15);
    }

    /// Tests that the description of the profile contains the
    /// complete set of gathered metrics.
    #[test]
    fn test_description() {
        let mut profile = Profile::new();
        profile.frame_time = 1000000;
        profile.frames = 60;
        profile.instructions = 1024;

        let description = profile.description();
        assert!(description.contains("Frame"));
        assert!(description.contains("Render BG"));
        assert!(description.contains("Render OBJ"));
        assert!(description.contains("Frames        60"));
        assert!(description.contains("Instructions  1024"));
        assert!(description.contains("Reads"));
        assert!(description.contains("Writes"));
    }

    /// Tests that the profile macros update the global profile
    /// instance and that the reset operation clears it.
    #[test]
    fn test_profile_macros() {
        GameBoy::reset_profile();

        profile_count_gb!(instructions);
        let result = profile_time_gb!(frame_time, 42);
        profile_read_gb!(0xc000u16);
        profile_write_gb!(0xff80u16);

        let profile = GameBoy::profile();
        assert_eq!(result, 42);
        assert_eq!(profile.instructions, 1);
        assert_eq!(profile.reads_wram, 1);
        assert_eq!(profile.writes_hram, 1);

        GameBoy::reset_profile();
        let profile = GameBoy::profile();
        assert_eq!(profile.instructions, 0);
        assert_eq!(profile.reads_wram, 0);
        assert_eq!(profile.writes_hram, 0);
    }
}
