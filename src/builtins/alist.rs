use crate::def_builtin;

use super::unpack::unpack_cons;

def_builtin! {assq(ctx, out, args, scope) [key <- code, list <- code] {
    let mut res = unpack_cons(&list);
    while let Ok(ref pair) = res {
        if let Ok(ref assoc) = unpack_cons(&pair.first) {
            if assoc.first == key {
                return Ok(out.root(&pair.first))
            }
        }
        res = unpack_cons(&pair.rest);
    }
    Ok(out.nil())
}}
