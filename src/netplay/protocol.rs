//! Netplay protocol definitions.
//!
//! This module defines all the message types used for network communication
//! in netplay sessions for serial/link cable data transfer.

use std::io::{Cursor, Read, Write};

use boytacean_common::{
    data::{read_u16, read_u64, read_u8, write_u16, write_u64, write_u8},
    error::Error,
};

/// Protocol version for compatibility checking.
pub const PROTOCOL_VERSION: u16 = 1;

/// Magic bytes for message framing ("BOYN").
pub const PROTOCOL_MAGIC: [u8; 4] = [0x42, 0x4f, 0x59, 0x4e];

/// Maximum message size in bytes (64KB).
pub const MAX_MESSAGE_SIZE: usize = 65536;

/// Role of this player in the netplay session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetplayRole {
    /// Host player - accepts connections and controls session.
    Host,
    /// Client player - connects to host.
    Client,
}

/// Current state of the netplay session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetplayState {
    /// Not connected to any session.
    Disconnected,
    /// Attempting to establish connection.
    Connecting,
    /// Actively playing.
    Playing,
}

/// Message types for netplay communication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetplayMessage {
    /// Initial handshake from client to host.
    Hello {
        /// Protocol version.
        version: u16,
        /// MD5 hash of ROM for verification.
        rom_hash: [u8; 16],
    },

    /// Handshake acknowledgment from host to client.
    HelloAck {
        /// Unique session identifier.
        session_id: u64,
        /// Assigned player ID (1 = host, 2 = client).
        player_id: u8,
    },

    /// Serial byte for link cable emulation.
    SerialByte {
        /// The byte being transferred.
        byte: u8,
    },

    /// Sync byte for link cable emulation.
    SyncByte {
        /// The byte being transferred.
        byte: u8,
    },

    /// Request for sync byte from peer (master requests slave's SB value).
    SyncRequest,

    /// Latency measurement ping.
    Ping {
        /// Timestamp when ping was sent.
        timestamp: u64,
    },

    /// Latency measurement pong.
    Pong {
        /// Original timestamp from ping.
        timestamp: u64,
    },

    /// Graceful disconnect notification.
    Disconnect,
}

/// Events emitted by the netplay system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetplayEvent {
    /// Connection established.
    Connected { player_id: u8 },
    /// Connection lost or terminated.
    Disconnected { reason: String },
    /// Serial byte received from remote.
    SerialReceived { byte: u8 },
    /// Sync data received from remote.
    SyncDataReceived { byte: u8 },
    /// Sync request received from remote (master wants our SB value).
    SyncRequested,
    /// Latency measurement updated.
    LatencyUpdate { latency_ms: u32 },
}

impl NetplayMessage {
    /// Message type identifier for serialization.
    pub fn message_type(&self) -> u8 {
        match self {
            NetplayMessage::Hello { .. } => 0x01,
            NetplayMessage::HelloAck { .. } => 0x02,
            NetplayMessage::SerialByte { .. } => 0x07,
            NetplayMessage::SyncByte { .. } => 0x08,
            NetplayMessage::SyncRequest => 0x09,
            NetplayMessage::Ping { .. } => 0x0b,
            NetplayMessage::Pong { .. } => 0x0c,
            NetplayMessage::Disconnect => 0x0d,
        }
    }

    /// Serialize message to bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        let mut cursor = Cursor::new(Vec::new());

        // Write magic header
        cursor.write_all(&PROTOCOL_MAGIC)?;

        // Write message type
        write_u8(&mut cursor, self.message_type())?;

        // Write message-specific payload
        match self {
            NetplayMessage::Hello { version, rom_hash } => {
                write_u16(&mut cursor, *version)?;
                cursor.write_all(rom_hash)?;
            }
            NetplayMessage::HelloAck {
                session_id,
                player_id,
            } => {
                write_u64(&mut cursor, *session_id)?;
                write_u8(&mut cursor, *player_id)?;
            }
            NetplayMessage::SerialByte { byte } => {
                write_u8(&mut cursor, *byte)?;
            }
            NetplayMessage::SyncByte { byte } => {
                write_u8(&mut cursor, *byte)?;
            }
            NetplayMessage::SyncRequest => {}
            NetplayMessage::Ping { timestamp } => {
                write_u64(&mut cursor, *timestamp)?;
            }
            NetplayMessage::Pong { timestamp } => {
                write_u64(&mut cursor, *timestamp)?;
            }
            NetplayMessage::Disconnect => {}
        }

        Ok(cursor.into_inner())
    }

    /// Deserialize message from bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 5 {
            return Err(Error::InvalidData);
        }

        let mut cursor = Cursor::new(data);

        // Verify magic header
        let mut magic = [0u8; 4];
        cursor.read_exact(&mut magic)?;
        if magic != PROTOCOL_MAGIC {
            return Err(Error::InvalidData);
        }

        // Read message type
        let msg_type = read_u8(&mut cursor)?;

        // Parse message-specific payload
        match msg_type {
            0x01 => {
                let version = read_u16(&mut cursor)?;
                let mut rom_hash = [0u8; 16];
                cursor.read_exact(&mut rom_hash)?;
                Ok(NetplayMessage::Hello { version, rom_hash })
            }
            0x02 => {
                let session_id = read_u64(&mut cursor)?;
                let player_id = read_u8(&mut cursor)?;
                Ok(NetplayMessage::HelloAck {
                    session_id,
                    player_id,
                })
            }
            0x07 => {
                let byte = read_u8(&mut cursor)?;
                Ok(NetplayMessage::SerialByte { byte })
            }
            0x08 => {
                let byte = read_u8(&mut cursor)?;
                Ok(NetplayMessage::SyncByte { byte })
            }
            0x09 => Ok(NetplayMessage::SyncRequest),
            0x0b => {
                let timestamp = read_u64(&mut cursor)?;
                Ok(NetplayMessage::Ping { timestamp })
            }
            0x0c => {
                let timestamp = read_u64(&mut cursor)?;
                Ok(NetplayMessage::Pong { timestamp })
            }
            0x0d => Ok(NetplayMessage::Disconnect),
            _ => Err(Error::InvalidData),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_hello() {
        let msg = NetplayMessage::Hello {
            version: PROTOCOL_VERSION,
            rom_hash: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        };

        let data = msg.serialize().unwrap();
        let decoded = NetplayMessage::deserialize(&data).unwrap();

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_serialize_deserialize_all_simple() {
        let messages = vec![
            NetplayMessage::SerialByte { byte: 0xab },
            NetplayMessage::SyncByte { byte: 0xcd },
            NetplayMessage::SyncRequest,
            NetplayMessage::Ping {
                timestamp: 123456789,
            },
            NetplayMessage::Pong {
                timestamp: 987654321,
            },
            NetplayMessage::Disconnect,
        ];

        for msg in messages {
            let data = msg.serialize().unwrap();
            let decoded = NetplayMessage::deserialize(&data).unwrap();
            assert_eq!(msg, decoded);
        }
    }

    #[test]
    fn test_invalid_magic() {
        let data = [0x00, 0x00, 0x00, 0x00, 0x01];
        let result = NetplayMessage::deserialize(&data);
        assert!(result.is_err());
    }
}
