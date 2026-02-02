//! Contains methods for cosets.
use std::vec;

use crate::group::{Dim, group_item::GroupItem};

/// An iterator over the elements of a coset.
///
/// # Safety
/// By creating a value of this type, you're asserting that this is a subset of a group 
/// that is a subgroup multiplied by an element of the group.
#[derive(Clone)]
pub struct Coset<I: Iterator>
where
    I::Item: GroupItem,
{
    /// The "dimension" of the iterator's type, used to distinguish between
    /// matrices of different sizes, permutations of different sizes, etc.
    dim: Dim<I>,

    /// The underlying iterator which actually outputs the groups.
    iter: I,
}


impl<I: Iterator> Iterator for Coset<I>
where
    I::Item: GroupItem,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<I: Iterator> Coset<I>
where
    I::Item: GroupItem,
{
    /// An iterator over the elements of a group.
    ///
    /// # Safety
    /// todo: make sure that it is actually a coset
    pub unsafe fn new(dim: Dim<I>, iter: I) -> Self {
        Self { dim, iter }
    }

    
    /// Gets all elements of `self` and stores them in a cache, so that they can
    /// be quickly iterated over again.
    pub fn cache(self) -> Coset<vec::IntoIter<I::Item>> {
        // Safety: cacheing a group does not change any of its algebraic properties.
        unsafe { Coset::new(self.dim, self.collect::<Vec<_>>().into_iter()) }
    }
}