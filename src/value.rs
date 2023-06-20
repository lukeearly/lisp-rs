use std::{marker::PhantomData, ops::Deref, ptr::NonNull};

use crate::{
    builtins::BuiltinFn,
    object::{self, LString, PackedPtr, UnpackedPtr},
    root::Gc,
};

// pub struct TypedObj<T> {
//     ptr: PackedPtr,
//     _phantom: PhantomData<T>
// }

#[derive(PartialEq, Eq, Clone)]
pub struct PackedValue<'guard> {
    ptr: PackedPtr,
    _phantom: PhantomData<&'guard ()>,
}

impl<'guard> PackedValue<'guard> {
    pub unsafe fn new(ptr: PackedPtr) -> Self {
        Self {
            ptr,
            _phantom: PhantomData,
        }
    }

    pub fn unpack(&self) -> Value<'guard> {
        unsafe { Value::from_unpacked_ptr(self.ptr.unpack()) }
    }

    pub unsafe fn unguard(&self) -> PackedPtr {
        self.ptr
    }
}

#[derive(PartialEq, Eq, Clone)]
pub enum Value<'guard> {
    Integer(isize),
    Cons(Gc<'guard, Cons<'guard>>),
    Symbol(Gc<'guard, LString>),
    String(Gc<'guard, LString>),
    Builtin(BuiltinFn),
    Nil,
}

impl<'guard> Value<'guard> {
    pub unsafe fn from_unpacked_ptr(unpacked: UnpackedPtr) -> Self {
        match unpacked {
            UnpackedPtr::Integer(n) => Self::Integer(n),
            UnpackedPtr::Cons(ptr) => Self::Cons(Gc::new(
                NonNull::new_unchecked(ptr.as_ptr() as *mut Cons).as_ref(),
            )),
            UnpackedPtr::Nil => Self::Nil,
            UnpackedPtr::Symbol(ptr) => Self::Symbol(Gc::new(ptr.as_ref())),
            UnpackedPtr::String(ptr) => Self::String(Gc::new(ptr.as_ref())),
            UnpackedPtr::Builtin(ptr) => Self::Builtin(ptr),
        }
    }

    pub unsafe fn to_unpacked_ptr(&self) -> UnpackedPtr {
        match self {
            Value::Integer(n) => UnpackedPtr::Integer(*n),
            Value::Cons(ptr) => UnpackedPtr::Cons(NonNull::new_unchecked(
                ptr.as_raw().as_ptr() as *mut object::RawCons
            )),
            Value::Symbol(ptr) => UnpackedPtr::Symbol(ptr.as_raw()),
            Value::String(ptr) => UnpackedPtr::String(ptr.as_raw()),
            Value::Nil => UnpackedPtr::Nil,
            Value::Builtin(ptr) => UnpackedPtr::Builtin(*ptr),
        }
    }

    pub fn pack(&self) -> PackedValue<'guard> {
        unsafe {
            PackedValue {
                ptr: self.to_unpacked_ptr().pack(),
                _phantom: PhantomData,
            }
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct Cons<'guard> {
    pub first: PackedValue<'guard>,
    pub rest: PackedValue<'guard>,
}

impl<'guard> Cons<'guard> {
    pub fn new(first: Value<'guard>, rest: Value<'guard>) -> Self {
        Self {
            first: first.pack(),
            rest: rest.pack(),
        }
    }
}
