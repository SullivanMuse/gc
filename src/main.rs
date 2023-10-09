use std::{alloc::{alloc, dealloc, handle_alloc_error, Layout}, ptr::null_mut, rc::Rc};

#[derive(Clone, Debug)]
enum Value {
    Uninit,
    Int(i64),
    Float(f64),
    String(Rc<str>),
    Product(Vec<Value>),
    Ref(*mut Ref),
}

impl Value {
    fn trace(&self) {
        match self {
            Self::Product(xs) => for x in xs {
                x.trace();
            }
            
            Self::Ref(r) => {
                // safety: r is nonnull because it was checked after allocation
                let r = unsafe { &mut **r };
                if !r.visited {
                    r.visited = true;
                    r.value.trace();
                }
            }
            
            _ => {}
        }
    }
    
    fn mutate<F: FnOnce(&mut Value)>(&self, f: F) {
        let Self::Ref(r) = self else { panic!(); };
        // safety: dereferencing r is safe because it is known to be nonnull
        f(unsafe { &mut (**r).value });
    }
}

trait Trace {
    fn trace(&self);
}

#[derive(Debug)]
struct Ref {
    next: *mut Ref,
    visited: bool,
    value: Value,
}

#[derive(Debug)]
struct Gc {
    next: *mut Ref,
}

impl Gc {
    pub fn new() -> Self {
        Self { next: null_mut() }
    }
    
    pub fn collect(&mut self, stack: &[Value]) {
        for value in stack {
            value.trace();
        }
        unsafe {
            let Some(mut prev) = self.next.as_mut() else { return; };
            if !prev.visited {
                dealloc((prev as *mut Ref) as *mut u8, Layout::new::<Ref>());
            }
            while let Some(curr) = prev.next.as_mut() {
                if curr.visited {
                    prev = curr;
                } else {
                    prev.next = curr.next;
                    dealloc((curr as *mut Ref) as *mut u8, Layout::new::<Ref>());
                }
            }
        }
    }
    
    pub fn alloc(&mut self) -> Value {
        let layout = Layout::new::<Ref>();
        
        // Allocate and check allocation
        let ptr = unsafe { alloc(layout) }.cast::<Ref>();
        if ptr.is_null() {
            handle_alloc_error(layout);
        }
    
        // Construct value
        unsafe {
            *ptr = Ref {
                next: self.next,
                visited: false,
                value: Value::Uninit,
            };
        }
        self.next = ptr;
        
        Value::Ref(self.next)
    }
    
    fn alloc_many(&mut self) -> Vec<Value> {
        let mut xs = Vec::new();
        for _ in 0..10 {
            xs.push(self.alloc());
        }
        xs
    }
}

fn main() {
    let mut gc = Gc::new();
    let xs = gc.alloc_many();
    for x in xs {
        let Value::Ref(x) = x else { panic!() };
        println!("{:?} {:?}", x, unsafe { &*x });
    }
}
