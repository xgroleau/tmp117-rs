#![no_std]
#![no_main]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::{interrupt, twim::Twim};
use tmp117::{asynchronous::Tmp117, register::Average, ShutdownMode};
use {defmt_rtt as _, embassy_nrf as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    info!("Start");

    let irq = interrupt::take!(SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0);
    let twi = Twim::new(p.TWISPI0, irq, p.P1_10, p.P1_11, Default::default());

    let tmp = Tmp117::<0x49, _, _, _, _>::new(twi);

    // Read and goes to shutdown mode
    info!("Transition to one shot");
    let tmp = tmp.to_oneshot(Average::Avg32).await.unwrap();

    info!("Reading temp");
    let (temperature, tmp) = tmp.read_temp().await.unwrap();
    // Verify type
    let tmp: Tmp117<0x49, _, _, _, ShutdownMode> = tmp;
    info!("Temperature {}", temperature);
}
