# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

* Support for CGB flag parsing

### Changed

*

### Fixed

* Major JoyPad issue with Action/Select read in register

## [0.7.2] - 2023-03-04

### Added

* Support for stereo sound 🔊

### Changed

* APU `clock()` method with `cycles` parameter, improving performance by an order of magnitude 💪

### Fixed

* Added reset of APU, which fixes annoying "garbage" data in buffer when restarting the state of the emulator

## [0.7.1] - 2023-03-02

### Changed

* Bumped emukit, fixing a lot of bugs

## [0.7.0] - 2023-03-01

### Added

* Support for Audio 🔈!!! - [#12](https://gitlab.stage.hive.pt/joamag/boytacean/-/issues/12)
* Support for WASM engine version printing

## [0.6.12] - 2023-02-21

### Fixed

* Build process for the docs.rs website

## [0.6.11] - 2023-02-21

### Fixed

* Unused code issue

## [0.6.10] - 2023-02-21

### Fixed

* Old compilation problem with `NUM_CPUS` generation

## [0.6.9] - 2023-02-21

### Changed

* Bumped emukit dependency

## [0.6.8] - 2023-02-21

### Added

* Support for the `build.rs` generation file that creates the `gen.rs` file
* Support for benchmark in the SDL frontend
* Palette switching for the SDL frontend

### Fixed

* Bug with ROM title that included 0x0 characters in it
* V-Sync issue with SDL

## [0.6.7] - 2023-02-13

### Changed

* Bumped base emukit version

## [0.6.6] - 2022-12-04

### Added

* Support for theme and palette selection
* Theme stored in `localStorage`

## [0.6.5] - 2022-11-27

### Added

* Canonical URL support for boytacean.joao.me

### Changed

* Small help changes regarding Gamepad

## [0.6.4] - 2022-11-22

### Fixed

* Emukit version bump

## [0.6.3] - 2022-11-21

### Fixed

* Emukit version bump

## [0.6.2] - 2022-11-21

### Changed

* Made UI generic by extracting components into [EmuKit](https://github.com/joamag/emukit) 🎉
* More generic help panels

## [0.6.1] - 2022-11-19

### Fixed

* Exclusion of files from `Cargo.toml`

## [0.6.0] - 2022-11-19

### Added

* Support for Ctrl+D (Speedup) and Ctrl+K (Keyboard toggle) shortcuts
* Initial help panel
* Palette debugging panel

### Fixed

* Android highlight color in buttons
* Android issue with arrow pointers

## [0.5.7] - 2022-11-17

### Fixed

* More issues related with bad PPU handling

## [0.5.6] - 2022-11-17

### Fixed

* Issue with background color and change of palette colors
* Issue related with STAT interrupt not being triggered for all conditions

## [0.5.5] - 2022-11-17

### Fixed

* PPU issue related to the maximum number of objects/sprite per line being 10, issue detected by ACID test
* Object pixel drawing priority issue, issue detected by ACID test
* Issue associated with the wrongful flipping of 8x16 sprites, issue detected by ACID test
* Issue associated with drawing of window tiles, due to extra `update_stat()` operations, issue detected by ACID test

## [0.5.4] - 2022-11-15

### Fixed

* Critical issue with loading of Boot ROM

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

* Arrow keys usage for on-screen Gamepad
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
