extern crate pest;

use pest::iterators::Pair;
use pest::Parser;

use crate::heap::{id, LAlloc};
use crate::let_slot;
use crate::object::{PackedPtr, RawCons};
use crate::root::{Root, Slot};
use crate::thread::MutatorCtx;
use crate::value::{Cons, Value};

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct LParser;

pub fn parse<'r>(
    str: &str,
    dest: &MutatorCtx,
    out: Slot<'r>,
) -> Result<Root<'r>, pest::error::Error<Rule>> {
    let res = LParser::parse(Rule::top_level, str);
    if let Err(err) = res {
        return Err(err);
    }
    match res.unwrap().next() {
        Some(pair) => Ok(sexp_to_object(pair, dest, out)),
        None => Ok(out.nil()),
    }
}

fn sexp_to_object<'r>(pair: Pair<Rule>, ctx: &MutatorCtx, out: Slot<'r>) -> Root<'r> {
    let rule = pair.as_rule();
    match rule {
        Rule::nil_term_list | Rule::custom_term_list => {
            let mut iter = pair.into_inner().rev();
            let out = if rule == Rule::nil_term_list {
                out.nil()
            } else {
                sexp_to_object(iter.next().unwrap(), ctx, out)
            };

            let_slot!(ctx: item);
            let mut item = item;
            let mut out = out;
            for inner_pair in iter {
                let entry = sexp_to_object(inner_pair, ctx, item);
                out = out.prepend(ctx, &entry.value());
                item = entry.slot();
            }
            out
        }
        Rule::binary => out.root(
            &Value::Integer(
                isize::from_str_radix(pair.as_str().trim_start_matches("0b"), 2).unwrap(),
            )
            .pack(),
        ),
        Rule::octal => out.root(
            &Value::Integer(
                isize::from_str_radix(pair.as_str().trim_start_matches("0"), 8).unwrap(),
            )
            .pack(),
        ),
        Rule::decimal => out.root(&Value::Integer(pair.as_str().parse().unwrap()).pack()),
        Rule::hexadecimal => out.root(
            &Value::Integer(
                isize::from_str_radix(pair.as_str().trim_start_matches("0x"), 16).unwrap(),
            )
            .pack(),
        ),
        Rule::symbol => out.intern(ctx, pair.as_str().to_string()),
        // Rule::string => {
        //     let inner = pair.into_inner().next().unwrap();
        //     let ptr = ctx
        //         .string_arena
        //         .lock()
        //         .unwrap()
        //         .intern(inner.as_str().to_string());
        //     PackedPtr::str_ptr(ptr)
        // }
        Rule::quote | Rule::quasiquote | Rule::unquote => {
            let inner = pair.into_inner().next().unwrap();
            let out = sexp_to_object(inner, ctx, out);
            let prefix = match rule {
                Rule::quote => ctx.common_symbols.quote,
                Rule::quasiquote => ctx.common_symbols.quasiquote,
                Rule::unquote => ctx.common_symbols.unquote,
                _ => unreachable!(),
            };
            out.singleton(ctx).prepend(ctx, &prefix)
        }
        // Rule::pair => {
        //     let mut iter = pair.into_inner();
        //     let first = iter.next().unwrap();
        //     let rest = iter.next().unwrap();

        //     let_slot!(ctx:left);
        //     let left = sexp_to_object(first, ctx, left);
        //     let out = sexp_to_object(rest, ctx, out);
        //     out.prepend(ctx, &left.value())
        // }
        _ => {
            println!("unimplemented rule: {:?}", pair.as_rule());
            // let ptr = ctx.string_arena.lock().unwrap().intern("nil".to_string());
            out.nil()
        }
    }
}
