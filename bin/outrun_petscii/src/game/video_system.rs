use c64_pac::Peripherals;
use ufmt_stdio::{uDebug, uDisplay, uWrite, ufmt::Formatter as uFormatter};

const BIT_8: u16 = 0x0100;
const BIT_7: u8 = 0x80;
const TIMER_RESET_VALUE: u16 = 0xffff;
const U8_BITS: usize = {
    let u8_bits = u8::BITS;
    assert!((u8_bits as u128) < (usize::MAX as u128));
    u8_bits as usize
};

// Total clock cycles elapsed to render one video frame.
// Sources: 6567R56A / 6567R8 / 6569 are from Christian Bauer's VIC-II article
// (https://www.cebix.net/VIC-Article.txt).  The 6572 is from VICE's machine definitions.
const NTSC_OLD_FRAME_CYCLES: u16 = 262 * 64; // 16_768  6567R56A
// const NTSC_FRAME_CYCLES: u16 = 263 * 65; // 17_095  6567R8 / 8562
const PAL_FRAME_CYCLES: u16 = 312 * 63; // 19_656  6569 / 8565
// const PAL_N_FRAME_CYCLES: u16 = 312 * 65; // 20_280  6572 (Drean/Argentina)

// Classification boundaries are the midpoint between two standards.  The
// original assembly for `outrun_petscii` only differentiates between PAL and
// NTSC raster speeds so we'll do the same here. (16_768 + 19_656) / 2 == 36424
const NTSC_PAL_BOUNDARY: u16 = (NTSC_OLD_FRAME_CYCLES + PAL_FRAME_CYCLES) / 2;

#[derive(Copy, Clone, Debug)]
pub enum VideoSystem {
    // 263 scan lines traced at 65cc each
    Ntsc,
    // 312 scan lines traced at 63cc each; SECAM specification is identical
    Pal,
}

impl VideoSystem {
    // Determine the video system raster scan rate (NTSC or PAL) by measuring the
    // number of clock cycles elapsed to render one video frame.
    #[inline(never)]
    pub fn detect(periphs: &Peripherals) -> VideoSystem {
        // 1. Disable screen to improve timing precision
        periphs.vic.scroly().modify(|_, w| w.screen().disabled());

        // 2. Start the timer
        // 2a. Load its latch (reset on underflow) values.  Low byte first by convention
        //    since hi byte will load the timer counter (iff timer is stopped).  This is
        //    of no consequence for us since we'll be force-loading the latch value into
        //    the counter with `force_latched_load().enabled()`.
        periphs
            .cia2
            .ti2alo()
            .write(|w| w.timer().set(TIMER_RESET_VALUE as u8));
        periphs
            .cia2
            .ti2ahi()
            .write(|w| w.timer().set((TIMER_RESET_VALUE >> U8_BITS) as u8));

        // 2b. Initialize & start the timer
        let ci2cra = periphs.cia2.ci2cra().read().bits();
        periphs.cia2.ci2cra().modify(|_, w| {
            w.select_timer_output()
                .disabled()
                .timer_run_mode()
                .continuous()
                .force_latched_load()
                .execute()
                .timer_input_mode()
                .cpu_cycles()
                .start_timer()
                .enabled()
        });

        // 3. On next frame start record timer's value (`t0`)
        let _line_count = Self::wait_for_start_of_frame(periphs);
        let t0 = Self::read_16_bit_timer(periphs);

        // 4. On next frame start record timer's value (`t1`)
        Self::wait_for_start_of_frame(periphs);
        let t1 = Self::read_16_bit_timer(periphs);

        // 5. Restore ci2cra
        periphs.cia2.ci2cra().write(|w| unsafe { w.bits(ci2cra) });

        // 6. Re-enable screen
        periphs.vic.scroly().modify(|_, w| w.screen().enabled());

        // 7. Timer descends, so get elapsed time with `t0.wrapping_sub(t1)`
        // Determine system type by comparing elapsed time with known values
        // (sourced from Christian Bauer's VIC-II reverse-engineering work at
        // www.cebix.net/VIC-Article.txt and additional sources such as VICE
        // emulator's c64/c64model.c for additional video systems not covered
        // by Bauer).
        let elapsed_cc = t0.wrapping_sub(t1);

        // 8. Classify the elapsed clock cycles
        // Further sanity checking is possible using `line_count` and the specific
        // clock cycle frame budgets defined above, but this detection method is
        // already more accurate/reliable than the original source.
        match elapsed_cc < NTSC_PAL_BOUNDARY {
            true => VideoSystem::Ntsc,
            false => VideoSystem::Pal,
        }
    }

    fn read_16_bit_timer(p: &Peripherals) -> u16 {
        loop {
            let hi = p.cia2.ti2ahi().read().bits();
            let lo = p.cia2.ti2alo().read().bits();
            let hi2 = p.cia2.ti2ahi().read().bits();
            let timestamp = u16::from_le_bytes([lo, hi]);
            // 16-bit timer is not hardware latched for consistent reads, so detect rollover
            // and if found, re-read (affects timing slightly)
            if hi == hi2 {
                break timestamp;
            }
        }
    }

    fn wait_for_start_of_frame(p: &Peripherals) -> u16 {
        // start_scan:
        // Let's track the raster line to capture its max value in a frame.  Note the
        // C64 `RASTER` register is 9 bits (up to 262/263/312 lines, depending on the
        // video system).  The 9th bit is in `vic.scroly.rst8()`, but we
        // won't need to read that here. More details below.
        let mut base_raster = p.vic.raster_r().read().bits();

        // Wait until we've left the current scanline
        let line_count_lo = loop {
            let new_raster = p.vic.raster_r().read().bits();
            let diff = base_raster.wrapping_sub(new_raster);
            if diff == 0 {
                continue;
            }
            // `base_raster.wrapping_sub(new_raster)` will always evaluate to $FF (-1)
            // unless the `raster_r` has rolled over (overflowed) to zero.
            // `raster_r` will roll over twice for each frame:
            // 1) when it transitions from scanline 255->256 with difference == $ff (-1)
            // 2) when it transitions from max scanline 262
            // (old NTSC), 263 (NTSC), or 312 (PAL, PAL-N) -> 0 with
            // difference == 6, 7 or 56.
            // In case 1) `base_raster` will always contain $FF, whereas in
            // case 2), `base_raster` will contain 6, 7 or 56, respectively (the
            // lower 8 bits of 262, 263 or 312).  We use this to distinguish
            // case 1) from 2) in order to disregard case 1).
            if diff & BIT_7 == 0 {
                // This is a new frame, not a line-255-overflow
                // Convert from line number to line count
                // Note: Cannot saturate since bit 7 was just verified as 0
                break base_raster.saturating_add(1);
            }
            base_raster = new_raster;
        };
        // All formats have > 255 lines (bit 8 == 1) so provide bit 8 in the result
        BIT_8 | u16::from(line_count_lo)
    }
}

impl uDebug for VideoSystem {
    fn fmt<W>(&self, f: &mut uFormatter<'_, W>) -> Result<(), W::Error>
    where W: uWrite + ?Sized {
        <Self as uDisplay>::fmt(self, f)
    }
}

impl uDisplay for VideoSystem {
    fn fmt<W>(&self, f: &mut uFormatter<'_, W>) -> Result<(), W::Error>
    where W: uWrite + ?Sized {
        f.write_str(match self {
            Self::Ntsc => "ntsc",
            Self::Pal => "pal",
        })
    }
}
