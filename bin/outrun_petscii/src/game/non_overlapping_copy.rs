use core::cmp::min;

pub fn non_overlapping_copy(src: &[u8], dst: &mut [u8]) {
    unsafe {
        core::ptr::copy_nonoverlapping(
            src.as_ptr(),
            dst.as_mut_ptr(),
            min(src.len(), dst.len()),
        )
    }

}