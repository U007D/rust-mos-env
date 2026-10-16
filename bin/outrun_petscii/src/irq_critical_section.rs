//! critical-section impl.  Ensures no two contexts can race for a critical section.
//! `Peripherals::take()` uses this.

use core::arch::asm;

use critical_section::RawRestoreState;

use crate::shared_const::SR_I_MASK;

struct IrqCriticalSection;
critical_section::set_impl!(IrqCriticalSection);

// SAFETY: `acquire` disables interrupts and returns the state of the status register's Interrupt
// flag state on function entry; `release` re-enables interrupts only if they were enabled then.
// Nesting is therefore sound. `sei` does not mask NMI, so an NMI handler must never call
// `critical_section::with`.
unsafe impl critical_section::Impl for IrqCriticalSection {
    // Capture, save and set interrupt disable state
    unsafe fn acquire() -> RawRestoreState {
        let sr_i_state: u8;
        unsafe {
            asm! {
            "php",
            "sei",
            "pla",
            out("a") sr_i_state,
            };
        }
        sr_i_state & SR_I_MASK == 0
    }

    unsafe fn release(was_enabled: RawRestoreState) {
        if was_enabled {
            unsafe {
                asm! {
                "cli",
                options(nomem, nostack)
                }
            }
        }
    }
}
