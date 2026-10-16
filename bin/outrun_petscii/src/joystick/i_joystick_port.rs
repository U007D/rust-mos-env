use c64_pac::cia1::{Ciapra, Ciaprb};

pub trait IJoystickPort {
    fn read_port(&self) -> u8;
}

impl IJoystickPort for Ciaprb {
    fn read_port(&self) -> u8 { self.read().bits() }
}

impl IJoystickPort for Ciapra {
    fn read_port(&self) -> u8 { self.read().bits() }
}
