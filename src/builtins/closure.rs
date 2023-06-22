use crate::{def_builtin, let_slot, value::Cons};

def_builtin!(closure(ctx, out) [&rest args] {
    let out = internal::closure_arg_check(ctx, out, args)?.slot();
    Ok(out.root(&args).prepend(ctx, &ctx.common_symbols.closure))
});

def_builtin!(closure_apply(ctx, out) [closure, &rest args] {
    let_slot!(ctx:flipped);
    let flipped = flipped.alloc_cons(ctx, Cons { first: args, rest: closure });
    internal::closure_apply_internal(ctx, out, flipped.value())
});

mod internal {
    use crate::{
        builtins::{
            eval::rust_eval,
            list::rust_len,
            unpack::{unpack_cons, unpack_int},
            BuiltinError,
        },
        def_builtin, let_slot,
        value::Cons,
    };
    def_builtin!(closure_arg_check(ctx, out) [bv: listp, fv: listp, body] {
        Ok(out.nil())
    });

    def_builtin!(closure_apply_internal(ctx, out) [args, bv: listp, fv: listp, body] {
        let_slot!(ctx:bound);
        let mut bound = bound.root(&bv);
        let mut args_iter = args;
        let mut fv_iter = fv;
        loop {
            let key = unpack_cons(fv_iter);
            let val = unpack_cons(args_iter);

            if key.is_err() && val.is_err() {
                break
            } else if key.is_err() {
                let expected = unpack_int(rust_len(ctx, out, fv)?.value()).unwrap() as usize;
                return Err(BuiltinError::TooManyArguments { string: "closure: too many arguments".into(), expected });
            } else if val.is_err() {
                let out = rust_len(ctx, out, fv)?;
                let expected = unpack_int(out.value()).unwrap() as usize;
                let out = rust_len(ctx, out.slot(), args)?;
                let provided = unpack_int(out.value()).unwrap() as usize;
                return Err(BuiltinError::NotEnoughArguments { string: "closure: not enough arguments".into(), expected, provided });
            } else {
                let key = key.unwrap();
                let val = val.unwrap();
                let_slot!(ctx:entry);
                let entry = entry.alloc_cons(ctx, Cons { first: key.first, rest: val.first });
                bound = bound.prepend(ctx, &entry.value());
                fv_iter = key.rest;
                args_iter = key.rest;
            }
        }
        rust_eval(ctx, out, body, bound.value())
    });
}
