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
pub use self::build::{
    COMPILATION_DATE, COMPILATION_TIME, COMPILER, COMPILER_VERSION, FEATURES_SEQ, HOST, MAKEFLAGS,
    NAME, OPT_LEVEL, PLATFORM_CPU_BITS, PLATFORM_CPU_BITS_INT, PROFILE, TARGET, VERSION,
};

#[rustfmt::skip]
#[cfg(not(feature = "gen-mock"))]
pub mod _build;
#[cfg(not(feature = "gen-mock"))]
pub use self::_build::{DEPENDENCIES, DEPENDENCIES_STR, FEATURES, FEATURES_STR};

pub fn dependencies_map() -> HashMap<&'static str, &'static str> {
    HashMap::from(DEPENDENCIES)
}
