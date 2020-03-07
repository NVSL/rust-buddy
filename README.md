# Buddy Memory Allocation
This repository contains the implementation of a simple [Buddy Memory Allocator](https://en.wikipedia.org/wiki/Buddy_memory_allocation). 
The primary data structure [BuddyAllocator](src/main.rs#L16) contains 32 lists of [Buddy](src/main.rs#L11) objects, each of which maintains an address of free memory block of size 2^i.

```rust
struct BuddyAllocator {
    buddies: [Option<Rc<RefCell<Buddy>>>; 32], // Free-lists
    available: usize,                          // Total available memory
    size: usize,                               // Total size of the memory
    last: usize                                // Index of the last free-list which may be used
}
```

You are given a bunch of options to operate on the memory, and you can see the free-lists, available space, and the allocated objects by choosing the `print` option. 

## Example
Let's assume that we have a memory of size 1024 bytes. Initially, there is only one giant block of 1024 bytes. The free-lists look like this:

```
     1 [ 0]
     2 [ 1]
     4 [ 2]
     8 [ 3]
    16 [ 4] 
    32 [ 5]
    64 [ 6]
   128 [ 7]
   256 [ 8]
   512 [ 9]
  1024 [10] (0..1024)
Available = 1024 bytes
```

To **allocate** a new object of `v1` with the size of 100 bytes. In the Buddy allocation algorithm, the object will allocate the upper nearest power of 2 bytes, which is 128 bytes in this case. Since there is no free block of 128 bytes available in list 7 (2^7), the allocator checks its neighbor list to see if there is any block of size 256 (2^8). If so, it splits it into two 128-byte blocks and uses one of them. If not, it reiterates the checking until either finds it of fails. In this case, it splits 1024 into two 512, then splits the first 512 into two 256, and finally it breaks one of 256 blocks and uses one of them. The free-lists are now like this:

```
     1 [ 0]
     2 [ 1]
     4 [ 2]
     8 [ 3]
    16 [ 4] 
    32 [ 5]
    64 [ 6]
   128 [ 7] (128..256)
   256 [ 8] (256..512)
   512 [ 9] (512..1024)
  1024 [10]
Available = 896 bytes
Variables:
      v1:    0..99   (100 bytes)
```

To **free** the object, the allocator tries to put the allocation (`v1` at 0..127) back to its place at list 7. However, it seems that releasing memory has a buddy block (128..256). So, it merges them to make a larger free block of 256 bytes and tries to put it in list 8. It reiterates the same steps until it can't. Now, we have a giant 1024-byte free block, again.

```
     1 [ 0]
     2 [ 1]
     4 [ 2]
     8 [ 3]
    16 [ 4]
    32 [ 5] 
    64 [ 6]
   128 [ 7]
   256 [ 8]
   512 [ 9]
  1024 [10] (0..1024)
Available = 1024 bytes
```

Enjoy!

