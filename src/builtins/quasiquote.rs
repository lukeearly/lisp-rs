use crate::{
    def_builtin, let_slot,
    value::{Cons, Value},
};

use super::{
    eval::rust_eval,
    unpack::{unpack_cons, unpack_int},
    BuiltinError,
};

def_builtin!(eval_quasiquote(ctx, out) [scope, datum, level] {
    let num = unpack_int(level).map_err(|_| BuiltinError::BadArgument("eval_quasiquote: level must be an integer".into()))?;

    if num == 0 {
        return rust_eval(ctx, out, datum, scope);
    }

    match datum.unpack() {
        Value::Cons(cons) => {
            if cons.first == ctx.common_symbols.unquote {
                if let Ok(rest) = unpack_cons(cons.rest) {
                    if rest.rest == Value::Nil.pack() {
                        let inner = rest.first;
                        rust_eval_quasiquote(ctx, out, scope, inner, Value::Integer(num - 1).pack())
                    } else {
                        Err(BuiltinError::BadArgument("syntax error: (unquote x) expected".into()))
                    }
                } else {
                    Err(BuiltinError::BadArgument("syntax error: (unquote x) expected".into()))
                }
            } else if cons.first == ctx.common_symbols.quasiquote {
                if let Ok(rest) = unpack_cons(cons.rest) {
                    if rest.rest == Value::Nil.pack() {
                        let inner = rest.first;
                        rust_eval_quasiquote(ctx, out, scope, inner, Value::Integer(num + 1).pack())
                    } else {
                        Err(BuiltinError::BadArgument("syntax error: (quasiquote x) expected".into()))
                    }
                } else {
                    Err(BuiltinError::BadArgument("syntax error: (quasiquote x) expected".into()))
                }
            } else {
                let_slot!(ctx:first);
                let_slot!(ctx:rest);
                let first = rust_eval_quasiquote(ctx, first, scope, cons.first, level)?;
                let rest = rust_eval_quasiquote(ctx, rest, scope, cons.rest, level)?;
                Ok(out.alloc_cons(ctx, Cons { first: first.value(), rest: rest.value() }))
            }
        },
        _ => Ok(out.root(&datum))
    }
});
