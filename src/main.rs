use std::io::{stdin, stdout, Write};

use crate::{
    builtins::{core, eval::rust_eval},
    value::Value,
};
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

    let stdin = stdin();
    let mut stdout = stdout();
    let_slot!(ctx: scope);
    let scope = core(&ctx, scope);
    unsafe { println!("Scope: {}", scope.value().unguard()) };
    loop {
        let mut buffer = String::new();
        print!(">>> ");
        stdout.flush().unwrap();
        stdin.read_line(&mut buffer).unwrap();
        let_slot!(ctx: parse_out);
        let res = parse::parse(buffer.as_str(), &ctx, parse_out);
        match res {
            Ok(code) => {
                let_slot!(ctx: eval_out);
                let res = rust_eval(&ctx, eval_out, code.value(), scope.value());
                match res {
                    Ok(eval_out) => unsafe {
                        println!("{}", eval_out.value().unguard());
                    },
                    Err(err) => {
                        println!("Eval error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }
}
