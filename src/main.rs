#![allow(dead_code)]
#![allow(unused_must_use)]
#![feature(assoc_int_consts)]
#![feature(untagged_unions)]

use std::mem;

#[allow(non_camel_case_types)]
type pptr = usize;

#[derive(Clone, Default)]
/// Buddy memory block
/// Each memory block has some meta-data information in form of `Buddy` data 
/// structure. It has a pointer to the next buddy block, if there is any. It 
/// also keeps a log of the next pointer for atomic operations.
struct Buddy {
    /// Next pointer
    /// We assume that usize::MAX is NULL
    next: pptr,
}

const META_SIZE: usize = mem::size_of::<Buddy>();

#[inline]
fn is_null(p: pptr) -> bool {
    p == usize::MAX
}

#[inline]
fn pptr_to_option(p: pptr) -> Option<pptr> {
    if is_null(p) { None } else { Some(p) }
}

#[inline]
fn option_to_pptr(p: Option<pptr>) -> pptr {
    if let Some(p) = p { p } else { usize::MAX }
}

/// Buddy Memory Allocator
/// It contains 60 free-lists of available buddy blocks to keep at most 2^64
/// bytes including meta-data information. A free-list k keeps all available 
/// memory blocks of size 2^k bytes plus an extra information for `Buddy` 
/// struct. Assuming that `Buddy` has a size of 8 bytes, the shape of lists 
/// can be like this:
/// 
///   [16]: [8|8] -> [8|8]
///   [32]: [8|24] -> [8|24] -> [8|24]
///   [64]: [8|56]
///   ...
/// 
/// The first 8 bytes of each block is meta-data. The rest is the actual 
/// memory handed to the user.
struct BuddyAllocator {
    buddies: [Option<pptr>; 64],
    available: usize,
    size: usize,
    commited: bool,
    last_idx: usize,
    raw_offset: pptr
}

const fn num_bits<T>() -> u32 { (mem::size_of::<T>() << 3) as u32 }

#[inline]
fn get_idx(x: usize) -> usize {
    assert!(x > 0);
    (num_bits::<usize>() - (x-1).leading_zeros()) as usize
}

fn deref(base: pptr, off: pptr) -> &'static mut Buddy {
    union U<'a> {
        off: pptr,
        obj: &'a mut Buddy
    }
    let u = U {off: base + off};
    unsafe { u.obj }
}

impl BuddyAllocator {
    pub fn new() -> Self {
        BuddyAllocator {
            buddies: [None; 64],
            available: 0,
            size: 0,
            commited: true,
            last_idx: 0,
            raw_offset: 0
        }
    }
    pub fn init(&mut self, size: usize, offset: pptr) {
        let mut idx = get_idx(size);
        if 1 << idx > size {
            idx -= 1;
        }
        self.buddies = [None; 64];
        self.size = 1 << idx;
        self.available = self.size - META_SIZE;
        self.buddies[idx] = Some(0);
        self.last_idx = idx;
        self.commited = true;
        self.raw_offset = offset;
        let b = deref(offset, 0);
        b.next = usize::MAX;
        println!("Memory is initiated with {} bytes", self.size);
    }
    fn apply(&mut self, to_add: &mut Vec<(usize, pptr)>) {
        for b in to_add {
            let n = deref(self.raw_offset, b.1);
            n.next = option_to_pptr(self.buddies[b.0]);
            self.buddies[b.0] = Some(b.1);
        }
    }
    fn find_free_memory(&mut self, idx: usize, 
        to_add: &mut Vec<(usize, pptr)>, 
        split: bool) 
    -> Option<pptr> {
        if idx > self.last_idx {
            None
        } else {
            let res;
            if let Some(b) = self.buddies[idx].clone() {
                // Remove the available block and return it
                let buddy = deref(self.raw_offset, b);
                self.buddies[idx] = pptr_to_option(buddy.next);
                res = b;
            } else {
                res = self.find_free_memory(idx+1, to_add, true)?;
            }
            if idx > 0 && split {
                to_add.push((idx-1, res + (1 << (idx-1))));
            }
            Some(res)
        }
    }

    /// Allocate new memory block
    pub fn alloc(&mut self, len: usize) -> Result<pptr, &str> {
        let mut to_add = vec!();
        let idx = get_idx(len + META_SIZE);
        if self.commited { self.tx_begin(); }
        match self.find_free_memory(idx, &mut to_add, false) {
            Some(res) => {
                self.apply(&mut to_add);
                self.available -= 1 << idx;
                Ok(res + META_SIZE)
            }
            None => Err("Out of memory")
        }
    }

    fn __free(&mut self, off: pptr, len: usize) {
        let idx = get_idx(len);
        let end = off + (1 << idx);
        if self.commited { self.tx_begin(); }
        if idx+1 <= self.last_idx {
            let mut curr = self.buddies[idx].clone();
            let mut prev: Option<pptr> = None;
            while let Some(b) = curr {
                let e = deref(self.raw_offset, b);
                let on_left = off & (1 << idx) == 0;
                if (b == end && on_left) || (b + len == off && !on_left)  {
                    let off = pptr::min(off, b);
                    if let Some(p) = prev {
                        let p = deref(self.raw_offset, p);
                        p.next = e.next;
                    } else {
                        self.buddies[idx] = pptr_to_option(e.next);
                    }
                    self.available -= len;
                    self.__free(off, len << 1);
                    return;
                }
                prev = Some(b);
                curr = pptr_to_option(e.next);
            }
        }
        let e = deref(self.raw_offset, off);
        e.next = option_to_pptr(self.buddies[idx]);
        self.available += len;
        self.buddies[idx] = Some(off);
    }

    /// Free memory block
    pub fn free(&mut self, off: pptr, len: usize) {
        let idx = get_idx(len + META_SIZE);
        let len = 1 << idx;
        let off = off - META_SIZE;
        self.available += META_SIZE;
        self.__free(off, len);
        self.available -= META_SIZE;
    }
    pub fn tx_begin(&mut self) {
        self.commited = false;
    }
    pub fn tx_end(&mut self) {
        self.commited = true;
    }
    pub fn print(&self) {
        println!();
        for idx in 4..self.last_idx+1 {
            print!("{:>6} [{:>2}] ", 1 << idx, idx);
            let mut curr = self.buddies[idx].clone();
            while let Some(b) = curr {
                print!("({}..{})", b, b + (1 << idx) - 1);
                let e = deref(self.raw_offset, b);
                curr = pptr_to_option(e.next);
            }
            println!();
        }
        println!("Available = {} bytes", self.available);
    }
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

    use std::path::PathBuf;
    use std::fs::OpenOptions;
    let filename = "image";
    let path = PathBuf::from(filename);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    file.set_len(1024*1024 as u64).unwrap();
    let mmap = unsafe { memmap::MmapOptions::new().map_mut(&file).unwrap() };
    let raw_offset = mmap.get(0).unwrap() as *const u8 as pptr;
    let mut a = BuddyAllocator::new();
    let mut id = 0;
    let mut map: HashMap<String, (pptr, usize)> = HashMap::new();
    
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
                a.init(len, raw_offset);
                map.clear();
            }
        }));
    }
}
