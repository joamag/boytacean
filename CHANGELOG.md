# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

*

### Changed

* New default demo ROM

### Fixed

*

## [0.5.3] - 2022-11-15

### Changed

* New default demo ROM

## [0.5.2] - 2022-11-14

### Added

* Support for Gamepad Web API - [#9](https://gitlab.stage.hive.pt/joamag/boytacean/-/issues/9)
* Support for palette changing using GET param - [#10](https://gitlab.stage.hive.pt/joamag/boytacean/-/issues/10)

### Fixed

* Start and Select buttons order

## [0.5.1] - 2022-11-14

### Changed

* Small cosmetic changes

## [0.5.0] - 2022-11-14

### Added

* Support for true fullscreen at a browser level
* Support for more flexible palette colors
* Support for setting palette colors using WASM
* Local storage usage for saving battery backed RAM

## [0.4.5] - 2022-11-12

### Fixed

* Critical error that prevented physical keyboard from working ⌨️

## [0.4.4] - 2022-11-12

### Added

* Support for responsive physical keyboard

## [0.4.3] - 2022-11-11

### Added

* Better debug panel support
* Support for some `GET` parameters
* Support for fullscreen on screen keyboard mode

## [0.4.2] - 2022-11-09

### Fixed

* Arrow keys usage for on-screen gamepad
* Wrong UX for keyboard focus and fullscreen

## [0.4.1] - 2022-11-06

### Added

* Logic frequency control using on click UI and keyboard
* Support for on screen keyboard for Game Boy
* Support for remote ROM loading using URL - [#3](https://gitlab.stage.hive.pt/joamag/boytacean/-/issues/3)

## [0.4.0] - 2022-11-01

### Added

* A whole new layout implemented using React.JS 🔥
* Instant boot support using the `GameBoy.boot()` method
* Support for pending cycles in web version

### Changed

* Improved drawing speed at the SDL example
* Better handling of `panic!()` in web version

### Fixed

* Issue related to STAT interrupt and H-Blank
* Issue related to overflow in sprite drawing
* Issue related to the RAM bank selection in some of the MBCs

## [0.3.0] - 2022-07-11

### Added

* Support for 8x16 sprites
* Support for MBC5, think Pokemon Yellow

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
