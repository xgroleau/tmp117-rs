//! Async drivers of the tmp117

use core::{convert::Infallible, future::Future};

use device_register_async::{EditRegister, ReadRegister, WriteRegister};
use embedded_hal::{digital::ErrorType, i2c::SevenBitAddress};
use embedded_hal_async::{delay::DelayNs, digital::Wait, i2c::I2c};

use crate::{register::*, Alert, ContinuousConfig, Error, Id, CELCIUS_CONVERSION};

use self::tmp117_ll::Tmp117LL;
pub mod tmp117_ll;

/// Dummy type for wait pin, should never be
pub struct DummyWait(());
impl ErrorType for DummyWait {
    type Error = Infallible;
}
impl Wait for DummyWait {
    async fn wait_for_high(&'_ mut self) -> Result<(), Self::Error> {
        unreachable!()
    }

    async fn wait_for_low(&'_ mut self) -> Result<(), Self::Error> {
        unreachable!()
    }

    async fn wait_for_rising_edge(&'_ mut self) -> Result<(), Self::Error> {
        unreachable!()
    }

    async fn wait_for_falling_edge(&'_ mut self) -> Result<(), Self::Error> {
        unreachable!()
    }

    async fn wait_for_any_edge(&'_ mut self) -> Result<(), Self::Error> {
        unreachable!()
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
    fn unwrap(self) -> P {
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
    /// # Warning
    /// You should use the `new_with_alert` function instead if possible
    /// It seems the tmp117 doesn't always set the data ready flag, so you should add a timeout when using `oneshot` wihout an alert pin.
    /// See [this](https://e2e.ti.com/support/sensors-group/sensors/f/sensors-forum/909104/tmp117-polling-the-data-ready-flag-seems-to-clear-it-inadvertently-when-using-1-shot-mode)
    /// and [this](https://e2e.ti.com/support/sensors-group/sensors/f/sensors-forum/1019457/tmp117-data_ready-flag-cleared-incorrectly-if-data-becomes-ready-during-read-of-configuration-register)
    /// for more information.
    /// TODO: Pass and use delay instead of polling to fix this
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
        let id: DeviceID = self.tmp_ll.read().await?;
        Ok(Id {
            device: id.device_id().into(),
            revision: id.revision().into(),
        })
    }

    async fn wait_eeprom(&mut self) -> Result<(), Error<E>> {
        let mut configuration: Configuration = self.tmp_ll.read().await?;
        while configuration.eeprom_busy() {
            configuration = self.tmp_ll.read().await?;
        }

        Ok(())
    }

    async fn read_temp_raw(&mut self) -> Result<f32, Error<E>> {
        let temp: Temperature = self.tmp_ll.read().await?;

        // Convert to i16 for two complements
        let val = (u16::from(temp) as i16) as f32 * CELCIUS_CONVERSION;
        Ok(val)
    }

    async fn check_alert(&mut self) -> Result<Alert, Error<E>> {
        let config: Configuration = self.tmp_ll.read().await?;
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

    async fn set_alert(&mut self) -> Result<(), Error<E>> {
        // If we have a pin
        if let Some(p) = &mut self.alert {
            // If in alert, just use it
            if let AlertPin::Alert(_) = p {
            } else {
                // If not, set it to alert
                self.tmp_ll
                    .edit(|r: &mut Configuration| {
                        r.set_dr_alert(AlertPinSelect::Alert);
                        r.set_polarity(Polarity::ActiveLow);
                    })
                    .await?;
            }
            self.alert = self.alert.take().map(|v| AlertPin::Alert(v.unwrap()));
        }
        Ok(())
    }

    async fn set_data_ready(&mut self) -> Result<(), Error<E>> {
        // If we have a pin
        if let Some(p) = &mut self.alert {
            // If in data ready, just use it
            if let AlertPin::DataReady(_) = p {
            } else {
                // If not, set it to data ready
                self.tmp_ll
                    .edit(|r: &mut Configuration| {
                        r.set_dr_alert(AlertPinSelect::DataReady);
                        r.set_polarity(Polarity::ActiveLow);
                    })
                    .await?;
            }
            self.alert = self.alert.take().map(|v| AlertPin::DataReady(v.unwrap()));
        }
        Ok(())
    }

    async fn wait_for_data(&mut self) -> Result<(), Error<E>> {
        // If we have a pin
        if let Some(AlertPin::DataReady(p)) = &mut self.alert {
            loop {
                // Wait for it to go low
                p.wait_for_low().await.map_err(|_| Error::AlertPin)?;

                // Clear flag in register
                let config: Configuration = self.tmp_ll.read().await?;

                // Validate that the data is ready
                if config.data_ready() {
                    break;
                }
            }
        } else {
            // Loop while the alert is not ok
            loop {
                let config: Configuration = self.tmp_ll.read().await?;
                if config.data_ready() {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn wait_for_alert(&mut self) -> Result<Alert, Error<E>> {
        if let Some(AlertPin::Alert(p)) = &mut self.alert {
            p.wait_for_low().await.map_err(|_| Error::AlertPin)?;
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
        self.set_data_ready().await?;
        if let Some(val) = config.high {
            let high: HighLimit = ((val / CELCIUS_CONVERSION) as u16).into();
            self.tmp_ll.write(high).await?;
        }
        if let Some(val) = config.low {
            let low: LowLimit = ((val / CELCIUS_CONVERSION) as u16).into();
            self.tmp_ll.write(low).await?;
        }
        if let Some(val) = config.offset {
            let off: TemperatureOffset = ((val / CELCIUS_CONVERSION) as u16).into();
            self.tmp_ll.write(off).await?;
        }

        self.tmp_ll
            .edit(|r: &mut Configuration| {
                r.set_mode(ConversionMode::Continuous);
                r.set_average(config.average);
                r.set_conversion(config.conversion);
            })
            .await?;
        Ok(ContinuousHandler { tmp117: self })
    }

    async fn set_oneshot(&mut self, average: Average) -> Result<(), Error<E>> {
        self.set_data_ready().await?;
        self.tmp_ll
            .edit(|r: &mut Configuration| {
                r.set_mode(ConversionMode::OneShot);
                r.set_average(average);
            })
            .await?;
        Ok(())
    }

    async fn set_shutdown(&mut self) -> Result<(), Error<E>> {
        self.tmp_ll
            .edit(|r: &mut Configuration| {
                r.set_mode(ConversionMode::Shutdown);
            })
            .await?;
        Ok(())
    }

    /// Resets the device and put it in shutdown
    pub async fn reset<D>(&mut self, delay: &mut D) -> Result<(), Error<E>>
    where
        D: DelayNs,
    {
        self.tmp_ll
            .edit(|r: &mut Configuration| {
                r.set_reset(true);
            })
            .await?;
        delay.delay_ms(2).await;
        self.set_shutdown().await
    }

    /// Write data to user eeprom. Note that this is blocking because we wait for write on the eeprom to complete
    pub async fn write_eeprom(&mut self, values: [u16; 3]) -> Result<(), Error<E>> {
        self.wait_eeprom().await?;
        self.tmp_ll.write(UEEPROM1::from(values[0])).await?;

        self.wait_eeprom().await?;
        self.tmp_ll.write(UEEPROM2::from(values[1])).await?;

        self.wait_eeprom().await?;
        self.tmp_ll.write(UEEPROM3::from(values[2])).await?;

        Ok(())
    }

    /// Read the data from the eeprom
    pub async fn read_eeprom(&mut self) -> Result<[u16; 3], Error<E>> {
        let u1: UEEPROM1 = self.tmp_ll.read().await?;
        let u2: UEEPROM2 = self.tmp_ll.read().await?;
        let u3: UEEPROM3 = self.tmp_ll.read().await?;

        Ok([u1.into(), u2.into(), u3.into()])
    }

    /// Wait for data and read the temperature in celsius and goes to shutdown since it's a oneshot
    pub async fn oneshot(&mut self, average: Average) -> Result<f32, Error<E>> {
        self.set_oneshot(average).await?;
        self.wait_for_data().await?;

        let res = self.read_temp_raw().await?;
        self.set_shutdown().await?;
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
        let config: Configuration = tmp117.tmp_ll.read().await?;
        if !config.data_ready() {
            return Err(Error::DataNotReady);
        }

        tmp117.read_temp_raw().await
    }

    /// Wait for the data to be ready and read the temperature in celsius
    pub async fn wait_temp(&mut self) -> Result<f32, Error<E>> {
        let tmp117 = unsafe { &mut *self.tmp117 };
        tmp117.set_data_ready().await?;
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
        tmp117.set_alert().await?;
        tmp117.wait_for_alert().await
    }
}
