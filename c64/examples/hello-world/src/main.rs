//! C64 "hello world" smoke test for the rust-mos toolchain: prints a greeting
//! through the SDK's libc, then cycles the VIC-II border color forever.
//!
//! Entry convention for this toolchain: `#[start]` is gone (E0557); the SDK's
//! crt0 calls an `extern "C" main`. The BASIC SYS stub and PRG load address
//! come from the SDK link step (mos-c64-clang).
#![no_std]
#![no_main]

// The c_uint gate: llvm-mos C int is 16-bit. This fails to compile if the
// toolchain shipped core::ffi with 32-bit c_uint (see the flake's
// primitives.rs patch).
const _: () = assert!(core::mem::size_of::<core::ffi::c_uint>() == 2);
const _: () = assert!(core::mem::size_of::<core::ffi::c_int>() == 2);

/// VIC-II border color register.
const BORDER: *mut u8 = 0xD020 as *mut u8;

extern "C" {
    /// SDK libc: write the null-terminated string plus a newline to the screen
    /// (via the KERNAL). Declared here so we need no `libc` crate.
    fn puts(s: *const u8) -> core::ffi::c_int;
}

#[no_mangle]
extern "C" fn main() -> ! {
    // Leading 0x0E switches the C64 to its lower/upper-case character set so the
    // mixed-case text renders correctly; the string is null-terminated for puts.
    unsafe { puts(b"\x0EHello, C64 world, from Rust!\0".as_ptr()) };

    let mut c: u8 = 0;
    loop {
        unsafe { core::ptr::write_volatile(BORDER, c & 0x0F) };
        c = c.wrapping_add(1);
        // Crude delay so the border visibly cycles in VICE. black_box keeps
        // the loop alive without inline asm (mos asm! support unverified).
        for i in 0..8192u16 {
            core::hint::black_box(i);
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
