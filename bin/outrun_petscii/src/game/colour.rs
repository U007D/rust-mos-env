use c64_pac::vic::Color as Colour;

use crate::game::title::SCREEN_CELLS;

const NYBBLE_MASK: u8 = 0x0f;
const PACKED_LEN: usize = SCREEN_CELLS / 2;
/// The C64 palette in hardware order. Indexed by the 4-bit colour code.
const PALETTE: [Colour; 16] = {
    use Colour as C;
    [
        C::Black,
        C::White,
        C::Red,
        C::Cyan,
        C::Purple,
        C::Green,
        C::Blue,
        C::Yellow,
        C::Orange,
        C::Brown,
        C::LightRed,
        C::DarkGray,
        C::MediumGray,
        C::LightGreen,
        C::LightBlue,
        C::LightGray,
    ]
};
pub static TITLE_COLOUR: [Colour; SCREEN_CELLS] = unpack(include_bytes!("../../data/title_colour.bin"));

pub const fn colour(nybble: u8) -> Colour { PALETTE[(nybble & NYBBLE_MASK) as usize] }

// Expand each packed byte to 2 color cells (16 colors == 4 bits). Packed in hi byte/lo byte order.
const fn unpack(packed: &[u8; PACKED_LEN]) -> [Colour; SCREEN_CELLS] {
    let mut out = [Colour::Black; SCREEN_CELLS];
    let mut i = 0;
    while i < PACKED_LEN {
        out[i * 2] = colour(packed[i] >> 4);
        out[i * 2 + 1] = colour(packed[i]);
        i += 1;
    }
    out
}
