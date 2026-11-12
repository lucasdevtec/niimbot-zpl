//! Byte transports. USB (CDC serial) now; Bluetooth (BLE) can implement the same trait.
use crate::{Error, Result};
use std::io::{Read, Write};
use std::time::Duration;

pub trait Transport: Send {
    fn write_all(&mut self, data: &[u8]) -> Result<()>;
    /// Reads whatever is available, waiting up to `timeout`. Empty vec on timeout.
    fn read(&mut self, timeout: Duration) -> Result<Vec<u8>>;
}

pub struct UsbSerial {
    port: Box<dyn serialport::SerialPort>,
}

/// Niimbot USB vendor id.
const NIIMBOT_VID: u16 = 0x3513;

/// Lists serial ports, Niimbot-looking ones first.
pub fn list_ports() -> Vec<String> {
    let mut ports = serialport::available_ports().unwrap_or_default();
    ports.sort_by_key(|p| match &p.port_type {
        serialport::SerialPortType::UsbPort(u) if u.vid == NIIMBOT_VID => 0,
        _ => 1,
    });
    ports.into_iter().map(|p| p.port_name).collect()
}

impl UsbSerial {
    pub fn open(path: &str) -> Result<Self> {
        let port = serialport::new(path, 115_200)
            .timeout(Duration::from_millis(500))
            .open()
            .map_err(|e| Error::Transport(e.to_string()))?;
        Ok(Self { port })
    }
}

impl Transport for UsbSerial {
    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        self.port.write_all(data)?;
        self.port.flush()?;
        Ok(())
    }
    fn read(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        self.port
            .set_timeout(timeout)
            .map_err(|e| Error::Transport(e.to_string()))?;
        let mut buf = [0u8; 512];
        match self.port.read(&mut buf) {
            Ok(n) => Ok(buf[..n].to_vec()),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }
}

/// Finds the most likely Niimbot USB port (by vendor id, else a ttyACM device).
pub fn find_niimbot() -> Option<String> {
    let ports = serialport::available_ports().ok()?;
    ports
        .iter()
        .find(|p| matches!(&p.port_type, serialport::SerialPortType::UsbPort(u) if u.vid == NIIMBOT_VID))
        .or_else(|| ports.iter().find(|p| p.port_name.contains("ttyACM")))
        .map(|p| p.port_name.clone())
}
