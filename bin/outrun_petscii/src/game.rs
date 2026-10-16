pub mod asm;
mod colour;
mod init;
mod music_irq;
mod nmi;
mod non_overlapping_copy;
mod screen;
mod title;
mod video_system;

pub use init::init;
use non_overlapping_copy::non_overlapping_copy;
pub use screen::screen_clear;
pub use title::title;

use crate::shared_const::*;

// Load custom character set; Linker script tells linker to place `.font` at $2000
#[unsafe(no_mangle)]
#[unsafe(link_section = ".font")]
pub static FONT: [u8; CHARSET_SIZE] = *include_bytes!("../data/font.chr");
