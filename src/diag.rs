use std::ptr::null;

use crate::gb::GameBoy;

/// Static mutable reference to the global instance of the
/// Game Boy emulator, going to be used for global diagnostics.
static mut GLOBAL_INSTANCE: *const GameBoy = null();

impl GameBoy {
    /// Sets the current instance as the one going to be used
    /// in panic diagnostics.
    pub fn set_diag(&self) {
        self.set_global();
    }

    /// Dumps the diagnostics for the global instance of the
    /// Boytacean emulator into stdout.
    pub fn dump_diagnostics() {
        if let Some(gb) = Self::get_global() {
            gb.dump_diagnostics_s();
        }
    }

    /// Sets the current instance as the global one going to
    /// be used in panic diagnostics.
    fn set_global(&self) {
        unsafe {
            GLOBAL_INSTANCE = self;
        }
    }

    /// Obtains the global instance of the Game Boy emulator
    /// ready to be used in diagnostics.
    ///
    /// If the global instance is not set, it will return `None`.
    fn get_global() -> Option<&'static Self> {
        unsafe {
            if GLOBAL_INSTANCE.is_null() {
                None
            } else {
                Some(&*GLOBAL_INSTANCE)
            }
        }
    }

    fn dump_diagnostics_s(&self) {
        println!("Dumping Boytacean diagnostics:");
        println!("{}", self.description_debug());
    }
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
        $crate::assert_gb!($cond, $fmt, $($arg)*);
    };
    ($cond:expr) => {
        $crate::assert_gb!($cond);
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
