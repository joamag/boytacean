//! Netplay support for multiplayer gaming.
//!
//! This module provides netplay functionality for link cable game
//! emulation (Pokemon trading, Tetris vs, etc.) over TCP.
//!
//! # Architecture
//!
//! The netplay system exchanges serial bytes over the network,
//! allowing link cable games to work over TCP.
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐
//! │   Player 1      │     │   Player 2      │
//! │   (Host)        │     │   (Client)      │
//! ├─────────────────┤     ├─────────────────┤
//! │   GameBoy       │     │   GameBoy       │
//! │   └─ Serial ────┼─────┼── Serial        │
//! │      NetworkDev │ TCP │   NetworkDev    │
//! └─────────────────┘     └─────────────────┘
//! ```
//!
//! # Usage
//!
//! ## Hosting a Session
//!
//! ```ignore
//! use boytacean::netplay::{NetplayConfig, NetplaySession, TcpServer, NetplayRole};
//!
//! let server = TcpServer::bind("0.0.0.0:12345")?;
//! let connection = server.accept_timeout(Duration::from_secs(30))?
//!     .expect("No client connected");
//!
//! let config = NetplayConfig {
//!     role: NetplayRole::Host,
//!     ..Default::default()
//! };
//! let session = NetplaySession::new(config, Box::new(connection));
//! ```
//!
//! ## Joining a Session
//!
//! ```ignore
//! use boytacean::netplay::{NetplayConfig, NetplaySession, TcpConnection, NetplayRole};
//!
//! let connection = TcpConnection::connect("192.168.1.100:12345")?;
//!
//! let config = NetplayConfig {
//!     role: NetplayRole::Client,
//!     ..Default::default()
//! };
//! let mut session = NetplaySession::new(config, Box::new(connection));
//! session.start_handshake()?;
//! ```

pub mod connection;
pub mod protocol;
pub mod session;
