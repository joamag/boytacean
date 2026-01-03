//! Network-based serial device for link cable emulation over network.
//!
//! This device implements the [`SerialDevice`] trait to provide network-based
//! link cable functionality for netplay multiplayer games like Pokemon trading
//! and Tetris vs.

use std::{any::Any, collections::VecDeque};

use crate::{infoln, serial::SerialDevice};

/// Default byte value returned when the receive buffer is empty.
const DEFAULT_BYTE: u8 = 0xff;

/// Callback type for network events.
pub type NetworkCallback = fn(NetworkEvent);

/// Events emitted by the network device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkEvent {
    /// A byte was sent to the remote peer.
    ByteSent(u8),
    /// A byte was received from the remote peer.
    ByteReceived(u8),
    /// A sync byte was received from the remote peer.
    SyncData(u8),
    /// The receive buffer is empty when trying to read.
    BufferEmpty,
    /// A sync request needs to be sent to the peer (master requests slave's SB).
    SyncRequestNeeded,
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
/// // queue a byte received from network
/// device.queue_received(0xAB);
///
/// // when the Game Boy reads, it gets the queued byte
/// let byte = device.send(); // returns 0xAB
///
/// // when the Game Boy sends, get it from pending queue
/// device.receive(0xCD);
/// let sent = device.pop_send(); // returns Some(0xCD)
/// ```
pub struct NetworkDevice {
    /// The value of the SB register (0xFF01) on the other/peer device.
    peer_sp: Option<u8>,

    /// Saved SB value for slave mode, captured when SC is written.
    saved_sb: Option<u8>,

    /// Bytes sent by the Game Boy, pending network transmission.
    send_buffer: VecDeque<u8>,

    /// Sync bytes pending network transmission.
    send_sync_buffer: VecDeque<u8>,

    /// Bytes received from the network, ready for Game Boy to read.
    receive_buffer: VecDeque<u8>,

    /// Default byte to return when buffer is empty.
    default_byte: u8,

    /// Callback for device events.
    callback: Option<NetworkCallback>,

    /// Whether this device is connected to a peer.
    connected: bool,

    /// Whether a sync request is pending (master waiting for slave's SB value).
    sync_pending: bool,

    /// Whether a sync request needs to be sent over the network.
    sync_request_queued: bool,
}

impl NetworkDevice {
    pub fn new() -> Self {
        Self {
            peer_sp: None,
            saved_sb: None,
            send_buffer: VecDeque::new(),
            send_sync_buffer: VecDeque::new(),
            receive_buffer: VecDeque::new(),
            default_byte: DEFAULT_BYTE,
            callback: None,
            connected: false,
            sync_pending: false,
            sync_request_queued: false,
        }
    }

    pub fn reset(&mut self) {
        self.peer_sp = None;
        self.saved_sb = None;
        self.send_buffer.clear();
        self.send_sync_buffer.clear();
        self.receive_buffer.clear();
        self.default_byte = DEFAULT_BYTE;
        self.connected = false;
        self.sync_pending = false;
        self.sync_request_queued = false;
    }

    pub fn clear(&mut self) {
        self.send_buffer.clear();
        self.send_sync_buffer.clear();
        self.receive_buffer.clear();
        self.sync_pending = false;
        self.sync_request_queued = false;
        self.saved_sb = None;
    }

    /// Queues a byte received from the network.
    ///
    /// This byte will be returned the next time the Game Boy reads from
    /// the serial port.
    pub fn queue_received(&mut self, byte: u8) {
        infoln!("[NETWORK] Queued received byte: 0x{:02x}", byte);
        self.receive_buffer.push_back(byte);

        // clears peer_sp since we're receiving a new byte from the master,
        // no longer needed to keep peer state
        self.peer_sp = None;

        if let Some(callback) = self.callback {
            callback(NetworkEvent::ByteReceived(byte));
        }
    }

    pub fn has_received(&self) -> bool {
        !self.receive_buffer.is_empty()
    }

