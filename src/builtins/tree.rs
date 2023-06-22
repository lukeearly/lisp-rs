use crate::def_builtin;

use super::{unpack::unpack_cons, BuiltinError};

/*
      1
    3   2
   7 5 6 4
*/

def_builtin!(bindex(ctx, out) [idx|int, tree] {
    if idx <= 0 {
        return Err(BuiltinError::BadArgument("bindex: idx must be positive".into()));
    }
    let mut i = idx;
    let mut n = 0;
    while i != 1 {
        i >>= 1;
        n += 1;
    }
    for i in (0..n).rev() {
        if let Ok(cons) = unpack_cons(tree) {
            if idx & 1 << i == 0 {
                tree = cons.rest;
            } else {
                tree = cons.first;
            }
        }
        else {
            return Ok(out.nil());
        }
    }

    Ok(out.root(&tree))
});
