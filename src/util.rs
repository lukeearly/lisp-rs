use std::ptr::NonNull;

pub fn rust_to_lisp_symbol(name: &str) -> String {
    let name = str::replace(&name, "_star", "*");
    let name = str::replace(&name, "_bang", "!");
    let name = str::replace(&name, "_quest", "?");
    let name = str::replace(&name, "__", "");
    let name = str::replace(&name, "_", "-");
    name
}

pub unsafe fn construct_non_null<T>(ptr: *mut T) -> NonNull<T> {
    if cfg!(debug_assertions) {
        NonNull::new(ptr).unwrap()
    } else {
        unsafe { NonNull::new_unchecked(ptr) }
    }
}
