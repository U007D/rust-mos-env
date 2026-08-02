//! Dependency-free C64 smoke test for the rust-mos toolchain.
//!
//! Entry convention for this generation: `#[start]` is gone (E0557); the
//! SDK's crt0 calls an `extern "C" main`. The BASIC SYS stub and PRG load
//! address come from the SDK link step (mos-c64-clang).
#![no_std]
#![no_main]

// The c_uint gate: llvm-mos C int is 16-bit. This fails to compile if the
// toolchain shipped core::ffi with 32-bit c_uint (see the flake's
// primitives.rs patch).
const _: () = assert!(core::mem::size_of::<core::ffi::c_uint>() == 2);
const _: () = assert!(core::mem::size_of::<core::ffi::c_int>() == 2);

/// VIC-II border color register.
const BORDER: *mut u8 = 0xD020 as *mut u8;

#[no_mangle]
extern "C" fn main() -> ! {
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
