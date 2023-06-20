use crate::builtins::unpack::unpack_cons;
use crate::value::Value;
use crate::{def_builtin, let_slot};

use super::alist::assq;
use super::types::rust::*;
use super::BuiltinError;

def_builtin!(eval(ctx, out, args, ext_scope) [code, int_scope] {
    // unsafe { println!("{}", int_scope.unguard()) }
    match code.unpack() {
        Value::Cons(ptr) => {
            if !proper_list_p(code) {
                return Err(BuiltinError::BadArgument("code contains an improper list".into()))
            }

            let left = &ptr.first;

            if *left == ctx.common_symbols.quote {
                return Ok(out.root(&unpack_cons(&ptr.rest).map_err(|_| BuiltinError::NotEnoughArguments { string: "quote".into(), expected: 1, provided: 0 })?.first));
            }

            let_slot!(ctx:left_eval_args);
            let left_eval_args = left_eval_args.nil();
            let left_eval_args = left_eval_args.prepend(ctx, &int_scope);
            let left_eval_args = left_eval_args.prepend(ctx, left);

            let_slot!(ctx:left_eval_out);
            let left_eval_out = eval(ctx, left_eval_out, left_eval_args.value(), None)?;
            
            match left_eval_out.value().unpack() {
                // Value::Builtin(fn_ptr) => fn_ptr(&ctx, out, ptr.rest.clone()),
                Value::Builtin(fn_ptr) => fn_ptr(&ctx, out, ptr.rest.clone(), Some(int_scope)),
                _ => Err(BuiltinError::NotCallable)
            }

            // } else if *left == ctx.common_symbols.first {
            //     return first(ctx, out, &ptr.rest);
            // } else if *left == ctx.common_symbols.rest {
            //     return rest(ctx, out, &ptr.rest);
            // }
        }
        Value::Symbol(ptr) => {
            let_slot!(ctx:assq_args);
            let_slot!(ctx:assq_code);

            let assq_code = assq_code.root(&code).quote(ctx);

            let assq_args = assq_args.root(&int_scope).quote(ctx).singleton(ctx);
            let assq_args = assq_args.prepend(ctx, &assq_code.value());
            drop(assq_code);

            let_slot!(ctx:assq_out);
            let assq_out = assq(ctx, assq_out, assq_args.value(), None)?;
            match assq_out.value().unpack() {
                Value::Cons(ptr) => Ok(out.root(&ptr.rest)),
                _ => Err(BuiltinError::UndefinedSymbol(ptr.to_string()))
            }
        }
        Value::Nil | Value::Integer(_) | Value::String(_) | Value::Builtin(_) => {
            // self-evaluating forms
            Ok(out.root(&code))
        }
    }
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
            let $out = eval(&$ctx, $out, $args.value(), None).unwrap();

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
