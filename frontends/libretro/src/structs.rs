use std::ffi::{c_char, c_float, c_uchar, c_uint, c_void};

#[repr(C)]
pub struct RetroGameInfo {
    pub path: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub meta: *const c_char,
}

#[repr(C)]
pub struct RetroGameInfoExt {
    pub full_path: *const c_char,
    pub archive_path: *const c_char,
    pub archive_file: *const c_char,
    pub dir: *const c_char,
    pub name: *const c_char,
    pub ext: *const c_char,
    pub meta: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub file_in_archive: c_uchar,
    pub persistent_data: c_uchar,
}

#[repr(C)]
pub struct RetroSystemInfo {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: c_uchar,
    pub block_extract: c_uchar,
}

#[repr(C)]
pub struct RetroGameGeometry {
    pub base_width: c_uint,
    pub base_height: c_uint,
    pub max_width: c_uint,
    pub max_height: c_uint,
    pub aspect_ratio: c_float,
}

#[repr(C)]
pub struct RetroSystemAvInfo {
    pub geometry: RetroGameGeometry,
    pub timing: RetroSystemTiming,
}

#[repr(C)]
pub struct RetroSystemTiming {
    pub fps: f64,
    pub sample_rate: f64,
}

#[repr(C)]
pub struct RetroVariable {
    pub key: *const c_char,
    pub value: *const c_char,
}

#[repr(C)]
pub struct RetroSystemContentInfoOverride {
    pub extensions: *const c_char,
    pub need_fullpath: c_uchar,
    pub persistent_data: c_uchar,
}
