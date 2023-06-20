use crate::alloc::AllocError;
use std::ptr::NonNull;

pub trait LAlloc {
    fn alloc<T, R, F: FnOnce(NonNull<T>) -> R>(&self, transformer: F) -> Result<R, AllocError>;

    fn object<T, R, F: FnOnce(NonNull<T>) -> R>(
        &self,
        transformer: F,
        obj: T,
    ) -> Result<R, AllocError> {
        self.alloc(|ptr| {
            unsafe { *ptr.as_ptr() = obj };
            transformer(ptr)
        })
    }
}

pub fn id<T>(ptr: NonNull<T>) -> NonNull<T> {
    ptr
}
