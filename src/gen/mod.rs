//! Constants that define the current build and execution environment.

#[cfg(feature = "gen-mock")]
pub mod mock;
#[cfg(feature = "gen-mock")]
pub use self::mock::*;

#[rustfmt::skip]
#[cfg(not(feature = "gen-mock"))]
pub mod build;
#[cfg(not(feature = "gen-mock"))]
pub use self::build::*;
