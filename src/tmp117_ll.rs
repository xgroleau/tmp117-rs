use embedded_hal::i2c::{blocking::I2c, SevenBitAddress};

use crate::register::{EditableRegister, Register, WritableRegister};

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
    pub fn new(i2c: T) -> Self {
        Self { i2c }
    }

    fn write_internal<R>(&mut self, reg: R) -> Result<(), E>
    where
        R: Register,
        u16: From<R>,
    {
        let val: u16 = reg.into();
        let packet = val.to_be_bytes();

        self.i2c.write(ADDR, &[R::ADDRESS, packet[0], packet[1]])
    }

    pub fn read<R>(&mut self) -> Result<R, E>
    where
        R: Register + From<u16>,
    {
        let mut buff = [0; 2];
        self.i2c.write(ADDR, &[R::ADDRESS])?;
        self.i2c.read(ADDR, &mut buff)?;
        let val = u16::from_be_bytes(buff[0..2].try_into().unwrap());
        Ok(val.into())
    }

    pub fn edit<R, F>(&mut self, f: F) -> Result<(), E>
    where
        F: FnOnce(R) -> R,
        R: EditableRegister + From<u16>,
        u16: From<R>,
    {
        let val: R = self.read()?;
        let new_val = f(val);
        self.write_internal(new_val)
    }

    pub fn write<R>(&mut self, reg: R) -> Result<(), E>
    where
        R: WritableRegister,
        u16: From<R>,
    {
        self.write_internal(reg)
    }
}
