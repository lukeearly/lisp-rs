use crate::builtins::alist::rust_assq;
use crate::builtins::func::rust_map_eval;
use crate::builtins::quasiquote::rust_eval_quasiquote;
use crate::builtins::unpack::unpack_cons;
use crate::object::TagType;
use crate::value::Value;
use crate::{def_builtin, let_slot};

use super::alist::assq;
use super::closure::rust_closure_apply;
use super::types::rust::*;
use super::BuiltinError;

def_builtin!(eval(ctx, out) [code, scope: listp] {
    // unsafe { println!("EVAL: {}", code.unguard()); };
    match code.unpack() {
        Value::Cons(ptr) => {
            if !proper_list_p(code) {
                return Err(BuiltinError::BadArgument("code contains an improper list".into()))
            }

            let left = ptr.first;
            let right = ptr.rest;

            if left == ctx.common_symbols.quote {
                return Ok(out.root(&unpack_cons(right).map_err(|_| BuiltinError::NotEnoughArguments { string: "quote".into(), expected: 1, provided: 0 })?.first));
            } else if left == ctx.common_symbols.quasiquote {
                return rust_eval_quasiquote(ctx, out, scope, unpack_cons(right).map_err(|_| BuiltinError::NotEnoughArguments { string: "quote".into(), expected: 1, provided: 0 })?.first, Value::Integer(1).pack());
            }

            let_slot!(ctx:left_out);
            let left_out = rust_eval(ctx, left_out, left, scope)?;

            if let Value::Object(ptr) = left_out.value().unpack() {
                if ptr.first == ctx.common_symbols.fexpr {
                    let_slot!(ctx:fexpr_arg);
                    let arg_root = fexpr_arg.root(&right).prepend(ctx, &scope);
                    return rust_apply(ctx, out, ptr.rest, arg_root.value());
                } else if ptr.first == ctx.common_symbols._macro {
                    let_slot!(ctx:macro_out);
                    let macro_out = rust_apply(ctx, macro_out, ptr.rest, right)?;
                    unsafe { println!("{}", macro_out.value().unguard()) }
                    return rust_eval(ctx, out, macro_out.value(), scope);
                }
            }

            let_slot!(ctx:map_out);
            let map_out = rust_map_eval(ctx, map_out, scope, right)?;

            let res = rust_apply(ctx, out, left_out.value(), map_out.value());

            if res.is_err() {
                unsafe { println!("{}", left.unguard()) }
                // unsafe { println!("{}", left_out.value().unguard()) }
            }

            res
        }
        Value::Symbol(ptr) => {
            let_slot!(ctx:assq_out);
            let assq_out = rust_assq(ctx, assq_out, code, scope)?;
            match assq_out.value().unpack() {
                Value::Cons(ptr) => Ok(out.root(&ptr.rest)),
                _ => {
                    // unsafe { println!("scope: {}", scope.unguard()) }
                    Err(BuiltinError::UndefinedSymbol(ptr.to_string()))
                }
            }
        }
        Value::Nil | Value::Integer(_) | Value::Function(_) | Value::Object(_) => {
            // self-evaluating forms
            Ok(out.root(&code))
        }
    }
});

def_builtin!(apply(ctx, out) [left, right] {
    match left.unpack() {
        Value::Function(fn_ptr) => fn_ptr(&ctx, out, right.clone()),
        Value::Object(cons) => {
            if cons.first == ctx.common_symbols.closure {
                rust_closure_apply(ctx, out, cons.rest, right)
            } else {
                Err(BuiltinError::NotCallable("apply: uncallable object".into(), TagType::Object))
            }
        },
        _ => {
            // unsafe { println!("{}", left.unguard()) }
            // unsafe { println!("{}", right.unguard()) }
            Err(BuiltinError::NotCallable("apply: uncallable object".into(), unsafe { left.unguard().tag_type() }))
        }
    }

    // } else if *left == ctx.common_symbols.first {
    //     return first(ctx, out, &ptr.rest);
    // } else if *left == ctx.common_symbols.rest {
    //     return rest(ctx, out, &ptr.rest);
    // }
});

#[cfg(test)]
mod test {
    use crate::{
        builtins::eval::eval,
        let_slot,
        root::{Root, Slot},
        thread::MutatorCtx,
        value::{Cons, Value},
    };

    macro_rules! eval_test {
        (| $ctx:ident, $args:ident | $args_block:block, | $out:ident | $out_block:block) => {
            let global = Box::leak(Box::new(crate::thread::GlobalState::new()));
            let ctx = crate::thread::MutatorCtx::new_from_global(global);

            let_slot!(ctx: $args);
            let $ctx = &ctx;
            let $args = { $args_block };

            let_slot!(ctx: $out);
            let $out = eval(&$ctx, $out, $args.value()).unwrap();

            $out_block
        };
    }

    #[test]
    fn eval_nil() {
        eval_test!(
            |ctx, args| {
                args.nil()
                    .map(|slot, root| slot.nil().singleton(ctx).prepend(ctx, &root.value()))
            },
            |out| { assert!(out.value() == Value::Nil.pack()) }
        );
    }

    #[test]
    fn eval_symbol() {
        eval_test!(
            |ctx, args| {
                let_slot!(ctx: code);
                let code = code.intern(ctx, "var".into());
                let args = args
                    .intern(ctx, "var".into())
                    .map(|slot, root| {
                        slot.alloc_cons(
                            ctx,
                            Cons {
                                first: root.value(),
                                rest: Value::Integer(2).pack(),
                            },
                        )
                    })
                    .singleton(ctx)
                    .singleton(ctx)
                    .prepend(ctx, &code.value());
                drop(code);
                args
            },
            |out| { assert!(out.value() == Value::Integer(2).pack()) }
        );
    }
}
