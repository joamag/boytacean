#[cfg(feature = "debug")]
#[macro_export]
macro_rules! debugln {
    ($($rest:tt)*) => {
        std::println!($($rest)*)
    }
}

#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! debugln {
    ($($rest:tt)*) => {
        ()
    };
}
