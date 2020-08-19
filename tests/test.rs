extern crate smolset;

use smolset::{SetMode, SmolSet};
use std::fmt::Write;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;

#[test]
fn test_basic_set() {
    let mut s: SmolSet<[u32; 2]> = SmolSet::new();
    assert_eq!(s.insert(1), true);
    assert_eq!(s.insert(2), true);
    assert_eq!(s.insert(2), false);
    assert_eq!(s.insert(3), true);
    assert_eq!(s.insert(2), false);
    assert_eq!(s.insert(3), false);
    assert!(s.contains(&1));
    assert!(s.contains(&2));
    assert!(s.contains(&3));
    assert!(!s.contains(&4));
    let expected = vec![1, 2, 3];
    assert_eq!(s.len(), expected.len());
    assert!(s
        .iter()
        .map(|r| *r)
        .collect::<Vec<u32>>()
        .iter()
        .all(|x| expected.contains(x)));
    s.clear();
    assert!(!s.contains(&1));
}

#[test]
fn test_remove() {
    let mut s: SmolSet<[u32; 2]> = SmolSet::new();
    assert_eq!(s.insert(1), true);
    assert_eq!(s.insert(2), true);
    assert_eq!(s.len(), 2);
    assert!(s.contains(&1));
    assert_eq!(s.remove(&1), true);
    assert_eq!(s.remove(&1), false);
    assert_eq!(s.len(), 1);
    assert!(!s.contains(&1));
    assert_eq!(s.insert(1), true);
    let expected = vec![1, 2];
    assert_eq!(s.len(), expected.len());
    assert!(s
        .iter()
        .map(|r| *r)
        .collect::<Vec<u32>>()
        .iter()
        .all(|x| expected.contains(x)));
}

#[test]
fn test_clone() {
    let mut s: SmolSet<[u32; 2]> = SmolSet::new();
    s.insert(1);
    s.insert(2);
    let c = s.clone();
    assert!(c.contains(&1));
    assert!(c.contains(&2));
    assert!(!c.contains(&3));
}

#[test]
fn test_debug_small() {
    let mut s: SmolSet<[u32; 2]> = SmolSet::new();
    s.insert(1);
    s.insert(2);
    let mut buf = String::new();
    write!(buf, "{:?}", s).unwrap();
    assert_eq!(&buf, "[1, 2]");
}

#[test]
fn test_from_iter() {
    let s: SmolSet<[usize; 4]> = vec![1, 2, 3, 4].into_iter().collect();
    assert_eq!(s.len(), 4);
}

#[test]
fn test_replace() {
    struct RingOf7 {
        pub value: u32,
    }

    impl PartialEq for RingOf7 {
        fn eq(&self, other: &Self) -> bool {
            self.value % 7 == other.value % 7
        }

        fn ne(&self, other: &Self) -> bool {
            self.value % 7 != other.value % 7
        }
    }

    impl From<RingOf7> for u32 {
        fn from(value: RingOf7) -> Self {
            value.value
        }
    }

    impl Hash for RingOf7 {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.value.hash(state)
        }
    }

    impl Eq for RingOf7 {}

    let mut lhs = SmolSet::<[RingOf7; 4]>::new();
    lhs.insert(RingOf7 { value: 1 });
    lhs.insert(RingOf7 { value: 2 });
    lhs.insert(RingOf7 { value: 3 });
    lhs.insert(RingOf7 { value: 4 });

    lhs.replace(RingOf7 { value: 8 });
    lhs.replace(RingOf7 { value: 9 });
    lhs.replace(RingOf7 { value: 10 });
    lhs.replace(RingOf7 { value: 11 });

    let expected = vec![8, 9, 10, 11];
    assert_eq!(lhs.len(), expected.len());
    assert!(lhs
        .iter()
        .map(|x| x.value)
        .collect::<Vec<u32>>()
        .iter()
        .all(|x| expected.contains(x)));
}

