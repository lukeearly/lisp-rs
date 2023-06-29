use std::{
    env, fs,
    io::{stdin, stdout, Write},
};

use root::{Root, Slot};
use value::PackedValue;

use crate::builtins::{core, eval::rust_eval};
#[macro_use]
extern crate pest_derive;

mod alloc;
mod arena;
mod builtins;
mod heap;
mod linked_list;
mod macros;
mod object;
mod parse;
mod print;
mod root;
mod sorted_vec;
mod thread;
mod util;
mod value;

fn main() {
    let global = Box::leak(Box::new(thread::GlobalState::new()));
    let ctx = thread::MutatorCtx::new_from_global(global);

    let_slot!(ctx: scope);
    let mut scope = core(&ctx, scope);

    global.alloc_state.lock().unwrap().gc();

    for path in env::args().skip(1) {
        let_slot!(ctx: eval_out);

        let source = fs::read_to_string(path).expect("cannot read file");
        if let Some(out) = eval_string(&ctx, scope.value(), eval_out, source.as_str()) {
            scope = scope.slot().root(&out.value());
        }
    }

    // unsafe { println!("Scope: {}", scope.value().unguard()) };

    return;

    let stdin = stdin();
    let mut stdout = stdout();
    loop {
        let mut buffer = String::new();
        print!(">>> ");
        stdout.flush().unwrap();
        stdin.read_line(&mut buffer).unwrap();

        let_slot!(ctx: eval_out);
        let out = eval_string(&ctx, scope.value(), eval_out, buffer.as_str());
    }
}

fn eval_string<'o, 'a>(
    ctx: &'o thread::MutatorCtx,
    scope: PackedValue<'a>,
    out: Slot<'o>,
    str: &str,
) -> Option<Root<'o>> {
    let_slot!(ctx: parse_out);
    let res = parse::parse(str, &ctx, parse_out);
    match res {
        Ok(code) => {
            let res = rust_eval(&ctx, out, code.value(), scope);
            match res {
                Ok(eval_out) => unsafe {
                    println!("{}", eval_out.value().unguard());
                    return Some(eval_out);
                },
                Err(err) => {
                    println!("Eval error: {:?}", err);
                    return None;
                }
            }
        }
        Err(err) => {
            println!("{}", err);
            return None;
        }
    }
}
