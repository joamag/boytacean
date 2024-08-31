//! Implementation of multiple devices using serial transfer (Link Cable) .
//!
//! Some of the devices are purely virtual and are used for testing purposes
//! (eg: [`buffer`] and [`buffer`]) while others emulate physical devices that can be connected
//! to the Game Boy (eg: [`printer`]).

pub mod buffer;
pub mod printer;
pub mod stdout;