    pub fn receive_buffer_len(&self) -> usize {
        self.receive_buffer.len()
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

    /// Queues a sync byte received from the network.
    ///
    /// This byte will be used to set the peer SP register value.
    /// Also clears the sync_pending flag since we received the requested sync.
    pub fn queue_sync_received(&mut self, byte: u8) {
        infoln!("[NETWORK] Set peer SP with received byte: 0x{:02x}", byte);
        self.peer_sp = Some(byte);
        self.sync_pending = false;
        if let Some(callback) = self.callback {
            callback(NetworkEvent::SyncData(byte));
        }
    }

    /// Queues a sync byte to be sent over the network.
    pub fn send_sync(&mut self, byte: u8) {
        infoln!(
            "[NETWORK] [send_sync()] Queued sync byte to be sent: 0x{:02x}",
            byte
        );
        self.send_sync_buffer.push_back(byte);
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

    /// Requests a sync from the peer by emitting a SyncRequestNeeded event.
    ///
    /// Sets sync_pending to true to block further transfers until sync arrives.
    pub fn request_sync(&mut self) {
        if self.sync_pending {
            return; // already waiting for sync
        }
        infoln!("[NETWORK] Requesting sync from peer");
        self.sync_pending = true;
        self.sync_request_queued = true;
        if let Some(callback) = self.callback {
            callback(NetworkEvent::SyncRequestNeeded);
        }
    }

    /// Returns and clears the sync request flag.
    ///
    /// Used by the main loop to send sync requests over the network.
    pub fn pop_sync_request(&mut self) -> bool {
        let was_queued = self.sync_request_queued;
        self.sync_request_queued = false;
        was_queued
    }

    pub fn has_sync_request(&self) -> bool {
        self.sync_request_queued
    }

    pub fn is_sync_pending(&self) -> bool {
        self.sync_pending
    }

    pub fn set_sync_pending(&mut self, pending: bool) {
        self.sync_pending = pending;
    }

    pub fn has_peer_sp(&self) -> bool {
        self.peer_sp.is_some()
    }

    /// Returns whether the device is ready for a master transfer.
    ///
    /// For master mode, we need peer_sp available and no pending sync request.
    pub fn is_master_ready(&self) -> bool {
        self.peer_sp.is_some() && !self.sync_pending
    }

    /// Gets the saved SB value (for slave mode sync responses).
    ///
    /// Returns the saved value and clears it.
    pub fn take_saved_sb(&mut self) -> Option<u8> {
        self.saved_sb.take()
    }

    pub fn saved_sb(&self) -> Option<u8> {
        self.saved_sb
    }

    pub fn set_default_byte(&mut self, byte: u8) {
        self.default_byte = byte;
    }

    pub fn default_byte(&self) -> u8 {
        self.default_byte
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn set_callback(&mut self, callback: NetworkCallback) {
        self.callback = Some(callback);
    }

    pub fn clear_callback(&mut self) {
        self.callback = None;
    }
}

impl SerialDevice for NetworkDevice {
    fn send(&mut self) -> u8 {
        // for master mode: uses peer_sp (slave's SB register value) if available,
        // keeps the value so repeated reads return the same value until a new
        // sync byte arrives from the slave
        if let Some(byte) = self.peer_sp {
            infoln!("[NETWORK] [send()] Using peer SP value: 0x{:02x}", byte);
            return byte;
        }

        // for slave mode: reads from receive_buffer (bytes sent by master)
        match self.receive_buffer.pop_front() {
            Some(byte) => {
                infoln!(
                    "[NETWORK] [send()] Received byte from buffer: 0x{:02x}",
                    byte
                );
                byte
            }
            None => {
                infoln!(
                    "[NETWORK] [send()] Buffer empty, returning default: 0x{:02x}",
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
        infoln!("[NETWORK] [receive()] Queued byte to send: 0x{:02x}", byte);
        self.send_buffer.push_back(byte);

        // clears peer_sp after transfer completes so the next transfer
        // requires a fresh sync byte from the slave, this prevents the
        // master from racing ahead using stale data
        self.peer_sp = None;

        if let Some(callback) = self.callback {
            callback(NetworkEvent::ByteSent(byte));
        }
    }

    fn sync(&mut self, clock_mode: bool, data: u8) {
        if clock_mode {
            // master mode: requests sync from slave if peer_sp is not available
            if self.peer_sp.is_none() && !self.sync_pending {
                self.request_sync();
            }
        } else {
            // slave mode: saves the SB value so it can be sent when master requests sync,
            // this captures the value at the moment of SC write (ready for transfer),
            // before it gets overwritten by processing received bytes
            infoln!("[NETWORK] [sync()] Slave saving SB value: 0x{:02x}", data);
            self.saved_sb = Some(data);

            // proactively sends sync byte to master so it often arrives before
            // the master needs it, reducing round-trip latency, the SyncRequest
            // mechanism remains as a fallback if this doesn't arrive in time
            self.send_sync(data);
        }
    }

    fn is_ready(&self) -> bool {
        !self.receive_buffer.is_empty()
    }

    fn awaiting_sync(&self) -> bool {
        self.sync_pending
    }

    fn description(&self) -> String {
        format!(
            "Network [connected={}, recv={}, send={}]",
            self.connected,
            self.receive_buffer.len(),
            self.send_buffer.len()
        )
    }

    fn state(&self) -> String {
        format!(
            "send={}, recv={}, sync={}",
            self.send_buffer.len(),
            self.receive_buffer.len(),
            self.send_sync_buffer.len()
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
    use crate::{devices::network::DEFAULT_BYTE, serial::SerialDevice};

    use super::NetworkDevice;

    #[test]
    fn test_network_device_basic() {
        let mut device = NetworkDevice::new();

        // initially empty
        assert!(!device.has_received());
        assert!(!device.has_pending());

        // queue some received bytes
        device.queue_received(0xab);
        device.queue_received(0xcd);

        assert!(device.has_received());
        assert_eq!(device.receive_buffer_len(), 2);

        // read them back
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

        // pop them for network transmission
        assert_eq!(device.pop_send(), Some(0x11));
        assert_eq!(device.pop_send(), Some(0x22));
        assert_eq!(device.pop_send(), Some(0x33));
        assert_eq!(device.pop_send(), None);
    }

    #[test]
    fn test_network_device_custom_default() {
        let mut device = NetworkDevice::new();
        device.set_default_byte(0x00);

        // returns custom default when buffer is empty
        assert_eq!(device.send(), 0x00);
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

    #[test]
    fn test_network_device_reset() {
        let mut device = NetworkDevice::new();

        device.queue_received(0x11);
        device.receive(0x22);
        device.set_connected(true);
        device.set_default_byte(0x00);

        device.reset();

        assert!(!device.has_received());
        assert!(!device.has_pending());
        assert!(!device.is_connected());
        assert_eq!(device.default_byte(), DEFAULT_BYTE);
    }

    #[test]
    fn test_network_device_sync_request() {
        let mut device = NetworkDevice::new();

        // initially no sync pending
        assert!(!device.is_sync_pending());
        assert!(!device.has_sync_request());

        // request sync
        device.request_sync();

        assert!(device.is_sync_pending());
        assert!(device.has_sync_request());

        // pop the request
        assert!(device.pop_sync_request());
        assert!(!device.has_sync_request());

        // sync still pending until we receive the response
        assert!(device.is_sync_pending());

        // receive sync response
        device.queue_sync_received(0x42);

        assert!(!device.is_sync_pending());
        assert!(device.has_peer_sp());
    }

    #[test]
    fn test_network_device_slave_sync() {
        let mut device = NetworkDevice::new();

        // slave mode: sync saves SB and sends proactively
        device.sync(false, 0x55);

        assert_eq!(device.saved_sb(), Some(0x55));
        assert!(device.has_pending_sync());
        assert_eq!(device.pop_sync(), Some(0x55));
    }

    #[test]
    fn test_network_device_master_sync() {
        let mut device = NetworkDevice::new();

        // master mode without peer_sp: requests sync
        device.sync(true, 0x00);

        assert!(device.is_sync_pending());
        assert!(device.has_sync_request());
    }
}
