use core::fmt::{Error as FmtError, Formatter as CoreFormatter, Write as FmtWrite};

use ufmt_stdio::uWrite;

pub struct FmtBridge<'a, 'b>(pub(super) &'a mut CoreFormatter<'b>);

impl uWrite for FmtBridge<'_, '_> {
    type Error = FmtError;

    fn write_str(&mut self, s: &str) -> Result<(), FmtError> {
        FmtWrite::write_str(&mut *self.0, s)
    }
}
