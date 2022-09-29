//! A no_std platform agnostic driver in rust  for the [TMP117](https://www.ti.com/product/TMP117) temperature sensor
//! using the [embedded-hal](https://github.com/rust-embedded/embedded-hal) and the [device-register](https://github.com/xgroleau/device-register) library.
//! A Sync and Async API is available, see the examples folder for more complete usage
//! The library makes usage of the [typestate](https://docs.rust-embedded.org/book/static-guarantees/typestate-programming.html) pattern.
//! The low level api is always available if the typestate is too constraining
//!
//! ## Usage
//!
//! ```no_run
//! // Pass the address of the tmp device
//! let tmp = Tmp117::<0x49, _, _, _>::new(spi);
//!
//! // Transition to oneshot mode
//! let tmp_one = tmp.to_oneshot(Average::NoAverage).unwrap();
//! // Read and transition to shutdown since it's a one shot
//! let (temperature, tmp_shut) = tmp_one.wait_temp().unwrap();
//!
//! // Transition to continuous mode
//! let mut tmp_cont = tmp_shut.to_continuous(Default::default()).unwrap();
//!
//! // Get the value continuously in continuous mode
//! for _ in 0..10 {
//!     let temp = tmp_cont.wait_temp().unwrap();
//!     info!("Temperature {}", temp);
//! };
//!
//! // Shutdown the device
//! let _  = tmp_cont.to_shutdown().unwrap();
//! ```
//!
//! ## MSRV
//! Currently only rust `nightly-2022-09-29` and more is garanted to work with the library, but some previous version may work
//!
//! ## License
//! Licensed under either of
//! - Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
//!   <http://www.apache.org/licenses/LICENSE-2.0>)
//!
//! - MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
//!
//! at your option.
//!
//! ## Contribution
//! Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
//!
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![deny(unsafe_code, missing_docs)]

use core::marker::PhantomData;

use device_register::{EditRegister, ReadRegister, WriteRegister};
use embedded_hal::i2c::{I2c, SevenBitAddress};
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

/// The continuous config
#[derive(Default)]
pub struct ContinousConfig {
    /// The average used, will use the one stored in the register if None
    pub average: Option<Average>,

    /// The convesion used, will use the one stored in the register if None
    pub conversion: Option<Conversion>,

    /// The high alert used, will use the one stored in the register if None
    pub high: Option<f32>,

    /// The low alert used, will use the one stored in the register if None
    pub low: Option<f32>,

    /// The temperature offset used, will use 0 if None
    pub offset: Option<f32>,
}

/// Conversion factor used by the device. One lsb is this value
pub const CELCIUS_CONVERSION: f32 = 0.0078125;

/// Typestate for unkown state. Only used on creation and reset when the state is unknown.
pub struct UnknownMode;

/// Typestate for continuous mode
pub struct ContinuousMode;

/// Typestate for shutdown mode
pub struct ShutdownMode;

/// Typestate for oneshot mode
pub struct OneShotMode;

/// The TMP117 driver. Note that the alert pin is not used in this driver,
/// see the async implementation if you want the driver to use the alert pin in the drive
pub struct Tmp117<const ADDR: u8, T, E, M>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    tmp_ll: Tmp117LL<ADDR, T, E>,
    mode: PhantomData<M>,
}

impl<const ADDR: u8, T, E> Tmp117<ADDR, T, E, UnknownMode>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    /// Create a new tmp117 from a i2c bus
    pub fn new(i2c: T) -> Tmp117<ADDR, T, E, UnknownMode> {
        Tmp117::<ADDR, T, E, UnknownMode> {
            tmp_ll: Tmp117LL::new(i2c),
            mode: PhantomData,
        }
    }

    /// Create a new tmp117 from a low level tmp117 driver
    pub fn new_from_ll(tmp_ll: Tmp117LL<ADDR, T, E>) -> Tmp117<ADDR, T, E, UnknownMode> {
        Tmp117::<ADDR, T, E, UnknownMode> {
            tmp_ll,
            mode: PhantomData,
        }
    }
}

impl<const ADDR: u8, T, E, M> Tmp117<ADDR, T, E, M>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    fn wait_eeprom(&mut self) -> Result<(), Error> {
        let mut configuration: Configuration = self.tmp_ll.read().map_err(Error::Bus)?;
        while configuration.eeprom_busy() {
            configuration = self.tmp_ll.read().map_err(Error::Bus)?;
        }

