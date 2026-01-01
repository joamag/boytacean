//! Netplay session management.
//!
//! This module provides the main [`NetplaySession`] struct that orchestrates
//! the netplay lifecycle for serial/link cable communication.

use std::{collections::VecDeque, time::Instant};

use boytacean_common::error::Error;

use crate::{
    infoln,
    netplay::{
        connection::NetplayConnection,
        protocol::{NetplayEvent, NetplayMessage, NetplayRole, NetplayState, PROTOCOL_VERSION},
    },
};

/// Configuration for a netplay session.
#[derive(Debug, Clone)]
pub struct NetplayConfig {
    /// Connection timeout in milliseconds.
    pub timeout_ms: u32,

    /// Role (host or client).
    pub role: NetplayRole,

    /// ROM hash for verification.
    pub rom_hash: [u8; 16],
}

impl Default for NetplayConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 5000,
            role: NetplayRole::Host,
            rom_hash: [0; 16],
        }
    }
}

/// Main netplay session manager.
///
/// Manages the connection and serial byte transfer between two players.
pub struct NetplaySession {
    /// Session configuration.
    config: NetplayConfig,

    /// Current session state.
    state: NetplayState,

    /// Network connection to peer.
    connection: Box<dyn NetplayConnection>,

    /// Unique session identifier.
    session_id: u64,

    /// This player's ID (1 = host, 2 = client).
    player_id: u8,

    /// Pending events to be processed.
    pending_events: VecDeque<NetplayEvent>,

    /// Ping history for latency estimation.
    ping_history: VecDeque<u32>,

    /// Average latency in milliseconds.
    avg_latency_ms: u32,

    /// Last ping sent timestamp.
    last_ping_time: Option<Instant>,

    /// Last ping sequence.
    last_ping_timestamp: u64,

    /// Serial byte receive queue.
    serial_recv_queue: VecDeque<u8>,

    /// Sync data receive queue.
    sync_recv_queue: VecDeque<u8>,
}

impl NetplaySession {
    /// Create a new netplay session.
    pub fn new(config: NetplayConfig, connection: Box<dyn NetplayConnection>) -> Self {
        let session_id = if config.role == NetplayRole::Host {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64
        } else {
            0
        };

        let player_id = match config.role {
            NetplayRole::Host => 1,
            NetplayRole::Client => 2,
        };

        Self {
            config,
            state: NetplayState::Connecting,
            connection,
            session_id,
            player_id,
            pending_events: VecDeque::new(),
            ping_history: VecDeque::with_capacity(10),
            avg_latency_ms: 0,
            last_ping_time: None,
            last_ping_timestamp: 0,
            serial_recv_queue: VecDeque::new(),
            sync_recv_queue: VecDeque::new(),
        }
    }

    /// Get the current session state.
    pub fn state(&self) -> NetplayState {
        self.state
    }

    /// Get this player's ID.
    pub fn player_id(&self) -> u8 {
        self.player_id
    }

    /// Get the session ID.
    pub fn session_id(&self) -> u64 {
        self.session_id
    }

    /// Get the average latency in milliseconds.
    pub fn latency_ms(&self) -> u32 {
        self.avg_latency_ms
    }

    /// Starts the handshake process.
    ///
    /// Sends the Hello message to the remote player.
    pub fn start_handshake(&mut self) -> Result<(), Error> {
        match self.config.role {
            NetplayRole::Client => {
                self.connection.send(&NetplayMessage::Hello {
                    version: PROTOCOL_VERSION,
                    rom_hash: self.config.rom_hash,
                })?;
            }
            NetplayRole::Host => {}
        }
        Ok(())
    }

    /// Process incoming messages and update state.
    pub fn poll(&mut self) -> Result<Vec<NetplayEvent>, Error> {
        let mut events = Vec::new();

        // processes all pending messages
        while let Some(msg) = self.connection.recv()? {
            self.handle_message(msg, &mut events)?;
        }

        // Check connection status
        if !self.connection.is_connected() && self.state != NetplayState::Disconnected {
            self.state = NetplayState::Disconnected;
            events.push(NetplayEvent::Disconnected {
                reason: "Connection lost".to_string(),
            });
        }

        // Add any pending events
        while let Some(event) = self.pending_events.pop_front() {
            events.push(event);
        }

        Ok(events)
    }

