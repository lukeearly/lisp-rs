pub mod alist;
pub mod closure;
pub mod control;
pub mod eval;
pub mod func;
pub mod list;
pub mod obj;
pub mod quasiquote;
pub mod tree;
pub mod types;
pub mod unpack;

use crate::{
    object::{PackedPtr, RawCons, TagType, UnpackedPtr},
    root::{Gc, Root, Slot},
    thread::MutatorCtx,
    value::{Cons, PackedValue},
};

pub type BuiltinResult<'out> = Result<Root<'out>, BuiltinError>;

#[derive(Debug)]
pub enum BuiltinError {
    NotCallable(String, TagType),
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

pub type BuiltinFunction =
    for<'o, 'a> fn(&'o MutatorCtx, Slot<'o>, PackedValue<'a>) -> BuiltinResult<'o>;

macro_rules! generate_scope {
    ($name:ident
        functions: [$($function_mod:ident :: $function:ident $(/ $function_name:ident)?),*]
        macros: [$($macro_mod:ident :: $macro:ident $(/ $macro_name:ident)?),*]
        fexprs: [$($fexpr_mod:ident :: $fexpr:ident $(/ $fexpr_name:ident)?),*]
    ) => {
        pub fn $name<'r>(ctx: &crate::thread::MutatorCtx, out: crate::root::Slot<'r>) -> crate::root::Root<'r> {
            let out = out.nil();
            crate::let_slot!(ctx:assoc);
            crate::let_slot!(ctx:name);
            let assoc = assoc.nil();
            let name = name.nil();
            $(
                #[allow(unused_variables)]
                let function_name = stringify!($function).rsplit("::").next().unwrap();
                $(let function_name = stringify!($function_name);)?
                let function_name = crate::util::rust_to_lisp_symbol(function_name);

                let name = name.slot().intern(ctx, function_name);
                let assoc = unsafe { assoc.slot().function($function_mod::$function) }.prepend(ctx, &name.value());
                let out = out.prepend(ctx, &assoc.value());
            )*
            $(
                #[allow(unused_variables)]
                let macro_name = stringify!($macro).rsplit("::").next().unwrap();
                $(let macro_name = stringify!($macro_name);)?
                let macro_name = crate::util::rust_to_lisp_symbol(macro_name);

                let name = name.slot().intern(ctx, macro_name);
                let assoc = unsafe { assoc.slot().function($macro_mod::$macro) }._macro(ctx).prepend(ctx, &name.value());
                let out = out.prepend(ctx, &assoc.value());
            )*
            $(
                #[allow(unused_variables)]
                let fexpr_name = stringify!($fexpr).rsplit("::").next().unwrap();
                $(let fexpr_name = stringify!($fexpr_name);)?
                let fexpr_name = crate::util::rust_to_lisp_symbol(fexpr_name);

                let name = name.slot().intern(ctx, fexpr_name);
                let assoc = unsafe { assoc.slot().function($fexpr_mod::$fexpr) }.fexpr(ctx).prepend(ctx, &name.value());
                let out = out.prepend(ctx, &assoc.value());
            )*
            out
        }
    }
}

generate_scope!(core
    functions: [
        eval::eval, eval::apply,
        list::first, list::rest, list::cons, list::list, list::nthrest, list::len, list::concat,
        obj::objfirst, obj::objrest, obj::obj,
        alist::assq,
        types::listp, types::nilp, types::consp, types::proper_list_p, types::objp,
        func::fold, func::foldr, func::map,
        closure::closure,
        control::with,
        tree::bindex
    ]

    macros: [

    ]

    fexprs: [
        closure::closure/lambda,
        control::bind, control::bind_star
    ]
);
