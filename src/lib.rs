use std::{alloc::{alloc, dealloc, handle_alloc_error, Layout}, ptr::null_mut};

/// Implement this on objects containing Gc references
trait Trace {
    /// Call mark on each Gc
    fn trace(&self);

    /// The value that will be initialized in a new GcBox
    fn uninit() -> Self;
}

/// Container for T objects
#[derive(Debug)]
struct GcBox<T> {
    next: *mut Self,
    visited: bool,
    value: T,
}

/// Smart pointers for GcBox
#[derive(Debug)]
struct Gc<T>(*mut GcBox<T>);

impl<T> Clone for Gc<T> {
    fn clone(&self) -> Self {
        Gc(self.0)
    }
}

impl<T> Gc<T> {
    fn mark(&self) {
        unsafe { &mut *self.0 }.visited = true;
    }

    fn mutate<F: FnOnce(&mut T)>(&self, f: F) {
        f(unsafe { &mut (*self.0).value })
    }
}

/// Object responsible for allocating and collecting GcBox objects
#[derive(Debug)]
struct GcCollector<T> {
    next: *mut GcBox<T>,
}

impl<T: std::fmt::Debug> GcCollector<T> {
    /// Create a new GcCollector
    pub fn new() -> Self {
        Self { next: null_mut() }
    }

    /// Collect GcBoxes that are not reachable from the stack
    pub fn collect<U: Trace>(&mut self, stack: &[U]) {
        for value in stack {
            value.trace();
        }
        unsafe {
            let Some(mut prev) = self.next.as_mut() else { return; };
            if !prev.visited {
                dbg!(&prev);
                dealloc((prev as *mut GcBox<T>) as *mut u8, Layout::new::<GcBox<T>>());
            }
            while let Some(curr) = prev.next.as_mut() {
                if curr.visited {
                    prev = curr;
                } else {
                    prev.next = curr.next;
                    dbg!(&curr);
                    dealloc((curr as *mut GcBox<T>) as *mut u8, Layout::new::<GcBox<T>>());
                }
            }
        }
    }

    /// Allocate a GcBox and return a reference object for it
    pub fn alloc(&mut self) -> Gc<T>
    where
        T: Trace,
    {
        let layout = Layout::new::<GcBox<T>>();

        // Allocate and check allocation
        let ptr = unsafe { alloc(layout) }.cast::<GcBox<T>>();
        if ptr.is_null() {
            handle_alloc_error(layout);
        }

        // Construct value
        unsafe {
            *ptr = GcBox {
                next: self.next,
                visited: false,
                value: T::uninit(),
            };
        }
        self.next = ptr;

        Gc(self.next)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug)]
    struct Graph {
        value: i64,
        children: Vec<Gc<Self>>,
    }

    impl Trace for Graph {
        fn trace(&self) {
            for child in &self.children {
                child.mark();
            }
        }

        fn uninit() -> Self {
            Self { value: 0, children: Vec::new() }
        }
    }

    /// 1 -> 2 -> 3 -> 1
    /// 4 -> []
    /// [1]
    #[test]
    fn test() {
        let mut gc: GcCollector<Graph> = GcCollector::new();

        let x1 = gc.alloc();
        let x2 = gc.alloc();
        let x3 = gc.alloc();
        let x4 = gc.alloc();
        x1.mutate(|x| { x.value = 1; x.children.push(x2.clone()); });
        x2.mutate(|x| { x.value = 2; x.children.push(x3.clone()); });
        x3.mutate(|x| { x.value = 3; x.children.push(x1.clone()); });
        x4.mutate(|x| { x.value = 4; });
        let g1 = Graph {
            value: 5,
            children: vec![x1],
        };
        let g2 = Graph {
            value: 6,
            children: vec![x4],
        };
        gc.collect(&[g1]);
    }
}
