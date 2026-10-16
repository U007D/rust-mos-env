use core::ptr::without_provenance_mut;

use c64_pac::Peripherals;

use crate::shared_const::*;

pub const SCREEN_MEMORY_BASE: usize = 0x0400;

pub fn screen_clear(periphs: &Peripherals) {
    // Set border & background to black
    periphs.vic.extcol().write(|w| w.color().black());
    periphs.vic.bgcol(0).write(|w| w.color().black());

    // Clear screen memory with spaces
    (0_usize..=u8::MAX.into()).for_each(|i| unsafe {
        *without_provenance_mut(SCREEN_MEMORY_BASE.wrapping_add(i)) = PETSCII_SPACE;
        *without_provenance_mut(SCREEN_MEMORY_BASE.wrapping_add(0x0100).wrapping_add(i)) = PETSCII_SPACE;
        *without_provenance_mut(SCREEN_MEMORY_BASE.wrapping_add(0x0200).wrapping_add(i)) = PETSCII_SPACE;
        *without_provenance_mut(SCREEN_MEMORY_BASE.wrapping_add(0x0300).wrapping_add(i)) = PETSCII_SPACE;
    });
}
