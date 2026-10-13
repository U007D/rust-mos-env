use core::mem::size_of;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(usize)]
pub enum VicIi {
    BorderColor = 0xd020,
    BackgroundColor = 0xd021,
}

impl From<VicIi> for usize {
    fn from(vic_ii: VicIi) -> Self {
        // INVARIANT: `VicII` must be `repr(usize)` to ensure no data loss.
        // Weak sanity check to ensure `VicII` is still a `usize` before casting using `as`.
        const _: () = assert!(size_of::<VicIi>() == size_of::<usize>());

        // Potentially lossy `as` guarded by weak invariant check above
        vic_ii as usize
    }
}
