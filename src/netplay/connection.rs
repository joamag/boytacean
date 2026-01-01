//! Network connection abstractions for netplay.
//!
//! This module provides the [`NetplayConnection`] trait and implementations
//! for different transport protocols (TCP, WebSocket).

use std::{
    collections::VecDeque,
    io::{BufReader, BufWriter, Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use boytacean_common::error::Error;

use crate::netplay::protocol::NetplayMessage;

/// Trait for netplay network connections.
///
/// This abstraction allows for different transport implementations
/// (TCP, WebSocket, etc.) to be used interchangeably.
pub trait NetplayConnection: Send {
    /// Send a message to the remote peer.
    fn send(&mut self, message: &NetplayMessage) -> Result<(), Error>;

    /// Receive a message from the remote peer (non-blocking).
    /// Returns None if no message is available.
    fn recv(&mut self) -> Result<Option<NetplayMessage>, Error>;

    /// Receive a message from the remote peer (blocking with timeout).
    fn recv_timeout(&mut self, timeout: Duration) -> Result<Option<NetplayMessage>, Error>;

    /// Check if the connection is still active.
    fn is_connected(&self) -> bool;

    /// Close the connection gracefully.
    fn close(&mut self);

    /// Get the current latency estimate in milliseconds.
    fn latency_ms(&self) -> u32;

    /// Get the remote address as a string.
    fn remote_addr(&self) -> String;
}

/// TCP-based netplay connection.
pub struct TcpConnection {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
    connected: bool,
    latency_ms: u32,
    remote_addr: String,
    read_buffer: Vec<u8>,
}

impl TcpConnection {
    pub fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self, Error> {
        let stream = TcpStream::connect(addr)?;
        Self::from_stream(stream)
    }

    pub fn connect_timeout<A: ToSocketAddrs>(addr: A, timeout: Duration) -> Result<Self, Error> {
        // `ToSocketAddrs` can return multiple addresses, try each one
        let addrs: Vec<_> = addr.to_socket_addrs()?.collect();
        if addrs.is_empty() {
            return Err(Error::CustomError("No addresses found".to_string()));
        }

        let stream = TcpStream::connect_timeout(&addrs[0], timeout)?;
        Self::from_stream(stream)
    }

    /// Creates a new TCP connection from an existing TCP stream (for accepted connections).
    pub fn from_stream(stream: TcpStream) -> Result<Self, Error> {
        let remote_addr = stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;

        let reader = BufReader::new(stream.try_clone()?);
        let writer = BufWriter::new(stream.try_clone()?);

        Ok(Self {
            stream,
            reader,
            writer,
            connected: true,
            latency_ms: 0,
            remote_addr,
            read_buffer: Vec::with_capacity(4096),
        })
    }

    /// Update latency estimate.
    pub fn set_latency(&mut self, latency_ms: u32) {
        self.latency_ms = latency_ms;
    }

    /// Read a length-prefixed message from the stream.
    fn read_message(&mut self) -> Result<Option<NetplayMessage>, Error> {
        // Try to read message length (4 bytes)
        if self.read_buffer.len() < 4 {
            let mut len_buf = [0u8; 4];
            match self.reader.read(&mut len_buf) {
                Ok(0) => {
                    self.connected = false;
                    return Ok(None);
                }
                Ok(n) => {
                    self.read_buffer.extend_from_slice(&len_buf[..n]);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    return Ok(None);
                }
                Err(e) => {
                    self.connected = false;
                    return Err(e.into());
                }
            }
        }

        if self.read_buffer.len() < 4 {
            return Ok(None);
        }

        let msg_len = u32::from_le_bytes([
            self.read_buffer[0],
            self.read_buffer[1],
            self.read_buffer[2],
            self.read_buffer[3],
        ]) as usize;

        if msg_len > super::protocol::MAX_MESSAGE_SIZE {
            self.connected = false;
            return Err(Error::CustomError("Message too large".to_string()));
        }

        // tries to read the full message from the buffer
        let total_len = 4 + msg_len;
        while self.read_buffer.len() < total_len {
            let mut chunk = vec![0u8; total_len - self.read_buffer.len()];
            match self.reader.read(&mut chunk) {
                Ok(0) => {
                    self.connected = false;
                    return Ok(None);
                }
                Ok(n) => {
                    self.read_buffer.extend_from_slice(&chunk[..n]);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    return Ok(None);
                }
                Err(e) => {
                    self.connected = false;
                    return Err(e.into());
                }
            }
        }

        // Parse the message
        let msg_data = &self.read_buffer[4..total_len];
        let message = NetplayMessage::deserialize(msg_data)?;

        // Remove processed data from buffer
        self.read_buffer.drain(..total_len);

        Ok(Some(message))
    }

    /// Write a length-prefixed message to the stream.
    fn write_message(&mut self, message: &NetplayMessage) -> Result<(), Error> {
        let data = message.serialize()?;
        let len = data.len() as u32;

        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(&data)?;
        self.writer.flush()?;

        Ok(())
    }
}

impl NetplayConnection for TcpConnection {
    fn send(&mut self, message: &NetplayMessage) -> Result<(), Error> {
        if !self.connected {
            return Err(Error::CustomError("Not connected".to_string()));
        }
        self.write_message(message)
    }

    fn recv(&mut self) -> Result<Option<NetplayMessage>, Error> {
        if !self.connected {
            return Ok(None);
        }
        self.read_message()
    }

    fn recv_timeout(&mut self, timeout: Duration) -> Result<Option<NetplayMessage>, Error> {
        if !self.connected {
            return Ok(None);
        }

        let start = Instant::now();
        loop {
            match self.read_message()? {
                Some(msg) => return Ok(Some(msg)),
                None => {
                    if start.elapsed() >= timeout {
                        return Ok(None);
                    }
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn close(&mut self) {
        self.connected = false;
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }

    fn latency_ms(&self) -> u32 {
        self.latency_ms
    }

    fn remote_addr(&self) -> String {
        self.remote_addr.clone()
    }
}

/// TCP server for hosting netplay sessions.
pub struct TcpServer {
    listener: TcpListener,
    running: Arc<AtomicBool>,
}

impl TcpServer {
    /// Create a new server listening on the specified address.
    pub fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self, Error> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        Ok(Self {
            listener,
            running: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Accept a new connection (non-blocking).
    pub fn accept(&self) -> Result<Option<TcpConnection>, Error> {
        match self.listener.accept() {
            Ok((stream, _)) => Ok(Some(TcpConnection::from_stream(stream)?)),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Accept a connection with timeout.
    pub fn accept_timeout(&self, timeout: Duration) -> Result<Option<TcpConnection>, Error> {
        let start = Instant::now();
        loop {
            match self.accept()? {
                Some(conn) => return Ok(Some(conn)),
                None => {
                    if start.elapsed() >= timeout {
                        return Ok(None);
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

    /// Get the local address the server is bound to.
    pub fn local_addr(&self) -> Result<String, Error> {
        Ok(self.listener.local_addr()?.to_string())
    }

    /// Stop the server.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if the server is still running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Thread-safe message queue for async communication.
#[derive(Default)]
pub struct MessageQueue {
    queue: Mutex<VecDeque<NetplayMessage>>,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn push(&self, message: NetplayMessage) {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(message);
    }

    pub fn pop(&self) -> Option<NetplayMessage> {
        let mut queue = self.queue.lock().unwrap();
        queue.pop_front()
    }

    pub fn len(&self) -> usize {
        let queue = self.queue.lock().unwrap();
        queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_tcp_server_client() {
        let server = TcpServer::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();

        let addr_clone = addr.clone();
        let client_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            TcpConnection::connect(&addr_clone).unwrap()
        });

        let server_conn = server.accept_timeout(Duration::from_secs(1)).unwrap();
        assert!(server_conn.is_some());

        let client_conn = client_thread.join().unwrap();
        assert!(client_conn.is_connected());
    }

    #[test]
    fn test_message_queue() {
        let queue = MessageQueue::new();

        assert!(queue.is_empty());

        queue.push(NetplayMessage::Ping { timestamp: 123 });
        queue.push(NetplayMessage::Pong { timestamp: 456 });

        assert_eq!(queue.len(), 2);

        let msg1 = queue.pop().unwrap();
        assert!(matches!(msg1, NetplayMessage::Ping { timestamp: 123 }));

        let msg2 = queue.pop().unwrap();
        assert!(matches!(msg2, NetplayMessage::Pong { timestamp: 456 }));

        assert!(queue.is_empty());
    }
}