        Ok(())
    }

    fn read_temp_raw(&mut self) -> Result<f32, Error> {
        let temp: Temperature = self.tmp_ll.read().map_err(Error::Bus)?;

        // Convert to i16 for two complements
        let val = (u16::from(temp) as i16) as f32 * CELCIUS_CONVERSION;
        Ok(val)
    }

    fn check_alert(&mut self) -> Result<Alert, Error> {
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

    fn wait_for_data(&mut self) -> Result<(), Error> {
        // Loop while the data is not ok
        loop {
            let config: Configuration = self.tmp_ll.read().map_err(Error::Bus)?;
            if config.data_ready() {
                break;
            }
        }
        Ok(())
    }

    fn wait_for_alert(&mut self) -> Result<Alert, Error> {
        loop {
            let alert = self.check_alert();
            if let Ok(Alert::None) = alert {
                continue;
            } else {
                return alert;
            }
        }
    }

    /// Go to continuous mode
    pub fn to_continuous(
        mut self,
        config: ContinousConfig,
    ) -> Result<Tmp117<ADDR, T, E, ContinuousMode>, Error> {
        self.tmp_ll
            .edit(|r: &mut Configuration| {
                r.set_mode(ConversionMode::Continuous);
                if let Some(val) = config.average {
                    r.set_average(val);
                }
                if let Some(val) = config.conversion {
                    r.set_conversion(val);
                }
                r
            })
            .map_err(Error::Bus)?;
        if let Some(val) = config.high {
            let high: HighLimit = ((val / CELCIUS_CONVERSION) as u16).into();
            self.tmp_ll.write(high).map_err(Error::Bus)?;
        }
        if let Some(val) = config.low {
            let low: LowLimit = ((val / CELCIUS_CONVERSION) as u16).into();
            self.tmp_ll.write(low).map_err(Error::Bus)?;
        }
        if let Some(val) = config.offset {
            let off: TemperatureOffset = ((val / CELCIUS_CONVERSION) as u16).into();
            self.tmp_ll.write(off).map_err(Error::Bus)?;
        }

        Ok(Tmp117::<ADDR, T, E, ContinuousMode> {
            tmp_ll: self.tmp_ll,
            mode: PhantomData,
        })
    }

    /// Go to oneshot mode
    pub fn to_oneshot(
        mut self,
        average: Average,
    ) -> Result<Tmp117<ADDR, T, E, OneShotMode>, Error> {
        self.tmp_ll
            .edit(|r: &mut Configuration| {
                r.set_mode(ConversionMode::OneShot);
                r.set_average(average);
                r
            })
            .map_err(Error::Bus)?;

        Ok(Tmp117::<ADDR, T, E, OneShotMode> {
            tmp_ll: self.tmp_ll,
            mode: PhantomData,
        })
    }

    /// Go to shotdown mode
    pub fn to_shutdown(mut self) -> Result<Tmp117<ADDR, T, E, ShutdownMode>, Error> {
        self.tmp_ll
            .edit(|r: &mut Configuration| {
                r.set_mode(ConversionMode::Shutdown);
                r
            })
            .map_err(Error::Bus)?;

        Ok(Tmp117::<ADDR, T, E, ShutdownMode> {
            tmp_ll: self.tmp_ll,
            mode: PhantomData,
        })
    }

    /// Reset  the device
    pub fn reset(mut self) -> Result<Tmp117<ADDR, T, E, UnknownMode>, Error> {
        self.tmp_ll
            .edit(|r: &mut Configuration| {
                r.set_reset(true);
                r
            })
            .map_err(Error::Bus)?;

        Ok(Tmp117::<ADDR, T, E, UnknownMode> {
            tmp_ll: self.tmp_ll,
            mode: PhantomData,
        })
    }

    /// Write data to user eeprom. Note that this is blocking because we wait for write on the eeprom to complete
    pub fn write_eeprom(&mut self, values: [u16; 3]) -> Result<(), Error> {
        self.wait_eeprom()?;
        self.tmp_ll
            .write(UEEPROM1::from(values[0]))
            .map_err(Error::Bus)?;

        self.wait_eeprom()?;
        self.tmp_ll
            .write(UEEPROM2::from(values[1]))
            .map_err(Error::Bus)?;

        self.wait_eeprom()?;
        self.tmp_ll
            .write(UEEPROM3::from(values[2]))
            .map_err(Error::Bus)?;

        Ok(())
    }

    /// Read the data from the eeprom
    pub fn read_eeprom(&mut self) -> Result<[u16; 3], Error> {
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
    /// Wait for data and read the temperature in celsius and goes to shutdown mode since it's a oneshot
    pub fn wait_temp(mut self) -> Result<(f32, Tmp117<ADDR, T, E, ShutdownMode>), Error> {
        self.wait_for_data()?;

        let val = self.read_temp_raw()?;
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
    /// Read the temperature in celsius, return an error if the value of the temperature is not ready
    pub fn read_temp(&mut self) -> Result<f32, Error> {
        let config: Configuration = self.tmp_ll.read().map_err(Error::Bus)?;
        if !config.data_ready() {
            return Err(Error::DataNotReady);
        }

        self.read_temp_raw()
    }

    /// Wait for the data to be ready and read the temperature in celsius
    pub fn wait_temp(&mut self) -> Result<f32, Error> {
        self.wait_for_data()?;
        self.read_temp_raw()
    }

    /// Check if an alert was triggered since the last calll
    pub fn get_alert(&mut self) -> Result<Alert, Error> {
        self.check_alert()
    }

    /// Wait for an alert to come and return it's value
    pub fn wait_alert(&mut self) -> Result<Alert, Error> {
        self.wait_for_alert()
    }
}
