//! The low level driver of the TPM117
use core::marker::PhantomData;

use device_register::{Register, RegisterInterface};
use embedded_hal::i2c::{I2c, SevenBitAddress};

use crate::error::ErrorLL;
use crate::register::Address;

/// The low level driver of the TPM117. Allows to read, write and edit the registers directly via the i2c bus
pub struct Tmp117LL<T, E> {
    i2c: T,
    addr: u8,
    e: PhantomData<E>,
}

impl<T, E> Tmp117LL<T, E>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    /// Creates a new instace of the Tmp117 from an i2c bus
    pub fn new(i2c: T, addr: u8) -> Self {
        Self {
            i2c,
            addr,
            e: PhantomData,
        }
    }
}

impl<T, E, R> RegisterInterface<R, Address> for Tmp117LL<T, E>
where
    R: Register<Address = Address> + Clone + TryFrom<u16>,
    u16: From<R>,
    E: embedded_hal::i2c::Error,
    T: embedded_hal::i2c::I2c + embedded_hal::i2c::ErrorType<Error = E>,
{
    type Error = ErrorLL<E>;

    fn read_register(&mut self) -> Result<R, Self::Error> {
        let mut buff = [0; 2];
        self.i2c
            .write_read(self.addr, &[R::ADDRESS.0], &mut buff)
            .map_err(ErrorLL::Bus)?;
        let val = u16::from_be_bytes(buff[0..2].try_into().unwrap());
        R::try_from(val).map_err(|_| ErrorLL::InvalidData)
    }

    fn write_register(&mut self, register: &R) -> Result<(), Self::Error> {
        let val: u16 = register.clone().into();
        let packet = val.to_be_bytes();

        self.i2c
            .write(self.addr, &[R::ADDRESS.0, packet[0], packet[1]])
            .map_err(ErrorLL::Bus)
    }
}
