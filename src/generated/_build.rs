//
// EVERYTHING BELOW THIS POINT WAS AUTO-GENERATED DURING COMPILATION. DO NOT MODIFY.
//
#[doc=r#"The Continuous Integration platform detected during compilation."#]
#[allow(dead_code)]
pub static CI_PLATFORM: Option<&str> = None;
#[doc=r#"The full version."#]
#[allow(dead_code)]
pub static PKG_VERSION: &str = "0.11.3";
#[doc=r#"The major version."#]
#[allow(dead_code)]
pub static PKG_VERSION_MAJOR: &str = "0";
#[doc=r#"The minor version."#]
#[allow(dead_code)]
pub static PKG_VERSION_MINOR: &str = "11";
#[doc=r#"The patch version."#]
#[allow(dead_code)]
pub static PKG_VERSION_PATCH: &str = "3";
#[doc=r#"The pre-release version."#]
#[allow(dead_code)]
pub static PKG_VERSION_PRE: &str = "";
#[doc=r#"A colon-separated list of authors."#]
#[allow(dead_code)]
pub static PKG_AUTHORS: &str = "Jo\u{e3}o Magalh\u{e3}es <joamag@gmail.com>";
#[doc=r#"The name of the package."#]
#[allow(dead_code)]
pub static PKG_NAME: &str = "boytacean";
#[doc=r#"The description."#]
#[allow(dead_code)]
pub static PKG_DESCRIPTION: &str = "A Game Boy emulator that is written in Rust.";
#[doc=r#"The homepage."#]
#[allow(dead_code)]
pub static PKG_HOMEPAGE: &str = "";
#[doc=r#"The license."#]
#[allow(dead_code)]
pub static PKG_LICENSE: &str = "Apache-2.0";
#[doc=r#"The source repository as advertised in Cargo.toml."#]
#[allow(dead_code)]
pub static PKG_REPOSITORY: &str = "https://github.com/joamag/boytacean";
#[doc=r#"The target triple that was being compiled for."#]
#[allow(dead_code)]
pub static TARGET: &str = "x86_64-pc-windows-msvc";
#[doc=r#"The host triple of the rust compiler."#]
#[allow(dead_code)]
pub static HOST: &str = "x86_64-pc-windows-msvc";
#[doc=r#"`release` for release builds, `debug` for other builds."#]
#[allow(dead_code)]
pub static PROFILE: &str = "debug";
#[doc=r#"The compiler that cargo resolved to use."#]
#[allow(dead_code)]
pub static RUSTC: &str = "rustc";
#[doc=r#"The documentation generator that cargo resolved to use."#]
#[allow(dead_code)]
pub static RUSTDOC: &str = "rustdoc";
#[doc=r#"Value of OPT_LEVEL for the profile used during compilation."#]
#[allow(dead_code)]
pub static OPT_LEVEL: &str = "0";
#[doc=r#"The parallelism that was specified during compilation."#]
#[allow(dead_code)]
pub static NUM_JOBS: u32 = 12;
#[doc=r#"Value of DEBUG for the profile used during compilation."#]
#[allow(dead_code)]
pub static DEBUG: bool = true;
#[doc=r#"The features that were enabled during compilation."#]
#[allow(dead_code)]
pub static FEATURES: [&str; 1] = ["DEFAULT"];
#[doc=r#"The features as a comma-separated string."#]
#[allow(dead_code)]
pub static FEATURES_STR: &str = "DEFAULT";
#[doc=r#"The features as above, as lowercase strings."#]
#[allow(dead_code)]
pub static FEATURES_LOWERCASE: [&str; 1] = ["default"];
#[doc=r#"The feature-string as above, from lowercase strings."#]
#[allow(dead_code)]
pub static FEATURES_LOWERCASE_STR: &str = "default";
#[doc=r#"The output of `rustc -V`"#]
#[allow(dead_code)]
pub static RUSTC_VERSION: &str = "rustc 1.85.0 (4d91de4e4 2025-02-17)";
#[doc=r#"The output of `rustdoc -V`; empty string if `rustdoc -V` failed to execute"#]
#[allow(dead_code)]
pub static RUSTDOC_VERSION: &str = "rustdoc 1.85.0 (4d91de4e4 2025-02-17)";
#[doc=r#"The target architecture, given by `CARGO_CFG_TARGET_ARCH`."#]
#[allow(dead_code)]
pub static CFG_TARGET_ARCH: &str = "x86_64";
#[doc=r#"The endianness, given by `CARGO_CFG_TARGET_ENDIAN`."#]
#[allow(dead_code)]
pub static CFG_ENDIAN: &str = "little";
#[doc=r#"The toolchain-environment, given by `CARGO_CFG_TARGET_ENV`."#]
#[allow(dead_code)]
pub static CFG_ENV: &str = "msvc";
#[doc=r#"The OS-family, given by `CARGO_CFG_TARGET_FAMILY`."#]
#[allow(dead_code)]
pub static CFG_FAMILY: &str = "windows";
#[doc=r#"The operating system, given by `CARGO_CFG_TARGET_OS`."#]
#[allow(dead_code)]
pub static CFG_OS: &str = "windows";
#[doc=r#"The pointer width, given by `CARGO_CFG_TARGET_POINTER_WIDTH`."#]
#[allow(dead_code)]
pub static CFG_POINTER_WIDTH: &str = "64";
#[doc=r#"An array of effective dependencies as documented by `Cargo.lock`."#]
#[allow(dead_code)]
pub static DEPENDENCIES: [(&str, &str); 181] = [("adler2", "2.0.0"), ("aho-corasick", "1.1.3"), ("android-tzdata", "0.1.1"), ("android_system_properties", "0.1.5"), ("anes", "0.1.6"), ("anstream", "0.6.18"), ("anstyle", "1.0.10"), ("anstyle-parse", "0.2.6"), ("anstyle-query", "1.1.2"), ("anstyle-wincon", "3.0.7"), ("autocfg", "1.4.0"), ("bit_field", "0.10.2"), ("bitflags", "1.3.2"), ("bitflags", "2.9.0"), ("boytacean", "0.11.3"), ("boytacean-common", "0.11.3"), ("boytacean-encoding", "0.11.3"), ("boytacean-hashing", "0.11.3"), ("boytacean-libretro", "0.11.3"), ("boytacean-sdl", "0.11.3"), ("built", "0.7.7"), ("bumpalo", "3.17.0"), ("bytemuck", "1.22.0"), ("byteorder", "1.5.0"), ("c_vec", "2.0.0"), ("cargo-lock", "10.1.0"), ("cast", "0.3.0"), ("cc", "1.2.16"), ("cfg-if", "1.0.0"), ("chrono", "0.4.40"), ("ciborium", "0.2.2"), ("ciborium-io", "0.2.2"), ("ciborium-ll", "0.2.2"), ("clap", "4.5.31"), ("clap_builder", "4.5.31"), ("clap_derive", "4.5.28"), ("clap_lex", "0.7.4"), ("color_quant", "1.1.0"), ("colorchoice", "1.0.3"), ("core-foundation-sys", "0.8.7"), ("crc32fast", "1.4.2"), ("criterion", "0.5.1"), ("criterion-plot", "0.5.0"), ("crossbeam-deque", "0.8.6"), ("crossbeam-epoch", "0.9.18"), ("crossbeam-utils", "0.8.21"), ("crunchy", "0.2.3"), ("displaydoc", "0.2.5"), ("either", "1.14.0"), ("equivalent", "1.0.2"), ("exr", "1.73.0"), ("fdeflate", "0.3.7"), ("flate2", "1.1.0"), ("form_urlencoded", "1.2.1"), ("gif", "0.13.1"), ("half", "2.4.1"), ("hashbrown", "0.15.2"), ("heck", "0.4.1"), ("heck", "0.5.0"), ("hermit-abi", "0.4.0"), ("iana-time-zone", "0.1.61"), ("iana-time-zone-haiku", "0.1.2"), ("icu_collections", "1.5.0"), ("icu_locid", "1.5.0"), ("icu_locid_transform", "1.5.0"), ("icu_locid_transform_data", "1.5.0"), ("icu_normalizer", "1.5.0"), ("icu_normalizer_data", "1.5.0"), ("icu_properties", "1.5.1"), ("icu_properties_data", "1.5.0"), ("icu_provider", "1.5.0"), ("icu_provider_macros", "1.5.0"), ("idna", "1.0.3"), ("idna_adapter", "1.2.0"), ("image", "0.24.9"), ("indexmap", "2.7.1"), ("indoc", "2.0.6"), ("is-terminal", "0.4.15"), ("is_terminal_polyfill", "1.70.1"), ("itertools", "0.10.5"), ("itoa", "1.0.15"), ("jpeg-decoder", "0.3.1"), ("js-sys", "0.3.77"), ("lazy_static", "1.5.0"), ("lebe", "0.5.2"), ("libc", "0.2.170"), ("litemap", "0.7.5"), ("lock_api", "0.4.12"), ("log", "0.4.26"), ("memchr", "2.7.4"), ("memoffset", "0.9.1"), ("miniz_oxide", "0.8.5"), ("num-traits", "0.2.19"), ("once_cell", "1.20.3"), ("oorandom", "11.1.4"), ("parking_lot", "0.12.3"), ("parking_lot_core", "0.9.10"), ("percent-encoding", "2.3.1"), ("plotters", "0.3.7"), ("plotters-backend", "0.3.7"), ("plotters-svg", "0.3.7"), ("png", "0.17.16"), ("portable-atomic", "1.11.0"), ("proc-macro2", "1.0.94"), ("pyo3", "0.20.3"), ("pyo3-build-config", "0.20.3"), ("pyo3-ffi", "0.20.3"), ("pyo3-macros", "0.20.3"), ("pyo3-macros-backend", "0.20.3"), ("qoi", "0.4.1"), ("quote", "1.0.39"), ("rayon", "1.10.0"), ("rayon-core", "1.12.1"), ("redox_syscall", "0.5.10"), ("regex", "1.11.1"), ("regex-automata", "0.4.9"), ("regex-syntax", "0.8.5"), ("rustversion", "1.0.20"), ("ryu", "1.0.20"), ("same-file", "1.0.6"), ("scopeguard", "1.2.0"), ("sdl2", "0.36.0"), ("sdl2-sys", "0.36.0"), ("semver", "1.0.26"), ("serde", "1.0.218"), ("serde_derive", "1.0.218"), ("serde_json", "1.0.140"), ("serde_spanned", "0.6.8"), ("shlex", "1.3.0"), ("simd-adler32", "0.3.7"), ("smallvec", "1.14.0"), ("stable_deref_trait", "1.2.0"), ("strsim", "0.11.1"), ("syn", "2.0.99"), ("synstructure", "0.13.1"), ("target-lexicon", "0.12.16"), ("tiff", "0.9.1"), ("tinystr", "0.7.6"), ("tinytemplate", "1.2.1"), ("toml", "0.8.20"), ("toml_datetime", "0.6.8"), ("toml_edit", "0.22.24"), ("unicode-ident", "1.0.18"), ("unindent", "0.2.4"), ("url", "2.5.4"), ("utf16_iter", "1.0.5"), ("utf8_iter", "1.0.4"), ("utf8parse", "0.2.2"), ("vcpkg", "0.2.15"), ("version-compare", "0.1.1"), ("walkdir", "2.5.0"), ("wasm-bindgen", "0.2.100"), ("wasm-bindgen-backend", "0.2.100"), ("wasm-bindgen-macro", "0.2.100"), ("wasm-bindgen-macro-support", "0.2.100"), ("wasm-bindgen-shared", "0.2.100"), ("web-sys", "0.3.77"), ("weezl", "0.1.8"), ("winapi-util", "0.1.9"), ("windows-core", "0.52.0"), ("windows-link", "0.1.0"), ("windows-sys", "0.59.0"), ("windows-targets", "0.52.6"), ("windows_aarch64_gnullvm", "0.52.6"), ("windows_aarch64_msvc", "0.52.6"), ("windows_i686_gnu", "0.52.6"), ("windows_i686_gnullvm", "0.52.6"), ("windows_i686_msvc", "0.52.6"), ("windows_x86_64_gnu", "0.52.6"), ("windows_x86_64_gnullvm", "0.52.6"), ("windows_x86_64_msvc", "0.52.6"), ("winnow", "0.7.3"), ("write16", "1.0.0"), ("writeable", "0.5.5"), ("yoke", "0.7.5"), ("yoke-derive", "0.7.5"), ("zerofrom", "0.1.6"), ("zerofrom-derive", "0.1.6"), ("zerovec", "0.10.4"), ("zerovec-derive", "0.10.3"), ("zune-inflate", "0.2.54")];
#[doc=r#"The effective dependencies as a comma-separated string."#]
#[allow(dead_code)]
pub static DEPENDENCIES_STR: &str = "adler2 2.0.0, aho-corasick 1.1.3, android-tzdata 0.1.1, android_system_properties 0.1.5, anes 0.1.6, anstream 0.6.18, anstyle 1.0.10, anstyle-parse 0.2.6, anstyle-query 1.1.2, anstyle-wincon 3.0.7, autocfg 1.4.0, bit_field 0.10.2, bitflags 1.3.2, bitflags 2.9.0, boytacean 0.11.3, boytacean-common 0.11.3, boytacean-encoding 0.11.3, boytacean-hashing 0.11.3, boytacean-libretro 0.11.3, boytacean-sdl 0.11.3, built 0.7.7, bumpalo 3.17.0, bytemuck 1.22.0, byteorder 1.5.0, c_vec 2.0.0, cargo-lock 10.1.0, cast 0.3.0, cc 1.2.16, cfg-if 1.0.0, chrono 0.4.40, ciborium 0.2.2, ciborium-io 0.2.2, ciborium-ll 0.2.2, clap 4.5.31, clap_builder 4.5.31, clap_derive 4.5.28, clap_lex 0.7.4, color_quant 1.1.0, colorchoice 1.0.3, core-foundation-sys 0.8.7, crc32fast 1.4.2, criterion 0.5.1, criterion-plot 0.5.0, crossbeam-deque 0.8.6, crossbeam-epoch 0.9.18, crossbeam-utils 0.8.21, crunchy 0.2.3, displaydoc 0.2.5, either 1.14.0, equivalent 1.0.2, exr 1.73.0, fdeflate 0.3.7, flate2 1.1.0, form_urlencoded 1.2.1, gif 0.13.1, half 2.4.1, hashbrown 0.15.2, heck 0.4.1, heck 0.5.0, hermit-abi 0.4.0, iana-time-zone 0.1.61, iana-time-zone-haiku 0.1.2, icu_collections 1.5.0, icu_locid 1.5.0, icu_locid_transform 1.5.0, icu_locid_transform_data 1.5.0, icu_normalizer 1.5.0, icu_normalizer_data 1.5.0, icu_properties 1.5.1, icu_properties_data 1.5.0, icu_provider 1.5.0, icu_provider_macros 1.5.0, idna 1.0.3, idna_adapter 1.2.0, image 0.24.9, indexmap 2.7.1, indoc 2.0.6, is-terminal 0.4.15, is_terminal_polyfill 1.70.1, itertools 0.10.5, itoa 1.0.15, jpeg-decoder 0.3.1, js-sys 0.3.77, lazy_static 1.5.0, lebe 0.5.2, libc 0.2.170, litemap 0.7.5, lock_api 0.4.12, log 0.4.26, memchr 2.7.4, memoffset 0.9.1, miniz_oxide 0.8.5, num-traits 0.2.19, once_cell 1.20.3, oorandom 11.1.4, parking_lot 0.12.3, parking_lot_core 0.9.10, percent-encoding 2.3.1, plotters 0.3.7, plotters-backend 0.3.7, plotters-svg 0.3.7, png 0.17.16, portable-atomic 1.11.0, proc-macro2 1.0.94, pyo3 0.20.3, pyo3-build-config 0.20.3, pyo3-ffi 0.20.3, pyo3-macros 0.20.3, pyo3-macros-backend 0.20.3, qoi 0.4.1, quote 1.0.39, rayon 1.10.0, rayon-core 1.12.1, redox_syscall 0.5.10, regex 1.11.1, regex-automata 0.4.9, regex-syntax 0.8.5, rustversion 1.0.20, ryu 1.0.20, same-file 1.0.6, scopeguard 1.2.0, sdl2 0.36.0, sdl2-sys 0.36.0, semver 1.0.26, serde 1.0.218, serde_derive 1.0.218, serde_json 1.0.140, serde_spanned 0.6.8, shlex 1.3.0, simd-adler32 0.3.7, smallvec 1.14.0, stable_deref_trait 1.2.0, strsim 0.11.1, syn 2.0.99, synstructure 0.13.1, target-lexicon 0.12.16, tiff 0.9.1, tinystr 0.7.6, tinytemplate 1.2.1, toml 0.8.20, toml_datetime 0.6.8, toml_edit 0.22.24, unicode-ident 1.0.18, unindent 0.2.4, url 2.5.4, utf16_iter 1.0.5, utf8_iter 1.0.4, utf8parse 0.2.2, vcpkg 0.2.15, version-compare 0.1.1, walkdir 2.5.0, wasm-bindgen 0.2.100, wasm-bindgen-backend 0.2.100, wasm-bindgen-macro 0.2.100, wasm-bindgen-macro-support 0.2.100, wasm-bindgen-shared 0.2.100, web-sys 0.3.77, weezl 0.1.8, winapi-util 0.1.9, windows-core 0.52.0, windows-link 0.1.0, windows-sys 0.59.0, windows-targets 0.52.6, windows_aarch64_gnullvm 0.52.6, windows_aarch64_msvc 0.52.6, windows_i686_gnu 0.52.6, windows_i686_gnullvm 0.52.6, windows_i686_msvc 0.52.6, windows_x86_64_gnu 0.52.6, windows_x86_64_gnullvm 0.52.6, windows_x86_64_msvc 0.52.6, winnow 0.7.3, write16 1.0.0, writeable 0.5.5, yoke 0.7.5, yoke-derive 0.7.5, zerofrom 0.1.6, zerofrom-derive 0.1.6, zerovec 0.10.4, zerovec-derive 0.10.3, zune-inflate 0.2.54";
//
// EVERYTHING ABOVE THIS POINT WAS AUTO-GENERATED DURING COMPILATION. DO NOT MODIFY.
//
