//! The definitions of drivers of the TMP117
#![allow(clippy::identity_op)]
#![allow(missing_docs)]

use bilge::prelude::*;
use device_register::{RERegister, RORegister, RWRegister};

/// The address of the register
pub struct Address(pub u8);

/// Temperature register. The value is in 1/7.8125 m°C.
/// Following a reset, the temperature register reads –256 °C until the first conversion,
/// including averaging, is complete. Is in two complements
#[bitsize(16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, DebugBits, RORegister, FromBits)]
#[register(ty = "Address", addr = "Address(0x00)")]
pub struct Temperature(pub u16);

/// Represent the dataready or alert pin select
#[bitsize(1)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, FromBits)]
pub enum AlertPinSelect {
    ///Alert pin reflects the status of the alert flag
    Alert = 0,

    ///Alert pin reflects the status of the data ready flag
    DataReady = 1,
}

/// Possible polarities
#[bitsize(1)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, FromBits)]
pub enum Polarity {
    ///Polarity set to active low
    ActiveLow = 0,

    ///Polarity set to active high
    ActiveHigh = 1,
}

/// Possible mode selection
#[bitsize(1)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, FromBits)]
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
#[bitsize(2)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, FromBits)]
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

impl Default for Average {
    fn default() -> Self {
        Self::NoAverage
    }
}

/// Conversion cycle. It depends on the average selected. The enum represents the values for no average.
/// | CONV      | AVG = 00      | AVG = 01      | AVG = 10      | AVG = 11      |
/// |-----------|---------------|---------------|---------------|---------------|
/// | 000       | 15.5 ms       | 125 ms        | 500 ms        | 1 s           |
/// | 001       | 125 ms        | 125 ms        | 500 ms        | 1 s           |
/// | 010       | 250 ms        | 250 ms        | 500 ms        | 1 s           |
/// | 011       | 500 ms        | 500 ms        | 500 ms        | 1 s           |
/// | 100       | 1 s           | 1 s           | 1 s           | 1 s           |
/// | 101       | 4s            | 4 is          | 4s            | 4s            |
/// | 110       | 8 s           | 8 S           | 8s            | 8 s           |
/// | 111       | 16 S          | 16 S          | 16 S          | 16 S          |
#[bitsize(3)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, FromBits)]
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
impl Default for Conversion {
    fn default() -> Self {
        Self::Ms15_5
    }
}

/// Conversion mode
#[bitsize(2)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug, TryFromBits)]
pub enum ConversionMode {
    /// Continuous conversion mode
    Continuous = 0b0,

    /// Shutdown conversion mode
    Shutdown = 0b01,

    /// Oneshot conversion monde
    OneShot = 0b11,
}

/// Configuration register of the tpm117
#[bitsize(16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, DebugBits, RERegister, TryFromBits)]
#[register(ty = "Address", addr = "Address(0x01)")]
pub struct Configuration {
    reserved: u1,

    /// Software reset. When enabled, cause a reset with a duration of 2ms. The bit will always read back 0
    pub reset: bool,

    /// Data ready or Alert pin select bit.
    pub dr_alert: AlertPinSelect,

    /// Alert pin polarity.
    pub polarity: Polarity,

    /// Thermal/alert mode select
    pub trigger_mode: TriggerMode,

    /// Average used for the conversion
    pub average: Average,

    /// Conversion cycle
    pub conversion: Conversion,

    /// Conversion mode
    pub mode: ConversionMode,

    /// EEPROM busy flag, either caused by programming or power-up
    pub eeprom_busy: bool,

    /// Data ready flag.
    /// This flag indicates that the conversion is complete and the
    /// temperature register can be read. Every time the temperature
    /// register or configuration register is read, this bit is cleared. This
    /// bit is set at the end of the conversion when the temperature
    /// register is updated. Data ready can be monitored on the ALERT
    /// pin by setting bit 2 of the configuration register.
    pub data_ready: bool,

    /// Alert mode:
    ///   Set when the conversion result is lower than the low limit.
    ///   Cleared when read.
    /// Thermal mode:
    ///   Always 0 in [Thermal](TriggerMode::Thermal) mode.
    pub low_alert: bool,

