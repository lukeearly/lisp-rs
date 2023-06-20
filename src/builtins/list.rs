use crate::{
    def_builtin,
    value::{Cons, Value},
};

def_builtin!(first(ctx, out, args, scope) [list <- code : listp] {
    match list.unpack() {
        Value::Nil => Ok(out.nil()),
        Value::Cons(gc) => Ok(out.root(&gc.first)),
        _ => unreachable!("Should be prevented by listp predicate")
    }
});

def_builtin!(rest(ctx, out, args, scope) [list <- code : listp] {
    match list.unpack() {
        Value::Nil => Ok(out.nil()),
        Value::Cons(gc) => Ok(out.root(&gc.rest)),
        _ => unreachable!("Should be prevented by listp predicate")
    }
});

def_builtin!(cons(ctx, out, args, scope) [first <- code, rest <- code] {
    Ok(out.alloc_cons(ctx, Cons { first, rest }))
});
