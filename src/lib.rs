#![doc = include_str!("../README.md")]
#![no_std]
#![deny(missing_docs)]

use device_register::{EditRegister, ReadRegister, WriteRegister};
use embedded_hal::{
    delay::DelayNs,
    i2c::{I2c, SevenBitAddress},
};
pub use error::Error;
use register::*;
use tmp117_ll::Tmp117LL;

pub mod asynchronous;
pub mod error;
pub mod register;
pub mod tmp117_ll;

/// Conversion factor used by the device. One lsb is this value
pub const CELCIUS_CONVERSION: f32 = 0.0078125;

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
pub struct ContinuousConfig {
    /// The average used, will use the one stored in the register if None
    pub average: Average,

    /// The convesion used, will use the one stored in the register if None
    pub conversion: Conversion,

    /// The high alert used, will use the one stored in the register if None
    pub high: Option<f32>,

    /// The low alert used, will use the one stored in the register if None
    pub low: Option<f32>,

    /// The temperature offset used, will use 0 if None
    pub offset: Option<f32>,
}
/// Represents the ID of the device.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Id {
    /// Should always be 0x117
    pub device: u16,
    /// Depends on the revision of the device
    pub revision: u8,
}

/// The TMP117 driver. Note that the alert pin is not used in this driver,
/// see the async implementation if you want the driver to use the alert pin in the drive
pub struct Tmp117<T, E> {
    tmp_ll: Tmp117LL<T, E>,
}

