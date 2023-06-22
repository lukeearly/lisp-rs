use crate::{
    builtins::eval::{rust_apply, rust_eval},
    def_builtin, let_slot,
};

use super::unpack::unpack_cons;

def_builtin!(fold(ctx, out) [func, list, init] {
    let mut accum = out.root(&init);

    let_slot!(ctx:arg_list);
    let mut arg_list = arg_list.nil();

    while let Ok(cons) = unpack_cons(list) {
        arg_list = arg_list.slot().nil().prepend(ctx, &accum.value()).prepend(ctx, &cons.first);
        accum = rust_apply(ctx, accum.slot(), func, arg_list.value())?;
        list = cons.rest;
    }

    Ok(accum)
});

def_builtin!(foldr(ctx, out) [func, list, init] {
    let mut accum = out.root(&init);

    let_slot!(ctx:arg_list);
    let mut arg_list = arg_list.nil();

    let mut stack = vec![];

    while let Ok(cons) = unpack_cons(list) {
        stack.push(cons.first);
        list = cons.rest;
    }

    while let Some(item) = stack.pop() {
        arg_list = arg_list.slot().nil().prepend(ctx, &accum.value()).prepend(ctx, &item);
        accum = rust_apply(ctx, accum.slot(), func, arg_list.value())?;
    }

    Ok(accum)
});

// (map f l) = (foldr (lambda (x acc) (cons (f x) acc)) list)
def_builtin!(map(ctx, out) [func, list] {
    let mut accum = out.nil();

    let_slot!(ctx:arg_list);
    let mut arg_list = arg_list.nil();

    let_slot!(ctx:f_out);
    let mut f_out = f_out.nil();

    let mut stack = vec![];

    while let Ok(cons) = unpack_cons(list) {
        stack.push(cons.first);
        list = cons.rest;
    }

    while let Some(item) = stack.pop() {
        arg_list = arg_list.slot().nil().prepend(ctx, &item);
        f_out = rust_apply(ctx, f_out.slot(), func, arg_list.value())?;
        accum = accum.prepend(ctx, &f_out.value());
    }

    Ok(accum)
});

def_builtin!(map_eval(ctx, out) [scope, list] {
    let mut accum = out.nil();

    let_slot!(ctx:f_out);
    let mut f_out = f_out.nil();

    let mut stack = vec![];

    while let Ok(cons) = unpack_cons(list) {
        stack.push(cons.first);
        list = cons.rest;
    }

    while let Some(item) = stack.pop() {
        f_out = rust_eval(ctx, f_out.slot(), item, scope)?;
        accum = accum.prepend(ctx, &f_out.value());
    }

    Ok(accum)
});
