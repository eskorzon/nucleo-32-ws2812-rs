// use defmt::*;
use core::fmt::Write;
use core::sync::atomic::Ordering;
use embassy_stm32::adc::Adc;
use embassy_stm32::peripherals;

use embassy_time::{
    Duration,
    Ticker
};
use heapless::String;

use {defmt_rtt as _, panic_probe as _};

use crate::{AMBIENT_LX, MAIN_CHANNEL};


#[embassy_executor::task]
pub(crate) async fn sense_ambient_light(mut adc: Adc<'static, peripherals::ADC1>, mut sensor_pin: peripherals::PA0) {
    let mut ticker = Ticker::every(Duration::from_millis(100));
    loop {
        ticker.next().await;
        let val = adc.read(&mut sensor_pin);
        AMBIENT_LX.store(val, Ordering::Relaxed);

        let mut msg = String::<64>::new();
        write!(msg, "LX Reading: {}", val).unwrap();
        MAIN_CHANNEL.send(msg).await;
    }
}
 