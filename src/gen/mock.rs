//! Global constants, such as compiler version used, features, platform information and others.

pub const COMPILATION_DATE: &str = "-";
pub const COMPILATION_TIME: &str = "-";
pub const NAME: &str = "-";
pub const VERSION: &str = "x.x.x";
pub const COMPILER: &str = "rustc";
pub const COMPILER_VERSION: &str = "x.x.x";
pub const HOST: &str = "-";
pub const TARGET: &str = "-";
pub const PROFILE: &str = "-";
pub const OPT_LEVEL: &str = "-";
pub const MAKEFLAGS: &str = "-";
pub const FEATURES_SEQ: [&str; 1] = ["cpu"];
pub const PLATFORM_CPU_BITS: &str = "64";
pub const PLATFORM_CPU_BITS_INT: usize = 64;

pub const FEATURES: [&str; 0] = [];
pub const FEATURES_STR: &str = r"";
pub const DEPENDENCIES: [(&str, &str); 0] = [];
pub const DEPENDENCIES_STR: &str = r"";
