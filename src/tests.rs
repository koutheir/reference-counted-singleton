#![cfg(test)]

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::mem::MaybeUninit;
use std::sync::{atomic, atomic::AtomicI32, Once};
use std::{cmp, io};

#[derive(Debug)]
struct T1<'t>(&'t AtomicI32);

impl<'t> T1<'t> {
    fn new(counter: &'t AtomicI32) -> io::Result<Self> {
        counter.store(1, atomic::Ordering::Release);
        Ok(Self(counter))
    }
}

impl<'t> Drop for T1<'t> {
    fn drop(&mut self) {
        self.0.store(-1, atomic::Ordering::Release);
    }
}

impl<'t> PartialEq for T1<'t> {
    fn eq(&self, other: &Self) -> bool {
        self.0
            .load(atomic::Ordering::Acquire)
            .eq(&other.0.load(atomic::Ordering::Acquire))
    }
}

impl<'t> Eq for T1<'t> {}

impl<'t> PartialOrd for T1<'t> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'t> Ord for T1<'t> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .load(atomic::Ordering::Acquire)
            .cmp(&other.0.load(atomic::Ordering::Acquire))
    }
}

impl<'t> Hash for T1<'t> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.load(atomic::Ordering::Acquire).hash(state)
    }
}

static T1_S_VALUE: AtomicI32 = AtomicI32::new(0);
static T1_S_INIT: Once = Once::new();
static mut T1_S: MaybeUninit<super::RefCountedSingleton<T1<'static>>> = MaybeUninit::uninit();

fn get_or_init_t2_s() -> &'static super::RefCountedSingleton<T1<'static>> {
    T1_S_INIT.call_once(|| unsafe {
        T1_S = MaybeUninit::new(super::RefCountedSingleton::default());
    });

    unsafe { T1_S.as_ptr().as_ref().unwrap() }
}

#[test]
fn ref_counted_singleton_static() {
    T1_S_VALUE.store(1000, atomic::Ordering::Release);

    let creator = || T1::new(&T1_S_VALUE);

    let s = get_or_init_t2_s();
    assert_eq!(T1_S_VALUE.load(atomic::Ordering::Acquire), 1000);

    assert!(s.get().is_none());
    assert_eq!(T1_S_VALUE.load(atomic::Ordering::Acquire), 1000);

    let r1 = s.get_or_init(creator).unwrap();
    assert_eq!(T1_S_VALUE.load(atomic::Ordering::Acquire), 1);

    let r2 = r1.clone();
    assert_eq!(T1_S_VALUE.load(atomic::Ordering::Acquire), 1);

    drop(r1);
    assert_eq!(T1_S_VALUE.load(atomic::Ordering::Acquire), 1);

    drop(r2);
    assert_eq!(T1_S_VALUE.load(atomic::Ordering::Acquire), -1);

    T1_S_VALUE.store(2000, atomic::Ordering::Release);

    assert!(s.get().is_none());
    assert_eq!(T1_S_VALUE.load(atomic::Ordering::Acquire), 2000);

    let r = s.get_or_init(creator).unwrap();
    assert_eq!(T1_S_VALUE.load(atomic::Ordering::Acquire), 1);

    drop(r);
    assert_eq!(T1_S_VALUE.load(atomic::Ordering::Acquire), -1);
}

#[test]
fn ref_counted_singleton_new() {
    let value = AtomicI32::new(42);

    let creator = || T1::new(&value);

    let s = super::RefCountedSingleton::<T1>::default();
    assert_eq!(value.load(atomic::Ordering::Acquire), 42);

    assert!(s.get().is_none());
    assert_eq!(value.load(atomic::Ordering::Acquire), 42);

    let r1 = s.get_or_init(creator).unwrap();
    assert_eq!(r1.0.load(atomic::Ordering::Acquire), 1);

    let r2 = r1.clone();
    assert_eq!(r2.0.load(atomic::Ordering::Acquire), 1);

    assert_eq!(r1, r2);
    assert_eq!(r1.partial_cmp(&r1), Some(cmp::Ordering::Equal));
    assert_eq!(r1.cmp(&r1), cmp::Ordering::Equal);

    let _ignored = format!("{:?}", &r1);

    let mut hm = HashSet::new();
    hm.insert(r1.clone());
    drop(hm);

    drop(r1);
    assert_eq!(value.load(atomic::Ordering::Acquire), 1);

    drop(r2);
    assert_eq!(value.load(atomic::Ordering::Acquire), -1);

    value.store(1024, atomic::Ordering::Release);

    assert!(s.get().is_none());
    assert_eq!(value.load(atomic::Ordering::Acquire), 1024);

    let r = s.get_or_init(creator).unwrap();
    assert_eq!(value.load(atomic::Ordering::Acquire), 1);

    drop(r);
    assert_eq!(value.load(atomic::Ordering::Acquire), -1);

    drop(s);
    assert_eq!(value.load(atomic::Ordering::Acquire), -1);
}

#[test]
fn ref_counted_singleton_error() {
    let creator = || Err(io::Error::from(io::ErrorKind::Other));

    let s = super::RefCountedSingleton::<T1>::default();
    assert!(s.get().is_none());
    assert!(s.get_or_init(creator).is_err());
    assert!(s.get_or_init(creator).is_err());
    assert!(s.get().is_none());
}
