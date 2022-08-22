//! Errors used for the driver

use embedded_hal::i2c::ErrorKind;

/// Error emitted by the TMP117 drivers
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Internal i2c bus error
    Bus(ErrorKind),

    /// Data is not ready
    DataNotReady,

    /// Alert pin error
    AlertPin,
}

impl<E> From<E> for Error
where
    E: embedded_hal::i2c::Error,
{
    fn from(e: E) -> Self {
        Error::Bus(e.kind())
    }
}
