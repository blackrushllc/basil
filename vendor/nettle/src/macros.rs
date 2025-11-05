/// Generates a Drop implementation for the given type that clears
/// Nettle's context struct.
macro_rules! impl_zeroing_drop_for {
    ($typ: ident) => {
        impl Drop for $typ {
            fn drop(&mut self) {
                unsafe {
                    libc::memset(self.context.as_mut() as *mut _ as _, 0,
                                 std::mem::size_of_val(self.context.as_ref()));
                }
            }
        }
    }
}
