pub mod alist;
pub mod closure;
pub mod control;
pub mod eval;
pub mod func;
pub mod list;
pub mod tree;
pub mod types;
pub mod unpack;

use crate::{
    object::{PackedPtr, RawCons, UnpackedPtr},
    root::{Gc, Root, Slot},
    thread::MutatorCtx,
    value::{Cons, PackedValue},
};

pub type BuiltinResult<'out> = Result<Root<'out>, BuiltinError>;

#[derive(Debug)]
pub enum BuiltinError {
    NotCallable,
    BadArgument(String),
    UndefinedSymbol(String),
    NotEnoughArguments {
        string: String,
        expected: usize,
        provided: usize,
    },
    TooManyArguments {
        string: String,
        expected: usize,
    },
}

pub type BuiltinMacro =
    for<'o, 'a> fn(&'o MutatorCtx, Slot<'o>, PackedValue<'a>, PackedValue<'a>) -> BuiltinResult<'o>;

pub type BuiltinFunction =
    for<'o, 'a> fn(&'o MutatorCtx, Slot<'o>, PackedValue<'a>) -> BuiltinResult<'o>;

macro_rules! generate_scope {
    ($name:ident [$($mod:ident :: $function:ident $(/ $macro:ident)?),*]) => {
        pub fn $name<'r>(ctx: &crate::thread::MutatorCtx, out: crate::root::Slot<'r>) -> crate::root::Root<'r> {
            let out = out.nil();
            crate::let_slot!(ctx:assoc);
            crate::let_slot!(ctx:name);
            let assoc = assoc.nil();
            let name = name.nil();
            $(
                #[allow(unused_variables)]
                let builtin_name = stringify!($function).rsplit("::").next().unwrap();
                let builtin_name = crate::util::rust_to_lisp_symbol(builtin_name);
                let name = name.slot().intern(ctx, builtin_name);
                let assoc = unsafe { assoc.slot().function($mod::$function) }.prepend(ctx, &name.value());
                let out = out.prepend(ctx, &assoc.value());
                $(
                    let macro_name = stringify!($macro);
                    let macro_name = crate::util::rust_to_lisp_symbol(macro_name);
                    let name = name.slot().intern(ctx, macro_name);
                    let assoc = unsafe { assoc.slot().function($mod::$function) }.prepend(ctx, &ctx.common_symbols._macro).prepend(ctx, &name.value());
                    let out = out.prepend(ctx, &assoc.value());
                )?
            )*
            out
        }
    }
}

generate_scope!(core [
        eval::eval,
        list::first, list::rest, list::cons, list::list, list::nthrest, list::len,
        alist::assq,
        types::listp, types::nilp, types::consp, types::proper_list_p,
        func::fold, func::foldr, func::map,
        closure::closure/lambda,
        tree::bindex
    ]
);
