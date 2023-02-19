//! Constants that define the current build and execution environment.

#[cfg(feature = "readonly")]
pub mod mock;
#[cfg(feature = "readonly")]
pub use self::mock::*;

#[rustfmt::skip]
#[cfg(not(feature = "readonly"))]
pub mod build;
#[cfg(not(feature = "readonly"))]
pub use self::build::*;