    /// Handle a received message from the remote player.
    ///
    /// Updates the session state and emits events based on
    /// the received message.
    fn handle_message(
        &mut self,
        msg: NetplayMessage,
        events: &mut Vec<NetplayEvent>,
    ) -> Result<(), Error> {
        match msg {
            NetplayMessage::Hello { version, rom_hash } => {
                if self.config.role != NetplayRole::Host {
                    return Err(Error::CustomError("Unexpected Hello".to_string()));
                }

                if version != PROTOCOL_VERSION {
                    self.connection.send(&NetplayMessage::Disconnect)?;
                    return Err(Error::CustomError(format!(
                        "Version mismatch: {version} vs {PROTOCOL_VERSION}"
                    )));
                }

                if rom_hash != self.config.rom_hash {
                    self.connection.send(&NetplayMessage::Disconnect)?;
                    return Err(Error::CustomError("ROM mismatch".to_string()));
                }

                self.connection.send(&NetplayMessage::HelloAck {
                    session_id: self.session_id,
                    player_id: 2,
                })?;

                self.state = NetplayState::Playing;
                events.push(NetplayEvent::Connected {
                    player_id: self.player_id,
                });
            }

            NetplayMessage::HelloAck {
                session_id,
                player_id,
            } => {
                if self.config.role != NetplayRole::Client {
                    return Err(Error::CustomError("Unexpected HelloAck".to_string()));
                }

                self.session_id = session_id;
                self.player_id = player_id;
                self.state = NetplayState::Playing;
                events.push(NetplayEvent::Connected {
                    player_id: self.player_id,
                });
            }

            NetplayMessage::SerialByte { byte } => {
                infoln!("[SESSION] Serial byte received: 0x{:02x}", byte);
                self.serial_recv_queue.push_back(byte);
                events.push(NetplayEvent::SerialReceived { byte });
            }

            NetplayMessage::SyncByte { byte } => {
                infoln!("[SESSION] Sync byte received: 0x{:02x}", byte);
                self.sync_recv_queue.push_back(byte);
                events.push(NetplayEvent::SerialReceived { byte });
            }

            NetplayMessage::Ping { timestamp } => {
                self.connection.send(&NetplayMessage::Pong { timestamp })?;
            }

            NetplayMessage::Pong { timestamp } => {
                if timestamp == self.last_ping_timestamp {
                    if let Some(ping_time) = self.last_ping_time {
                        let latency = ping_time.elapsed().as_millis() as u32;
                        self.update_latency(latency);
                        events.push(NetplayEvent::LatencyUpdate {
                            latency_ms: self.avg_latency_ms,
                        });
                    }
                }
            }

            NetplayMessage::Disconnect => {
                self.state = NetplayState::Disconnected;
                events.push(NetplayEvent::Disconnected {
                    reason: "Remote disconnected".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Updates latency estimate with a new sample.
    fn update_latency(&mut self, sample: u32) {
        self.ping_history.push_back(sample);
        if self.ping_history.len() > 10 {
            self.ping_history.pop_front();
        }

        let sum: u32 = self.ping_history.iter().sum();
        self.avg_latency_ms = sum / self.ping_history.len() as u32;
    }

    /// Queue a serial byte to send to the remote.
    pub fn send_serial_byte(&mut self, byte: u8) -> Result<(), Error> {
        infoln!("[SESSION] Serial byte sent: 0x{:02x}", byte);
        self.connection.send(&NetplayMessage::SerialByte { byte })?;
        Ok(())
    }

    /// Queue a sync byte to send to the remote.
    pub fn send_sync_byte(&mut self, byte: u8) -> Result<(), Error> {
        infoln!("[SESSION] Sync byte sent: 0x{:02x}", byte);
        self.connection.send(&NetplayMessage::SyncByte { byte })?;
        Ok(())
    }

    /// Get the next received serial byte.
    pub fn recv_serial_byte(&mut self) -> Option<u8> {
        self.serial_recv_queue.pop_front()
    }

    /// Check if there are pending serial bytes.
    pub fn has_serial_byte(&self) -> bool {
        !self.serial_recv_queue.is_empty()
    }

    /// Send a ping for latency measurement.
    pub fn send_ping(&mut self) -> Result<(), Error> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        self.last_ping_timestamp = timestamp;
        self.last_ping_time = Some(Instant::now());

        self.connection.send(&NetplayMessage::Ping { timestamp })?;

        Ok(())
    }

    /// Gracefully disconnect.
    pub fn disconnect(&mut self) -> Result<(), Error> {
        if self.state != NetplayState::Disconnected {
            let _ = self.connection.send(&NetplayMessage::Disconnect);
            self.connection.close();
            self.state = NetplayState::Disconnected;
        }
        Ok(())
    }

    /// Get the remote address.
    pub fn remote_addr(&self) -> String {
        self.connection.remote_addr()
    }
}

impl Drop for NetplaySession {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    struct MockConnection {
        sent: VecDeque<NetplayMessage>,
        recv_queue: VecDeque<NetplayMessage>,
        connected: bool,
    }

    impl MockConnection {
        fn new() -> Self {
            Self {
                sent: VecDeque::new(),
                recv_queue: VecDeque::new(),
                connected: true,
            }
        }
    }

    impl NetplayConnection for MockConnection {
        fn send(&mut self, message: &NetplayMessage) -> Result<(), Error> {
            self.sent.push_back(message.clone());
            Ok(())
        }

        fn recv(&mut self) -> Result<Option<NetplayMessage>, Error> {
            Ok(self.recv_queue.pop_front())
        }

        fn recv_timeout(&mut self, _timeout: Duration) -> Result<Option<NetplayMessage>, Error> {
            Ok(self.recv_queue.pop_front())
        }

        fn is_connected(&self) -> bool {
            self.connected
        }

        fn close(&mut self) {
            self.connected = false;
        }

        fn latency_ms(&self) -> u32 {
            10
        }

        fn remote_addr(&self) -> String {
            "mock".to_string()
        }
    }

    #[test]
    fn test_session_creation() {
        let conn = MockConnection::new();
        let config = NetplayConfig::default();
        let session = NetplaySession::new(config, Box::new(conn));

        assert_eq!(session.state(), NetplayState::Connecting);
        assert_eq!(session.player_id(), 1);
    }
}
