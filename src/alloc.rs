use crate::heap::LAlloc;
use crate::object::OBJECT_ALIGNMENT;
use crate::root::RootList;
use crate::root::RootNode;
use crate::sorted_vec::SortedVec;
use std::alloc;
use std::collections::HashSet;
use std::mem::size_of;
use std::mem::take;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::TryLockError;

const IMMIX_LINES: usize = 254;
const IMMIX_LINE_SIZE: usize = 128;
const IMMIX_USABLE_SIZE: usize = IMMIX_LINES * IMMIX_LINE_SIZE;
const IMMIX_META_SIZE: usize = 256;
const IMMIX_BLOCK_SIZE: usize = IMMIX_USABLE_SIZE + IMMIX_META_SIZE;
const IMMIX_MIN_STARTING_SIZE: usize = 8;

const IMMIX_BLOCK_ALIGNMENT: usize = IMMIX_BLOCK_SIZE;

const MEDIUM_OBJECT_SIZE: usize = IMMIX_LINE_SIZE;

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
struct Block {
    ptr: *mut u8,
}

#[derive(Debug, PartialEq)]
pub enum AllocError {
    InvalidInput,
    OutOfMemory,
    GcTryAgain,
}

impl Block {
    fn new() -> Result<Self, AllocError> {
        unsafe {
            let layout =
                alloc::Layout::from_size_align_unchecked(IMMIX_BLOCK_SIZE, IMMIX_BLOCK_ALIGNMENT);

            let ptr = alloc::alloc(layout);
            if ptr.is_null() {
                Err(AllocError::OutOfMemory)
            } else {
                Ok(Block { ptr })
            }
        }
    }

    /// Only to be used on blocks created with new()
    unsafe fn deallocate(&mut self) {
        unsafe {
            let layout =
                alloc::Layout::from_size_align_unchecked(IMMIX_BLOCK_SIZE, IMMIX_BLOCK_ALIGNMENT);

            alloc::dealloc(self.ptr, layout)
        }
    }

    /// Do not deallocate blocks generated in this way, only use if lock is secured
    unsafe fn block_from_ptr(obj: *mut u8) -> (Self, usize, usize) {
        let ptr = (obj as usize) / IMMIX_BLOCK_ALIGNMENT * IMMIX_BLOCK_ALIGNMENT;
        let block = Block {
            ptr: ptr as *mut u8,
        };
        let offset_in_block = obj as usize - ptr;
        let line = offset_in_block / IMMIX_LINE_SIZE * IMMIX_LINE_SIZE;

        let offset_in_line = offset_in_block - IMMIX_LINE_SIZE * line;
        (block, line, offset_in_line)
    }

    fn line_live(&self, i: usize) -> bool {
        assert!(i < IMMIX_LINES);
        unsafe { self.unchecked_line_live(i) }
    }

    fn set_line_live(&mut self, i: usize, live: bool) {
        assert!(i < IMMIX_LINES);
        unsafe { self.unchecked_set_line_live(i, live) }
    }

    unsafe fn unchecked_line_live(&self, i: usize) -> bool {
        *self.ptr.add(i) != 0
    }

    unsafe fn unchecked_set_line_live(&mut self, i: usize, live: bool) {
        *self.ptr.add(i) = if live { 1 } else { 0 };
    }

    fn block_live(&self) -> bool {
        unsafe { *self.ptr.add(IMMIX_LINES + 1) != 0 }
    }

    fn set_block_live(&mut self, live: bool) {
        unsafe { *self.ptr.add(IMMIX_LINES + 1) = if live { 1 } else { 0 } };
    }

    fn reset_marks(&mut self) {
        for i in 0..IMMIX_LINES {
            unsafe { self.unchecked_set_line_live(i, false) };
        }
    }
}

#[derive(Debug, Clone)]
struct BumpPointer {
    cursor: NonNull<u8>,
    limit: NonNull<u8>,
}

impl BumpPointer {
    unsafe fn unchecked_bump<T>(&mut self, size: usize) -> NonNull<T> {
        let ptr = NonNull::new_unchecked(self.cursor.as_ptr() as *mut T);
        self.cursor = NonNull::new_unchecked(self.cursor.as_ptr().add(size));
        ptr
    }

    fn bump<T>(&mut self, size: usize) -> Option<NonNull<T>> {
        if self.free_size() >= size {
            unsafe { Some(self.unchecked_bump(size)) }
        } else {
            None
        }
    }

