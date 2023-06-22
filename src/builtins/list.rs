use crate::{
    builtins::eval::rust_eval,
    def_builtin, let_slot,
    value::{Cons, Value},
};

use super::{
    unpack::{unpack_cons, unpack_int},
    BuiltinError,
};

def_builtin!(first(ctx, out) [list: listp] {
    match list.unpack() {
        Value::Nil => Ok(out.nil()),
        Value::Cons(gc) => Ok(out.root(&gc.first)),
        _ => unreachable!("Should be prevented by listp predicate")
    }
});

def_builtin!(rest(ctx, out) [list: listp] {
    match list.unpack() {
        Value::Nil => Ok(out.nil()),
        Value::Cons(gc) => Ok(out.root(&gc.rest)),
        _ => unreachable!("Should be prevented by listp predicate")
    }
});

def_builtin!(cons(ctx, out) [first, rest] {
    Ok(out.alloc_cons(ctx, Cons { first, rest }))
});

def_builtin!(list(ctx, out) [&rest list] {
    Ok(out.root(&list))
});

def_builtin!(nthrest(ctx, out) [idx|int, list] {
    while idx > 0 {
        if let Ok(cons) = unpack_cons(list) {
            list = cons.rest;
        } else {
            return Ok(out.nil())
        }
        idx -= 1;
    }
    Ok(out.root(&list))
});

def_builtin!(len(ctx, out) [list: listp] {
    let mut n = 0;
    while let Ok(cons) = unpack_cons(list) {
        n += 1;
        list = cons.rest;
    }
    Ok(out.root(&Value::Integer(n).pack()))
});
