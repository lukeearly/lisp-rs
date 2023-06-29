use crate::{
    builtins::{eval::rust_eval, list::rust_concat, BuiltinError},
    def_builtin, let_slot,
    value::Value,
};

use super::unpack::unpack_cons;

def_builtin!(with(ctx, out) [alist, closure] {
    let (bv, fv, body) = match closure.unpack() {
        Value::Object(cons) => {
            let a = unpack_cons(cons.rest).map_err(|_| BuiltinError::BadArgument("with: malformed closure".into()))?;
            let b = unpack_cons(a.rest).map_err(|_| BuiltinError::BadArgument("with: malformed closure".into()))?;
            let c = unpack_cons(b.rest).map_err(|_| BuiltinError::BadArgument("with: malformed closure".into()))?;
            (a.first, b.first, c.first)
        }
        _ => return Err(BuiltinError::BadArgument("with: second argument must be a closure".into()))
    };

    let_slot!(ctx:new_scope);
    let new_scope = rust_concat(ctx, new_scope, alist, bv)?;
    Ok(out.nil().prepend(ctx, &body).prepend(ctx, &fv).prepend(ctx, &bv).prepend_obj(ctx, &ctx.common_symbols.closure))
});

// def_builtin!(let__(ctx, out) [scope, list, body] {
// let_slot!(ctx:bindings);
// let bindings = map_keys(ctx, );
// let_slot!(ctx:new_scope);
// let new_scope = rust_concat(ctx, new_scope, list, scope)?;
// rust_eval(ctx, out, body, new_scope.value())
// });

def_builtin!(bind(ctx, out) [scope, &rest alist] {
    let_slot!(ctx:eval_out);
    let mut eval_out = eval_out.nil();
    let mut out = out.root(&scope);

    while let Ok(cons) = unpack_cons(alist) {
        let (key, expr) = if let Ok(kv) = unpack_cons(cons.first) {
            (kv.first, kv.rest)
        } else {
            (cons.first, Value::Nil.pack())
        };

        eval_out = rust_eval(ctx, eval_out.slot(), expr, scope)?;
        eval_out = eval_out.prepend(ctx, &key);

        out = out.prepend(ctx, &eval_out.value());

        alist = cons.rest;
    }
    // unsafe { println!("BIND: {}", out.value().unguard()) };
    Ok(out)
});

def_builtin!(bind_star(ctx, out) [scope, &rest alist] {
    let_slot!(ctx:eval_out);
    let mut eval_out = eval_out.nil();

    let mut out = out.root(&scope);

    while let Ok(cons) = unpack_cons(alist) {
        let (key, expr) = if let Ok(kv) = unpack_cons(cons.first) {
            (kv.first, kv.rest)
        } else {
            (cons.first, Value::Nil.pack())
        };

        eval_out = rust_eval(ctx, eval_out.slot(), expr, out.value())?;
        eval_out = eval_out.prepend(ctx, &key);

        out = out.prepend(ctx, &eval_out.value());

        alist = cons.rest;
    }
    Ok(out)
});
