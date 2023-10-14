use std::{alloc::{alloc, dealloc, handle_alloc_error, Layout}, ptr::null_mut};

#[derive(Clone, Debug, PartialEq, Eq)]
struct GcBox {
    next: *mut Self,
    visited: bool,
    value: i64,
    children: Vec<Gc>,
}

impl GcBox {
    fn new() -> Self {
        Self {
            next: null_mut(),
            visited: false,
            value: 0,
            children: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Gc(*mut GcBox);

impl Gc {
    fn get(&self) -> &GcBox {
        unsafe { & *self.0 }
    }

    unsafe fn get_mut(&self) -> &mut GcBox {
        unsafe { &mut *self.0 }
    }

    fn set_next(&self, next: *mut GcBox) {
        unsafe { self.get_mut() }.next = next;
    }

    fn get_next(&self) -> *mut GcBox {
        self.get().next
    }

    fn set_value(&self, value: i64) {
        unsafe { self.get_mut() }.value = value;
    }

    fn push(&self, other: &Self) {
        unsafe { self.get_mut() }.children.push(other.clone());
    }

    fn mark(&self) {
        let gcbox = unsafe { self.get_mut() };
        if !gcbox.visited {
            gcbox.visited = true;
            for child in &gcbox.children {
                child.mark();
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GcMac {
    head: GcBox,
    stack: Vec<Gc>,
}

impl GcMac {
    fn new() -> Self {
        Self {
            head: GcBox::new(),
            stack: Vec::new(),
        }
    }

    fn mark(&self) {
        for value in &self.stack {
            value.mark();
        }
    }

    fn sweep(&mut self) {
        let layout = Layout::new::<GcBox>();
        let mut prev = &mut self.head as *mut GcBox;
        while !prev.is_null() {
            unsafe {
                let next = (*prev).next;
                if (*next).visited {
                    prev = next;
                } else {
                    // reroute prev to the box after the next and deallocate the box
                    (*prev).next = (*next).next;
                    println!("{:?}", (*next).value);
                    dealloc(next as *mut u8, layout);
                }
            }
        }
    }

    fn collect(&mut self) {
        self.mark();
        self.sweep();
    }

    fn alloc(&mut self) -> Gc {
        let layout = Layout::new::<GcBox>();
        let r = unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                handle_alloc_error(layout);
            }
            let ptr = ptr as *mut GcBox;
            *ptr = GcBox::new();
            Gc(ptr)
        };
        r.set_next(self.head.next);
        self.head.next = r.0;
        r
    }

    fn push(&mut self, obj: &Gc) {
        self.stack.push(obj.clone());
    }
}

fn main() {
    let mut gc = GcMac::new();
    let a = gc.alloc();
    a.set_value(0);
    let b = gc.alloc();
    b.set_value(1);
    let c = gc.alloc();
    c.set_value(2);
    let d = gc.alloc();
    d.set_value(3);
    let e = gc.alloc();
    e.set_value(4);
    let f = gc.alloc();
    f.set_value(5);

    a.push(&b);
    b.push(&c);
    c.push(&a);

    d.push(&e);
    e.push(&f);
    f.push(&d);

    gc.push(&a);
    for x in [&a, &b, &c, &d, &e, &f] {
        dbg!(x.get().value, x.get().visited);
    }
    gc.mark();
    for x in [&a, &b, &c, &d, &e, &f] {
        dbg!(x.get().value, x.get().visited);
    }
    gc.sweep();
}
