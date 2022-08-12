use embedded_hal::i2c::SevenBitAddress;
use embedded_hal_async::i2c::I2c;

use crate::{
    register::{EditableRegister, Register, WritableRegister},
    tmp117_ll::Error,
};

pub struct TMP117LL<const ADDR: u8, T, E>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    i2c: T,
}

impl<const ADDR: u8, T, E> TMP117LL<ADDR, T, E>
where
    T: I2c<SevenBitAddress, Error = E>,
    E: embedded_hal::i2c::Error,
{
    pub async fn new(i2c: T) -> Self {
        Self { i2c }
    }

    async fn write_internal<R>(&mut self, reg: R) -> Result<(), Error<E>>
    where
        R: Register,
        u16: From<R>,
    {
        let val: u16 = reg.into();
        let packet = val.to_be_bytes();

        self.i2c
            .write(ADDR, &[R::ADDRESS, packet[0], packet[1]])
            .await
            .map_err(Error::Bus)
    }

    pub async fn read<R>(&mut self) -> Result<R, Error<E>>
    where
        R: Register + From<u16>,
    {
        let mut buff = [0; 2];
        self.i2c
            .write(ADDR, &[R::ADDRESS])
            .await
            .map_err(Error::Bus)?;
        self.i2c.read(ADDR, &mut buff).await.map_err(Error::Bus)?;
        let val = u16::from_be_bytes(buff[0..2].try_into().unwrap());
        Ok(val.into())
    }

    pub async fn edit<R, F>(&mut self, f: F) -> Result<(), Error<E>>
    where
        F: FnOnce(R) -> R,
        R: EditableRegister + From<u16>,
        u16: From<R>,
    {
        let val: R = self.read().await?;
        let new_val = f(val);
        self.write_internal(new_val).await
    }

    pub async fn write<R>(&mut self, reg: R) -> Result<(), Error<E>>
    where
        R: WritableRegister,
        u16: From<R>,
    {
        self.write_internal(reg).await
    }
}