    fn free_size(&self) -> usize {
        self.limit.as_ptr() as usize - self.cursor.as_ptr() as usize
    }
}

#[derive(Debug, Clone)]
struct ImmixBlockHandler {
    block: Block,
    bump: BumpPointer,
}
unsafe impl Send for ImmixBlockHandler {}

impl ImmixBlockHandler {
    fn new() -> Result<Self, AllocError> {
        let res = Block::new();
        if let Ok(block) = res {
            Ok(unsafe { Self::from_block_and_lines(block, 0, IMMIX_LINES) })
        } else {
            Err(res.unwrap_err())
        }
    }

    unsafe fn from_block_and_lines(block: Block, start: usize, limit: usize) -> Self {
        let bump = BumpPointer {
            cursor: NonNull::new_unchecked(block.ptr.add(start * IMMIX_LINE_SIZE)),
            limit: NonNull::new_unchecked(block.ptr.add(limit * IMMIX_LINE_SIZE)),
        };
        ImmixBlockHandler { block, bump }
    }

    fn from_block(block: Block, min_size: usize, start_line: usize) -> Option<Self> {
        if !block.block_live() {
            return Some(unsafe { Self::from_block_and_lines(block, 0, IMMIX_LINES) });
        }

        let mut start = None;
        for i in start_line..IMMIX_LINES {
            if block.line_live(i) {
                match start {
                    Some(j) => {
                        if (i - j) * IMMIX_LINE_SIZE > min_size {
                            return Some(unsafe { Self::from_block_and_lines(block, j, i) });
                        }

                        start = None;
                    }
                    None => (),
                }
            } else {
                match start {
                    Some(_) => (),
                    None => start = Some(i),
                }
            }
        }
        None
    }

    fn mark_medium_object_on_alloc(&mut self, size: usize) {
        let (b, l, x) = unsafe { Block::block_from_ptr(self.bump.cursor.as_ptr()) };
        assert_eq!(b, self.block);

        let num_lines = (x + size - 1) / IMMIX_LINE_SIZE + 1;

        for i in 0..num_lines {
            self.block.set_line_live(l + i, true);
        }
    }
}

pub struct GlobalImmixAllocator {
    blocks: Mutex<Vec<Block>>, // TODO: sort by free size?
    local_lists: Mutex<Vec<Arc<Mutex<ImmixMutatorState>>>>,
}

impl GlobalImmixAllocator {
    pub fn new() -> Self {
        GlobalImmixAllocator {
            blocks: Mutex::new(Vec::new()),
            local_lists: Mutex::new(Vec::new()),
        }
    }

    fn add_local_list(&mut self, list: Arc<Mutex<ImmixMutatorState>>) {
        self.local_lists.lock().unwrap().push(list)
    }

    fn remove_local_list(&mut self, list: &Arc<Mutex<ImmixMutatorState>>) -> bool {
        let mut locals = self.local_lists.lock().unwrap();
        if let Some(i) = locals.iter().position(|e| Arc::ptr_eq(e, list)) {
            locals.remove(i);
            true
        } else {
            false
        }
    }

    fn return_blocks(&mut self, mut list: Vec<Block>) {
        self.blocks.lock().unwrap().append(&mut list)
    }

    fn request_block(
        &mut self,
        size: usize,
        gc_on_fail: bool,
    ) -> Result<ImmixBlockHandler, AllocError> {
        let mut blocks = self.blocks.lock().unwrap();

        let mut item = None;
        for (i, b) in blocks.iter().enumerate() {
            if let Some(bh) = ImmixBlockHandler::from_block(b.clone(), size, 0) {
                item = Some((i, bh));
                break;
            }
        }

        if let Some((i, bh)) = item {
            blocks.remove(i);
            Ok(bh)
        } else if gc_on_fail {
            drop(blocks);
            self.gc();
            Err(AllocError::GcTryAgain)
        } else {
            ImmixBlockHandler::new()
        }
    }

    // fn release(&mut self, id: usize) {
    //     for arc in &self.blocks {
    //         // let arc = Arc::clone(arc);
    //         if let Ok(mut lock) = arc.try_lock() {
    //             if lock.owner == Some(id) {
    //                 lock.owner = None;
    //             }
    //         }
    //     }
    // }

