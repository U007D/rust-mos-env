#![feature(start)]
#![no_std]

mod panic;

use core::{slice::from_ref, str::from_utf8_unchecked};

use rand::{Rng, SeedableRng, rngs::SmallRng};
use ufmt_stdio::{print, ufmt};

const PETSCII_BACKSLASH_ASC: u8 = 205;
const TIMER_A: u16 = 0xDC04;

#[start]
pub fn main(_argc: isize, _argv: *const *const u8) -> isize {
    let seed = unsafe { core::ptr::read_volatile(TIMER_A as *const u16) as u64 };
    let mut rng = SmallRng::seed_from_u64(seed);

    loop {
        let asc = PETSCII_BACKSLASH_ASC + rng.gen_range(0_u8..=1);
        let ch = unsafe { from_utf8_unchecked(from_ref(&asc)) };
        print!("{}", ch);
    }
}
