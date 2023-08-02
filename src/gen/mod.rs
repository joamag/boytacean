//! Constants that define the current build and execution environment.

use std::collections::HashMap;

#[cfg(feature = "gen-mock")]
pub mod mock;
#[cfg(feature = "gen-mock")]
pub use self::mock::*;

#[rustfmt::skip]
#[cfg(not(feature = "gen-mock"))]
pub mod build;
#[cfg(not(feature = "gen-mock"))]
pub use self::build::*;

#[rustfmt::skip]
#[cfg(not(feature = "gen-mock"))]
pub mod _build;
#[cfg(not(feature = "gen-mock"))]
pub use self::_build::*;

pub fn dependencies_map() -> HashMap<&'static str, &'static str> {
    HashMap::from(DEPENDENCIES)
}
