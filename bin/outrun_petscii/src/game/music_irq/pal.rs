use core::arch::naked_asm;

use c64_pac::Vic;

use crate::game::asm::{pop_regs, push_regs, rti};
use crate::game::music_irq::music_advance;

#[naked]
pub unsafe extern "C" fn music_irq_pal() {
    unsafe {
        naked_asm! {
            push_regs!(),
            "jsr {body}",
            pop_regs!(),
            rti!(),
            body = sym music_irq_pal_inner
        };
    }
}

// Plain function. Ordinary prologue/epilogue, so it can use c64_pac normally.
extern "C" fn music_irq_pal_inner() {
    // SAFETY: this handler's only hardware access is the VICIRQ_W ack strobe, which nothing outside
    // of the NTSC & PAL versions of this function this ISR write.  Exactly one of these two ISR's
    // is selected during initialization and used exclusively.
    let vic = unsafe { Vic::steal() };
    vic.vicirq_w().write(|w| w.raster().ack());
    
    unsafe { music_advance(); }
}
