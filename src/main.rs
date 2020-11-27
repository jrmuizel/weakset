// https://www.ralfj.de/blog/2018/04/10/safe-intrusive-collections-with-pinning.html

use std::{cell::{Cell, RefCell}, collections::HashSet, rc::Rc, sync::Arc, thread};
use std::pin::Pin;
use std::marker::PhantomPinned;

use thread::Thread;

struct WeakSet<T> {
    objects: RefCell<HashSet<*const Entry<T>>>,
    _p: PhantomPinned,
}

pub struct Entry<T> {
    x: T,
    // set to Some if we are part of some collection
    collection: Cell<Option<*const WeakSet<T>>>,
    _p: PhantomPinned,
}

pub struct Iter<'a, K: 'a> {
    base: std::collections::hash_set::Iter<'a, *const Entry<K>>,
}

impl<'a, K> Iterator for Iter<'a, K> {
    type Item = &'a Entry<K>;

    #[inline]
    fn next(&mut self) -> Option<&'a Entry<K>> {
        self.base.next().map(|x| unsafe { &**x })
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
}

impl<T> WeakSet<T> {
    fn new() -> Self {
        WeakSet { objects: RefCell::new(HashSet::new()), _p: PhantomPinned }
    }

    // Add the entry to the collection
    fn insert(self: Pin<&mut Self>, entry: Pin<&Entry<T>>) {
        if entry.collection.get().is_some() {
            panic!("Can't insert the same object into multiple collections");
        }
        // Pointer from collection to entry
        let this : &mut Self = unsafe { Pin::get_unchecked_mut(self) };
        this.objects.borrow_mut().insert(&*entry as *const _);
        // Pointer from entry to collection
        entry.collection.set(Some(this as *const _));
    }

    fn iter(self: Pin<&Self>) -> Iter<'_, T> {
        let k = self.objects.borrow();
        Iter{ base: k.iter() }
    }
    
    // Show all entries of the collection
    fn print_all(self: Pin<&Self>)
    where T: ::std::fmt::Debug
    {
        print!("[");
        for entry in self.objects.borrow().iter() {
            let entry : &Entry<T> = unsafe { &**entry };
            print!(" {:?},", entry.x);
        }
        println!(" ]");
    }
}

impl<T> Drop for WeakSet<T> {
    fn drop(&mut self) {
        // Go through the entries to remove pointers to collection
        for entry in self.objects.borrow().iter() {
            let entry : &Entry<T> = unsafe { &**entry };
            entry.collection.set(None);
        }
    }
}

impl<T> Entry<T> {
    fn new(x: T) -> Self {
        Entry { x, collection: Cell::new(None), _p: PhantomPinned }
    }
}

impl<T> Drop for Entry<T> {
    fn drop(&mut self) {
        // Go through collection to remove this entry
        if let Some(collection) = self.collection.get() {
            let collection : &WeakSet<T> = unsafe { &*collection };
            collection.objects.borrow_mut().remove(&(self as *const _));
        }
    }
}

fn main() {
    let mut collection = Box::pin(WeakSet::new());
    let mut entry = Box::pin(Entry::new(42));
    let mut entry2 = Arc::pin(Entry::new(43));
    let entry3 = entry2.clone();
    collection.as_mut().insert(entry.as_ref());
    collection.as_mut().insert(entry2.as_ref());
    collection.as_ref().print_all(); // Prints "[ 42, ]"
    drop(entry); // Dropping the entry removes it
    collection.as_ref().print_all(); // Prints "[ ]"
    drop(entry2);
    collection.as_ref().print_all(); // Prints "[ ]"
    drop(entry3);
    collection.as_ref().print_all(); // Prints "[ ]"

    //thread::spawn(|| {drop(entry3); println!("fod");});
}
