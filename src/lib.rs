//! A library to manipulate the TI [TMP117](https://www.ti.com/product/TMP117)
#![no_std]
#![no_main]
#![deny(unsafe_code, missing_docs)]

use core::marker::PhantomData;

use embedded_hal::i2c::{blocking::I2c, SevenBitAddress};
pub use error::Error;
pub use modular_bitfield::Specifier;
use register::*;
use tmp117_ll::Tmp117LL;

pub mod asynchronous;
pub mod error;
pub mod register;
pub mod tmp117_ll;

/// The types of alerts possible
pub enum Alert {
    /// No alert were triggered
    None,
    /// A high alert was triggered
    High,

    /// A low alert was triggered
    Low,

    /// A high and a low alert was triggered
    HighLow,
}

/// Conversion factor used by the device. One lsb is this value
pub const CELCIUS_CONVERSION: f32 = 7.8125;

/// Typestate for unkown state. Only used on creation and reset when the state is unknown.
pub struct Unknown;

/// Typestate for continuous mode
pub struct ContinuousMode;

/// Typestate for shutdown mode
pub struct ShutdownMode;

/// Typestate for oneshot mode
pub struct OneShotMode;

/// The TMP117 driver
pub struct Tmp117<const ADDR: u8, T, E, M>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    tmp_ll: Tmp117LL<ADDR, T, E>,
    mode: PhantomData<M>,
}

impl<const ADDR: u8, T, E, M> Tmp117<ADDR, T, E, M>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    /// Create a new tmp117 from a i2c bus
    pub fn new(i2c: T) -> Tmp117<ADDR, T, E, Unknown> {
        Tmp117::<ADDR, T, E, Unknown> {
            tmp_ll: Tmp117LL::new(i2c),
            mode: PhantomData,
        }
    }

    /// Create a new tmp117 from a low level tmp117 driver
    pub fn new_from_ll(tmp_ll: Tmp117LL<ADDR, T, E>) -> Tmp117<ADDR, T, E, Unknown> {
        Tmp117::<ADDR, T, E, Unknown> {
            tmp_ll,
            mode: PhantomData,
        }
    }

    fn wait_eeprom(&mut self) -> Result<(), Error<E>> {
        while self
            .tmp_ll
            .read::<Configuration>()
            .map_err(Error::Bus)?
            .eeprom_busy()
        {}

        Ok(())
    }

    /// Go to continuous mode
    pub fn to_continuous(
        mut self,
        average: Average,
        conversion: Conversion,
    ) -> Result<Tmp117<ADDR, T, E, ContinuousMode>, Error<E>> {
        self.tmp_ll
            .edit(|r: Configuration| {
                r.with_mode(ConversionMode::Continuous)
                    .with_average(average)
                    .with_conversion(conversion)
            })
            .map_err(Error::Bus)?;

        Ok(Tmp117::<ADDR, T, E, ContinuousMode> {
            tmp_ll: self.tmp_ll,
            mode: PhantomData,
        })
    }

    /// Go to oneshot mode
    pub fn to_oneshot(
        mut self,
        average: Average,
    ) -> Result<Tmp117<ADDR, T, E, OneShotMode>, Error<E>> {
        self.tmp_ll
            .edit(|r: Configuration| r.with_mode(ConversionMode::OneShot).with_average(average))
            .map_err(Error::Bus)?;

        Ok(Tmp117::<ADDR, T, E, OneShotMode> {
            tmp_ll: self.tmp_ll,
            mode: PhantomData,
        })
    }

    /// Go to shotdown mode
    pub fn to_shutdown(mut self) -> Result<Tmp117<ADDR, T, E, ShutdownMode>, Error<E>> {
        self.tmp_ll
            .edit(|r: Configuration| r.with_mode(ConversionMode::Shutdown))
            .map_err(Error::Bus)?;

        Ok(Tmp117::<ADDR, T, E, ShutdownMode> {
            tmp_ll: self.tmp_ll,
            mode: PhantomData,
        })
    }

    /// Reset  the device
    pub fn reset(mut self) -> Result<Tmp117<ADDR, T, E, Unknown>, Error<E>> {
        self.tmp_ll
            .edit(|r: Configuration| r.with_reset(true))
            .map_err(Error::Bus)?;

        Ok(Tmp117::<ADDR, T, E, Unknown> {
            tmp_ll: self.tmp_ll,
            mode: PhantomData,
        })
    }

    /// Write data to user eeprom. Note that this is blocking because we wait for write on the eeprom to complete
    pub fn write_eeprom(&mut self, values: [u16; 3]) -> Result<(), Error<E>> {
        self.wait_eeprom()?;
        self.tmp_ll
            .write::<UEEPROM1>(values[0].into())
            .map_err(Error::Bus)?;

        self.wait_eeprom()?;
        self.tmp_ll
            .write::<UEEPROM2>(values[1].into())
            .map_err(Error::Bus)?;

        self.wait_eeprom()?;
        self.tmp_ll
            .write::<UEEPROM3>(values[2].into())
            .map_err(Error::Bus)?;

        Ok(())
    }

    /// Read the data from the eeprom
    pub fn read_eeprom(&mut self) -> Result<[u16; 3], Error<E>> {
        let u1: UEEPROM1 = self.tmp_ll.read().map_err(Error::Bus)?;
        let u2: UEEPROM2 = self.tmp_ll.read().map_err(Error::Bus)?;
        let u3: UEEPROM3 = self.tmp_ll.read().map_err(Error::Bus)?;

        Ok([u1.into(), u2.into(), u3.into()])
    }
}

impl<const ADDR: u8, T, E> Tmp117<ADDR, T, E, OneShotMode>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    /// Read the temperature and goes to shutdown mode since it's a oneshot
    pub fn read_temp(mut self) -> Result<(f32, Tmp117<ADDR, T, E, ShutdownMode>), Error<E>> {
        let config: Configuration = self.tmp_ll.read().map_err(Error::Bus)?;
        if !config.data_ready() {
            return Err(Error::DataNotReady);
        }

        let temp: Temperature = self.tmp_ll.read().map_err(Error::Bus)?;
        // Convert to i16 for two complements
        let val = (u16::from(temp) as i16) as f32 * CELCIUS_CONVERSION;
        Ok((
            val,
            Tmp117::<ADDR, T, E, ShutdownMode> {
                tmp_ll: self.tmp_ll,
                mode: PhantomData,
            },
        ))
    }
}

impl<const ADDR: u8, T, E> Tmp117<ADDR, T, E, ContinuousMode>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    /// Read the temperature
    pub fn read_temp(&mut self) -> Result<f32, Error<E>> {
        let config: Configuration = self.tmp_ll.read().map_err(Error::Bus)?;
        if !config.data_ready() {
            return Err(Error::DataNotReady);
        }

        let temp: Temperature = self.tmp_ll.read().map_err(Error::Bus)?;

        // Convert to i16 for two complements
        let val = (u16::from(temp) as i16) as f32 * CELCIUS_CONVERSION;
        Ok(val)
    }

    /// Check if an alert was triggered since the last calll
    pub fn check_alert(&mut self) -> Result<Alert, Error<E>> {
        let config: Configuration = self.tmp_ll.read().map_err(Error::Bus)?;
        if config.high_alert() && config.low_alert() {
            Ok(Alert::HighLow)
        } else if config.high_alert() {
            Ok(Alert::High)
        } else if config.low_alert() {
            Ok(Alert::Low)
        } else {
            Ok(Alert::None)
        }
    }
}
