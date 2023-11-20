#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use embassy_stm32::dma::NoDma;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{
    Input,
    Level,
    Output,
    Pull,
    Speed,
};
use embassy_stm32::peripherals;
use embassy_stm32::spi::Config;
use embassy_stm32::time::Hertz;
use embassy_time::{
    Duration,
    Timer
};
use smart_leds::{SmartLedsWrite, RGB8};

mod spi_full_duplex;
use spi_full_duplex::MySpi;

use ws2812_spi as ws2812;
use crate::ws2812::Ws2812;


#[embassy_executor::task]
async fn handle_button(mut led: Output<'static, peripherals::PB0>, butt: Input<'static, peripherals::PB7>, exti: peripherals::EXTI7) {
    let mut button = ExtiInput::new(butt, exti);
    
    loop {
        button.wait_for_rising_edge().await;
        match led.get_output_level() {
            Level::High => led.set_low(),
            Level::Low => led.set_high()
        }
        info!("boop detected");
    }
}


#[embassy_executor::task]
async fn pulse_light(spi: MySpi<'static, peripherals::SPI1, NoDma, NoDma>) {
    const N_LED: usize = 11;
    const DELAY_MS: u64 = 100;
    const LED_SPACING: usize = 11; 
    const ON_COLOR: RGB8 = RGB8 {r: 1, g: 0, b: 0};

    let mut ws = Ws2812::new(spi);
    let mut data: [RGB8; N_LED] = [ON_COLOR; N_LED];
    loop {
        // idea is to iterate to move lights through box in a cycle: [100] -> [010] -> [001]
        for k in 0..LED_SPACING {   // iterate through each led in the box
            for i in 0..N_LED {
                let color: RGB8 = match i % LED_SPACING == k {
                    true => ON_COLOR,
                    _ => RGB8::default()
                };
                data[i] = color;
            }
            ws.write(data.iter().cloned()).unwrap();
            Timer::after(Duration::from_millis(DELAY_MS)).await;
        }

        // reverse direction at end of array
        for k in (0..LED_SPACING).rev() {   // iterate through each led in the box in reverse
            for i in 0..N_LED {
                let color: RGB8 = match i % LED_SPACING == k {
                    true => ON_COLOR,
                    _ => RGB8::default()
                };
                data[i] = color;
            }
            ws.write(data.iter().cloned()).unwrap();
            Timer::after(Duration::from_millis(DELAY_MS)).await;
        }
    }
}

 
 #[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let led = Output::new(p.PB0, Level::High, Speed::Low);
    let butt = Input::new(p.PB7, Pull::Down);
    let exti = p.EXTI7;

    let spi_peri = p.SPI1;
    let sck = p.PB3;
    let mosi = p.PB5;
    let miso = p.PB4;

    let mut spi_config = Config::default();
    spi_config.frequency = Hertz(3_000_000);
    let spi = MySpi::new(
        spi_peri, sck, mosi, miso, NoDma, NoDma, spi_config
    );

    spawner.spawn(pulse_light(spi)).unwrap();
    spawner.spawn(handle_button(led, butt, exti)).unwrap();
}
