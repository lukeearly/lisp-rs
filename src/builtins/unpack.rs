use crate::{
    object::TagType,
    root::Gc,
    value::{Cons, PackedValue, Value},
};

pub fn unpack_cons<'guard>(arg: PackedValue<'guard>) -> Result<Gc<'guard, Cons<'guard>>, TagType> {
    match arg.unpack() {
        Value::Cons(gc) => Ok(gc),
        _ => Err(unsafe { arg.unguard().tag_type() }),
    }
}

pub fn unpack_int<'guard>(arg: PackedValue<'guard>) -> Result<isize, TagType> {
    match arg.unpack() {
        Value::Integer(n) => Ok(n),
        _ => Err(unsafe { arg.unguard().tag_type() }),
    }
}
