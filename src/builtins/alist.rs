use crate::def_builtin;

use super::unpack::unpack_cons;

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
