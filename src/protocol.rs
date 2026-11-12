//! Niimbot packet protocol: 55 55 | cmd | len | data | xor | AA AA
use crate::{transport::Transport, Error, Result};
use image::GrayImage;
use std::time::{Duration, Instant};

pub mod cmd {
    pub const SET_LABEL_DENSITY: u8 = 0x21;
    pub const SET_LABEL_TYPE: u8 = 0x23;
    pub const PRINT_START: u8 = 0x01;
    pub const PAGE_START: u8 = 0x03;
    pub const SET_PAGE_SIZE: u8 = 0x13;
    pub const SET_QUANTITY: u8 = 0x15;
    pub const PRINT_EMPTY_ROW: u8 = 0x84;
    pub const PRINT_BITMAP_ROW: u8 = 0x85;
    pub const PAGE_END: u8 = 0xE3;
    pub const PRINT_END: u8 = 0xF3;
    pub const PRINT_STATUS: u8 = 0xA3;
    pub const HEARTBEAT: u8 = 0xDC;
    pub const CALIBRATE_LABEL: u8 = 0x8E;
    pub const GET_INFO: u8 = 0x40;
}

/// B1 printhead: 384 dots (48 bytes) at 203 dpi.
pub const B1_WIDTH_PX: u32 = 384;

/// Reply id for a request: usually `cmd + 1`; density/type answer with `+ 0x10`, status with `0xB3`.
fn reply_id(cmd: u8) -> u8 {
    match cmd {
        cmd::SET_LABEL_DENSITY | cmd::SET_LABEL_TYPE | cmd::PRINT_STATUS => cmd + 0x10,
        c => c + 1,
    }
}

pub fn encode(cmd: u8, data: &[u8]) -> Vec<u8> {
    let mut p = vec![0x55, 0x55, cmd, data.len() as u8];
    p.extend_from_slice(data);
    let x = data.iter().fold(cmd ^ data.len() as u8, |a, b| a ^ b);
    p.extend_from_slice(&[x, 0xAA, 0xAA]);
    p
}

/// Parses packets out of a byte stream. Returns (cmd, data) pairs.
pub fn decode(buf: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut out = vec![];
    let mut i = 0;
    while i + 6 <= buf.len() {
        if buf[i] == 0x55 && buf[i + 1] == 0x55 {
            let (c, l) = (buf[i + 2], buf[i + 3] as usize);
            if i + 6 + l <= buf.len() && buf[i + 4 + l + 1..i + 4 + l + 3] == [0xAA, 0xAA] {
                let d = buf[i + 4..i + 4 + l].to_vec();
                let x = d.iter().fold(c ^ l as u8, |a, b| a ^ b);
                if x == buf[i + 4 + l] {
                    out.push((c, d));
                }
                i += 6 + l;
                continue;
            }
        }
        i += 1;
    }
    out
}

pub struct Printer<T: Transport> {
    t: T,
}

impl<T: Transport> Printer<T> {
    pub fn new(t: T) -> Self {
        Self { t }
    }

    fn send(&mut self, cmd: u8, data: &[u8]) -> Result<()> {
        self.t.write_all(&encode(cmd, data))
    }

    /// Send and wait for the reply. Stale bytes are dropped first so a leftover packet
    /// is never mistaken for this command's answer. Traffic is logged to stderr.
    fn request(&mut self, cmd: u8, data: &[u8]) -> Result<Vec<u8>> {
        self.request_with(cmd, data, None, Duration::from_secs(5))
    }

    /// Like `request`, but when `expect` is set, packets with another command id (e.g.
    /// unsolicited status notifications sent while printing) are logged and skipped.
    fn request_with(
        &mut self,
        cmd: u8,
        data: &[u8],
        expect: Option<u8>,
        wait: Duration,
    ) -> Result<Vec<u8>> {
        while !self.t.read(Duration::from_millis(1))?.is_empty() {}
        self.send(cmd, data)?;
        eprintln!("-> {cmd:#04x} {data:02x?}");
        let deadline = Instant::now() + wait;
        let (mut acc, mut seen) = (vec![], vec![]);
        while Instant::now() < deadline {
            acc.extend(self.t.read(Duration::from_millis(200))?);
            for (c, d) in decode(&acc) {
                if seen.iter().any(|s| *s == (c, d.clone())) {
                    continue;
                }
                eprintln!("<- {c:#04x} {d:02x?}");
                if expect.is_none_or(|e| e == c) {
                    return Ok(d);
                }
                seen.push((c, d));
            }
        }
        // Unknown firmware quirk: if something answered but with an unexpected id, use it
        // (the status byte is still validated by the caller) rather than failing outright.
        if let Some((c, d)) = seen.last() {
            eprintln!("!! no {expect:02x?} reply to {cmd:#04x}; falling back to {c:#04x}");
            return Ok(d.clone());
        }
        Err(Error::Protocol(format!(
            "no response to cmd {cmd:#x}{}",
            if seen.is_empty() {
                String::new()
            } else {
                format!(" (only got {seen:02x?})")
            }
        )))
    }

