pub const fn as_bytes<T: Sized>(p: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            std::ptr::from_ref::<T>(p).cast::<u8>(),
            std::mem::size_of::<T>(),
        )
    }
}

pub const fn slice_as_bytes<T: Sized>(p: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(p.as_ptr().cast::<u8>(), std::mem::size_of_val(p)) }
}
