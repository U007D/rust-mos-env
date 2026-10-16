#![allow(dead_code)]

pub mod fmt_bridge;

use core::{error::Error as CoreError,
           fmt::{Debug, Display, Formatter as CoreFormatter, Result as FmtResult},
           result::Result as CoreResult};

use fmt_bridge::FmtBridge;
use ufmt_stdio::{uDebug, uDisplay, uWrite, ufmt::Formatter};

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[repr(isize)]
pub enum Error {
    NoHardwareFound,
}

impl Error {
    const fn message(&self) -> &'static str {
        use Error as E;
        match *self {
            E::NoHardwareFound => "Error: No hardware peripherals found",
        }
    }
}

impl CoreError for Error {}

impl Debug for Error {
    fn fmt(&self, f: &mut CoreFormatter<'_>) -> FmtResult {
        uDebug::fmt(self, &mut Formatter::new(&mut FmtBridge(f)))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut CoreFormatter<'_>) -> FmtResult {
        uDisplay::fmt(self, &mut Formatter::new(&mut FmtBridge(f)))
    }
}

impl uDebug for Error {
    fn fmt<W>(&self, f: &mut Formatter<'_, W>) -> CoreResult<(), W::Error>
    where W: uWrite + ?Sized {
        f.write_str(self.message())
    }
}

impl uDisplay for Error {
    fn fmt<W>(&self, f: &mut Formatter<'_, W>) -> CoreResult<(), W::Error>
    where W: uWrite + ?Sized {
        f.write_str(self.message())
    }
}
