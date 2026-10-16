mod i_joystick_port;
mod state;

use c64_pac::cia1::{Ciapra, Ciaprb};
pub use i_joystick_port::IJoystickPort;
use state::JoystickState;

pub struct Joystick<'a, P: IJoystickPort> {
    port: &'a P,
}

impl<'a> Joystick<'a, Ciaprb> {
    pub fn port_1(port: &'a Ciaprb) -> Self { Self { port } }
}

impl<'a> Joystick<'a, Ciapra> {
    pub fn port_2(port: &'a Ciapra) -> Self { Self { port } }
}

impl<'a, P: IJoystickPort> Joystick<'a, P> {
    pub fn state(&self) -> JoystickState {
        unsafe { JoystickState::new_unchecked(self.port.read_port()) }
    }
}
