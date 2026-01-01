//! Network-based serial device for link cable emulation over network.
//!
//! This device implements the [`SerialDevice`] trait to provide network-based
//! link cable functionality for netplay multiplayer games like Pokemon trading
//! and Tetris vs.

use std::{any::Any, collections::VecDeque};

use crate::{infoln, serial::SerialDevice};

/// Callback type for network events.
pub type NetworkCallback = fn(NetworkEvent);

/// Events emitted by the network device.
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// A byte was sent to the remote.
    ByteSent(u8),
    /// A byte was received from the remote.
    ByteReceived(u8),
    /// The receive buffer is empty when trying to read.
    BufferEmpty,
}

/// Network-based serial device for link cable emulation.
///
/// This device buffers bytes for send/receive over a network connection.
/// The actual network transport is handled externally (by [`NetplaySession`]),
/// and this device just manages the byte queues.
///
/// # Usage
///
/// ```ignore
/// use boytacean::devices::network::NetworkDevice;
///
/// let mut device = NetworkDevice::new();
///
/// // Queue a byte received from network
/// device.queue_received(0xAB);
///
/// // When the Game Boy reads, it gets the queued byte
/// let byte = device.send(); // Returns 0xAB
///
/// // When the Game Boy sends, get it from pending queue
/// device.receive(0xCD);
/// let sent = device.pop_pending(); // Returns Some(0xCD)
/// ```
pub struct NetworkDevice {
    /// Bytes received from the network, ready to be read by the Game Boy.
    receive_buffer: VecDeque<u8>,

    /// Bytes sent by the Game Boy, pending to be sent over network.
    pending_buffer: VecDeque<u8>,

    /// Callback for device events.
    callback: Option<NetworkCallback>,

    /// Default byte to return when buffer is empty.
    default_byte: u8,

    /// Whether this device is connected.
    connected: bool,

    /// Whether this device acts as master (internal clock).
    /// When true, this side drives the serial clock.
    /// When false, this side acts as slave (external clock from remote).
    is_master: bool,

    /// Optional byte transformation function for protocol-specific handling.
    /// Used for game-specific responses (e.g., Tetris player 2 handshake).
    byte_transform: Option<fn(u8, bool) -> u8>,

    /// Statistics: bytes sent.
    bytes_sent: u64,

    /// Statistics: bytes received.
    bytes_received: u64,
}

