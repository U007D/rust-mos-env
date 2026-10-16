use core::arch::naked_asm;

use super::asm::rti;

#[naked]
pub unsafe extern "C" fn nmi() {
    unsafe {
        naked_asm! { rti!() }
    }
}
