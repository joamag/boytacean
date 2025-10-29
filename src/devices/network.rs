use std::{
    fmt::{self, Display, Formatter},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

use boytacean_common::error::Error;

use crate::serial::SerialDevice;

pub enum NetworkMode {
    Client,
    Server,
}

pub struct NetworkDevice {
    stream: TcpStream,
}

impl NetworkDevice {
    pub fn connect(addr: &str) -> Result<Self, Error> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nonblocking(true)?;
        Ok(Self { stream })
    }

    pub fn listen(addr: &str) -> Result<Self, Error> {
        let listener = TcpListener::bind(addr)?;
        let (stream, _) = listener.accept()?;
        stream.set_nonblocking(true)?;
        Ok(Self { stream })
    }
}

impl SerialDevice for NetworkDevice {
    fn send(&mut self) -> u8 {
        let mut buffer = [0u8; 1];
        match self.stream.read_exact(&mut buffer) {
            Ok(_) => buffer[0],
            Err(_) => 0xff,
        }
    }

    fn receive(&mut self, byte: u8) {
        let _ = self.stream.write_all(&[byte]);
    }

    fn allow_slave(&self) -> bool {
        true
    }

    fn description(&self) -> String {
        String::from("Network")
    }

    fn state(&self) -> String {
        String::from("")
    }
}

impl Display for NetworkDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Network")
    }
}
