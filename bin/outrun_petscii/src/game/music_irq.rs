mod ntsc;
mod pal;
mod song;

pub use ntsc::music_irq_ntsc;
pub use pal::music_irq_pal;
pub use song::Song;

// Load music. `.prg` format uses two-byte header to tell the C64 the load address for this program.
// Linker script tells linker to place `.music` at $b000.  So let's skip the first two bytes of this
// `.prg` and keep just the music data in memory.
const MUSIC_PRG_HEADER_BYTES: usize = 2;
const MUSIC_PRG: &[u8] = include_bytes!("../../data/music.prg");
const _: () = assert!(MUSIC_PRG.len() >= MUSIC_PRG_HEADER_BYTES);
const MUSIC_PRG_LEN: usize = MUSIC_PRG.len();
const MUSIC_LEN: usize = MUSIC_PRG_LEN.saturating_sub(MUSIC_PRG_HEADER_BYTES);
// Chop header bytes (first `MUSIC_PRG_HEADER_BYTES` bytes) from
#[unsafe(no_mangle)]
#[unsafe(link_section = ".music")]
pub static MUSIC: [u8; MUSIC_LEN] = {
    let mut res = [0_u8; MUSIC_LEN];
    let mut i = MUSIC_PRG_HEADER_BYTES;
    while i < MUSIC_PRG_LEN {
        res[i.saturating_sub(MUSIC_PRG_HEADER_BYTES)] = MUSIC_PRG[i];
        i += 1;
    }
    res
};

// These two symbols, defined in `memory-c64.x` point into the `music.prg` file based at $b000.
unsafe extern "C" {
    pub(super) fn music_init(tune: u8);
    fn music_advance();
}

