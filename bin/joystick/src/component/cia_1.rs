use core::mem::size_of;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(usize)]
pub enum Cia1 {
    PortA = 0xdc00,
    PortB = 0xdc01,
}

impl From<Cia1> for usize {
    fn from(cia_1: Cia1) -> Self {
        // INVARIANT: `Cia1` must be `repr(usize)` to ensure no data loss.
        // Weak sanity check to ensure `Cia1` is still a `usize` before casting using `as`.
        const _: () = assert!(size_of::<Cia1>() == size_of::<usize>());

        // Potentially lossy `as` guarded by weak invariant check above
        cia_1 as usize
    }
}
