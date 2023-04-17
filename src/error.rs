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
}
