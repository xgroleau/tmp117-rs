//! Async low level driver of the tmp117
use core::future::Future;

use device_register::Register;
use device_register_async::RegisterInterface;
use embedded_hal::i2c::{ErrorKind, SevenBitAddress};
use embedded_hal_async::i2c::I2c;

use crate::register::Address;

/// Async low level driver of the TPM117. Allows to read, write and edit the registers directly via the i2c bus
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
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    type ReadOutput<'a> = impl Future<Output = Result<R, R::Error>>
    where
        Self: 'a ;

    fn read_register(&mut self) -> Self::ReadOutput<'_> {
        async {
            let mut buff = [0; 2];
            self.i2c
                .write(ADDR, &[R::ADDRESS.0])
                .await
                .map_err(|e| e.kind())?;
            self.i2c.read(ADDR, &mut buff).await.map_err(|e| e.kind())?;
            let val = u16::from_be_bytes(buff[0..2].try_into().unwrap());
            Ok(val.into())
        }
    }

    type WriteOutput<'a> = impl Future<Output = Result<(), R::Error>>
    where
        Self: 'a,
        R: 'a;

    fn write_register<'a>(&'a mut self, register: &'a R) -> Self::WriteOutput<'a> {
        async {
            let val: u16 = register.clone().into();
            let packet = val.to_be_bytes();

            self.i2c
                .write(ADDR, &[R::ADDRESS.0, packet[0], packet[1]])
                .await
                .map_err(|e| e.kind())
        }
    }
}
