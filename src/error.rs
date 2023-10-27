//! Errors used for the driver

/// Error emitted by the TMP117 drivers
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Error<E> {
    /// Internal i2c bus error
    Bus(E),

    /// Data is not ready
    DataNotReady,

    /// Alert pin error
    AlertPin,

    /// Received Invalid data
    InvalidData,
}

/// Error emitted by the low level TMP117 drivers
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ErrorLL<E> {
    /// Internal i2c bus error
    Bus(E),

    /// Received Invalid data
    InvalidData,
}

impl<E> From<ErrorLL<E>> for Error<E> {
    fn from(value: ErrorLL<E>) -> Self {
        match value {
            ErrorLL::Bus(e) => Error::Bus(e),
            ErrorLL::InvalidData => Error::InvalidData,
        }
    }
}
