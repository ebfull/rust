// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Reject mixing cyclic structure and Drop when using TypedArena.
//
// (Compare against compile-fail/dropck_vec_cycle_checked.rs)
//
// (Also compare against compile-fail/dropck_tarena_unsound_drop.rs,
//  which is a reduction of this code to more directly show the reason
//  for the error message we see here.)

#![allow(unstable)]
#![feature(unsafe_destructor)]

extern crate arena;

use arena::TypedArena;
use std::cell::Cell;
use id::Id;

mod s {
    #![allow(unstable)]
    use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

    static S_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

    pub fn next_count() -> usize {
        S_COUNT.fetch_add(1, Ordering::SeqCst) + 1
    }
}

mod id {
    use s;
    #[derive(Debug)]
    pub struct Id {
        orig_count: usize,
        count: usize,
    }

    impl Id {
        pub fn new() -> Id {
            let c = s::next_count();
            println!("building Id {}", c);
            Id { orig_count: c, count: c }
        }
        pub fn count(&self) -> usize {
            println!("Id::count on {} returns {}", self.orig_count, self.count);
            self.count
        }
    }

    impl Drop for Id {
        fn drop(&mut self) {
            println!("dropping Id {}", self.count);
            self.count = 0;
        }
    }
}

trait HasId {
    fn count(&self) -> usize;
}

#[derive(Debug)]
struct CheckId<T:HasId> {
    v: T
}

#[allow(non_snake_case)]
fn CheckId<T:HasId>(t: T) -> CheckId<T> { CheckId{ v: t } }

#[unsafe_destructor]
impl<T:HasId> Drop for CheckId<T> {
    fn drop(&mut self) {
        assert!(self.v.count() > 0);
    }
}

#[derive(Debug)]
struct C<'a> {
    id: Id,
    v: Vec<CheckId<Cell<Option<&'a C<'a>>>>>,
}

impl<'a> HasId for Cell<Option<&'a C<'a>>> {
    fn count(&self) -> usize {
        match self.get() {
            None => 1,
            Some(c) => c.id.count(),
        }
    }
}

impl<'a> C<'a> {
    fn new() -> C<'a> {
        C { id: Id::new(), v: Vec::new() }
    }
}

fn f<'a>(arena: &'a TypedArena<C<'a>>) {
    let c1 = arena.alloc(C::new());
    let c2 = arena.alloc(C::new());
    let c3 = arena.alloc(C::new());

    c1.v.push(CheckId(Cell::new(None)));
    c1.v.push(CheckId(Cell::new(None)));
    c2.v.push(CheckId(Cell::new(None)));
    c2.v.push(CheckId(Cell::new(None)));
    c3.v.push(CheckId(Cell::new(None)));
    c3.v.push(CheckId(Cell::new(None)));

    c1.v[0].v.set(Some(c2));
    c1.v[1].v.set(Some(c3));
    c2.v[0].v.set(Some(c2));
    c2.v[1].v.set(Some(c3));
    c3.v[0].v.set(Some(c1));
    c3.v[1].v.set(Some(c2));
}

fn main() {
    let arena = TypedArena::new();
    f(&arena); //~ ERROR `arena` does not live long enough
}
