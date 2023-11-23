#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::fmt::Write;

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
use embassy_sync::mutex::Mutex;
use embassy_sync::channel::Channel;

use embassy_time::{
    Delay,
    Duration,
    Ticker,
    Timer
};

use heapless::{
    String,
    Vec
};

use smart_leds::{SmartLedsWrite, RGB8};

use ws2812_spi as ws2812;
use crate::ws2812::Ws2812;

mod adc_reader;
use adc_reader::adc_reader;

mod button_board;
use button_board::button_board;

mod spi_full_duplex; 
use spi_full_duplex::MySpi;


pub(crate) static ADC_VEC: Mutex<ThreadModeRawMutex, Vec<u16, 8>> = Mutex::<
    ThreadModeRawMutex,
    Vec<u16, 8>
>::new(Vec::<u16, 8>::new());

pub(crate) static BUTTON_BOARD_VEC: Mutex<ThreadModeRawMutex, Vec<bool, 4>> = Mutex::<
    ThreadModeRawMutex,
    Vec<bool, 4>
>::new(Vec::<bool, 4>::new());

#[repr(usize)]
pub(crate) enum AdcPins {
    AmbientLx = 0,
}

pub(crate) static MAIN_CHANNEL: Channel<ThreadModeRawMutex, String<64>, 8> = Channel::new();
pub(crate) static BTN_CH: Channel<ThreadModeRawMutex, Vec::<u8, 2>, 8> = Channel::new();


#[embassy_executor::task]
pub async fn motor_button(
    mut mtr: Output<'static, peripherals::PB6>,
    mut button: ExtiInput<'static, peripherals::PB7>
) {
    const HOLD_MS: u64 = 100;

    loop {
        button.wait_for_rising_edge().await;
        mtr.set_high();

        Timer::after_millis(HOLD_MS).await;
        mtr.set_low();

        let ambient_lx: u16;
        {
            let adc_vec = ADC_VEC.lock().await;
            ambient_lx = adc_vec[AdcPins::AmbientLx as usize];
        }

        let mut msg = String::<64>::new();
        write!(
            msg, "boop detected. activating motor. light level: {}", ambient_lx
        ).unwrap();
        MAIN_CHANNEL.send(msg).await;
    }
}


#[embassy_executor::task]
pub async fn led_button(
    mut led: Output<'static, peripherals::PB0>,
    mut button: ExtiInput<'static, peripherals::PB7>
) {
    loop {
        button.wait_for_rising_edge().await;
        led.toggle();

        let mut msg = String::<64>::new();
        let ambient_lx: u16;
        {
            let adc_vec = ADC_VEC.lock().await;
            ambient_lx = adc_vec[AdcPins::AmbientLx as usize];
        }
        write!(
            msg, "boop detected. light level: {}", if ambient_lx < 9 { "dark" } else { "light" }
        ).unwrap();
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


macro_rules! monitor_exti {
    ($name:ident, $exti:ty) => {
        #[embassy_executor::task]
        pub(crate) async fn $name(pin_no: u8, mut exti: $exti) {
            loop {
                exti.wait_for_any_edge().await;
                let mut val = Vec::<u8, 2>::new();
                val.extend_from_slice(&[pin_no, exti.is_high() as u8]).unwrap();
                BTN_CH.send(val).await;
            }
        }
    }
}
 

 #[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    // Configure LED and button
    // let led = Output::new(p.PB0, Level::High, Speed::Low);   Actual LED
    let mtr = Output::new(p.PB6, Level::High, Speed::Low);      // Sp
    let button = Input::new(p.PB7, Pull::Down);
    let button = ExtiInput::new(button, p.EXTI7);

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

    let a0 = p.PA0;
    let a1 = p.PA1;
    let a2 = p.PA3;
    let a3 = p.PA4;
    let a4 = p.PA5;
    let a5 = p.PA6;
    let a6 = p.PA7;
    let a7 = p.PA2;

    {
        let mut val = BUTTON_BOARD_VEC.lock().await;
        val.extend_from_slice(
            &[false, false, false, false]
        ).unwrap();
    }

    monitor_exti!(exti1, ExtiInput<'static, peripherals::PB1>);
    monitor_exti!(exti9, ExtiInput<'static, peripherals::PA9>);
    monitor_exti!(exti10, ExtiInput<'static, peripherals::PA10>);
    monitor_exti!(exti8, ExtiInput<'static, peripherals::PA8>);

    unwrap!(spawner.spawn(exti1(0, ExtiInput::new(Input::new(p.PB1, Pull::Down), p.EXTI1))));
    unwrap!(spawner.spawn(exti9(1, ExtiInput::new(Input::new(p.PA9, Pull::Down), p.EXTI9))));
    unwrap!(spawner.spawn(exti10(2, ExtiInput::new(Input::new(p.PA10, Pull::Down), p.EXTI10))));
    unwrap!(spawner.spawn(exti8(3, ExtiInput::new(Input::new(p.PA8, Pull::Down), p.EXTI8))));

    unwrap!(spawner.spawn(pulse_light(spi)));
    unwrap!(spawner.spawn(motor_button(mtr, button)));
    unwrap!(spawner.spawn(adc_reader(
        adc, a0, a1, a2, a3, a4, a5, a6, a7
    )));
    unwrap!(spawner.spawn(button_board()));

    loop {
        let val = MAIN_CHANNEL.receive().await;
        info!("{}", val.as_str());
    }
}
