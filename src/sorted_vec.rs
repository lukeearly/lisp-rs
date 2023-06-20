use std::ops::Range;

pub struct SortedVec<T: Ord> {
    base: Vec<T>,
}

impl<T: Ord> SortedVec<T> {
    pub fn new() -> Self {
        Self { base: Vec::new() }
    }

    pub fn with_capacity(size: usize) -> Self {
        Self {
            base: Vec::with_capacity(size),
        }
    }

    pub fn from_vec(mut base: Vec<T>) -> Self {
        base.sort_unstable();
        Self { base }
    }

    pub unsafe fn from_sorted_vec(base: Vec<T>) -> Self {
        Self { base }
    }

    pub fn insert(&mut self, e: T) {
        let idx = self.base.partition_point(|x| x < &e);
        self.base.insert(idx, e);
    }

    pub fn left_point(&mut self, e: &T) -> usize {
        self.base.partition_point(|x| x < e)
    }

    pub fn right_point(&mut self, e: &T) -> usize {
        self.base.partition_point(|x| x <= e)
    }

    pub fn range(&mut self, e: &T) -> Range<usize> {
        self.left_point(e)..self.right_point(e)
    }

    pub fn binary_search(&self, e: &T) -> Result<usize, usize> {
        self.base.binary_search(e)
    }

    pub fn base(&self) -> &Vec<T> {
        &self.base
    }

    pub unsafe fn base_mut(&mut self) -> &mut Vec<T> {
        &mut self.base
    }
}
