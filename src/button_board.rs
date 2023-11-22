use core::fmt::Write;
use embassy_stm32::exti::ExtiInput;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_stm32::peripherals;
use embassy_sync::signal::Signal;
use heapless::String;

use {defmt_rtt as _, panic_probe as _};

use crate::{BUTTON_BOARD_VEC, MAIN_CHANNEL};

static EDGE_SIGNAL: Channel<ThreadModeRawMutex, bool, 4> = Channel::<ThreadModeRawMutex, bool, 4>::new();

// D6..D9 = R1..R4
#[embassy_executor::task]
pub(crate) async fn r1_listener(pin_no: usize, mut exti: ExtiInput<'static, peripherals::PB1>) {
    loop {
        exti.wait_for_any_edge().await;
        {
            let mut val = BUTTON_BOARD_VEC.lock().await;
            (*val)[pin_no] = exti.is_high();
        }
        EDGE_SIGNAL.send(true).await;
    }
}


#[embassy_executor::task]
pub(crate) async fn r2_listener(pin_no: usize, mut exti: ExtiInput<'static, peripherals::PC14>) {
    loop {
        exti.wait_for_any_edge().await;
        {
            let mut val = BUTTON_BOARD_VEC.lock().await;
            (*val)[pin_no] = exti.is_high();
        }
        EDGE_SIGNAL.send(true).await;
    }
}


#[embassy_executor::task]
pub(crate) async fn r3_listener(pin_no: usize, mut exti: ExtiInput<'static, peripherals::PC15>) {
    loop {
        exti.wait_for_any_edge().await;
        {
            let mut val = BUTTON_BOARD_VEC.lock().await;
            (*val)[pin_no] = exti.is_high();
        }
        EDGE_SIGNAL.send(true).await;
    }
}


#[embassy_executor::task]
pub(crate) async fn r4_listener(pin_no: usize, mut exti: ExtiInput<'static, peripherals::PA8>) {
    loop {
        exti.wait_for_any_edge().await;
        {
            let mut val = BUTTON_BOARD_VEC.lock().await;
            (*val)[pin_no] = exti.is_high();
        }
        EDGE_SIGNAL.send(true).await;
    }
}


#[embassy_executor::task]
pub(crate) async fn button_board() {
    loop {
        let _ = EDGE_SIGNAL.receive().await;
        // Check the value of all the EXTI pins
        let r1: bool;
        let r2: bool;
        let r3: bool;
        let r4: bool;
        {
            let val = BUTTON_BOARD_VEC.lock().await;
            r1 = (*val)[0];
            r2 = (*val)[1];
            r3 = (*val)[2];
            r4 = (*val)[3];
        }
        let mut msg = String::<64>::new();
        write!(
            msg, "Button Board: {} {} {} {}", r1 as u8, r2 as u8, r3 as u8, r4 as u8
        ).unwrap();
        MAIN_CHANNEL.send(msg).await;
    }
}