impl<T, E> Tmp117<T, E>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    /// Create a new tmp117 from a i2c bus
    pub fn new(i2c: T, addr: u8) -> Self {
        Tmp117::<T, E> {
            tmp_ll: Tmp117LL::new(i2c, addr),
        }
    }

    /// Create a new tmp117 from a low level tmp117 driver
    pub fn new_from_ll(tmp_ll: Tmp117LL<T, E>) -> Self {
        Tmp117::<T, E> { tmp_ll }
    }

    /// Returns the ID of the device
    pub fn id(&mut self) -> Result<Id, Error<E>> {
        let id: DeviceID = self.tmp_ll.read()?;
        Ok(Id {
            device: id.device_id().into(),
            revision: id.revision().into(),
        })
    }

    fn wait_eeprom(&mut self) -> Result<(), Error<E>> {
        let mut configuration: Configuration = self.tmp_ll.read()?;
        while configuration.eeprom_busy() {
            configuration = self.tmp_ll.read()?;
        }

        Ok(())
    }

    fn read_temp_raw(&mut self) -> Result<f32, Error<E>> {
        let temp: Temperature = self.tmp_ll.read()?;

        // Convert to i16 for two complements
        let val = (u16::from(temp) as i16) as f32 * CELCIUS_CONVERSION;
        Ok(val)
    }

    fn check_alert(&mut self) -> Result<Alert, Error<E>> {
        let config: Configuration = self.tmp_ll.read()?;
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

    fn wait_for_data(&mut self) -> Result<(), Error<E>> {
        // Loop while the data is not ok
        loop {
            let config: Configuration = self.tmp_ll.read()?;
            if config.data_ready() {
                break;
            }
        }
        Ok(())
    }

    fn wait_for_alert(&mut self) -> Result<Alert, Error<E>> {
        loop {
            let alert = self.check_alert();
            if let Ok(Alert::None) = alert {
                continue;
            } else {
                return alert;
            }
        }
    }

    fn set_continuous(
        &mut self,
        config: ContinuousConfig,
    ) -> Result<ContinuousHandler<'_, T, E>, Error<E>> {
        if let Some(val) = config.high {
            let high: HighLimit = ((val / CELCIUS_CONVERSION) as u16).into();
            self.tmp_ll.write(high)?;
        }
        if let Some(val) = config.low {
            let low: LowLimit = ((val / CELCIUS_CONVERSION) as u16).into();
            self.tmp_ll.write(low)?;
        }
        if let Some(val) = config.offset {
            let off: TemperatureOffset = ((val / CELCIUS_CONVERSION) as u16).into();
            self.tmp_ll.write(off)?;
        }

        self.tmp_ll.edit(|r: &mut Configuration| {
            r.set_mode(ConversionMode::Continuous);
            r.set_polarity(Polarity::ActiveLow);
            r.set_average(config.average);
            r.set_conversion(config.conversion);
        })?;

        Ok(ContinuousHandler { tmp117: self })
    }

    fn set_oneshot(&mut self, average: Average) -> Result<(), Error<E>> {
        self.tmp_ll.edit(|r: &mut Configuration| {
            r.set_mode(ConversionMode::OneShot);
            r.set_polarity(Polarity::ActiveLow);
            r.set_average(average);
        })?;
        Ok(())
    }

    fn set_shutdown(&mut self) -> Result<(), Error<E>> {
        self.tmp_ll.edit(|r: &mut Configuration| {
            r.set_mode(ConversionMode::Shutdown);
        })?;
        Ok(())
    }

    /// Resets the device and put it in shutdown
    pub fn reset<D>(&mut self, delay: &mut D) -> Result<(), Error<E>>
    where
        D: DelayNs,
    {
        self.tmp_ll.edit(|r: &mut Configuration| {
            r.set_reset(true);
        })?;
        delay.delay_ms(2);
        self.set_shutdown()?;
        Ok(())
    }

    /// Write data to user eeprom. Note that this is blocking because we wait for write on the eeprom to complete
    pub fn write_eeprom(&mut self, values: [u16; 3]) -> Result<(), Error<E>> {
        self.wait_eeprom()?;
        self.tmp_ll.write(UEEPROM1::from(values[0]))?;

        self.wait_eeprom()?;
        self.tmp_ll.write(UEEPROM2::from(values[1]))?;

        self.wait_eeprom()?;
        self.tmp_ll.write(UEEPROM3::from(values[2]))?;

        Ok(())
    }

    /// Read the data from the eeprom
    pub fn read_eeprom(&mut self) -> Result<[u16; 3], Error<E>> {
        let u1: UEEPROM1 = self.tmp_ll.read()?;
        let u2: UEEPROM2 = self.tmp_ll.read()?;
        let u3: UEEPROM3 = self.tmp_ll.read()?;

        Ok([u1.into(), u2.into(), u3.into()])
    }

    /// Wait for data and read the temperature in celsius and shutdown since it's a oneshot
    pub fn oneshot(&mut self, average: Average) -> Result<f32, Error<E>> {
        self.set_oneshot(average)?;
        self.wait_for_data()?;
        let data = self.read_temp_raw()?;
        Ok(data)
    }

    /// Pass a config and closure for the continuous mode.
    /// The device gets set to continuous, then the function is called with the handler
    /// and finally the device is shutdown
    pub fn continuous<F>(&mut self, config: ContinuousConfig, f: F) -> Result<(), Error<E>>
    where
        F: FnOnce(ContinuousHandler<'_, T, E>) -> Result<(), Error<E>>,
    {
        let handler = self.set_continuous(config)?;
        f(handler)?;
        self.set_shutdown()
    }
}

/// Handler for the continuous mode
pub struct ContinuousHandler<'a, T, E> {
    tmp117: &'a mut Tmp117<T, E>,
}

impl<'a, T, E> ContinuousHandler<'a, T, E>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    /// Read the temperature in celsius, return an error if the value of the temperature is not ready
    pub fn read_temp(&mut self) -> Result<f32, Error<E>> {
        let config: Configuration = self.tmp117.tmp_ll.read()?;
        if !config.data_ready() {
            return Err(Error::DataNotReady);
        }

        let val = self.tmp117.read_temp_raw()?;
        Ok(val)
    }

    /// Wait for the data to be ready and read the temperature in celsius
    pub fn wait_temp(&mut self) -> Result<f32, Error<E>> {
        self.tmp117.wait_for_data()?;
        let val = self.tmp117.read_temp_raw()?;
        Ok(val)
    }

    /// Check if an alert was triggered since the last calll
    pub fn get_alert(&mut self) -> Result<Alert, Error<E>> {
        let val = self.tmp117.check_alert()?;
        Ok(val)
    }

    /// Wait for an alert to come and return it's value
    pub fn wait_alert(&mut self) -> Result<Alert, Error<E>> {
        let val = self.tmp117.wait_for_alert()?;
        Ok(val)
    }
}
