#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::fmt::Write;
use core::sync::atomic::{AtomicU16, Ordering};

use defmt::{info, unwrap};
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, Resolution};
use embassy_stm32::dma::NoDma;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{
    Input,
    Level,
    Output,
    Pull,
    Speed,
};
use embassy_stm32::pac;
use embassy_stm32::peripherals;
use embassy_stm32::spi::Config;
use embassy_stm32::time::Hertz;

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

use embassy_time::{
    Delay,
    Duration,
    Ticker
};

use heapless::String;

use smart_leds::{SmartLedsWrite, RGB8};

use ws2812_spi as ws2812;
use crate::ws2812::Ws2812;

mod ambient_light_sensor;
use ambient_light_sensor::sense_ambient_light;
mod spi_full_duplex;
use spi_full_duplex::MySpi;


pub(crate) static AMBIENT_LX: AtomicU16 = AtomicU16::new(0);
pub(crate) static MAIN_CHANNEL: Channel<ThreadModeRawMutex, String<64>, 5> = Channel::new();


#[embassy_executor::task]
pub async fn handle_button(
    mut led: Output<'static, peripherals::PB0>,
    butt: Input<'static, peripherals::PB7>,
    exti: peripherals::EXTI7
) {
    let mut button = ExtiInput::new(butt, exti);
    
    loop {
        button.wait_for_rising_edge().await;
        match led.get_output_level() {
            Level::High => led.set_low(),
            Level::Low => led.set_high()
        }

        let val = if AMBIENT_LX.load(Ordering::Relaxed) < 9 {
            "dark"
        }
        else {
            "light"
        };

        let mut msg = String::<64>::new();
        write!(msg, "boop detected. light level: {}", val).unwrap();
        MAIN_CHANNEL.send(msg).await;
    }
}

 
#[embassy_executor::task]
async fn pulse_light(spi: MySpi<'static, peripherals::SPI1, NoDma, NoDma>) {
    const N_LED: usize = 11;
    const DELAY_MS: u64 = 1000;
    const LED_SPACING: usize = 11; 
    const ON_COLOR: RGB8 = RGB8 {r: 255, g: 0, b: 0};

    let mut ticker = Ticker::every(Duration::from_millis(DELAY_MS));

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
            unwrap!(ws.write(data.iter().cloned()));
            ticker.next().await;
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
            unwrap!(ws.write(data.iter().cloned()));
            ticker.next().await;
        }
    }
}
 
 #[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    // Configure LED and button
    let led = Output::new(p.PB0, Level::High, Speed::Low);
    let butt = Input::new(p.PB7, Pull::Down);
    let exti = p.EXTI7;

    // Configure SPI pins
    let spi_peri = p.SPI1;
    let sck = p.PB3;
    let mosi = p.PB5;
    let miso = p.PB4;

    let mut spi_config = Config::default();
    spi_config.frequency = Hertz(3_000_000);
    let spi = MySpi::new(
        spi_peri, sck, mosi, miso, NoDma, NoDma, spi_config
    );

    // Configure ADC bus
    pac::RCC.ccipr().modify(|w| {
        w.set_adcsel(pac::rcc::vals::Adcsel::SYS);
    });
    pac::RCC.ahb2enr().modify(|w| w.set_adcen(true));

    let mut adc = Adc::new(p.ADC1, &mut Delay);
    adc.set_resolution(Resolution::EightBit);

    let sensor_pin = p.PA0;

    unwrap!(spawner.spawn(pulse_light(spi)));
    unwrap!(spawner.spawn(handle_button(led, butt, exti)));
    unwrap!(spawner.spawn(sense_ambient_light(adc, sensor_pin)));

    loop {
        let val = MAIN_CHANNEL.receive().await;
        info!("{}", val.as_str());
    }
}
