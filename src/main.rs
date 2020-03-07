#![allow(dead_code)]
#![allow(unused_must_use)]

use std::cell::RefCell;
use std::rc::{Rc,Weak};

#[allow(non_camel_case_types)]
type pptr = usize;

#[derive(Clone)]
struct Buddy {
    off: pptr,
    next: Option<Rc<RefCell<Buddy>>>
}

struct BuddyAllocator {
    buddies: [Option<Rc<RefCell<Buddy>>>; 32],
    available: usize,
    size: usize,
    last: usize
}

const fn num_bits<T>() -> u32 { (std::mem::size_of::<T>() << 3) as u32 }

#[inline]
fn get_idx(x: usize) -> usize {
    assert!(x > 0);
    (num_bits::<usize>() - (x-1).leading_zeros()) as usize
}

impl BuddyAllocator {
    pub fn new() -> Self {
        BuddyAllocator {
            buddies: Default::default(),
            available: 0,
            size: 0,
            last: 0
        }
    }
    pub fn init(&mut self, size: usize) {
        let mut idx = get_idx(size);
        if 1 << idx > size {
            idx -= 1;
        }
        self.last = usize::min(idx + 1, 31);
        self.available = 1 << idx;
        self.size = self.available;
        self.buddies[idx] = Some(Rc::new(RefCell::new(Buddy{
            off: 0,
            next: None
        })));
        println!("Memory is initiated with {} bytes", self.size);
    }
    fn apply(&mut self, to_rem: &mut Vec<usize>, to_add: &mut Vec<(usize, pptr)>) {
        for b in to_rem {
            let d = self.buddies[*b].as_ref().unwrap();
            let nxt = if let Some(pn) = &d.borrow().next {
                Some(pn.clone())
            } else {
                None
            };
            self.buddies[*b] = nxt;
        }
        for b in to_add {
            let n = if let Some(d) = &self.buddies[b.0] {
                Buddy {
                    off: b.1,
                    next: Some(d.clone())
                }
            } else {
                Buddy{
                    off: b.1,
                    next: None
                }
            };
            self.buddies[b.0] = Some(Rc::new(RefCell::new(n)));
        }
    }
    fn find_free_memory(&mut self, idx: usize, 
        to_rem: &mut Vec<usize>, 
        to_add: &mut Vec<(usize, pptr)>, 
        lend: bool) 
    -> Option<pptr> {
        if idx == 32 {
            None
        } else {
            let res;
            if let Some(b) = self.buddies[idx].as_ref() {
                to_rem.push(idx);
                res = b.borrow().off;
            } else {
                res = self.find_free_memory(idx+1, to_rem, to_add, true)?;
            }
            if idx > 0 && lend {
                to_add.push((idx-1, res + (1 << (idx-1))));
            }
            Some(res)
        }
    }
    pub fn alloc(&mut self, len: usize) -> Result<pptr, &str> {
        let mut to_rem = vec!();
        let mut to_add = vec!();
        let idx = get_idx(len);
        match self.find_free_memory(idx, &mut to_rem, &mut to_add, false) {
            Some(res) => {
                if res >= self.size {
                    Err("Out of memory")
                } else {
                    self.apply(&mut to_rem, &mut to_add);
                    self.available -= 1 << idx;
                    Ok(res)
                }
            }
            None => Err("Out of memory")
        }
    }
    pub fn free(&mut self, off: pptr, len: usize) {
        let idx = get_idx(len);
        let len = 1 << idx;
        let end = off + len;
        if idx+1 <= self.last {
            let mut curr = self.buddies[idx].clone();
            let mut prev: Weak<RefCell<Buddy>> = Weak::new();
            while let Some(b) = curr {
                let e = b.borrow();
                if e.off == end || e.off + len == off  {
                    let off = pptr::min(off,e.off);
                    if let Some(p) = prev.upgrade() {
                        p.borrow_mut().next = e.next.clone();
                    } else {
                        self.buddies[idx] = None;
                    }
                    self.free(off, len << 1);
                    return;
                }
                prev = Rc::downgrade(&b);
                curr = if let Some(nxt) = &e.next {
                    Some(nxt.clone())
                } else {
                    None
                };
            }
        }
        self.available += len;
        let n = if let Some(d) = &self.buddies[idx] {
            Buddy {
                off,
                next: Some(d.clone())
            }
        } else {
            Buddy{
                off,
                next: None
            }
        };
        self.buddies[idx] = Some(Rc::new(RefCell::new(n)));
    }
    pub fn print(&self) {
        println!();
        for idx in 0..self.last {
            print!("{:>6} [{:>2}] ", 1 << idx, idx);
            let mut curr = self.buddies[idx].clone();
            while let Some(b) = curr {
                let b = b.borrow();
                print!("({}..{})", b.off, b.off + (1 << idx));
                curr = if let Some(nxt) = b.next.clone() {
                    Some(nxt)
                } else {
                    None
                };
            }
            println!();
        }
        println!("Available = {} bytes", self.available);
    }
}

fn print_help() {
    println!("Usage: ");
}

fn input(print_options: bool, msg: &str) -> Option<String> {
    use std::io::{stdin,stdout,Write};
    let mut s=String::new();
    if print_options {
        println!("\nOptions:");
        println!("  i - Init memory");
        println!("  a - Allocate new variable given a length");
        println!("  f - Free a variable given its name");
        println!("  p - Print info");
        println!("  q - Quit");
    }
    print!("{}", msg);
    let _=stdout().flush();
    stdin().read_line(&mut s).expect("Did not enter a correct string");
    if let Some('\n')=s.chars().next_back() {
        s.pop();
    }
    if let Some('\r')=s.chars().next_back() {
        s.pop();
    }
    if let "q" = &*s {
        None
    } else {
        Some(s)
    }
}

fn main() {
    use std::collections::HashMap;

    let mut id = 0;
    let mut a = BuddyAllocator::new();
    let mut map: HashMap<String, (pptr, usize)> = HashMap::new();
    
    a.init(1024);
    while let Some(cmd) = input(true, "Your choice: ") {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let "a" = &*cmd {
                let len = input(false, "Length: ").expect("Wrong input");
                let len: usize = len.parse().expect("Expected an integer");
                assert!(len > 0);
                id += 1;
                let v = a.alloc(len).expect("Out of memory");
                let name = format!("v{}", id);
                map.insert(name.clone(), (v, len));
                println!("`{}` is allocated at address {}", name, v);
            } else if let "f" = &*cmd {
                let name = input(false, "Variable ident: ").expect("Wrong input");
                if let Some(v) = map.remove(&name) {
                    a.free(v.0, v.1);
                    println!("`{}` is deleted from memory", name);
                } else {
                    println!("No such variable `{}`", name);
                }
            } else if let "p" = &*cmd {
                a.print();
                if !map.is_empty() {
                    println!("Variables:");
                    for (n, v) in &map {
                        println!("{:>8}: {:>4}..{:<4} ({} bytes)", n, v.0, v.0+v.1-1, v.1);
                    }
                }
            } else if let "i" = &*cmd {
                let len = input(false, "Size: ").expect("Wrong input");
                let len: usize = len.parse().expect("Expected an integer");
                a = BuddyAllocator::new();
                a.init(len);
                map.clear();
            }
        }));
    }
}
