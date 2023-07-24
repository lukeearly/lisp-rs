use std::{cell::Cell, fmt::Display, marker::PhantomData, ops::Deref, pin::Pin, ptr::NonNull};

use crate::{
    alloc::ImmixMutator,
    builtins::BuiltinFunction,
    heap::LAlloc,
    let_slot,
    linked_list::{LinkedList, LinkedListIter, LinkedListNode},
    object::{PackedPtr, RawCons},
    thread::MutatorCtx,
    value::{Cons, PackedValue, Value},
};

pub struct Root<'slot> {
    slot: Slot<'slot>,
}

pub struct Slot<'slot>(Pin<&'slot RootNode>);

impl<'slot> Slot<'slot> {
    pub unsafe fn new(inner: Pin<&'slot RootNode>, ctx: &MutatorCtx) -> Self {
        ctx.alloc.add_root(inner);
        Slot(inner)
    }

    pub unsafe fn new_out_of_list(inner: Pin<&'slot RootNode>) -> Self {
        Slot(inner)
    }

    pub fn root_raw(self, ptr: PackedPtr) -> Root<'slot> {
        let root = Root { slot: self };
        root.slot.0.ptr.set(ptr);
        root
    }

    pub fn root<'guard>(self, ptr: &PackedValue<'guard>) -> Root<'slot> {
        let root = Root { slot: self };
        root.slot.0.ptr.set(unsafe { ptr.unguard() });
        root
    }

    pub fn alloc_obj(self, ctx: &MutatorCtx, cons: Cons) -> Root<'slot> {
        ctx.alloc
            .object(
                |ptr| {
                    self.root_raw(PackedPtr::obj_ptr(unsafe {
                        NonNull::new_unchecked(ptr.as_ptr() as *mut RawCons)
                    }))
                },
                cons,
            )
            .unwrap()
    }

    pub fn alloc_cons(self, ctx: &MutatorCtx, cons: Cons) -> Root<'slot> {
        ctx.alloc
            .object(
                |ptr| {
                    self.root_raw(PackedPtr::cons_ptr(unsafe {
                        NonNull::new_unchecked(ptr.as_ptr() as *mut RawCons)
                    }))
                },
                cons,
            )
            .unwrap()
    }

    pub fn alloc_raw_cons(self, ctx: &MutatorCtx, cons: RawCons) -> Root<'slot> {
        ctx.alloc
            .object(|ptr| self.root_raw(PackedPtr::cons_ptr(ptr)), cons)
            .unwrap()
    }

    pub fn intern(self, ctx: &MutatorCtx, name: String) -> Root<'slot> {
        let sym = ctx.string_arena.lock().unwrap().intern(name);
        self.root_raw(PackedPtr::sym_ptr(sym))
    }

    pub fn t(self, ctx: &MutatorCtx) -> Root<'slot> {
        self.intern(ctx, "t".to_string())
    }

    pub fn nil(self) -> Root<'slot> {
        self.root_raw(PackedPtr::nil())
    }

    pub unsafe fn function(self, fn_ptr: BuiltinFunction) -> Root<'slot> {
        self.root_raw(PackedPtr::fun_ptr(fn_ptr))
    }

    pub fn singleton<'guard>(self, ctx: &MutatorCtx, ptr: &PackedValue<'guard>) -> Root<'slot> {
        self.alloc_cons(
            ctx,
            Cons {
                first: ptr.clone(),
                rest: Value::Nil.pack(),
            },
        )
    }
}

impl<'slot> Root<'slot> {
    pub unsafe fn packed(&self) -> PackedPtr {
        self.slot.0.ptr.get()
    }

