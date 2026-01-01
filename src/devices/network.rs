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
    /// A byte was sent to the remote peer.
    ByteSent(u8),
    /// A byte was received from the remote peer.
    ByteReceived(u8),
    /// A sync payload request was received from the remote peer.
    SyncData(u8),
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
    /// The value of the SB register (0xFF01) on the other/peer device.
    ///
    /// Useful for master devices to handle incoming bytes from the peer.
    peer_sp: Option<u8>,

    /// Bytes sent by the Game Boy, pending to be sent over network.
    send_buffer: VecDeque<u8>,

    /// Bytes sent by the Game Boy, pending to be sent over network.
    send_sync_buffer: VecDeque<u8>,

    /// Bytes received from the network, ready to be read by the Game Boy.
    receive_buffer: VecDeque<u8>,

    /// Callback for device events.
    callback: Option<NetworkCallback>,

    /// Default byte to return when buffer is empty.
    default_byte: u8,

    /// Whether this device is connected.
    connected: bool,
}

impl NetworkDevice {
    pub fn new() -> Self {
        Self {
            peer_sp: None,
            send_buffer: VecDeque::new(),
            send_sync_buffer: VecDeque::new(),
            receive_buffer: VecDeque::new(),
            callback: None,
            default_byte: 0xff,
            connected: false,
        }
    }

    /// Queues a byte received from the network.
    ///
    /// This byte will be returned the next time the Game Boy reads from
    /// the serial port.
    pub fn queue_received(&mut self, byte: u8) {
        infoln!("[NETWORK] Queued received byte: 0x{:02x}", byte);

        self.receive_buffer.push_back(byte);

        // unsets the peer SP since we're receiving a new byte from
        // the master device, no longer needed to keep peer state
        self.peer_sp = None;

        if let Some(callback) = self.callback {
            callback(NetworkEvent::ByteReceived(byte));
        }
    }

    /// Queues a sync byte received from the network.
    ///
    /// This byte will be used to set the peer SP register value.
    pub fn queue_sync_received(&mut self, byte: u8) {
        infoln!("[NETWORK] Set peer SP with received byte: 0x{:02x}", byte);
        self.peer_sp = Some(byte);
        if let Some(callback) = self.callback {
            callback(NetworkEvent::SyncData(byte));
        }
    }

    /// Queues a sync byte to be sent out.
    ///
    /// This byte will be sent over the network to the remote peer.
    pub fn send_sync(&mut self, byte: u8) {
        infoln!(
            "[NETWORK] [send_sync()]  Queued sync byte to be sent out: 0x{:02x}",
            byte
        );
        self.send_sync_buffer.push_back(byte);
    }

    /// Pops a byte from the pending send buffer.
    ///
    /// Returns the next byte that the Game Boy has sent and needs to be
    /// transmitted over the network.
    pub fn pop_send(&mut self) -> Option<u8> {
        self.send_buffer.pop_front()
    }

    pub fn has_pending(&self) -> bool {
        !self.send_buffer.is_empty()
    }

    pub fn send_buffer_len(&self) -> usize {
        self.send_buffer.len()
    }

    pub fn pop_sync(&mut self) -> Option<u8> {
        self.send_sync_buffer.pop_front()
    }

    pub fn has_pending_sync(&self) -> bool {
        !self.send_sync_buffer.is_empty()
    }

    pub fn send_sync_buffer_len(&self) -> usize {
        self.send_sync_buffer.len()
    }

    pub fn receive_buffer_len(&self) -> usize {
        self.receive_buffer.len()
    }

    pub fn has_received(&self) -> bool {
        !self.receive_buffer.is_empty()
    }

    pub fn clear(&mut self) {
        self.send_buffer.clear();
        self.send_sync_buffer.clear();
        self.receive_buffer.clear();
    }

    /// Sets the callback for device events.
    pub fn set_callback(&mut self, callback: NetworkCallback) {
        self.callback = Some(callback);
    }

    pub fn clear_callback(&mut self) {
        self.callback = None;
    }

    /// Sets the default byte returned when the receive buffer is empty.
    pub fn set_default_byte(&mut self, byte: u8) {
        self.default_byte = byte;
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

impl SerialDevice for NetworkDevice {
    fn send(&mut self) -> u8 {
        // TODO: this sounds like a hack we need to better define the
        // way we're going to handle the sync data.
        if let Some(byte) = self.peer_sp.take() {
            infoln!(
                "[NETWORK] [send()] Handles a peer SP byte internally: 0x{:02x}",
                byte
            );
            return byte;
        }

        match self.receive_buffer.pop_front() {
            Some(byte) => {
                infoln!(
                    "[NETWORK] [send()] Handles a received byte internally: 0x{:02x}",
                    byte
                );
                self.receive_buffer.clear(); // TODO: this also sounds like a hack, but makes sense only use the last byte
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
        self.send_buffer.push_back(byte);
        if let Some(callback) = self.callback {
            callback(NetworkEvent::ByteSent(byte));
        }
    }

    fn sync(&mut self, clock_mode: bool, data: u8) {
        if clock_mode {
        } else {
            self.send_sync(data);
        }
    }

    fn is_ready(&self) -> bool {
        !self.receive_buffer.is_empty()
    }

    fn description(&self) -> String {
        format!(
            "Network [connected={}, recv={}, pend={}]",
            self.connected,
            self.receive_buffer.len(),
            self.send_buffer.len()
        )
    }

    fn state(&self) -> String {
        format!(
            "send_buffer={}, receive_buffer={}",
            self.send_buffer.len(),
            self.receive_buffer.len()
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
        assert_eq!(device.send_buffer_len(), 3);

        // Pop them for network transmission
        assert_eq!(device.pop_send(), Some(0x11));
        assert_eq!(device.pop_send(), Some(0x22));
        assert_eq!(device.pop_send(), Some(0x33));
        assert_eq!(device.pop_send(), None);
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