impl NetworkDevice {
    pub fn new() -> Self {
        Self {
            receive_buffer: VecDeque::new(),
            pending_buffer: VecDeque::new(),
            callback: None,
            default_byte: 0xff,
            connected: false,
            is_master: true,
            byte_transform: None,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    /// Sets whether this device acts as master (drives the clock).
    pub fn set_master(&mut self, is_master: bool) {
        self.is_master = is_master;
    }

    /// Checks if this device acts as master (drives the clock).
    pub fn is_master(&self) -> bool {
        self.is_master
    }

    /// Sets a byte transformation function for protocol-specific handling.
    ///
    /// The function receives the byte being sent and whether this device
    /// is master, returning the transformed byte.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Tetris player 2 handshake: transform 0x55 poll to 0x29 response
    /// device.set_byte_transform(|byte, is_master| {
    ///     if !is_master && byte == 0x55 { 0x29 } else { byte }
    /// });
    /// ```
    pub fn set_byte_transform(&mut self, transform: fn(u8, bool) -> u8) {
        self.byte_transform = Some(transform);
    }

    /// Clears the byte transformation function.
    pub fn clear_byte_transform(&mut self) {
        self.byte_transform = None;
    }

    /// Sets the callback for device events.
    pub fn set_callback(&mut self, callback: NetworkCallback) {
        self.callback = Some(callback);
    }

    pub fn clear_callback(&mut self) {
        self.callback = None;
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Sets the default byte returned when the receive buffer is empty.
    pub fn set_default_byte(&mut self, byte: u8) {
        self.default_byte = byte;
    }

    /// Queues a byte received from the network.
    ///
    /// This byte will be returned the next time the Game Boy reads from
    /// the serial port.
    pub fn queue_received(&mut self, byte: u8) {
        infoln!("[NETWORK] Queued received byte: 0x{:02x}", byte);
        self.receive_buffer.push_back(byte);
        self.bytes_received += 1;
        if let Some(callback) = self.callback {
            callback(NetworkEvent::ByteReceived(byte));
        }
    }

    /// Pops a byte from the pending send buffer.
    ///
    /// Returns the next byte that the Game Boy has sent and needs to be
    /// transmitted over the network.
    pub fn pop_pending(&mut self) -> Option<u8> {
        self.pending_buffer.pop_front()
    }

    pub fn drain_pending(&mut self) -> Vec<u8> {
        self.pending_buffer.drain(..).collect()
    }

    pub fn has_received(&self) -> bool {
        !self.receive_buffer.is_empty()
    }

    pub fn has_pending(&self) -> bool {
        !self.pending_buffer.is_empty()
    }

    pub fn receive_buffer_len(&self) -> usize {
        self.receive_buffer.len()
    }

    pub fn pending_buffer_len(&self) -> usize {
        self.pending_buffer.len()
    }

    pub fn clear(&mut self) {
        self.receive_buffer.clear();
        self.pending_buffer.clear();
    }

    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }

    pub fn bytes_received(&self) -> u64 {
        self.bytes_received
    }

    pub fn reset_stats(&mut self) {
        self.bytes_sent = 0;
        self.bytes_received = 0;
    }
}

impl SerialDevice for NetworkDevice {
    fn send(&mut self) -> u8 {
        match self.receive_buffer.pop_front() {
            Some(byte) => {
                infoln!(
                    "[NETWORK] [send()] Handles a received byte internally: 0x{:02x}",
                    byte
                );
                byte
            }
            None => {
                infoln!(
                    "[NETWORK] [send()] No byte to be sent out, returning default byte: 0x{:02x}",
                    self.default_byte
                );
                if let Some(callback) = self.callback {
                    callback(NetworkEvent::BufferEmpty);
                }
                self.default_byte
            }
        }
    }

    fn receive(&mut self, byte: u8) {
        infoln!(
            "[NETWORK] [receive()] Queued byte to be sent out: 0x{:02x}",
            byte
        );
        let actual_byte = match self.byte_transform {
            Some(transform) => transform(byte, self.is_master),
            None => byte,
        };
        self.pending_buffer.push_back(actual_byte);
        self.bytes_sent += 1;
        if let Some(callback) = self.callback {
            callback(NetworkEvent::ByteSent(actual_byte));
        }
    }

    fn is_ready(&self) -> bool {
        !self.receive_buffer.is_empty()
    }

    fn force_clock(&self) -> Option<bool> {
        // Force clock mode based on master/slave designation
        // In Tetris, both Game Boys default to external clock (slave mode)
        // waiting for the other to drive. We need one to be master.
        // The host acts as master (internal clock), client as slave (external clock).
        Some(self.is_master)
    }

    fn description(&self) -> String {
        format!(
            "Network(connected={}, recv={}, pend={})",
            self.connected,
            self.receive_buffer.len(),
            self.pending_buffer.len()
        )
    }

    fn state(&self) -> String {
        format!(
            "sent={}, recv={}, pending={}",
            self.bytes_sent,
            self.bytes_received,
            self.pending_buffer.len()
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Default for NetworkDevice {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_device_basic() {
        let mut device = NetworkDevice::new();

        // Initially empty
        assert!(!device.has_received());
        assert!(!device.has_pending());

        // Queue some received bytes
        device.queue_received(0xab);
        device.queue_received(0xcd);

        assert!(device.has_received());
        assert_eq!(device.receive_buffer_len(), 2);

        // Read them back
        assert_eq!(device.send(), 0xab);
        assert_eq!(device.send(), 0xcd);
        assert_eq!(device.send(), 0xff);
    }

    #[test]
    fn test_network_device_send() {
        let mut device = NetworkDevice::new();

        // Game Boy sends bytes
        device.receive(0x11);
        device.receive(0x22);
        device.receive(0x33);

        assert!(device.has_pending());
        assert_eq!(device.pending_buffer_len(), 3);

        // Pop them for network transmission
        assert_eq!(device.pop_pending(), Some(0x11));
        assert_eq!(device.pop_pending(), Some(0x22));
        assert_eq!(device.pop_pending(), Some(0x33));
        assert_eq!(device.pop_pending(), None);
    }

    #[test]
    fn test_network_device_drain() {
        let mut device = NetworkDevice::new();

        device.receive(0xaa);
        device.receive(0xbb);
        device.receive(0xcc);

        let drained = device.drain_pending();
        assert_eq!(drained, vec![0xaa, 0xbb, 0xcc]);
        assert!(!device.has_pending());
    }

    #[test]
    fn test_network_device_statistics() {
        let mut device = NetworkDevice::new();

        device.queue_received(0x01);
        device.queue_received(0x02);
        device.receive(0x03);
        device.receive(0x04);
        device.receive(0x05);

        assert_eq!(device.bytes_received(), 2);
        assert_eq!(device.bytes_sent(), 3);

        device.reset_stats();
        assert_eq!(device.bytes_received(), 0);
        assert_eq!(device.bytes_sent(), 0);
    }

    #[test]
    fn test_network_device_custom_default() {
        let mut device = NetworkDevice::new();
        device.set_default_byte(0x00);

        assert_eq!(device.send(), 0x00); // Custom default
    }

    #[test]
    fn test_network_device_clear() {
        let mut device = NetworkDevice::new();

        device.queue_received(0x11);
        device.receive(0x22);

        device.clear();

        assert!(!device.has_received());
        assert!(!device.has_pending());
    }
}
