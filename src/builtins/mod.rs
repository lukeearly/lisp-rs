pub mod alist;
pub mod eval;
pub mod list;
pub mod types;
pub mod unpack;

use std::cell::Cell;

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

pub type BuiltinFn = for<'o> fn(
    &'o MutatorCtx,
    Slot<'o>,
    PackedValue<'o>,
    Option<PackedValue<'o>>,
) -> BuiltinResult<'o>;

macro_rules! generate_scope {
    ($name:ident [$($builtin:expr),*]) => {
        pub fn $name<'r>(ctx: &crate::thread::MutatorCtx, out: crate::root::Slot<'r>) -> crate::root::Root<'r> {
            let out = out.nil();
            crate::let_slot!(ctx:assoc);
            crate::let_slot!(ctx:name);
            let assoc = assoc.nil();
            let name = name.nil();
            $(
                let name = name.slot().intern(ctx, crate::util::rust_to_lisp_symbol(stringify!($builtin).rsplit("::").next().unwrap()));
                let assoc = unsafe { assoc.slot().builtin($builtin) }.prepend(ctx, &name.value());
                let out = out.prepend(ctx, &assoc.value());
            )*
            out
        }
    }
}

generate_scope!(core [ eval::eval, list::first, list::rest, list::cons, alist::assq, types::listp, types::nilp, types::consp, types::proper_list_p ]);
