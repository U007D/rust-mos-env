mod state;

use core::ptr::read_volatile;

use super::Cia1;
use state::JoyState;

pub struct Joystick {
    base_addr: usize,
}

impl Joystick {
    pub fn new(base_addr: Cia1) -> Self {
        Self {
            base_addr: usize::from(base_addr),
        }
    }

    pub fn state(&self) -> JoyState {
        JoyState::new(unsafe { read_volatile(self.base_addr as *const u8) })
    }
}
