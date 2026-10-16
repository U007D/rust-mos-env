use core::ptr::without_provenance_mut;

use c64_pac::Peripherals;

use super::non_overlapping_copy;
use crate::game::{colour::TITLE_COLOUR, screen::SCREEN_MEMORY_BASE};

pub const SCREEN_CELLS: usize = 1000;
static TITLE_SCREEN: [u8; SCREEN_CELLS] = *include_bytes!("../../data/title_screen.bin");

pub fn title(periphs: &Peripherals) {
    TITLE_COLOUR.iter().enumerate().for_each(|(i, &colour)| {
        periphs.colorram.cram(i).write(|w| w.color().variant(colour));
    });
    let pixels_dst =
        unsafe { &mut *without_provenance_mut::<[u8; SCREEN_CELLS]>(SCREEN_MEMORY_BASE) };
    non_overlapping_copy(&TITLE_SCREEN, pixels_dst);
}
