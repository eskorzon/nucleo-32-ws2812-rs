// use core::fmt::Write;
use {defmt_rtt as _, panic_probe as _};

use embassy_stm32::adc::Adc;
use embassy_stm32::peripherals;

use embassy_time::{
    Duration,
    Ticker
};
use heapless::Vec;
// use heapless::String;

use crate::{
    ADC_VEC,
    // MAIN_CHANNEL,
    // MSG_SIZE
};


#[embassy_executor::task]
pub(crate) async fn adc_reader(
    mut adc: Adc<'static, peripherals::ADC1>,
    mut a0: peripherals::PA0,
    mut a1: peripherals::PA1,
    mut a2: peripherals::PA3,
    mut a3: peripherals::PA4,
    mut a4: peripherals::PA5,
    mut a5: peripherals::PA6,
    mut a6: peripherals::PA7,
    mut a7: peripherals::PA2,
) {
    let mut ticker = Ticker::every(Duration::from_millis(10));

    loop {
        ticker.next().await;

        let mut vals = Vec::<u16, 8>::new();
        vals.extend_from_slice(
            &[
                adc.read(&mut a0),
                adc.read(&mut a1),
                adc.read(&mut a2),
                adc.read(&mut a3),
                adc.read(&mut a4),
                adc.read(&mut a5),
                adc.read(&mut a6),
                adc.read(&mut a7)
            ]
        ).unwrap();

        {
            let mut adc_vec = ADC_VEC.lock().await;
            // *adc_vec = vals.clone();
            *adc_vec = vals;
        }
        // let mut msg = String::<MSG_SIZE>::new();
        // write!(
        //     msg, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
        //     vals[0], vals[1], vals[2], vals[3], vals[4], vals[5], vals[6], vals[7]
        // ).unwrap();
        // MAIN_CHANNEL.send(msg).await;
    }
}