    unsafe fn lock_all_lists<'a>(
        locals: &'a MutexGuard<'a, Vec<Arc<Mutex<ImmixMutatorState>>>>,
    ) -> Vec<MutexGuard<'a, ImmixMutatorState>> {
        let mut guards = Vec::with_capacity(locals.len());

        let mut i: usize = 0;
        loop {
            if guards.len() == locals.len() {
                return guards;
            } else if guards.is_empty() {
                guards.push(locals[i].lock().unwrap());
            } else {
                let result = locals[i].try_lock();
                if let Ok(guard) = result {
                    guards.push(guard);
                } else if let Err(TryLockError::Poisoned(p)) = result {
                    panic!("Poisoned mutex: {}", p);
                } else {
                    guards.clear();
                    continue;
                }
            }
            i = (i + 1) % locals.len();
        }
    }

    unsafe fn mark_ptr(ptr: *mut u8, size: usize) {
        let (mut start_block, start_line, _) = unsafe { Block::block_from_ptr(ptr) };
        let (end_block, end_line, _) = unsafe { Block::block_from_ptr(ptr.add(size - 1)) };

        assert!(start_block == end_block);
        assert!(end_line < IMMIX_LINES);
        for line in start_line..end_line {
            unsafe {
                start_block.unchecked_set_line_live(line, true);
            }
            start_block.set_block_live(true);
        }
    }

    fn gc(&mut self) {
        let mut global_blocks = self.blocks.lock().unwrap();
        let locals = self.local_lists.lock().unwrap();
        let mut multilock = unsafe { Self::lock_all_lists(&locals) };

        let mut stack = vec![];
        let mut seen = HashSet::new();

        for b in global_blocks.iter_mut() {
            b.reset_marks();
        }

        for l in multilock.iter_mut() {
            for b in unsafe { l.blocks.base_mut().iter_mut() } {
                b.reset_marks();
            }

            l.head.block.set_block_live(true);

            for r in l.roots.cursor() {
                if seen.insert(r.ptr()) {
                    stack.push(r.ptr());
                }
            }
        }

        while !stack.is_empty() {
            let obj = stack.remove(0);

            for (ptr, size) in obj.heap_ptrs() {
                unsafe { Self::mark_ptr(ptr.as_ptr() as *mut u8, size) };
            }

            for ptr in obj.obj_ptrs() {
                let inner_obj = unsafe { *ptr.as_ptr() };
                if seen.insert(inner_obj) {
                    stack.push(inner_obj);
                }
            }
        }

        let mut dead_blocks = vec![];
        for l in multilock.iter_mut() {
            unsafe { l.blocks.base_mut() }.retain(|b| {
                if b.block_live() {
                    true
                } else {
                    dead_blocks.push(b.clone());
                    false
                }
            })
        }

        drop(multilock);
        drop(global_blocks);
        drop(locals);

        self.return_blocks(dead_blocks);
    }
}

struct ImmixMutatorState {
    head: ImmixBlockHandler,
    blocks: SortedVec<Block>,
    roots: RootList,
}

impl ImmixMutatorState {
    fn find_hole(&mut self, size: usize) -> Option<ImmixBlockHandler> {
        let (block, line, offset) =
            unsafe { Block::block_from_ptr(self.head.bump.cursor.as_ptr().offset(-1)) };
        assert_eq!(block, self.head.block);

        if let Some(bh) = ImmixBlockHandler::from_block(self.head.block.clone(), size, line + 1) {
            return Some(bh);
        }
        // right point to avoid repeating head and moving backwards
        let idx = self.blocks.right_point(&self.head.block);
        for b in self.blocks.base().iter().skip(idx) {
            if let Some(bh) = ImmixBlockHandler::from_block(b.clone(), size, 0) {
                return Some(bh);
            }
        }
        None
    }

    fn set_head(&mut self, bh: ImmixBlockHandler) {
        self.head = bh;
    }

    fn try_allocate_local<T>(&mut self, size: usize) -> Option<NonNull<T>> {
        if let Some(mut bh) = self.find_hole(size) {
            if size < MEDIUM_OBJECT_SIZE {
                self.set_head(bh);
            } else {
                bh.mark_medium_object_on_alloc(size);
            }
            unsafe { Some(self.head.bump.unchecked_bump(size)) }
        } else {
            None
        }
    }
}

pub struct ImmixMutator<'a> {
    global: &'a Mutex<GlobalImmixAllocator>,
    local_state: Arc<Mutex<ImmixMutatorState>>,
}

