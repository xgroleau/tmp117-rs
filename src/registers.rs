use modular_bitfield::prelude::*;
use register_macros::{RERegister, RORegister, RWRegister};

/// Temperature register. The value is in 1/7.8125 m°C.
/// Following a reset, the temperature register reads –256 °C until the first conversion,
/// including averaging, is complete.
#[bitfield]
#[repr(u16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, RORegister)]
pub struct Temperature(i16);

/// Represent the dataready or alert pin select
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, BitfieldSpecifier)]
#[bits = 1]
pub enum AlertPin {
    ///Alert pin reflects the status of the alert flag
    Alert = 0,

    ///Alert pin reflects the status of teh data ready flag
    DataReady = 1,
}

/// Possible polarities
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, BitfieldSpecifier)]
#[bits = 1]
pub enum Polarity {
    ///Polarity set to active low
    ActiveLow = 0,

    ///Polarity set to active high
    ActiveHigh = 1,
}

/// Possible mode selection
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, BitfieldSpecifier)]
#[bits = 1]
pub enum TriggerMode {
    /// Alert mode
    Alert = 0,

    /// Thermal mode
    Thermal = 1,
}

/// Conversion averaging modes. Determines the number of
/// conversion results that are collected and averaged before
/// updating the temperature register. The average is an
/// accumulated average and not a running average.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, BitfieldSpecifier)]
#[bits = 2]
pub enum Average {
    /// No averaging
    NoAverage = 0,

    /// 8 averaged conversions
    Avg8 = 1,

    /// 32 averaged conversions
    Avg32 = 2,

    /// 64 averaged conversions
    Avg64 = 3,
}

/// Conversion cycle. It depends on the average selected. The enum represents the values for no average.
/// | CONV[2:0] | AVG[1:0] = 00 | AVG[1:0] = 01 | AVG[1:0] = 10 | AVG[1:0] = 11 |
/// |-----------|---------------|---------------|---------------|---------------|
/// | 000       | 15.5 ms       | 125 ms        | 500 ms        | 1 s           |
/// | 001       | 125 ms        | 125 ms        | 500 ms        | 1 s           |
/// | 010       | 250 ms        | 250 ms        | 500 ms        | 1 s           |
/// | 011       | 500 ms        | 500 ms        | 500 ms        | 1 s           |
/// | 100       | 1 s           | 1 s           | 1 s           | 1 s           |
/// | 101       | 4s            | 4 is          | 4s            | 4s            |
/// | 110       | 8 s           | 8 S           | 8s            | 8 s           |
/// | 111       | 16 S          | 16 S          | 16 S          | 16 S          |
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, BitfieldSpecifier)]
#[bits = 3]
pub enum Conversion {
    /// 15.5ms cycle time without average.
    Ms15_5 = 0,

    /// 125ms cycle time without average.
    Ms125 = 1,

    /// 250ms cycle time without average.
    Ms250 = 2,

    /// 500ms cycle time without average.
    Ms500 = 3,

    /// 1000ms cycle time without average.
    Ms1000 = 4,

    /// 4000ms cycle time without average.
    Ms4000 = 5,

    /// 8000ms cycle time without average.
    Ms8000 = 6,

    /// 16000ms cycle time without average.
    Ms16000 = 7,
}

/// Conversion mode
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, BitfieldSpecifier)]
#[bits = 3]
pub enum ConversionMode {
    /// Continous conversion mode
    Continuous = 0,

    /// Shutdown conversion mode
    Shutdown = 1,

    /// Oneshot conversion monde
    OneShot = 3,
}

/// Configuration register
#[bitfield]
#[repr(u16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, RERegister)]
pub struct Configuration {
    #[skip]
    __: B1,

    /// Software reset. When enabled, cause a reset with a duration of 2ms. The bit will always read back 0
    reset: bool,

    /// Data ready or Alert pin select bit.
    dr_alert: AlertPin,

    /// Alert pin polarity.
    polarity: Polarity,

    /// Thermal/alert mode select
    trigger_mode: TriggerMode,

    /// Average used for the conversion
    average: Average,

    /// Conversion cycle
    conversion: Conversion,

    /// Conversion mode
    mode: ConversionMode,

    /// EEPROM busy flag, either caused by programming or power-up
    #[skip(setters)]
    eeprom_busy: bool,

    /// Data ready flag.
    /// This flag indicates that the conversion is complete and the
    /// temperature register can be read. Every time the temperature
    /// register or configuration register is read, this bit is cleared. This
    /// bit is set at the end of the conversion when the temperature
    /// register is updated. Data ready can be monitored on the ALERT
    /// pin by setting bit 2 of the configuration register.
    #[skip(setters)]
    data_ready: bool,

    /// Alert mode:
    ///   Set when the conversion result is lower than the low limit.
    ///   Cleared when read.
    /// Thermal mode:
    ///   Always 0 in [Thermal](TriggerMode::Thermal) mode.
    #[skip(setters)]
    low_alert: bool,

    /// Alert mode:
    ///   Set when the conversion result is higher than the high limit.
    ///   Cleared when read.
    /// Thermal mode:
    ///   Set when the conversion result is higher than the therm limit
    ///   Cleared when the conversion result is lower than the hysteresis
    #[skip(setters)]
    high_alert: bool,
}
