use core::{arch::naked_asm, cell::Cell};

use c64_pac::Vic;

use crate::game::{asm::{pop_regs, push_regs, rti}, music_irq::music_advance};

struct MusicTimer<T>(Cell<T>);
unsafe impl<T> Sync for MusicTimer<T> {}
impl<T: Copy> MusicTimer<T> {
    const fn new(v: T) -> Self { Self(Cell::new(v)) }

    fn get(&self) -> T { self.0.get() }

    fn set(&self, v: T) { self.0.set(v) }
}

static MUSIC_TIMER: MusicTimer<u8> = MusicTimer::new(0);

#[naked]
pub unsafe extern "C" fn music_irq_ntsc() {
    unsafe {
        naked_asm! (
            push_regs!(),
            "jsr {body}",
            pop_regs!(),
            rti!(),
            body = sym music_irq_ntsc_inner
        )
    }
}

// Plain function. Ordinary prologue/epilogue, so it can use c64_pac normally.
extern "C" fn music_irq_ntsc_inner() {
    // SAFETY: this handler's only hardware access is the VICIRQ_W ack strobe, which nothing outside
    // of the NTSC & PAL versions of this function this ISR write.  Exactly one of these two ISR's
    // is selected during initialization and used exclusively.
    let vic = unsafe { Vic::steal() };
    vic.vicirq_w().write(|w| w.raster().ack());

    // Advance the music tick but skip calling `music_advance` every 5th frame
    let clock = MUSIC_TIMER.get();
    match clock {
        // Eighth frame?  Reset clock and *don't* play next music frame
        3 => MUSIC_TIMER.set(0),
        _ => {
            // Otherwise advance clock and play next music frame
            MUSIC_TIMER.set(clock.wrapping_add(1));
            unsafe {
                music_advance();
            }
        },
    }
}
