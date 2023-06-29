use core::panic;
use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::size_of,
    ptr::NonNull,
    slice, string,
};

use crate::{builtins::BuiltinFunction, util::construct_non_null};

pub const OBJECT_ALIGNMENT: usize = 8;

struct GuardPtr<'a, T> {
    ptr: NonNull<T>,
    phantom: PhantomData<&'a T>,
}

#[derive(Clone, Copy, Eq)]
pub union PackedPtr {
    tag: usize,
    integer: isize,
    cons: NonNull<RawCons>,
    lstr: NonNull<LString>,
    fun: BuiltinFunction,
}

impl PackedPtr {
    pub fn nil() -> Self {
        unsafe { PackedPtr { tag: 0 }.add_tag(TagType::Symbol as usize) }
    }

    pub fn integer(n: isize) -> Self {
        unsafe {
            PackedPtr {
                tag: PackedPtr { integer: n }.tag << 2 | TagType::Integer as usize,
            }
        }
    }

    pub fn cons_ptr(ptr: NonNull<RawCons>) -> Self {
        unsafe { PackedPtr { cons: ptr }.add_tag(TagType::Cons as usize) }
    }

    pub fn obj_ptr(ptr: NonNull<RawCons>) -> Self {
        unsafe { PackedPtr { cons: ptr }.add_tag(TagType::Object as usize) }
    }

    pub fn sym_ptr(ptr: NonNull<LString>) -> Self {
        unsafe { PackedPtr { lstr: ptr }.add_tag(TagType::Symbol as usize) }
    }

    pub fn fun_ptr(ptr: BuiltinFunction) -> Self {
        unsafe { PackedPtr { fun: ptr }.add_tag(TagType::Function as usize) }
    }

    unsafe fn add_tag(&self, tag: usize) -> Self {
        PackedPtr {
            tag: (self.tag & !7) | tag,
        }
    }

    unsafe fn get_integer(&self) -> isize {
        let n = PackedPtr { tag: self.tag >> 2 }.integer;
        // sign extend
        let m = n >> 61;
        n | (m << 62) | (m << 63)
    }

    unsafe fn get_cons_ptr(&self) -> NonNull<RawCons> {
        PackedPtr { tag: self.tag & !7 }.cons
    }

    unsafe fn get_sym_ptr(&self) -> NonNull<LString> {
        PackedPtr { tag: self.tag & !7 }.lstr
    }

    unsafe fn get_fun_ptr(&self) -> BuiltinFunction {
        PackedPtr { tag: self.tag & !7 }.fun
    }

    pub fn tag_type(&self) -> TagType {
        let tag = unsafe { self.tag };
        match tag {
            t if (t & 3) == TagType::Integer as usize => TagType::Integer,
            t if (t & 7) == TagType::Cons as usize => TagType::Cons,
            t if (t & 7) == TagType::Object as usize => TagType::Object,
            t if t == 0 as usize => TagType::Nil,
            t if (t & 7) == TagType::Symbol as usize => TagType::Symbol,
            t if (t & 7) == TagType::Function as usize => TagType::Function,
            _ => panic!("Heap corrupted"),
        }
    }

    pub fn unpack(&self) -> UnpackedPtr {
        unsafe {
            match self.tag_type() {
                TagType::Integer => UnpackedPtr::Integer(self.get_integer()),
                TagType::Cons => UnpackedPtr::Cons(self.get_cons_ptr()),
                TagType::Object => UnpackedPtr::Object(self.get_cons_ptr()),
                TagType::Nil => UnpackedPtr::Nil,
                TagType::Symbol => UnpackedPtr::Symbol(self.get_sym_ptr()),
                TagType::Function => UnpackedPtr::Function(self.get_fun_ptr()),
                _ => panic!("Heap corrupted"),
            }
        }
    }

    pub fn heap_ptrs(&self) -> Vec<(NonNull<u8>, usize)> {
        use crate::object::UnpackedPtr::*;
        unsafe {
            match self.unpack() {
                Cons(ptr) | Object(ptr) => vec![(
                    NonNull::new_unchecked(ptr.as_ptr() as *mut u8),
                    size_of::<crate::object::RawCons>(),
                )],
                _ => vec![],
            }
        }
    }

    pub fn obj_ptrs(&self) -> Vec<PackedPtr> {
        use crate::object::UnpackedPtr::*;
        unsafe {
            match self.unpack() {
                Cons(ptr) | Object(ptr) => {
                    let cons = *ptr.as_ptr();

                    vec![cons.first, cons.rest]
                }
                _ => vec![],
            }
        }
    }
}

impl Hash for PackedPtr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { self.tag }.hash(state)
    }
}

impl PartialEq for PackedPtr {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.tag == other.tag }
    }
}

/**
 * xx00 - smallint (62-bit)
 * x001 - cons
 * x010 - string
 * x011 - symbol
 * x101 - function
 * vector
 * bigint
 * closure
 * map
 */
#[derive(PartialEq, Eq, Debug)]
pub enum TagType {
    Symbol = 0b000,
    Cons = 0b001,
    Function = 0b010,
    Integer = 0b011,
    // Vector,
    // Bigint,
    // Closure,
    // Map,
    // (Integer = 0b111)
    Object = 0b100,
    Nil,
}

#[derive(PartialEq, Debug)]
pub enum UnpackedPtr {
    Integer(isize),
    Cons(NonNull<RawCons>),
    Object(NonNull<RawCons>),
    Nil,
    Symbol(NonNull<LString>),
    Function(BuiltinFunction),
}

impl UnpackedPtr {
    pub fn pack(&self) -> PackedPtr {
        match *self {
            UnpackedPtr::Integer(n) => PackedPtr::integer(n),
            UnpackedPtr::Cons(ptr) => PackedPtr::cons_ptr(ptr),
            UnpackedPtr::Object(ptr) => PackedPtr::obj_ptr(ptr),
            UnpackedPtr::Nil => PackedPtr::nil(),
            UnpackedPtr::Symbol(ptr) => PackedPtr::sym_ptr(ptr),
            UnpackedPtr::Function(ptr) => PackedPtr::fun_ptr(ptr),
        }
    }
}

#[repr(align(4))]
#[derive(Clone, Copy)]
pub struct RawCons {
    pub first: PackedPtr,
    pub rest: PackedPtr,
}

#[repr(align(4))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LString {
    pub start: *const u8,
    pub len: usize,
}

impl LString {
    // Unsafe because this leaks memory
    pub unsafe fn leak_str(name: String) -> Self {
        let len = name.len();
        let start = Box::leak(name.into_boxed_str()).as_ptr();
        LString { start, len }
    }
}

impl ToString for LString {
    fn to_string(&self) -> String {
        let slice = unsafe { slice::from_raw_parts(self.start, self.len) };
        string::String::from_utf8_lossy(slice).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment() {
        assert_eq!(size_of::<PackedPtr>(), OBJECT_ALIGNMENT)
    }

    #[test]
    fn test_positive_integer() {
        let n = 13435;
        let object = PackedPtr::integer(n);
        assert_eq!(object.unpack(), UnpackedPtr::Integer(n));
    }

    #[test]
    fn test_negative_integer() {
        let n = -193847;
        let object = PackedPtr::integer(n);
        assert_eq!(object.unpack(), UnpackedPtr::Integer(n));
    }
}
