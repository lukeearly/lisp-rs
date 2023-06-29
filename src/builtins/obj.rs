use crate::{
    def_builtin,
    value::{Cons, Value},
};

def_builtin!(objfirst(ctx, out) [obj: objp] {
    match obj.unpack() {
        Value::Object(gc) => Ok(out.root(&gc.first)),
        _ => unreachable!("Should be prevented by objp predicate")
    }
});

def_builtin!(objrest(ctx, out) [obj: objp] {
    match obj.unpack() {
        Value::Object(gc) => Ok(out.root(&gc.rest)),
        _ => unreachable!("Should be prevented by objp predicate")
    }
});

def_builtin!(obj(ctx, out) [first, rest] {
    Ok(out.alloc_obj(ctx, Cons { first, rest }))
});