#[test]
fn test_eq_both_stack() {
    let mut lhs = SmolSet::<[u32; 4]>::new();
    lhs.insert(1);
    lhs.insert(2);

    let mut rhs = SmolSet::<[u32; 4]>::new();
    rhs.insert(1);
    rhs.insert(2);

    assert_eq!(lhs, rhs);
}

#[test]
fn test_eq_both_heap() {
    let expected = (0..100).collect::<Vec<u32>>();
    let lhs = SmolSet::<[u32; 4]>::from_iter(expected.clone());
    let rhs = SmolSet::<[u32; 4]>::from_iter(expected.clone());

    assert_eq!(lhs, rhs);
}

#[test]
fn test_eq_stack_heap() {
    let expected = (0..5).collect::<Vec<u32>>();
    let mut lhs = SmolSet::<[u32; 10]>::from_iter(expected.clone());
    let rhs = SmolSet::<[u32; 10]>::from_iter(expected.clone());

    (100..200).for_each(|x| assert!(lhs.insert(x)));
    (100..200).for_each(|x| assert!(lhs.remove(&x)));

    assert_eq!(lhs.mode(), SetMode::Heap);
    assert_eq!(rhs.mode(), SetMode::Stack);

    assert_eq!(lhs, rhs);
}

#[test]
fn test_intersection() {
    let mut lhs = SmolSet::<[u32; 4]>::new();
    lhs.insert(1);
    lhs.insert(3);
    lhs.insert(5);
    lhs.insert(4);
    lhs.insert(8);
    lhs.insert(10);

    let mut rhs = SmolSet::<[u32; 4]>::new();
    rhs.insert(4);
    rhs.insert(8);
    rhs.insert(10);

    assert!(lhs.intersection(&rhs).all(|x| x % 2 == 0));
}

#[test]
fn test_union() {
    let mut lhs = SmolSet::<[u32; 4]>::new();
    lhs.insert(1);
    lhs.insert(2);
    lhs.insert(3);
    lhs.insert(4);

    let mut rhs = SmolSet::<[u32; 4]>::new();
    rhs.insert(3);
    rhs.insert(4);
    rhs.insert(5);
    rhs.insert(6);

    let union = lhs.union(&rhs).collect::<Vec<_>>();
    let expected = vec![1, 2, 3, 4, 5, 6];
    assert_eq!(union.len(), expected.len());
    assert!(expected
        .iter()
        .collect::<Vec<&u32>>()
        .iter()
        .all(|x| union.contains(x)));
}

#[test]
fn test_difference() {
    let mut lhs = SmolSet::<[u32; 4]>::new();
    lhs.insert(1);
    lhs.insert(2);
    lhs.insert(3);
    lhs.insert(4);

    let mut rhs = SmolSet::<[u32; 4]>::new();
    rhs.insert(3);
    rhs.insert(4);
    rhs.insert(5);
    rhs.insert(6);

    let union = lhs.difference(&rhs).collect::<Vec<_>>();
    let expected = vec![1, 2];
    assert_eq!(union.len(), expected.len());
    assert!(expected
        .iter()
        .collect::<Vec<&u32>>()
        .iter()
        .all(|x| union.contains(x)));
}

#[test]
fn test_symmetric_difference() {
    let mut lhs = SmolSet::<[u32; 4]>::new();
    lhs.insert(1);
    lhs.insert(2);
    lhs.insert(3);
    lhs.insert(4);

    let mut rhs = SmolSet::<[u32; 4]>::new();
    rhs.insert(3);
    rhs.insert(4);
    rhs.insert(5);
    rhs.insert(6);

    let symmetric_difference = lhs.symmetric_difference(&rhs).collect::<Vec<_>>();
    let expected = vec![1, 2, 5, 6];
    assert_eq!(symmetric_difference.len(), expected.len());
    assert!(expected
        .iter()
        .collect::<Vec<&u32>>()
        .iter()
        .all(|x| { symmetric_difference.contains(x) }));
}
