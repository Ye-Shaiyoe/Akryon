pub const PAGE_SIZE: usize = 4096;
const MAX_PHYSICAL_MEM: usize = 128 * 1024 * 1024;
const TOTAL_PAGES: usize = MAX_PHYSICAL_MEM / PAGE_SIZE;
const BITMAP_ENTRIES: usize = TOTAL_PAGES / 32;

static mut BITMAP: [u32; BITMAP_ENTRIES] = [0xFFFFFFFF; BITMAP_ENTRIES];
static mut TOTAL_FRAMES: usize = 0;
static mut USED_FRAMES: usize = 0;

pub fn init(mem_size: usize, kernel_start: usize, kernel_end: usize) {
    let pages = mem_size / PAGE_SIZE;
    unsafe {
        TOTAL_FRAMES = if pages > TOTAL_PAGES { TOTAL_PAGES } else { pages };
        USED_FRAMES = TOTAL_FRAMES;

        let end_idx = (TOTAL_FRAMES + 31) / 32;
        for i in 0..end_idx {
            BITMAP[i] = 0xFFFFFFFF;
        }

        let start_page = (kernel_end + PAGE_SIZE - 1) / PAGE_SIZE;
        for page in start_page..TOTAL_FRAMES {
            free_frame_raw(page * PAGE_SIZE);
        }

        let kernel_start_page = kernel_start / PAGE_SIZE;
        let kernel_end_page = (kernel_end + PAGE_SIZE - 1) / PAGE_SIZE;
        for page in kernel_start_page..kernel_end_page {
            set_frame_raw(page * PAGE_SIZE);
        }

        for page in 0..256 {
            set_frame_raw(page * PAGE_SIZE);
        }
    }
}

unsafe fn set_frame_raw(addr: usize) {
    let frame = addr / PAGE_SIZE;
    let idx = frame / 32;
    let off = frame % 32;
    if idx < BITMAP_ENTRIES && (BITMAP[idx] & (1 << off)) == 0 {
        BITMAP[idx] |= 1 << off;
        USED_FRAMES += 1;
    }
}

unsafe fn free_frame_raw(addr: usize) {
    let frame = addr / PAGE_SIZE;
    let idx = frame / 32;
    let off = frame % 32;
    if idx < BITMAP_ENTRIES && (BITMAP[idx] & (1 << off)) != 0 {
        BITMAP[idx] &= !(1 << off);
        USED_FRAMES -= 1;
    }
}

pub fn alloc_frame() -> Option<usize> {
    unsafe {
        let max_idx = (TOTAL_FRAMES + 31) / 32;
        for idx in 0..max_idx {
            if BITMAP[idx] != 0xFFFFFFFF {
                for bit in 0..32 {
                    let frame = idx * 32 + bit;
                    if frame >= TOTAL_FRAMES {
                        return None;
                    }
                    if (BITMAP[idx] & (1 << bit)) == 0 {
                        BITMAP[idx] |= 1 << bit;
                        USED_FRAMES += 1;
                        return Some(frame * PAGE_SIZE);
                    }
                }
            }
        }
        None
    }
}

pub fn reserve_frame(addr: usize) {
    unsafe {
        set_frame_raw(addr);
    }
}

pub fn free_frame(addr: usize) {
    unsafe {
        free_frame_raw(addr);
    }
}

pub fn total_memory() -> usize {
    unsafe { TOTAL_FRAMES * PAGE_SIZE }
}

pub fn used_memory() -> usize {
    unsafe { USED_FRAMES * PAGE_SIZE }
}

pub fn free_memory() -> usize {
    unsafe { (TOTAL_FRAMES.saturating_sub(USED_FRAMES)) * PAGE_SIZE }
}
