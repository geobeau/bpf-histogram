#![no_std]

#[repr(C)]
pub struct Key<T: Sized> {
    pub bucket: u32,
    pub sub_key: T,
}
