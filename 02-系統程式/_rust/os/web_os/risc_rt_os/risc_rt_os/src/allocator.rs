// 簡單的 Bump 分配器
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;

pub struct BumpAllocator {
    heap: UnsafeCell<[u8; 1024 * 1024]>, // 1MB 堆
    next: UnsafeCell<usize>,
}

unsafe impl Sync for BumpAllocator {}

impl BumpAllocator {
    pub const fn new() -> Self {
        BumpAllocator {
            heap: UnsafeCell::new([0; 1024 * 1024]),
            next: UnsafeCell::new(0),
        }
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        
        let next = self.next.get();
        let mut current = *next;
        
        // 對齊
        let remainder = current % align;
        if remainder != 0 {
            current += align - remainder;
        }
        
        let new_next = current + size;
        
        if new_next > 1024 * 1024 {
            return core::ptr::null_mut();
        }
        
        *next = new_next;
        
        let heap = self.heap.get();
        (*heap).as_mut_ptr().add(current)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump 分配器不支持釋放單個對象
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}