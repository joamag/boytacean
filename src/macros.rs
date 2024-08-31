//! Assorted set of macros to be used in the context of Boytacean.

#[cfg(feature = "debug")]
#[macro_export]
macro_rules! debugln {
    ($($rest:tt)*) => {
        {
            std::print!("[DEBUG] ");
            std::println!($($rest)*);
        }
    }
}

#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! debugln {
    ($($rest:tt)*) => {
        ()
    };
}

#[macro_export]
macro_rules! infoln {
    ($($rest:tt)*) => {
        {
            std::print!("[INFO] ");
            std::println!($($rest)*);
        }
    }
}

#[cfg(feature = "pedantic")]
#[macro_export]
macro_rules! warnln {
    ($($rest:tt)*) => {
        {
            $crate::panic_gb!($($rest)*);
        }
    }
}

#[cfg(not(feature = "pedantic"))]
#[macro_export]
macro_rules! warnln {
    ($($rest:tt)*) => {
        {
            std::print!("[WARNING] ");
            std::println!($($rest)*);
        }
    }
}
