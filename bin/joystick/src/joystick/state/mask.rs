use core::mem::size_of;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum JoyMask {
    Up = 0b0_0001,
    Down = 0b0_0010,
    Left = 0b0_0100,
    Right = 0b0_1000,
    Fire = 0b1_0000,
}

impl From<JoyMask> for u8 {
    fn from(joy_mask: JoyMask) -> Self {
        // INVARIANT: `JoyMask` must be `repr(u8)` to ensure no data loss.
        // Weak sanity check to ensure `JoyMask` is still a `u8` before casting using `as`.
        const _: () = assert!(size_of::<JoyMask>() == size_of::<u8>());

        // Potentially lossy `as` guarded by weak invariant check above
        joy_mask as u8
    }
}
