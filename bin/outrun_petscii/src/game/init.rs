use core::{arch::asm, ptr};

use c64_pac::Peripherals;

use super::{asm, music_irq::{Song, music_init, music_irq_ntsc, music_irq_pal}, nmi::nmi, video_system::VideoSystem};
use crate::{error::{Error, Result}, shared_const::*};

pub fn init() -> Result<Peripherals> {
    // 1. Get hardware peripherals
    let periphs = Peripherals::take().ok_or_else(|| Error::NoHardwareFound)?;

    // 2. "Take over" the computer:
    // i) disable IRQs,
    // ii) disable CIAs,
    // iii) set `NMI_VECTOR` to simply `rti` (do nothing; this disables `Restore` keypress),
    // iv) determine NTSC or PAL video timings
    // v) set `IRQ_VECTOR` to call NTSC or PAL music player IRQ
    // vi) bank out BASIC and kernal ROMs
    // vii) set up raster IRQ notification every line 0 & discard stale pending VIC IRQs
    // viii) enable the raster IRQ
    // ix) display `outrun_petscii` font residing at $2000
    // x) enable IRQ's
    // xi) initialize music player in `../data/music.prg`

    // i) Disable IRQ's
    unsafe {
        asm! { asm::sei!(), options(nomem, nostack) }
    }

    // ii) Silence and clear CIAs
    // Disable all CIA1 & 2 peripherals
    periphs.cia1.ciaicr_w().write(|w| {
        w.timer_a()
            .set()
            .timer_b()
            .set()
            .alarm()
            .set()
            .serial()
            .set()
            .flag()
            .set()
            .mode()
            .disabled()
    });
    periphs.cia2.ci2icr_w().write(|w| {
        w.timer_a()
            .set()
            .timer_b()
            .set()
            .alarm()
            .set()
            .serial()
            .set()
            .flag()
            .set()
            .mode()
            .disabled()
    });
    // SIDE-EFFECT: reading clears all flags and releases the line
    periphs.cia1.ciaicr_r().read();
    // SIDE-EFFECT: reading clears all flags and releases the line
    periphs.cia2.ci2icr_r().read();

    // iii) Install NMI vector
    unsafe {
        ptr::without_provenance_mut::<unsafe extern "C" fn() -> ()>(NMI_VECTOR).write_volatile(nmi);
    }

    // iv) determine NTSC or PAL video timings
    let video_system = VideoSystem::detect(&periphs);

    // v) set `IRQ_VECTOR` to call NTSC or PAL music player IRQ
    use VideoSystem as Vs;
    let music_player = match video_system {
        Vs::Ntsc => music_irq_ntsc,
        Vs::Pal => music_irq_pal,
    };
    unsafe {
        ptr::without_provenance_mut::<unsafe extern "C" fn() -> ()>(IRQ_VECTOR)
            .write_volatile(music_player);
    }

    // vi) Bank out BASIC & KERNAL ROMs
    periphs.cpuport.r6510().modify(|_, w| {
        // Setting `HIRAM` (kernal ROM @ $e000) to RAM also unconditionally switches out BASIC
        // ROM from $a000.  Mapping the C64 documents these fields as independent, which is
        // incorrect.  https://sta.c64.org/cbm64mem.html shows the pattern of the low three bits
        // are intertwined.  In order to preserve access to the CIA, VIC-II, and more, the bit
        // pattern must be 0b101, not 0b100.  This is why `LORAM` is set to BASIC ROM below,
        // even though we want RAM there.  Setting `HIRAM` to RAM does this for us, and the
        // setting of `LORAM` to 1 has the side-effect of preserving I/O @ $d000.
        // This behavior is undocumented by Mapping the C64.
        w.loram().basic_rom().hiram().ram().cass_motor().stopped()
    });

    // vii) Set up raster-based interrupts every line 0 & discard any stale pending VIC IRQ
    periphs.vic.raster_w().write(|w| w.raster_line().set(0));
    periphs.vic.scroly().write(|w| {
        w.rst8()
            .clear_bit()
            .screen()
            .enabled()
            .row_select()
            .rows25()
            .yscroll()
            .set(3)
    });
    periphs.vic.vicirq_w().write(|w| w.raster().ack());

    // viii) enable the raster IRQ
    periphs.vic.irqmask().write(|w| w.raster().enabled());

    // ix) display `outrun_petscii` font residing at $2000
    periphs.cia2.ci2pra().modify(|_, w| w.vicbank().bank0());
    periphs
        .vic
        .vmcsb()
        .write(|w| w.video_matrix().offset_0400().char_base().offset_2000());

    // x) enable IRQ's
    unsafe {
        asm! { asm::cli!(), options(nomem, nostack) }
    }

    // xi) init
    unsafe { music_init(Song::MagicalSoundShower.into()) }

    Ok(periphs)
}
