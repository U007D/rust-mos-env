#![allow(dead_code)]

// Kernal addresses
// TODO: Consider replacing this section with typesafe, PAC-like definitions at
// the HAL level
pub const IRQ_VECTOR: usize = 0xfffe;
pub const NMI_VECTOR: usize = 0xfffa;

// 6502 status register Carry flag mask
pub const SR_C_MASK: u8 = 0b1 << 0;
// 6502 status register Zero flag mask
pub const SR_Z_MASK: u8 = 0b1 << 1;
// 6502 status register Interrupt disable mask
pub const SR_I_MASK: u8 = 0b1 << 2;
// 6502 status register (binary coded) Decimal mask
pub const SR_D_MASK: u8 = 0b1 << 3;
// 6502 status register Break flag mask
pub const SR_B_MASK: u8 = 0b1 << 4;
// 6502 Overflow status register mask
pub const SR_V_MASK: u8 = 0b1 << 6;
// 6502 Negative status register mask
pub const SR_N_MASK: u8 = 0b1 << 7;

// Well-known values
pub const DEFAULT_CHARSET_UPPERCASE: u8 = 0b010;
// 256 chars * 8 bytes each (each character is an 8x8 array of pixels)
pub const CHARSET_SIZE: usize = 0x0800;

// PETSCII values
pub const PETSCII_SPACE: u8 = 0x20;

