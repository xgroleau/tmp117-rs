//! Async drivers of the tmp117

use core::future::Future;

use device_register_async::{EditRegister, ReadRegister, WriteRegister};
use embedded_hal::{digital::ErrorType, i2c::SevenBitAddress};
use embedded_hal_async::{delay::DelayUs, digital::Wait, i2c::I2c};

use crate::{register::*, Alert, ContinuousConfig, Error, Id, CELCIUS_CONVERSION};

use self::tmp117_ll::Tmp117LL;
pub mod tmp117_ll;

/// Dummy type for wait pin, should never be
pub struct DummyWait(());
impl ErrorType for DummyWait {
    type Error = ();
}
impl Wait for DummyWait {
    async fn wait_for_high(&'_ mut self) -> Result<(), Self::Error> {
        todo!()
    }

    async fn wait_for_low(&'_ mut self) -> Result<(), Self::Error> {
        todo!()
    }

    async fn wait_for_rising_edge(&'_ mut self) -> Result<(), Self::Error> {
        todo!()
    }

    async fn wait_for_falling_edge(&'_ mut self) -> Result<(), Self::Error> {
        todo!()
    }

    async fn wait_for_any_edge(&'_ mut self) -> Result<(), Self::Error> {
        todo!()
    }
}

/// The status of the alert pin
enum AlertPin<P> {
    /// Unkown, right after boot
    Unkown(P),
    /// Currently in data ready mode
    DataReady(P),
    /// Currently in alert mode
    Alert(P),
}
impl<P> AlertPin<P> {
    /// Borrow a mutable reference to then internal pin without caring for it's state
    pub fn borrow_mut(&mut self) -> &mut P {
        match self {
            AlertPin::Unkown(p) => p,
            AlertPin::DataReady(p) => p,
            AlertPin::Alert(p) => p,
        }
    }

    pub fn unwrap(self) -> P {
        match self {
            AlertPin::Unkown(p) => p,
            AlertPin::DataReady(p) => p,
            AlertPin::Alert(p) => p,
        }
    }
}

/// The TMP117 driver. Note that the alert pin is optional, but it is recommended to pass it if possible
/// If the alert pin is `None`, the driver will poll the config register instead of waiting for the pin.
pub struct Tmp117<const ADDR: u8, T, E, P> {
    tmp_ll: Tmp117LL<ADDR, T, E>,
    alert: Option<AlertPin<P>>,
}

impl<const ADDR: u8, T, E> Tmp117<ADDR, T, E, DummyWait>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error + Copy,
{
    /// Create a new tmp117 from a i2c bus
    pub fn new(i2c: T) -> Tmp117<ADDR, T, E, DummyWait> {
        Tmp117::<ADDR, T, E, DummyWait> {
            tmp_ll: Tmp117LL::new(i2c),
            alert: None,
        }
    }
}

