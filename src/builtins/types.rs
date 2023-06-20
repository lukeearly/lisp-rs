use crate::{
    builtins::BuiltinResult,
    def_builtin, let_slot,
    object::{PackedPtr, TagType, UnpackedPtr},
    root::{Gc, Root, Slot},
    thread::MutatorCtx,
    value::{PackedValue, Value},
};

use super::{unpack::unpack_cons, BuiltinError};

macro_rules! generate_predicate {
    ($($name:ident),*) => {
        $(def_builtin! {$name(ctx, out, args, scope) [arg <- code] {
            if rust::$name(arg) {
                Ok(out.root(&ctx.common_symbols.t))
            } else {
                Ok(out.nil())
            }
        }})*
    };
}

generate_predicate!(consp, nilp, listp, proper_list_p);

pub mod rust {
    use crate::{
        builtins::unpack::unpack_cons,
        object::TagType,
        value::{PackedValue, Value},
    };

    pub fn consp(arg: PackedValue) -> bool {
        match arg.unpack() {
            Value::Cons(_) => true,
            _ => false,
        }
    }

    pub fn nilp(arg: PackedValue) -> bool {
        match arg.unpack() {
            Value::Nil => true,
            _ => false,
        }
    }

    pub fn listp(arg: PackedValue) -> bool {
        match arg.unpack() {
            Value::Cons(_) => true,
            Value::Nil => true,
            _ => false,
        }
    }

    pub fn proper_list_p(arg: PackedValue) -> bool {
        let mut res = unpack_cons(&arg);
        while let Ok(ref pair) = res {
            res = unpack_cons(&pair.rest);
        }

        if unsafe { res.unwrap_err_unchecked() == TagType::Nil } {
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        builtins::types::*,
        let_slot,
        object::{PackedPtr, RawCons},
        value::Cons,
    };

    #[test]
    fn test_nil_types() {
        let global = Box::leak(Box::new(crate::thread::GlobalState::new()));
        let ctx = crate::thread::MutatorCtx::new_from_global(global);

        let_slot!(ctx: args);
        let args = args.nil().singleton(&ctx);

        test_types(&ctx, args, [true, false, true, true]);
    }

    #[test]
    fn test_improper_cons_types() {
        let global = Box::leak(Box::new(crate::thread::GlobalState::new()));
        let ctx = crate::thread::MutatorCtx::new_from_global(global);

        let_slot!(ctx: args);
        let args = args
            .alloc_cons(&ctx, Cons::new(Value::Integer(2), Value::Integer(3)))
            .singleton(&ctx);

        test_types(&ctx, args, [false, true, true, false]);
    }

    #[test]
    fn test_proper_cons_types() {
        let global = Box::leak(Box::new(crate::thread::GlobalState::new()));
        let ctx = crate::thread::MutatorCtx::new_from_global(global);

        let_slot!(ctx: args);
        let args = args
            .alloc_cons(&ctx, Cons::new(Value::Integer(2), Value::Nil))
            .singleton(&ctx);

        test_types(&ctx, args, [false, true, true, true]);
    }

    fn test_types(ctx: &MutatorCtx, ref args: Root, bools: [bool; 4]) {
        let_slot!(ctx: t);
        let t = t.root(&ctx.common_symbols.t);

        let_slot!(ctx: nil);
        let nil = nil.nil();

        let_slot!(ctx: out);
        let out = nilp(&ctx, out, args.value(), None).unwrap();
        assert!(out.value() == (if bools[0] { &t } else { &nil }).value());

        let out = out.slot();
        let out = consp(&ctx, out, args.value(), None).unwrap();
        assert!(out.value() == (if bools[1] { &t } else { &nil }).value());

        let out = out.slot();
        let out = listp(&ctx, out, args.value(), None).unwrap();
        assert!(out.value() == (if bools[2] { &t } else { &nil }).value());

        let out = out.slot();
        let out = proper_list_p(&ctx, out, args.value(), None).unwrap();
        assert!(out.value() == (if bools[3] { &t } else { &nil }).value());
    }
}
