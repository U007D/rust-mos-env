//! Ported from [OutRun_PETSCII](https://csdb.dk/release/?id=189844):
//! Code and graphics by aNdy (CSDb id=6404)
//! Music (from C64 game) by jason c. brooke (CSDb id=8136)

#![no_std]
#![no_main]
// `naked_asm!()` support
#![feature(asm_experimental_arch, naked_functions, never_type)]

mod error;
mod game;
mod irq_critical_section;
mod joystick;
mod panic;
mod shared_const;

#[allow(unused_imports)]
use error::{Error, Result};
pub use joystick::Joystick;

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    let _err = inner_main().unwrap_err();

    // Note: calling Panic handler adds 76 bytes (64b `_sigabort` + 9b `raise` +
    // 3b `abort`)
    panic!();
}

fn inner_main() -> Result<!> {
    // Initialize game environment; acquire initialized `Peripherals` struct
    let periphs = game::init()?;
    game::screen_clear(&periphs);
    game::title(&periphs);

    let joy_2 = Joystick::port_2(periphs.cia1.ciapra());
    
    loop {
        match joy_2.state().is_fire() {
            false => periphs.vic.extcol().write(|w| w.color().black()),
            true => periphs.vic.extcol().write(|w| w.color().white()),
        };
    }
}