impl<const ADDR: u8, T, E, P> Tmp117<ADDR, T, E, P>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error + Copy,
    P: Wait,
{
    /// Create a new tmp117 from a i2c bus and alert pin
    pub fn new_alert(i2c: T, alert: P) -> Self {
        Self {
            tmp_ll: Tmp117LL::new(i2c),
            alert: Some(AlertPin::Unkown(alert)),
        }
    }

    /// Create a new tmp117 from a low level tmp117 driver
    pub fn new_from_ll(tmp_ll: Tmp117LL<ADDR, T, E>, alert: P) -> Self {
        Self {
            tmp_ll,
            alert: Some(AlertPin::Unkown(alert)),
        }
    }

    /// Returns the ID of the device
    pub async fn id(&mut self) -> Result<Id, Error<E>> {
        let id: DeviceID = self.tmp_ll.read().await.map_err(Error::Bus)?;
        Ok(Id {
            device: id.device_id(),
            revision: id.revision(),
        })
    }

    async fn wait_eeprom(&mut self) -> Result<(), Error<E>> {
        let mut configuration: Configuration = self.tmp_ll.read().await.map_err(Error::Bus)?;
        while configuration.eeprom_busy() {
            configuration = self.tmp_ll.read().await.map_err(Error::Bus)?;
        }

        Ok(())
    }

    async fn read_temp_raw(&mut self) -> Result<f32, Error<E>> {
        let temp: Temperature = self.tmp_ll.read().await.map_err(Error::Bus)?;

        // Convert to i16 for two complements
        let val = (u16::from(temp) as i16) as f32 * CELCIUS_CONVERSION;
        Ok(val)
    }

    async fn check_alert(&mut self) -> Result<Alert, Error<E>> {
        let config: Configuration = self.tmp_ll.read().await.map_err(Error::Bus)?;
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

    async fn wait_for_data(&mut self) -> Result<(), Error<E>> {
        // If we have a pin
        if let Some(p) = &mut self.alert {
            // If in data ready, just use it
            if let AlertPin::DataReady(_) = p {
            } else {
                // If not, set it to data ready
                self.tmp_ll
                    .edit(|r: &mut Configuration| {
                        r.set_dr_alert(AlertPinSelect::DataReady);
                        r.set_polarity(Polarity::ActiveHigh);
                        r
                    })
                    .await
                    .map_err(Error::Bus)?;
            }
            // Wait for it to go high
            p.borrow_mut()
                .wait_for_low()
                .await
                .map_err(|_| Error::AlertPin)?;
            self.alert = self.alert.take().map(|v| AlertPin::DataReady(v.unwrap()));

            // Clear flag in register
            let config: Configuration = self.tmp_ll.read().await.map_err(Error::Bus)?;
            assert!(config.data_ready());
        } else {
            // Loop while the alert is not ok
            loop {
                let config: Configuration = self.tmp_ll.read().await.map_err(Error::Bus)?;
                if config.data_ready() {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn wait_for_alert(&mut self) -> Result<Alert, Error<E>> {
        if let Some(p) = &mut self.alert {
            if let AlertPin::Alert(_) = p {
            } else {
                self.tmp_ll
                    .edit(|r: &mut Configuration| {
                        r.set_dr_alert(AlertPinSelect::Alert);
                        r.set_polarity(Polarity::ActiveHigh);
                        r
                    })
                    .await
                    .map_err(Error::Bus)?;
            }
            p.borrow_mut()
                .wait_for_high()
                .await
                .map_err(|_| Error::AlertPin)?;
            self.alert = self.alert.take().map(|v| AlertPin::Alert(v.unwrap()));
            self.check_alert().await
        } else {
            loop {
                let alert = self.check_alert().await;
                if let Ok(Alert::None) = alert {
                    continue;
                } else {
                    return alert;
                }
            }
        }
    }

    async fn set_continuous(
        &mut self,
        config: ContinuousConfig,
    ) -> Result<ContinuousHandler<ADDR, T, E, P>, Error<E>> {
        if let Some(val) = config.high {
            let high: HighLimit = ((val / CELCIUS_CONVERSION) as u16).into();
            self.tmp_ll.write(high).await.map_err(Error::Bus)?;
        }
        if let Some(val) = config.low {
            let low: LowLimit = ((val / CELCIUS_CONVERSION) as u16).into();
            self.tmp_ll.write(low).await.map_err(Error::Bus)?;
        }
        if let Some(val) = config.offset {
            let off: TemperatureOffset = ((val / CELCIUS_CONVERSION) as u16).into();
            self.tmp_ll.write(off).await.map_err(Error::Bus)?;
        }

        let config = Configuration::new()
            .with_mode(ConversionMode::Continuous)
            .with_polarity(Polarity::ActiveLow)
            .with_average(config.average)
            .with_conversion(config.conversion);

        self.tmp_ll
            .edit(|r: &mut Configuration| {
                *r = config;
                r
            })
            .await
            .map_err(Error::Bus)?;
        Ok(ContinuousHandler { tmp117: self })
    }

    async fn set_oneshot(&mut self, average: Average) -> Result<(), Error<E>> {
        let config = Configuration::new()
            .with_mode(ConversionMode::OneShot)
            .with_polarity(Polarity::ActiveLow)
            .with_average(average);
        self.tmp_ll
            .edit(|r: &mut Configuration| {
                *r = config;
                r
            })
            .await
            .map_err(Error::Bus)
    }

    async fn set_shutdown(&mut self) -> Result<(), Error<E>> {
        let config = Configuration::new().with_mode(ConversionMode::Shutdown);
        self.tmp_ll
            .edit(|r: &mut Configuration| {
                *r = config;
                r
            })
            .await
            .map_err(Error::Bus)
    }

    /// Resets the device and put it in shutdown
    pub async fn reset<D>(&mut self, mut delay: D) -> Result<(), Error<E>>
    where
        D: DelayUs,
    {
        self.tmp_ll
            .edit(|r: &mut Configuration| {
                r.set_reset(true);
                r
            })
            .await
            .map_err(Error::Bus)?;
        delay.delay_ms(2).await.map_err(|_| Error::Delay)?;
        self.set_shutdown().await
    }

    /// Write data to user eeprom. Note that this is blocking because we wait for write on the eeprom to complete
    pub async fn write_eeprom(&mut self, values: [u16; 3]) -> Result<(), Error<E>> {
        self.wait_eeprom().await?;
        self.tmp_ll
            .write(UEEPROM1::from(values[0]))
            .await
            .map_err(Error::Bus)?;

        self.wait_eeprom().await?;
        self.tmp_ll
            .write(UEEPROM2::from(values[1]))
            .await
            .map_err(Error::Bus)?;

        self.wait_eeprom().await?;
        self.tmp_ll
            .write(UEEPROM3::from(values[2]))
            .await
            .map_err(Error::Bus)?;

        Ok(())
    }

    /// Read the data from the eeprom
    pub async fn read_eeprom(&mut self) -> Result<[u16; 3], Error<E>> {
        let u1: UEEPROM1 = self.tmp_ll.read().await.map_err(Error::Bus)?;
        let u2: UEEPROM2 = self.tmp_ll.read().await.map_err(Error::Bus)?;
        let u3: UEEPROM3 = self.tmp_ll.read().await.map_err(Error::Bus)?;

        Ok([u1.into(), u2.into(), u3.into()])
    }

    /// Wait for data and read the temperature in celsius and goes to shutdown since it's a oneshot
    pub async fn oneshot(&mut self, average: Average) -> Result<f32, Error<E>> {
        self.set_oneshot(average).await?;
        self.wait_for_data().await?;

        let res = self.read_temp_raw().await?;
        Ok(res)
    }

    /// Pass a config and closure for the continuous mode.
    /// The device gets set to continuous, then the function is called with the handler
    /// and finally the device is shutdown
    /// A pointer is passed since lifetime cannot be described for async closure in this situation
    pub async fn continuous<F, Fut>(
        &mut self,
        config: ContinuousConfig,
        f: F,
    ) -> Result<(), Error<E>>
    where
        F: FnOnce(ContinuousHandler<ADDR, T, E, P>) -> Fut,
        Fut: Future<Output = Result<(), Error<E>>>,
    {
        let continuous = self.set_continuous(config).await?;
        f(continuous).await?;
        self.set_shutdown().await
    }
}

/// Handler for the continuous mode
///
/// # Safety
/// Note that it is only safe to use in the [Tmp117::continuous] closure since
/// it uses a pointer to the tmp117 to circuvent issues with async closure lifetime
pub struct ContinuousHandler<const ADDR: u8, T, E, P> {
    tmp117: *mut Tmp117<ADDR, T, E, P>,
}

impl<'a, const ADDR: u8, T, E, P> ContinuousHandler<ADDR, T, E, P>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error + Copy,
    P: Wait,
{
    /// Read the temperature in celsius, return an error if the value of the temperature is not valid
    pub async fn read_temp(&mut self) -> Result<f32, Error<E>> {
        let tmp117 = unsafe { &mut *self.tmp117 };
        let config: Configuration = tmp117.tmp_ll.read().await.map_err(Error::Bus)?;
        if !config.data_ready() {
            return Err(Error::DataNotReady);
        }

        tmp117.read_temp_raw().await
    }

    /// Wait for the data to be ready and read the temperature in celsius
    pub async fn wait_temp(&mut self) -> Result<f32, Error<E>> {
        let tmp117 = unsafe { &mut *self.tmp117 };
        tmp117.wait_for_data().await?;
        tmp117.read_temp_raw().await
    }

    /// Check if an alert was triggered since the last calll
    pub async fn get_alert(&mut self) -> Result<Alert, Error<E>> {
        let tmp117 = unsafe { &mut *self.tmp117 };
        tmp117.check_alert().await
    }

    /// Wait for an alert to come and return it's value
    pub async fn wait_alert(&mut self) -> Result<Alert, Error<E>> {
        let tmp117 = unsafe { &mut *self.tmp117 };
        tmp117.wait_for_alert().await
    }
}
