use std::sync::Mutex;

use crate::{
    alloc::{GlobalImmixAllocator, ImmixMutator},
    arena::{Arena, CommonSymbols},
};

pub struct GlobalState {
    pub alloc_state: Mutex<GlobalImmixAllocator>,
    pub string_arena: Mutex<Arena>,
    pub common_symbols: &'static CommonSymbols,
}

impl GlobalState {
    pub fn new() -> Self {
        let mut arena = Arena::new();
        let common_symbols = Box::leak(Box::new(CommonSymbols::new(&mut arena)));
        GlobalState {
            alloc_state: Mutex::new(GlobalImmixAllocator::new()),
            string_arena: Mutex::new(arena),
            common_symbols: common_symbols,
        }
    }
}

pub struct MutatorCtx {
    pub alloc: ImmixMutator<'static>,
    pub string_arena: &'static Mutex<Arena>,
    pub common_symbols: &'static CommonSymbols,
}

impl MutatorCtx {
    pub fn new_from_global(global: &'static GlobalState) -> Self {
        MutatorCtx {
            alloc: ImmixMutator::new(&global.alloc_state),
            string_arena: &global.string_arena,
            common_symbols: &global.common_symbols,
        }
    }
}