    /// Alert mode:
    ///   Set when the conversion result is higher than the high limit.
    ///   Cleared when read.
    /// Thermal mode:
    ///   Set when the conversion result is higher than the therm limit
    ///   Cleared when the conversion result is lower than the hysteresis
    pub high_alert: bool,
}

/// The high limit register is a 16-bit, read/write register that stores the high limit for comparison with the temperature result.
/// One LSB equals 7.8125 m°C. The range of the register is ±256 °C. Negative numbers are represented in binary
/// two's complement format. Following power-up or a general-call reset, the high-limit register is loaded with the
/// stored value from the EEPROM. The factory default reset value is 6000h. Is written in two's complement.
#[bitsize(16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, DebugBits, RWRegister, FromBits)]
#[register(ty = "Address", addr = "Address(0x02)")]
pub struct HighLimit(pub u16);

/// The low limit register is configured as a 16-bit, read/write register that stores the low limit for comparison with the
/// temperature result. One LSB equals 7.8125 m°C. The range of the register is ±256 °C. Negative numbers
/// are represented in binary two's complement format. The data format is the same as the temperature register.
/// Following power-up or reset, the low-limit register is loaded with the stored value from the EEPROM. The factory
/// default reset value is 8000h.Is written in two's complement.
#[bitsize(16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, DebugBits, RWRegister, FromBits)]
#[register(ty = "Address", addr = "Address(0x03)")]
pub struct LowLimit(pub u16);

/// The eeprom configuration register
#[bitsize(16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, DebugBits, RERegister, FromBits)]
#[register(ty = "Address", addr = "Address(0x04)")]
pub struct EEPROM {
    reserved: u14,

    /// EEPROM busy flag, either caused by programming or power-up
    ///Mirror the `eeprom_busy` in the [Configuration](Configuration) register
    pub busy: bool,

    /// If the eeprom is unlock. If unlocked, any writes to the registers program will be written to the eeprom
    pub unlock: bool,
}

/// The EEPROM1 register is a 16-bit register that be used as a scratch pad by the customer to store general-
/// purpose data. This register has a corresponding EEPROM location. Writes to this address when the EEPROM is
/// locked write data into the register and not to the EEPROM. Writes to this register when the EEPROM is unlocked
/// causes the corresponding EEPROM location to be programmed.
/// To support NIST tracebility, do not delete or reprogram the [UEEPROM1](UEEPROM1) register
#[bitsize(16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, DebugBits, RWRegister, FromBits)]
#[register(ty = "Address", addr = "Address(0x05)")]
pub struct UEEPROM1(pub u16);

/// Same function as register [UEEPROM1](UEEPROM1) minus the ID for NSIT tracability
#[bitsize(16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, DebugBits, RWRegister, FromBits)]
#[register(ty = "Address", addr = "Address(0x06)")]
pub struct UEEPROM2(pub u16);

/// Same function as register [UEEPROM1](UEEPROM1) minus the ID for NSIT tracability
#[bitsize(16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, DebugBits, RWRegister, FromBits)]
#[register(ty = "Address", addr = "Address(0x07)")]
pub struct UEEPROM3(pub u16);

/// This 16-bit register is to be used as a user-defined temperature offset register during system calibration. The
/// offset will be added to the temperature result after linearization. It has a same resolution of 7.8125 m°C and
/// same range of ±256 °C as the temperature result register. The data format is the same as the temperature
/// register. If the added result is out of boundary, then the temperature result will show as the maximum or
/// minimum value. Is written in two's complement.
#[bitsize(16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, DebugBits, RWRegister, FromBits)]
#[register(ty = "Address", addr = "Address(0x08)")]
pub struct TemperatureOffset(pub u16);

/// Indicates the device ID
#[bitsize(16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, DebugBits, RORegister, FromBits)]
#[register(ty = "Address", addr = "Address(0x0F)")]
pub struct DeviceID {
    /// Indicates the device ID
    pub device_id: u12,

    /// Indicates the revision number
    pub revision: u4,
}