impl<'a> ImmixMutator<'a> {
    pub fn new(global: &'a Mutex<GlobalImmixAllocator>) -> Self {
        let mut lock = global.lock().unwrap();
        let local_state = Arc::new(Mutex::new(ImmixMutatorState {
            head: lock.request_block(IMMIX_MIN_STARTING_SIZE, false).unwrap(),
            blocks: SortedVec::new(),
            roots: RootList::new(),
        }));
        lock.add_local_list(local_state.clone());
        ImmixMutator {
            global,
            local_state,
        }
    }

    pub fn add_root(&self, root: Pin<&RootNode>) {
        self.local_state.lock().unwrap().roots.add_root(root);
    }
}

impl<'a> LAlloc for ImmixMutator<'a> {
    fn alloc<T, R, F: FnOnce(NonNull<T>) -> R>(&self, transformer: F) -> Result<R, AllocError> {
        let size = ((size_of::<T>() + OBJECT_ALIGNMENT - 1) / OBJECT_ALIGNMENT) * OBJECT_ALIGNMENT;
        if size > IMMIX_USABLE_SIZE {
            return Err(AllocError::InvalidInput);
        }

        let mut list = self.local_state.lock().unwrap();
        if list.head.bump.free_size() >= size {
            unsafe { Ok(transformer(list.head.bump.unchecked_bump(size))) }
        } else if let Some(ptr) = list.try_allocate_local(size) {
            Ok(transformer(ptr))
        } else {
            drop(list);
            let mut global = self.global.lock().unwrap();
            let mut res = global.request_block(size, true);
            if let Err(AllocError::GcTryAgain) = res {
                if let Some(ptr) = self.local_state.lock().unwrap().try_allocate_local(size) {
                    return Ok(transformer(ptr));
                }
                res = global.request_block(size, false);
            }
            if let Ok(block_handler) = res {
                let mut list = self.local_state.lock().unwrap();
                let old_head = list.head.block.clone();
                list.blocks.insert(old_head);
                list.head = block_handler;
                unsafe { Ok(transformer(list.head.bump.unchecked_bump(size))) }
            } else {
                Err(res.unwrap_err())
            }
        }
    }
}

impl<'a> Drop for ImmixMutator<'a> {
    fn drop(&mut self) {
        let mut global = self.global.lock().unwrap();
        let mut list = self.local_state.lock().unwrap();
        global.remove_local_list(&self.local_state);
        global.return_blocks(take(unsafe { &mut list.blocks.base_mut() }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heap::id;
    use crate::heap::LAlloc;

    #[test]
    fn test_block() {
        let mut block = ImmixBlockHandler::new().unwrap();
        for _ in 0..IMMIX_USABLE_SIZE {
            let _: NonNull<u8> = block.bump.bump(1).unwrap();
        }
        assert_eq!(block.bump.cursor, block.bump.limit);
    }

    #[test]
    fn test_alloc() {
        let global = Mutex::new(GlobalImmixAllocator::new());
        let allocator = ImmixMutator::new(&global);

        for _ in 0..IMMIX_USABLE_SIZE / OBJECT_ALIGNMENT {
            let _: NonNull<u8> = allocator.alloc(id).unwrap();
        }

        let n = allocator.local_state.lock().unwrap().blocks.base().len();
        assert_eq!(n, 0);

        let _: NonNull<u8> = allocator.alloc(id).unwrap();

        let n = allocator.local_state.lock().unwrap().blocks.base().len();
        assert_eq!(n, 1);
    }

    #[test]
    fn test_round_robin() {
        let state = Mutex::new(GlobalImmixAllocator::new());
        let allocators: Vec<ImmixMutator> = (0..4).map(|_| ImmixMutator::new(&state)).collect();

        for i in 0..IMMIX_USABLE_SIZE / OBJECT_ALIGNMENT * 4 {
            let _: NonNull<u8> = allocators[i % 4].alloc(id).unwrap();
        }

        let n: usize = allocators
            .iter()
            .map(|a| a.local_state.lock().unwrap().blocks.base().len())
            .sum();
        assert_eq!(n, 0);

        let _: NonNull<u8> = allocators[0].alloc(id).unwrap();

        let n: usize = allocators
            .iter()
            .map(|a| a.local_state.lock().unwrap().blocks.base().len())
            .sum();
        assert_eq!(n, 1);
    }
}