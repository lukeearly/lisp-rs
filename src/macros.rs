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
macro_rules! drop_first {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

#[macro_export]
macro_rules! def_builtin {
    ($name:ident ($($ctx:ident)?, $($out:ident)?) $([$($arg_name:ident $(<- $scope:ident)? $(: $arg_type:ident)? $(| $unpack_to:ident)?),*$(,)?$(&rest $rest:ident)?])? $block:block) => {
        paste::paste!{
            #[allow(unused_assignments)]
            #[allow(unused_mut)]
            #[allow(unused_variables)]
            pub fn $name<'o, 'a>(ctx: &'o crate::thread::MutatorCtx, out: crate::root::Slot<'o>, args: crate::value::PackedValue<'a>) -> crate::builtins::BuiltinResult<'o> {
                let arg_expected = 0usize $($(
                    + crate::drop_first!($arg_name 1usize)
                )*)?;
                $(
                    let mut arg_idx = 0;
                    let mut remaining_args = args;
                    $(
                        let cons = crate::builtins::unpack::unpack_cons(remaining_args).map_err(|_| crate::builtins::BuiltinError::NotEnoughArguments { string: stringify!($name).into(),expected: arg_expected, provided: arg_idx })?;
                        arg_idx += 1;
                        let $arg_name = cons.first;
                        $(
                            crate::let_slot!(ctx:root);
                            let root = crate::builtins::eval::rust_eval(ctx, root, $arg_name, $scope)?;
                            let $arg_name = root.value();
                        )?
                        remaining_args = cons.rest;
                    )*
                    $(
                        let $rest = remaining_args;
                        remaining_args = crate::value::Value::Nil.pack();
                    )?
                    match remaining_args.unpack() {
                        crate::value::Value::Nil => (),
                        _ => return Err(crate::builtins::BuiltinError::TooManyArguments { string: stringify!($name).into(), expected: arg_expected })
                    }
                )?

                [<rust_ $name>](
                    $(crate::drop_first!($ctx ctx),)?
                    $(crate::drop_first!($out out),)?
                    $($($arg_name,)* $($rest)?)?
                )
            }

            #[allow(unused_mut)]
            pub fn [<rust_ $name>]<'o, 'a>($($ctx: &'o crate::thread::MutatorCtx,)? $(mut $out: crate::root::Slot<'o>,)? $($(
                mut $arg_name: crate::value::PackedValue<'a>,
            )* $($rest: crate::value::PackedValue<'a>,)?)?) -> crate::builtins::BuiltinResult<'o> {
                #[warn(unused_assignments)]
                #[warn(unused_mut)]
                #[warn(unused_variables)]
                $(
                    $(
                        $(
                            if !crate::builtins::types::rust::$arg_type($arg_name) {
                                return Err(crate::builtins::BuiltinError::BadArgument(format!("{}: {} is not {}", stringify!($name), unsafe { $arg_name.unguard() }, stringify!($arg_type))));
                            }
                        )?
                        $(
                            let mut $arg_name = crate::builtins::unpack::[<unpack_ $unpack_to>]($arg_name).map_err(|_| crate::builtins::BuiltinError::BadArgument(format!("{}: {} is not {}", stringify!($name), unsafe { $arg_name.unguard() }, stringify!($unpack_to))))?;
                        )?
                    )*
                )?
                { $block }
            }
        }
    }
}
