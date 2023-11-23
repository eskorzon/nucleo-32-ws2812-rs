use core::fmt::Write;
use heapless::{String, Vec};

use {defmt_rtt as _, panic_probe as _};

use crate::{BUTTON_BOARD_VEC, BTN_CH, MAIN_CHANNEL, MSG_SIZE};


#[embassy_executor::task]
pub(crate) async fn button_board() {
    loop {
        let veccy_boi = BTN_CH.receive().await;
        let pin: usize = veccy_boi[0] as usize;
        let val: bool = veccy_boi[1] != 0;

        let state_copy: Vec<bool, 4>;
        {
            let mut state = BUTTON_BOARD_VEC.lock().await;
            (*state)[pin] = val;
            state_copy = state.clone();
        }

        let mut msg = String::<MSG_SIZE>::new();
        write!(
            msg, "Button Board: {} {} {} {}", state_copy[0] as u8, state_copy[1] as u8, state_copy[2] as u8, state_copy[3] as u8
        ).unwrap();
        MAIN_CHANNEL.send(msg).await;
    }
}