    /// Like `request`, but expects the usual reply id (`cmd + 1`) and fails if the
    /// printer answers with a non-1 status byte. `wait` covers slow steps like page end.
    fn request_ok(&mut self, cmd: u8, data: &[u8]) -> Result<()> {
        let wait = if cmd == cmd::PAGE_END {
            Duration::from_secs(30)
        } else {
            Duration::from_secs(5)
        };
        let d = self.request_with(cmd, data, Some(reply_id(cmd)), wait)?;
        match d.first() {
            Some(1) => Ok(()),
            _ => Err(Error::Protocol(format!(
                "printer rejected cmd {cmd:#x} (reply {d:02x?})"
            ))),
        }
    }

    /// Reads a device info value (`key`: 10 = battery %, 9 = firmware, 11 = serial, 8 = model).
    pub fn info(&mut self, key: u8) -> Result<u64> {
        let d = self.request(cmd::GET_INFO, &[key])?;
        Ok(d.iter().fold(0u64, |a, b| (a << 8) | *b as u64))
    }

    pub fn battery(&mut self) -> Result<u8> {
        Ok(self.info(10)?.min(100) as u8)
    }

    pub fn firmware(&mut self) -> Result<String> {
        Ok(format!("{:.2}", self.info(9)? as f64 / 100.0))
    }

    /// Gap calibration: the printer feeds a few labels to measure the gap. Manual use only.
    pub fn calibrate(&mut self) -> Result<()> {
        self.request(cmd::CALIBRATE_LABEL, &[1])?;
        std::thread::sleep(Duration::from_millis(4000));
        Ok(())
    }

    pub fn heartbeat(&mut self) -> Result<Vec<u8>> {
        self.request(cmd::HEARTBEAT, &[1])
    }

    /// Prints a 1-bit-ish image (dark pixel < 128). Width must be <= 384; it is
    /// centered on the printhead. Image rows run along the paper feed direction.
    pub fn print(&mut self, img: &GrayImage, density: u8, copies: u16) -> Result<()> {
        let (w, h) = img.dimensions();
        if w > B1_WIDTH_PX {
            return Err(Error::Protocol(format!("image wider than {B1_WIDTH_PX}px")));
        }
        self.request_ok(cmd::SET_LABEL_DENSITY, &[density.clamp(1, 5)])?;
        self.request_ok(cmd::SET_LABEL_TYPE, &[1])?; // 1 = gap label
        self.request_ok(cmd::PRINT_START, &[1])?;
        self.request_ok(cmd::PAGE_START, &[1])?;
        let mut size = (h as u16).to_be_bytes().to_vec();
        size.extend((w as u16).to_be_bytes());
        size.extend(copies.max(1).to_be_bytes());
        self.request_ok(cmd::SET_PAGE_SIZE, &size)?;
        for y in 0..h {
            let mut bytes = vec![0u8; (w as usize).div_ceil(8)];
            let mut black = 0u32;
            for x in 0..w {
                if img.get_pixel(x, y)[0] < 128 {
                    bytes[(x / 8) as usize] |= 0x80 >> (x % 8);
                    black += 1;
                }
            }
            if black == 0 {
                self.send(cmd::PRINT_EMPTY_ROW, &[(y >> 8) as u8, y as u8, 1])?;
            } else {
                let mut d = vec![(y >> 8) as u8, y as u8, 0, black.min(255) as u8, 0, 1];
                d.extend(bytes);
                self.send(cmd::PRINT_BITMAP_ROW, &d)?;
            }
        }
        self.request_ok(cmd::PAGE_END, &[1])?;
        // Wait until the printer reports all pages done.
        let deadline = Instant::now() + Duration::from_secs(30 + 5 * copies as u64);
        while Instant::now() < deadline {
            let d = self.request_with(
                cmd::PRINT_STATUS,
                &[1],
                Some(reply_id(cmd::PRINT_STATUS)),
                Duration::from_secs(5),
            )?;
            let page = d.get(1).copied().unwrap_or(0) as u16;
            if page >= copies.max(1) {
                break;
            }
            std::thread::sleep(Duration::from_millis(300));
        }
        self.request(cmd::PRINT_END, &[1])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let p = encode(0x21, &[3]);
        assert_eq!(p, vec![0x55, 0x55, 0x21, 1, 3, 0x21 ^ 1 ^ 3, 0xAA, 0xAA]);
        assert_eq!(decode(&p), vec![(0x21, vec![3])]);
    }
}
