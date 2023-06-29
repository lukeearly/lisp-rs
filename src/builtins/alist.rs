use crate::{def_builtin, let_slot, value::Cons};

use super::{
    list::rust_len,
    unpack::{unpack_cons, unpack_int},
    BuiltinError,
};

def_builtin!(assq(ctx, out) [key, list] {
    let mut res = unpack_cons(list);
    while let Ok(pair) = res {
        if let Ok(assoc) = unpack_cons(pair.first) {
            if assoc.first == key {
                return Ok(out.root(&pair.first))
            }
        }
        res = unpack_cons(pair.rest);
    }
    Ok(out.nil())
});

def_builtin!(zip_alist(ctx, out) [keys, vals, init] {
    let mut out = out.root(&init);
    let mut keys_iter = keys;
    let mut vals_iter = vals;
    loop {
        let key = unpack_cons(keys_iter);
        let val = unpack_cons(vals_iter);

        if key.is_err() && val.is_err() {
            break
        } else if key.is_err() {
            let expected = unpack_int(rust_len(ctx, out.slot(), keys)?.value()).unwrap() as usize;
            return Err(BuiltinError::TooManyArguments { string: "more values than keys".into(), expected });
        } else if val.is_err() {
            let out = rust_len(ctx, out.slot(), keys)?;
            let expected = unpack_int(out.value()).unwrap() as usize;
            let out = rust_len(ctx, out.slot(), vals)?;
            let provided = unpack_int(out.value()).unwrap() as usize;
            return Err(BuiltinError::NotEnoughArguments { string: "more keys than values".into(), expected, provided });
        } else {
            let key = key.unwrap();
            let val = val.unwrap();
            let_slot!(ctx:entry);
            let entry = entry.alloc_cons(ctx, Cons { first: key.first, rest: val.first });
            out = out.prepend(ctx, &entry.value());
            keys_iter = key.rest;
            vals_iter = val.rest;
        }
    }

    Ok(out)
});
