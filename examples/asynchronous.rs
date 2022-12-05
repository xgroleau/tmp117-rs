#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::{interrupt, twim::Twim};
use tmp117::{asynchronous::Tmp117, register::Average};
use {defmt_rtt as _, embassy_nrf as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    info!("Start");

    let irq = interrupt::take!(SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0);
    let twi = Twim::new(p.TWISPI0, irq, p.P1_10, p.P1_11, Default::default());

    let mut tmp = Tmp117::<0x49, _, _, _>::new(twi);

    // Read and goes to shutdown mode
    info!("Reading temp once");
    let temperature = tmp.oneshot(Average::NoAverage).await.unwrap();
    info!("Temperature {}", temperature);

    info!("Using continuous mode");
    tmp.continuous(Default::default(), |t| async move {
        let t = unsafe { &mut *t };
        for _ in 0..10 {
            let temp = t.wait_temp().await?;
            info!("Temperature {}", temp);
        }
        Ok(())
    })
    .await
    .unwrap();

    let mut eeprom_data = tmp.read_eeprom().await.unwrap();
    info!("Eeprom data before: {}", eeprom_data);

    eeprom_data[2] += 1;

    info!("Writing {} to eeprom", eeprom_data);
    tmp.write_eeprom(eeprom_data).await.unwrap();

    let eeprom_data2 = tmp.read_eeprom().await.unwrap();
    assert_eq!(eeprom_data, eeprom_data2);

    cortex_m::asm::bkpt();
}
