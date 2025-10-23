// smallset: a Rust crate for small unordered sets of elements, built on top of
// `smallvec`.
//
// Copyright (c) 2016 Chris Fallin <cfallin@c1f.net>. Released under the MIT license.
//

extern crate smallvec;

use std::{borrow::Borrow, collections::HashSet, fmt, hash::Hash, iter::FromIterator};

use smallvec::{Array, Drain, SmallVec};

/// A `SmolSet` is an unordered set of elements. It is designed to work best
/// for very small sets (no more than ten or so elements). In order to support
/// small sets very efficiently, it stores elements in a simple unordered array.
/// When the set is smaller than the size of the array `A`, all elements are
/// stored inline, without heap allocation. This is accomplished by using a
/// `smallvec::SmallVec`.
///
/// The insert, remove, and query methods on `SmolSet` have `O(n)` time
/// complexity in the current set size: they perform a linear scan to determine
/// if the element in question is present. This is inefficient for large sets,
/// but fast and cache-friendly for small sets.
///
/// Example usage:
///
/// ```
/// use smolset::SmolSet;
///
/// // `s` and its elements will be completely stack-allocated in this example.
/// let mut s: SmolSet<[u32; 4]> = SmolSet::new();
/// s.insert(1);
/// s.insert(2);
/// s.insert(3);
/// assert_eq!(s.len(), 3);
/// assert!(s.contains(&1));
/// ```
///
/// TODO: Add the ability to switch modes explicitly.
///
pub struct SmolSet<A: Array>
where A::Item: PartialEq + Eq
{
    inner: InnerSmolSet<A>,
}

impl<A: Array> Default for SmolSet<A>
where A::Item: PartialEq + Eq + Hash
{
    fn default() -> Self { SmolSet::new() }
}

/// Internal (and true) representation of the `SmolSet`.
/// Created so that user are not aware of the sum type.
pub enum InnerSmolSet<A: Array>
where A::Item: PartialEq + Eq
{
    Stack(SmallVec<A>),
    Heap(HashSet<A::Item>),
}

impl<A: Array> Default for InnerSmolSet<A>
where A::Item: PartialEq + Eq
{
    fn default() -> Self { InnerSmolSet::Stack(SmallVec::new()) }
}

impl<A: Array> Clone for InnerSmolSet<A>
where A::Item: PartialEq + Eq + Clone
{
    fn clone(&self) -> Self {
        match &self {
            InnerSmolSet::Stack(elements) => InnerSmolSet::Stack(elements.clone()),
            InnerSmolSet::Heap(elements) => InnerSmolSet::Heap(elements.clone()),
        }
    }
}

