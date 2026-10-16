/* Linker script for mos-c64-none, the only machine this game targets.
 * Selected by build.rs from the target vendor.
 *
 * The C64 PRG loads at $0801, so the fixed placements below sit safely above the
 * BASIC header the SDK emits there.
 *
 *   .font  $2000  VIC-II character base. The VIC-II can only see 16K at a time
 *                 and reads character data at a fixed offset within that bank,
 *                 so this address is a hardware constraint, not a preference.
 *   .music $B000  Under the BASIC ROM. Usable as RAM because the C64 SDK's
 *                 startup unmaps BASIC ($01 = $3E) before main runs.
 */

INCLUDE link.ld

SECTIONS {
  .font  0x2000 : { KEEP(*(.font))  }
  .music 0xb000 : {
    music_init = . + 0x219;   /* select tune; takes the tune number in A */
    music_advance = . + 0x69; /* render one frame; call from the raster IRQ */
    KEEP(*(.music))
  }
} INSERT AFTER .data;
