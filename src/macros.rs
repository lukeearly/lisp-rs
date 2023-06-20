#[macro_export]
macro_rules! lisp_read {
    ($local:expr, $expression:tt) => {{
        use crate::{parse, thread};
        parse::parse(stringify!($expression).into(), &mut $local)
    }};
}

#[macro_export]
macro_rules! let_slot {
    ($($ctx:ident : $slot:ident),* $(,)?) => { $(
        let $slot = unsafe { crate::root::RootNode::new() };
        let $slot = unsafe { crate::root::Slot::new(std::pin::Pin::new_unchecked(&$slot), &$ctx) };
    )* };
    ($(($ctx:expr) : $slot:ident),* $(,)?) => { $(
        let $slot = unsafe { crate::root::RootNode::new() };
        let $slot = unsafe { crate::root::Slot::new(std::pin::Pin::new_unchecked(&$slot), &$ctx) };
    )* };
}

#[macro_export]
macro_rules! replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

#[macro_export]
macro_rules! def_builtin {
    ($name:ident($ctx:ident, $out:ident, $args:ident, $scope:ident) $([$($arg_name:ident $(<- $code_name:ident)? $(: $arg_type:ident)?),*$(& $rest:ident)?])? $block:block) => {
        #[allow(unused_mut)]
        #[allow(unused_assignments)]
        pub fn $name<'o>($ctx: &'o crate::thread::MutatorCtx, mut $out: crate::root::Slot<'o>, $args: crate::value::PackedValue<'o>, $scope: Option<crate::value::PackedValue<'o>>) -> crate::builtins::BuiltinResult<'o> {
            let arg_expected = 0usize $($(
                + crate::replace_expr!($arg_name 1usize)
            )*)?;
            let mut arg_idx = 0;
            let mut remaining_args = &$args;
            $($(
                let cons = crate::builtins::unpack::unpack_cons(remaining_args).map_err(|_| crate::builtins::BuiltinError::NotEnoughArguments { string: stringify!($name).into(),expected: arg_expected, provided: arg_idx })?;
                arg_idx += 1;
                let $arg_name = cons.first.clone();
                $(
                    let $code_name = $arg_name;
                    crate::let_slot!($ctx:eval_out);
                    crate::let_slot!($ctx:eval_args);
                    let eval_args = eval_args.root(&$scope.clone().unwrap_or(crate::value::Value::Nil.pack())).singleton($ctx).prepend($ctx, &$code_name);
                    let eval_out = crate::builtins::eval::eval($ctx, eval_out, eval_args.value(), None)?;
                    let $arg_name = eval_out.value();
                )?
                $(
                    if !crate::builtins::types::rust::$arg_type($arg_name.clone()) {
                        return Err(crate::builtins::BuiltinError::BadArgument(format!("{} is not {}", unsafe { $arg_name.unguard() }, stringify!($arg_type))));
                    }
                )?
                remaining_args = &cons.rest;
            )*
                $(
                    let $rest = remaining_args;
                    let nil = crate::value::Value::Nil.pack();
                    remaining_args = &nil;
                )?
            )?
            match remaining_args.unpack() {
                crate::value::Value::Nil => (),
                _ => return Err(crate::builtins::BuiltinError::TooManyArguments { string: stringify!($name).into(), expected: arg_expected })
            }
            drop(arg_expected);
            drop(arg_idx);
            drop(remaining_args);
            $block
        }
    }
}