impl<A: Array> PartialEq for SmolSet<A>
where A::Item: Eq + PartialEq + Hash
{
    fn eq(&self, other: &Self) -> bool {
        fn set_same<A: Array>(stack: &SmallVec<A>, heap: &HashSet<A::Item>) -> bool
        where A::Item: Eq + PartialEq {
            stack.len() == heap.len() && heap.iter().all(|x| stack.contains(x))
        }

        match (&self.inner, &other.inner) {
            (InnerSmolSet::Stack(lhs), InnerSmolSet::Stack(rhs)) => lhs.eq(rhs),
            (InnerSmolSet::Heap(lhs), InnerSmolSet::Heap(rhs)) => lhs.eq(rhs),
            (InnerSmolSet::Stack(stack), InnerSmolSet::Heap(heap)) => set_same(stack, heap),
            (InnerSmolSet::Heap(heap), InnerSmolSet::Stack(stack)) => set_same(stack, heap),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum SetMode {
    Stack,
    Heap,
}

impl<A: Array> SmolSet<A>
where A::Item: PartialEq + Eq + Hash
{
    /// Creates a new, empty `SmolSet`.
    pub fn new() -> SmolSet<A> {
        SmolSet {
            inner: InnerSmolSet::Stack(SmallVec::new()),
        }
    }

    pub fn mode(&self) -> SetMode {
        match self.inner {
            InnerSmolSet::Stack(_) => SetMode::Stack,
            InnerSmolSet::Heap(_) => SetMode::Heap,
        }
    }

    /// Returns the number of elements in this set.
    pub fn is_empty(&self) -> bool { self.len() == 0 }

    /// Inserts `elem` into the set if not yet present. Returns `true` if the
    /// set did not have this element present, or `false` if it already had this
    /// element present.
    pub fn insert(&mut self, elem: A::Item) -> bool {
        match &mut self.inner {
            InnerSmolSet::Stack(elements) => {
                if elements.contains(&elem) {
                    false
                } else {
                    if elements.len() < A::size() {
                        elements.push(elem);
                    } else {
                        let ee =
                            HashSet::from_iter(elements.drain(..).chain(std::iter::once(elem)));
                        self.inner = InnerSmolSet::Heap(ee);
                    }
                    true
                }
            }
            InnerSmolSet::Heap(elements) => elements.insert(elem),
        }
    }

    /// Removes `elem` from the set. Returns `true` if the element was removed,
    /// or `false` if it was not found.
    pub fn remove<Q>(&mut self, elem: &Q) -> bool
    where
        A::Item: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        match &mut self.inner {
            InnerSmolSet::Stack(elements) => {
                if let Some(pos) = elements.iter().position(|x| elem.eq(x.borrow())) {
                    elements.swap_remove(pos);
                    true
                } else {
                    false
                }
            }
            InnerSmolSet::Heap(elements) => elements.remove(elem),
        }
    }

    /// Tests whether `elem` is present. Returns `true` if it is present, or
    /// `false` if not.
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        A::Item: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        match &self.inner {
            InnerSmolSet::Stack(elements) => elements.iter().any(|x| value.eq(x.borrow())),
            InnerSmolSet::Heap(elements) => elements.contains(value),
        }
    }

    /// Returns an iterator over the set elements. Elements will be returned in
    /// an arbitrary (unsorted) order.
    pub fn iter(&self) -> SmolSetIter<A> {
        match &self.inner {
            InnerSmolSet::Stack(element) => {
                SmolSetIter {
                    inner: InnerSmolSetIter::Stack(element.iter()),
                }
            }
            InnerSmolSet::Heap(element) => {
                SmolSetIter {
                    inner: InnerSmolSetIter::Heap(element.iter()),
                }
            }
        }
    }

    /// Returns the current length of the set.
    pub fn len(&self) -> usize {
        match &self.inner {
            InnerSmolSet::Stack(elements) => elements.len(),
            InnerSmolSet::Heap(elements) => elements.len(),
        }
    }

    /// Clears the set.
    pub fn clear(&mut self) {
        match &mut self.inner {
            InnerSmolSet::Stack(elements) => elements.clear(),
            InnerSmolSet::Heap(_) => {
                self.inner = Default::default();
            }
        }
    }

    /// If the given `elem` exists in the set, returns the reference to the value inside the set.
    /// Where they are equal (in the case where the set is in stack mode) or they hash equally (if the set is in heap mode).
    pub fn get<Q>(&self, value: &Q) -> Option<&A::Item>
    where
        A::Item: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        match &self.inner {
            InnerSmolSet::Stack(elements) => elements.iter().find(|&x| value.eq(x.borrow())),
            InnerSmolSet::Heap(elements) => elements.get(value),
        }
    }

    /// If the given `elem` exists in the set, returns the value inside the set where they are either equal or hash equally.
    /// Then, remove that value from the set.
    pub fn take<Q>(&mut self, value: &Q) -> Option<A::Item>
    where
        A::Item: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        match &mut self.inner {
            InnerSmolSet::Stack(elements) => {
                elements
                    .iter()
                    .position(|x| value.eq(x.borrow()))
                    .map(|pos| elements.swap_remove(pos))
            }
            InnerSmolSet::Heap(elements) => elements.take(value),
        }
    }

    /// Adds a value to the set, replacing the existing value, if any, that is equal to the given one. Returns the replaced value.
    pub fn replace(&mut self, mut value: A::Item) -> Option<A::Item> {
        match &mut self.inner {
            InnerSmolSet::Stack(elements) => {
                for elem in elements.iter_mut() {
                    if *elem == value {
                        std::mem::swap(elem, &mut value);
                        return Some(value);
                    }
                }

                elements.push(value);
                None
            }
            InnerSmolSet::Heap(elements) => elements.replace(value),
        }
    }

    /// Empties the set and returns an iterator over it.
    pub fn drain(&mut self) -> SmallDrain<A> {
        match &mut self.inner {
            InnerSmolSet::Stack(vec) => SmallDrain::FromVec(vec.drain(..)),
            InnerSmolSet::Heap(set) => SmallDrain::FromSet(set.drain()),
        }
    }

    /// Removes all elements in the set that does not satisfy the given predicate `f`.
    pub fn retain<F>(&mut self, mut f: F)
    where F: FnMut(&<A as Array>::Item) -> bool {
        match &mut self.inner {
            InnerSmolSet::Stack(elements) => elements.retain(|i| f(i)),
            InnerSmolSet::Heap(elements) => elements.retain(f),
        }
    }

    /// Returns an iterator over the intersection of the 2 sets.
    pub fn intersection<'a>(&'a self, other: &'a Self) -> SmallIntersection<'a, A::Item> {
        match &self.inner {
            InnerSmolSet::Stack(elements) => {
                let result = elements
                    .iter()
                    .filter(|x| other.contains(x))
                    .collect::<Vec<&'a A::Item>>();
                SmallIntersection {
                    data:  result,
                    index: 0,
                }
            }

            InnerSmolSet::Heap(elements) => {
                let result = elements
                    .iter()
                    .filter(|x| other.contains(x))
                    .collect::<Vec<&'a A::Item>>();
                SmallIntersection {
                    data:  result,
                    index: 0,
                }
            }
        }
    }

    /// Returns an iterator over the union of the 2 sets.
    pub fn union<'a>(&'a self, other: &'a Self) -> SmallUnion<'a, A::Item> {
        match &self.inner {
            InnerSmolSet::Stack(elements) => {
                let mut lhs = elements.iter().collect::<Vec<&'a A::Item>>();
                let mut rhs = other
                    .iter()
                    .filter(|x| !lhs.contains(x))
                    .collect::<Vec<&'a A::Item>>();
                lhs.append(&mut rhs);
                SmallUnion {
                    data:  lhs,
                    index: 0,
                }
            }

            InnerSmolSet::Heap(elements) => {
                let mut lhs = elements.iter().collect::<Vec<&'a A::Item>>();
                let mut rhs = other
                    .iter()
                    .filter(|x| !lhs.contains(x))
                    .collect::<Vec<&'a A::Item>>();
                lhs.append(&mut rhs);
                SmallUnion {
                    data:  rhs,
                    index: 0,
                }
            }
        }
    }

    /// Returns an iterator over the difference of the 2 sets.
    pub fn difference<'a>(&'a self, other: &'a Self) -> SmallDifference<'a, A::Item> {
        match &self.inner {
            InnerSmolSet::Stack(elements) => {
                let lhs = elements
                    .iter()
                    .filter(|x| !other.contains(x))
                    .collect::<Vec<&'a A::Item>>();
                SmallDifference {
                    data:  lhs,
                    index: 0,
                }
            }

            InnerSmolSet::Heap(elements) => {
                let lhs = elements
                    .iter()
                    .filter(|x| !other.contains(x))
                    .collect::<Vec<&'a A::Item>>();
                SmallDifference {
                    data:  lhs,
                    index: 0,
                }
            }
        }
    }

    /// Returns an iterator over the symmetric difference of the 2 sets.
    pub fn symmetric_difference<'a>(
        &'a self,
        other: &'a Self,
    ) -> SmallSymmetricDifference<'a, A::Item> {
        match &self.inner {
            InnerSmolSet::Stack(elements) => {
                let mut lhs = elements
                    .iter()
                    .filter(|x| !other.contains(x))
                    .collect::<Vec<&'a A::Item>>();
                let mut rhs = other
                    .iter()
                    .filter(|x| !elements.contains(x))
                    .collect::<Vec<&'a A::Item>>();
                lhs.append(&mut rhs);
                SmallSymmetricDifference {
                    data:  lhs,
                    index: 0,
                }
            }

            InnerSmolSet::Heap(elements) => {
                let mut lhs = elements
                    .iter()
                    .filter(|x| other.contains(x))
                    .collect::<Vec<&'a A::Item>>();
                let mut rhs = other
                    .iter()
                    .filter(|x| elements.contains(x))
                    .collect::<Vec<&'a A::Item>>();
                lhs.append(&mut rhs);
                SmallSymmetricDifference {
                    data:  lhs,
                    index: 0,
                }
            }
        }
    }
}

