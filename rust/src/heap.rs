use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr;

struct BlockHeader {
    size: usize,
    free: bool,
    next: *mut BlockHeader,
}

struct AllocatorInner {
    head: *mut BlockHeader,
    heap_start: usize,
    heap_end: usize,
}

pub struct KernelAllocator {
    inner: UnsafeCell<AllocatorInner>,
}

unsafe impl Send for KernelAllocator {}
unsafe impl Sync for KernelAllocator {}

impl KernelAllocator {
    pub const fn empty() -> Self {
        Self {
            inner: UnsafeCell::new(AllocatorInner {
                head: ptr::null_mut(),
                heap_start: 0,
                heap_end: 0,
            }),
        }
    }

    pub unsafe fn init(&self, start: usize, size: usize) {
        let inner = &mut *self.inner.get();
        inner.heap_start = (start + 7) & !7;
        inner.heap_end = start + size;
        inner.head = inner.heap_start as *mut BlockHeader;

        let available = inner.heap_end.saturating_sub(inner.heap_start + core::mem::size_of::<BlockHeader>());
        (*inner.head).size = available;
        (*inner.head).free = true;
        (*inner.head).next = ptr::null_mut();
    }
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let inner = &mut *self.inner.get();
        let align = layout.align().max(core::mem::align_of::<BlockHeader>());
        let needed_size = (layout.size() + 7) & !7;

        let mut curr = inner.head;
        while !curr.is_null() {
            let header = &mut *curr;
            if header.free {
                let data_ptr = (curr as usize + core::mem::size_of::<BlockHeader>()) as *mut u8;
                let misalign = (data_ptr as usize) % align;
                let adjustment = if misalign == 0 { 0 } else { align - misalign };
                let total_needed = needed_size + adjustment;

                if header.size >= total_needed {
                    let remaining = header.size - total_needed;
                    let header_size = core::mem::size_of::<BlockHeader>();

                    if remaining >= header_size + 16 {
                        let next_header_addr = (data_ptr as usize) + total_needed;
                        let next_header = next_header_addr as *mut BlockHeader;
                        (*next_header).size = remaining - header_size;
                        (*next_header).free = true;
                        (*next_header).next = header.next;

                        header.size = total_needed;
                        header.next = next_header;
                    }

                    header.free = false;
                    return data_ptr.add(adjustment);
                }
            }
            curr = header.next;
        }

        ptr::null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let inner = &mut *self.inner.get();
        let header_addr = (ptr as usize) - core::mem::size_of::<BlockHeader>();
        let mut curr = inner.head;
        let mut target: *mut BlockHeader = ptr::null_mut();

        while !curr.is_null() {
            let data_start = (curr as usize) + core::mem::size_of::<BlockHeader>();
            let data_end = data_start + (*curr).size;
            if (ptr as usize) >= data_start && (ptr as usize) < data_end {
                target = curr;
                break;
            }
            curr = (*curr).next;
        }

        if target.is_null() {
            target = header_addr as *mut BlockHeader;
        }

        (*target).free = true;

        let mut c = inner.head;
        while !c.is_null() {
            let next = (*c).next;
            if !next.is_null() && (*c).free && (*next).free {
                let expected_next = (c as usize) + core::mem::size_of::<BlockHeader>() + (*c).size;
                if expected_next == (next as usize) {
                    (*c).size += core::mem::size_of::<BlockHeader>() + (*next).size;
                    (*c).next = (*next).next;
                    continue;
                }
            }
            c = (*c).next;
        }
    }
}
