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
macro_rules! warnln {
    ($($rest:tt)*) => {
        {
            std::print!("[WARNING] ");
            std::println!($($rest)*);
        }
    }
}
