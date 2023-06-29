use crate::{
    builtins::{unpack::unpack_obj, BuiltinError},
    def_builtin, let_slot,
    value::Cons,
};

def_builtin!(closure(ctx, out) [&rest args] {
    let out = internal::closure_arg_check(ctx, out, args)?.slot();
    Ok(out.root(&args).prepend_obj(ctx, &ctx.common_symbols.closure))
});

def_builtin!(closure_apply(ctx, out) [closure_data, &rest args] {
    let_slot!(ctx:flipped);
    let flipped = flipped.alloc_cons(ctx, Cons { first: args, rest: closure_data });
    internal::closure_apply_internal(ctx, out, flipped.value())
});

mod internal {
    use crate::{
        builtins::{alist::rust_zip_alist, eval::rust_eval, list::rust_len, BuiltinError},
        def_builtin, let_slot,
        value::Cons,
    };
    def_builtin!(closure_arg_check(ctx, out) [bv: listp, fv: listp, body] {
        Ok(out.nil())
    });

    def_builtin!(closure_apply_internal(ctx, out) [args, bv: listp, fv: listp, body] {
        let_slot!(ctx:bound);
        let mut bound = rust_zip_alist(ctx, bound, fv, args, bv)?;
        rust_eval(ctx, out, body, bound.value())
    });
}
