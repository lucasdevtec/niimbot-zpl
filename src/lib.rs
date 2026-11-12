//! Niimbot B1 driver: ZPL -> bitmap -> printer protocol over a pluggable transport.
pub mod data;
pub mod designer;
pub mod protocol;
pub mod transport;
pub mod zpl;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("transport: {0}")]
    Transport(String),
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
