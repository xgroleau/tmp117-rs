//! The low level driver of the TPM117
use device_register::{Register, RegisterInterface};
use embedded_hal::{
    i2c::ErrorKind,
    i2c::{I2c, SevenBitAddress},
};

use crate::register::Address;

/// The low level driver of the TPM117. Allows to read, write and edit the registers directly via the i2c bus
pub struct Tmp117LL<const ADDR: u8, T, E>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    i2c: T,
}

impl<const ADDR: u8, T, E> Tmp117LL<ADDR, T, E>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    /// Creates a new instace of the Tmp117 from an i2c bus
    pub fn new(i2c: T) -> Self {
        Self { i2c }
    }
}

impl<const ADDR: u8, T, E, R> RegisterInterface<R, Address, ErrorKind> for Tmp117LL<ADDR, T, E>
where
    R: Register<Address = Address, Error = ErrorKind> + Clone + From<u16>,
    u16: From<R>,
    E: embedded_hal::i2c::Error,
    T: embedded_hal::i2c::I2c + embedded_hal::i2c::ErrorType<Error = E>,
{
    fn read_register(&mut self) -> Result<R, R::Error> {
        let mut buff = [0; 2];
        self.i2c
            .write(ADDR, &[R::ADDRESS.0])
            .map_err(|e| e.kind())?;
        self.i2c.read(ADDR, &mut buff).map_err(|e| e.kind())?;
        let val = u16::from_be_bytes(buff[0..2].try_into().unwrap());
        Ok(val.into())
    }

    fn write_register(&mut self, register: &R) -> Result<(), R::Error> {
        let val: u16 = register.clone().into();
        let packet = val.to_be_bytes();

        self.i2c
            .write(ADDR, &[R::ADDRESS.0, packet[0], packet[1]])
            .map_err(|r| r.kind())
    }
}
