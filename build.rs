#![allow(clippy::uninlined_format_args)]

/// Build script (https://doc.rust-lang.org/cargo/reference/build-scripts.html)
/// This script is executed as the first step in the compilation process.
/// Here we export metadata constants to a `constants/generated.rs` file which is then
/// imported and used by the remaining crate.
///
/// # Examples
///
/// In C you can use the preprocessor macro `__DATE__` to save the compilation date like:
///
/// ```c
/// #define COMPILATION_DATE __DATE__
/// ```
///
/// Rust does not have such preprocessor macros, so we use this script and do:
///
/// ```rust
/// let now_utc = chrono::Utc::now();
/// write_str_constant(
///     &mut file,
///     "COMPILATION_DATE",
///     &format!("{}", now_utc.format("%b %d %Y")),
/// );
/// ```
use chrono::Utc;
use regex::Regex;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::{env, str};

const BUILD_OUT_FILE: &str = "build.rs";
const SOURCE_DIR: &str = "./src/gen";

fn main() {
    // in case we're running under docs.rs then we must return the control
    // flow immediately as it's not possible to generate files under the
    // expected read only file system present in `docs.rs` build environment
    if std::env::var("DOCS_RS").is_ok() {
        return;
    }

    // opens the target destination file panicking with a proper message in
    // case it was not possible to open it (eg: directory inexistent)
    let dest_path = Path::new(SOURCE_DIR).join(Path::new(BUILD_OUT_FILE));
    let mut file = OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open(dest_path)
        .unwrap_or_else(|_| panic!("Can't open '{}'", BUILD_OUT_FILE));

    let module_doc_string = "//! Global constants, such as compiler version used, algorithms, compression and filters supported and others\n";
    writeln!(file, "{}", module_doc_string).unwrap();

    let generated_annotation = "// @generated\n";
    writeln!(file, "{}", generated_annotation).unwrap();

    let now_utc = Utc::now();
    write_str_constant(
        &mut file,
        "COMPILATION_DATE",
        &format!("{}", now_utc.format("%b %d %Y")),
    );
    write_str_constant(
        &mut file,
        "COMPILATION_TIME",
        &format!("{}", now_utc.format("%H:%M:%S")),
    );

    write_str_constant(
        &mut file,
        "NAME",
        option_env!("CARGO_PKG_NAME").unwrap_or("UNKNOWN"),
    );

    write_str_constant(
        &mut file,
        "VERSION",
        option_env!("CARGO_PKG_VERSION").unwrap_or("UNKNOWN"),
    );

    write_str_constant(&mut file, "COMPILER", "rustc");

    let compiler_version = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "UNKNOWN".to_string());
    let re = Regex::new("rustc ([\\d.\\d.\\d]*)").unwrap();
    let compiler_version = re
        .captures(&compiler_version)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str();
    write_str_constant(&mut file, "COMPILER_VERSION", compiler_version);

    write_str_constant(
        &mut file,
        "HOST",
        &env::var("HOST").unwrap_or_else(|_| String::from("UNKNOWN")),
    );

    write_str_constant(
        &mut file,
        "TARGET",
        &env::var("TARGET").unwrap_or_else(|_| String::from("UNKNOWN")),
    );

    write_str_constant(
        &mut file,
        "PROFILE",
        &env::var("PROFILE").unwrap_or_else(|_| String::from("UNKNOWN")),
    );

    write_str_constant(
        &mut file,
        "OPT_LEVEL",
        &env::var("OPT_LEVEL").unwrap_or_else(|_| String::from("UNKNOWN")),
    );

    write_str_constant(
        &mut file,
        "MAKEFLAGS",
        option_env!("CARGO_MAKEFLAGS").unwrap_or("UNKNOWN"),
    );

    let mut features = vec!["cpu"];

    if cfg!(feature = "wasm-extension") {
        features.push("wasm")
    }

    if cfg!(feature = "python-extension") {
        features.push("python")
    }

    write_vec_constant(&mut file, "FEATURES", features);

    write_str_constant(
        &mut file,
        "PLATFORM_CPU_BITS",
        &(std::mem::size_of::<usize>() * 8).to_string(),
    );

    write_constant(&mut file, "DEFAULT_THREAD_POOL_SIZE", num_cpus::get());
    write_constant(&mut file, "MAX_THREAD_POOL_SIZE", num_cpus::get() * 10);
}

fn write_constant<T>(file: &mut File, key: &str, val: T)
where
    T: std::fmt::Display,
{
    writeln!(
        file,
        "pub const {}: {} = {};",
        key,
        std::any::type_name::<T>(),
        val
    )
    .unwrap_or_else(|_| panic!("Failed to write '{}' to 'build_constants.rs'", key));
}

fn write_str_constant(file: &mut File, key: &str, val: &str) {
    writeln!(file, "pub const {}: &str = \"{}\";", key, val)
        .unwrap_or_else(|_| panic!("Failed to write '{}' to 'build_constants.rs'", key));
}

fn write_vec_constant<T>(file: &mut File, key: &str, vec: Vec<T>)
where
    T: std::fmt::Display,
{
    let mut list_str = String::new();
    let mut is_first = true;
    for value in &vec {
        if is_first {
            is_first = false;
        } else {
            list_str.push_str(", ");
        }
        list_str.push_str(format!("\"{}\"", value).as_str());
    }
    writeln!(
        file,
        "pub const {}: [{}; {}] = [{}];",
        key,
        std::any::type_name::<T>(),
        vec.len(),
        list_str
    )
    .unwrap_or_else(|_| panic!("Failed to write '{}' to 'build_constants.rs'", key));
}
