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

## Another Example
Just for fun, let's play with the allocator. We first allocate 7 objects with sizes of 5, 1, 3, 8, 7, 1, and 2 bytes. The we start removing them in a random order. The final shape of free-lists should be exactly similar to its initial shape (which is).

```
     1 [ 0]             v1:    0..4    (5 bytes)        1 [ 0]   
     2 [ 1]             v2:    8..8    (1 bytes)        2 [ 1]   
     4 [ 2]             v3:   12..14   (3 bytes)        4 [ 2]   
     8 [ 3]         =>  v4:   16..23   (8 bytes) =>     8 [ 3]   
    16 [ 4]             v5:   24..30   (7 bytes)       16 [ 4]   
    32 [ 5] (0..31)     v6:    9..9    (1 bytes)       32 [ 5]   
Available = 32 bytes    v7:   10..11   (2 bytes)   Available = 0 bytes
														      
          1 [ 0]                         1 [ 0] (9..9)                  1 [ 0] (9..9)  
          2 [ 1]                         2 [ 1]                         2 [ 1]         
 free(v5) 4 [ 2]                free(v6) 4 [ 2]                free(v1) 4 [ 2]         
 =======> 8 [ 3] (24..31)       =======> 8 [ 3] (24..31)       =======> 8 [ 3] (0..7)(24..31)
         16 [ 4]                        16 [ 4]                        16 [ 4]         
         32 [ 5]                        32 [ 5]                        32 [ 5]         
     Available = 8 bytes            Available = 9 bytes            Available = 9 bytes 

          1 [ 0] (9..9)                  1 [ 0]                         1 [ 0]         
          2 [ 1] (10..11)                2 [ 1]                         2 [ 1]         
 free(v7) 4 [ 2]                free(v2) 4 [ 2] (8..11)        free(v4) 4 [ 2] (8..11) 
 =======> 8 [ 3] (0..7)(24..31) =======> 8 [ 3] (0..7)(24..31) =======> 8 [ 3] (0..7)  
         16 [ 4]                        16 [ 4]                        16 [ 4] (16..31)
         32 [ 5]                        32 [ 5]                        32 [ 5]         
     Available = 19 bytes           Available = 20 bytes            Available = 28 bytes
 
          1 [ 0]
          2 [ 1]
 free(v3) 4 [ 2]
 =======> 8 [ 3]
         16 [ 4]
         32 [ 5] (0..31)
     Available = 32 bytes
```

Enjoy!

