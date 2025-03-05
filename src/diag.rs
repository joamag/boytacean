//! Low-level diagnostic utilities for debugging purposes.
//!
//! Some of the implementations make use of unsafe code to store
//! a global instance of the emulator, which is going to be used
//! in panic diagnostics

use std::ptr::null;

use crate::gb::GameBoy;

/// Static mutable reference to the global instance of the
/// Game Boy emulator, going to be used for global diagnostics.
static mut GLOBAL_INSTANCE: *const GameBoy = null();

// Static mutable flag to enable or disable pedantic diagnostics.
#[cfg(feature = "pedantic")]
pub static mut PEDANTIC: bool = true;

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
