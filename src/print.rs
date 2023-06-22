use core::slice;
use std::{fmt::Display, string};

use crate::object::PackedPtr;

impl Display for PackedPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::object::UnpackedPtr::*;
        match self.unpack() {
            Integer(n) => {
                write!(f, "{}", n)
            }
            Cons(ptr) => {
                let mut cons = unsafe { *(ptr.as_ptr()) };
                // return write!(f, "({} . {})", cons.first, cons.rest);
                write!(f, "({}", cons.first)?;
                loop {
                    let rest = cons.rest.unpack();
                    match rest {
                        Cons(ptr) => {
                            cons = unsafe { *(ptr.as_ptr()) };
                            write!(f, " {}", cons.first)?;
                        }
                        Nil => {
                            write!(f, ")")?;
                            break;
                        }
                        _ => {
                            write!(f, " . {})", cons.rest)?;
                            break;
                        }
                    }
                }
                Ok(())
            }
            Symbol(ptr) => {
                let sym = unsafe { *(ptr.as_ptr()) };
                // let slice = unsafe { slice::from_raw_parts(sym.start, sym.len) };
                write!(f, "{}", sym.to_string())
            }
            Nil => {
                write!(f, "()")
            }
            Function(_) => write!(f, "<BUILTIN>"),
            Object(_) => write!(f, "<OBJECT>"),
        }
    }
}