/// Implement into iterator for the SmolSet
impl<A> IntoIterator for SmolSet<A>
where
    A: Array,
    A::Item: PartialEq + Eq + Hash,
{
    type Item = A::Item;
    type IntoIter = SmolSetIntoIter<A>;

    fn into_iter(self) -> Self::IntoIter {
        match self.inner {
            InnerSmolSet::Stack(element) => {
                SmolSetIntoIter {
                    inner: InnerSmolSetIntoIter::Stack(element.into_iter()),
                }
            }
            InnerSmolSet::Heap(element) => {
                SmolSetIntoIter {
                    inner: InnerSmolSetIntoIter::Heap(element.into_iter()),
                }
            }
        }
    }
}

/// Implement into iterator for borrowed SmolSet
impl<'a, A> IntoIterator for &'a SmolSet<A>
where
    A: Array,
    A::Item: PartialEq + Eq + Hash,
{
    type Item = &'a A::Item;
    type IntoIter = SmolSetIter<'a, A>;

    fn into_iter(self) -> Self::IntoIter { self.iter() }
}

/// Iterator returned upon calling `drain`.
pub enum SmallDrain<'a, T: Array> {
    FromVec(Drain<'a, T>),
    FromSet(std::collections::hash_set::Drain<'a, T::Item>),
}

impl<'a, T: Array> Iterator for SmallDrain<'a, T> {
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SmallDrain::FromVec(drain) => drain.next(),
            SmallDrain::FromSet(drain) => drain.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            SmallDrain::FromVec(drain) => drain.size_hint(),
            SmallDrain::FromSet(drain) => drain.size_hint(),
        }
    }
}

