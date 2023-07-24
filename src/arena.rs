use std::{collections::HashMap, ptr::NonNull, sync::Mutex};

use crate::{
    alloc::{GlobalImmixAllocator, ImmixMutator},
    heap::{id, LAlloc},
    object::LString,
};

pub struct Arena {
    map: HashMap<String, NonNull<LString>>,
    arena: ImmixMutator<'static>,
}

impl Arena {
    pub fn new() -> Self {
        let map = HashMap::new();
        let state = Box::new(Mutex::new(GlobalImmixAllocator::new()));
        let arena = ImmixMutator::new(Box::leak(state));
        Arena { map, arena }
    }

    pub fn get(&mut self, name: &String) -> Option<NonNull<LString>> {
        self.map.get(name).copied()
    }

    pub fn intern(&mut self, name: String) -> NonNull<LString> {
        if let Some(sym) = self.map.get(&name) {
            *sym
        } else {
            let sym = self.dangling(name.clone());
            self.map.insert(name, sym);
            sym
        }
    }

    pub fn dangling(&mut self, name: String) -> NonNull<LString> {
        unsafe { self.arena.object(id, LString::leak_str(name)).unwrap() }
    }
}

macro_rules! symbol_cache {
    ($name:ident [$($sym_name:ident $(: $sym_str:literal)?),*]) => {
        pub struct $name {
            $(pub $sym_name: crate::value::PackedValue<'static>,)*
        }

        impl $name {
            pub fn new(arena: &mut Arena) -> Self {
                $(
                    let value = crate::util::rust_to_lisp_symbol(stringify!($sym_name));
                    $(let value = $sym_str;)?
                    let $sym_name = crate::value::Value::Symbol(unsafe { crate::root::Gc::new(arena.intern(value.into()).as_ref())}).pack();
                )*
                Self {
                    $($sym_name,)*
                }
            }
        }
    };
}

symbol_cache!(CommonSymbols [quote, quasiquote, unquote, t, lambda, _macro: "macro", fexpr, closure]);
