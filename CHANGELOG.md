# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

* Support for 8x16 sprites
* Support for MBC5, think Pokemon Yellow

### Changed

*

### Fixed

* Issue with MBC1 and Advanced ROM Banking Mode
* Issue related to LDC power of and return mode

## [0.2.0] - 2022-07-10

### Added

* Support for drag and drop loading in SDL
* SDL fixes related to timing
* Support for drawing windows
* Initial experimental support for MBC3 (for Pokemon Red/Blue)

### Fixed

* Timer related issue, made test on inst timing pass
* Clear first frame issue, with `first_frame` flag

## [0.1.1] - 2022-07-08

### Fixed

* License name in the `Cargo.toml` file

## [0.1.0] - 2022-07-08

### Added

* Support for sprite drawing, works with Tetris
* Support for timers
* Initial working version 🥳

### Fixed

* Problem in the switching of the LCD mode
