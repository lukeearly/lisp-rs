use std::{
    cell::Cell,
    marker::{PhantomData, PhantomPinned},
    pin::Pin,
    ptr::NonNull,
};

#[derive(Clone)]
pub struct LinkedList<T: LinkedListNode + ?Sized> {
    prev: Cell<Option<NonNull<LinkedList<T>>>>,
    next: Cell<Option<NonNull<T>>>,
    _phantom: PhantomPinned,
}

impl<T: LinkedListNode + ?Sized> Default for LinkedList<T> {
    fn default() -> Self {
        Self {
            prev: Cell::new(None),
            next: Cell::new(None),
            _phantom: PhantomPinned,
        }
    }
}

pub trait LinkedListNode {
    fn pointers(&self) -> &LinkedList<Self>;
}

impl<T: LinkedListNode + ?Sized> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let prev = self.prev.get();
        let next = self.next.get();
        if let Some(prev) = prev {
            unsafe { prev.as_ref() }.next.set(next);
        }
        if let Some(next) = next {
            unsafe { next.as_ref() }.pointers().prev.set(prev);
        }
    }
}

pub struct LinkedListIter<'a, T: LinkedListNode + ?Sized> {
    next: Option<NonNull<T>>,
    _phantom: PhantomData<&'a T>,
}

impl<T: LinkedListNode + ?Sized> LinkedList<T> {
    pub fn cursor<'a>(&'a self) -> LinkedListIter<'a, T> {
        LinkedListIter {
            next: self.next.get(),
            _phantom: PhantomData,
        }
    }

    pub fn insert_after(&self, node: Pin<&T>) {
        node.pointers().prev.set(Some(NonNull::from(&*self)));

        node.pointers().next.set(self.next.get());

        if let Some(next) = self.next.get() {
            unsafe { next.as_ref() }
                .pointers()
                .prev
                .set(Some(NonNull::from(&*node.pointers())));
        }

        self.next.set(Some(NonNull::from(&*node)));
    }
}

impl<'a, T: LinkedListNode + ?Sized> Iterator for LinkedListIter<'a, T> {
    type Item = Pin<&'a T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.next {
            self.next = unsafe { next.as_ref() }.pointers().next.get();
            unsafe { Some(Pin::new_unchecked(&*next.as_ptr())) }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct Node {
        list: LinkedList<Node>,
        data: u8,
    }

    impl LinkedListNode for Node {
        fn pointers(&self) -> &LinkedList<Self> {
            &self.list
        }
    }

    #[test]
    fn one_node() {
        let mut node = Node {
            list: Default::default(),
            data: 10,
        };
        let mut node = unsafe { Pin::new_unchecked(&mut node) };

        assert_eq!(node.data, 10);
    }
}