    pub fn value<'root: 'guard, 'guard>(&'root self) -> PackedValue<'guard> {
        unsafe { PackedValue::new(self.packed()) }
    }

    pub fn slot(self) -> Slot<'slot> {
        self.slot
    }

    pub fn map<F: FnOnce(Slot<'slot>, &Root) -> Root<'slot>>(self, f: F) -> Root<'slot> {
        let tmp = unsafe { RootNode::new() };
        let tmp = unsafe { Slot::new_out_of_list(Pin::new_unchecked(&tmp)) };
        self.slot.0.list.insert_after(tmp.0); // add to list without ctx
        let tmp = tmp.root(&self.value());
        let slot = self.slot();
        f(slot, &tmp)
    }

    pub fn singleton(self, ctx: &MutatorCtx) -> Root<'slot> {
        self.map(|slot, root| {
            slot.alloc_cons(
                ctx,
                Cons {
                    first: root.value(),
                    rest: Value::Nil.pack(),
                },
            )
        })
    }

    pub fn prepend(self, ctx: &MutatorCtx, val: &PackedValue) -> Root<'slot> {
        self.map(|slot, root| {
            slot.alloc_cons(
                ctx,
                Cons {
                    first: val.clone(),
                    rest: root.value(),
                },
            )
        })
    }

    pub fn prepend_obj(self, ctx: &MutatorCtx, val: &PackedValue) -> Root<'slot> {
        self.map(|slot, root| {
            slot.alloc_obj(
                ctx,
                Cons {
                    first: val.clone(),
                    rest: root.value(),
                },
            )
        })
    }

    pub fn quote(self, ctx: &MutatorCtx) -> Root<'slot> {
        self.singleton(ctx).prepend(ctx, &ctx.common_symbols.quote)
    }

    pub fn fexpr(self, ctx: &MutatorCtx) -> Root<'slot> {
        self.prepend_obj(ctx, &ctx.common_symbols.fexpr)
    }

    pub fn _macro(self, ctx: &MutatorCtx) -> Root<'slot> {
        self.prepend_obj(ctx, &ctx.common_symbols._macro)
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct Gc<'guard, T> {
    ptr: &'guard T,
}

impl<'guard, T> Gc<'guard, T> {
    pub unsafe fn new(ptr: &'guard T) -> Self {
        Gc { ptr }
    }

    pub unsafe fn as_raw(&self) -> NonNull<T> {
        NonNull::from(self.ptr)
    }
}

impl<'guard, T> Deref for Gc<'guard, T> {
    type Target = T;

    fn deref(&self) -> &'guard Self::Target {
        self.ptr
    }
}

// impl<'guard, T> Display for Gc<'guard, T> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "Gc({:?})", self.ptr as *const T)
//     }
// }

pub struct RootList(Pin<Box<LinkedList<RootNode>>>);

impl RootList {
    pub fn new() -> Self {
        Self(Box::pin(LinkedList::default()))
    }

    pub fn add_root(&self, root: Pin<&RootNode>) {
        LinkedList::insert_after(&self.0, root);
    }

    pub fn cursor<'a>(&'a self) -> LinkedListIter<'a, RootNode> {
        self.0.cursor()
    }
}

pub struct RootNode {
    ptr: Cell<PackedPtr>,
    list: LinkedList<RootNode>,
}

impl RootNode {
    pub unsafe fn new() -> Self {
        RootNode {
            ptr: Cell::new(PackedPtr::nil()),
            list: Default::default(),
        }
    }
}

impl RootNode {
    pub fn ptr(&self) -> PackedPtr {
        self.ptr.get()
    }
}

impl LinkedListNode for RootNode {
    fn pointers(&self) -> &LinkedList<Self> {
        &self.list
    }
}

mod test {
    use std::ptr::NonNull;

    use crate::{heap::LAlloc, let_slot, object::PackedPtr, root::Root};

    #[test]
    fn test() {
        let global = Box::leak(Box::new(crate::thread::GlobalState::new()));
        let ctx = crate::thread::MutatorCtx::new_from_global(global);

        let_slot!(ctx: slot);
        let root = slot.root_raw(PackedPtr::integer(2));

        let slot = root.slot();
    }
}
