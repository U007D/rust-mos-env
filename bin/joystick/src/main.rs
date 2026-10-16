#![no_std]
#![no_main]

mod color;
mod component;
mod joystick;
mod panic;

use core::ptr::write_volatile;

use color::Color;
use component::{Cia1, VicIi};
use joystick::Joystick;

#[unsafe(no_mangle)]
extern "C" fn main(_argc: isize, _argv: *const *const u8) -> isize {
    let joy_1 = Joystick::new(Cia1::PortB);

    loop {
        match   joy_1.state().is_fire() {
            true => set_border_color(Color::Red),
            false => set_border_color(Color::LightBlue),
        }
    }
}

fn set_border_color(color: Color) {
    unsafe { write_volatile((usize::from(VicIi::BorderColor)) as *mut Color, color) };
}