/// Iterator returned upon calling `intersection`.
pub struct SmallIntersection<'a, T> {
    data:  Vec<&'a T>,
    index: usize,
}

impl<'a, T> Iterator for SmallIntersection<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.data.len() {
            None
        } else {
            let ptr = self.data.as_ptr();
            self.index += 1;
            unsafe { Some(std::ptr::read(ptr.add(self.index - 1))) }
        }
    }
}

/// Iterator returned upon calling `union`.
pub struct SmallUnion<'a, T> {
    data:  Vec<&'a T>,
    index: usize,
}

impl<'a, T> Iterator for SmallUnion<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.data.len() {
            None
        } else {
            let ptr = self.data.as_ptr();
            self.index += 1;
            unsafe { Some(std::ptr::read(ptr.add(self.index - 1))) }
        }
    }
}

/// Iterator returned upon calling `difference`.
pub struct SmallDifference<'a, T> {
    data:  Vec<&'a T>,
    index: usize,
}

impl<'a, T> Iterator for SmallDifference<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.data.len() {
            None
        } else {
            let ptr = self.data.as_ptr();
            self.index += 1;
            unsafe { Some(std::ptr::read(ptr.add(self.index - 1))) }
        }
    }
}

/// Iterator returned upon calling `symmteric_difference`.
pub struct SmallSymmetricDifference<'a, T> {
    data:  Vec<&'a T>,
    index: usize,
}

impl<'a, T> Iterator for SmallSymmetricDifference<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.data.len() {
            None
        } else {
            let ptr = self.data.as_ptr();
            self.index += 1;
            unsafe { Some(std::ptr::read(ptr.add(self.index - 1))) }
        }
    }
}

impl<A: Array> Clone for SmolSet<A>
where A::Item: PartialEq + Eq + Clone
{
    fn clone(&self) -> SmolSet<A> {
        SmolSet {
            inner: self.inner.clone(),
        }
    }
}

impl<A: Array> fmt::Debug for SmolSet<A>
where A::Item: PartialEq + Eq + fmt::Debug
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.inner {
            InnerSmolSet::Stack(elements) => write!(f, "{:?}", elements.as_slice()),
            InnerSmolSet::Heap(elements) => write!(f, "{elements:?}"),
        }
    }
}

impl<A: Array> FromIterator<A::Item> for SmolSet<A>
where A::Item: PartialEq + Eq + Hash
{
    fn from_iter<T>(iter: T) -> Self
    where T: IntoIterator<Item = A::Item> {
        iter.into_iter().fold(SmolSet::new(), |mut acc, x| {
            acc.insert(x);
            acc
        })
    }
}

/// Into Iterator of the set returned upon calling `into_iter`.
/// This is required to be an abstraction over the enum.
pub struct SmolSetIntoIter<A>
where
    A: Array,
    A::Item: PartialEq + Eq + Hash,
{
    inner: InnerSmolSetIntoIter<A>,
}

/// Iterator of the set returned upon calling `iter`.
/// This is required to be an abstraction over the enum.
pub struct SmolSetIter<'a, A: Array>
where A::Item: PartialEq + Eq + Hash + 'a
{
    inner: InnerSmolSetIter<'a, A>,
}

pub enum InnerSmolSetIntoIter<A>
where
    A: Array,
    A::Item: PartialEq + Eq + Hash,
{
    Stack(smallvec::IntoIter<A>),
    Heap(std::collections::hash_set::IntoIter<A::Item>),
}

pub enum InnerSmolSetIter<'a, A: Array>
where A::Item: PartialEq + Eq + Hash + 'a
{
    Stack(std::slice::Iter<'a, A::Item>),
    Heap(std::collections::hash_set::Iter<'a, A::Item>),
}

impl<A> Iterator for SmolSetIntoIter<A>
where
    A: Array,
    A::Item: PartialEq + Eq + Hash,
{
    type Item = A::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            InnerSmolSetIntoIter::Stack(vec) => vec.next(),
            InnerSmolSetIntoIter::Heap(hashset) => hashset.next(),
        }
    }
}

impl<'a, A: Array> Iterator for SmolSetIter<'a, A>
where A::Item: PartialEq + Eq + Hash + 'a
{
    type Item = &'a A::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            InnerSmolSetIter::Stack(iter) => iter.next(),
            InnerSmolSetIter::Heap(iter) => iter.next(),
        }
    }
}
