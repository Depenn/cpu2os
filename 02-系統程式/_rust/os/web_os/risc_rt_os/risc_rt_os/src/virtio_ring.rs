// VirtIO Ring 結構（簡化版）
use core::ptr::{read_volatile, write_volatile};

#[repr(C, align(16))]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

#[repr(C, align(2))]
pub struct VirtqAvail {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; 256],
    pub used_event: u16,
}

#[repr(C, align(4))]
pub struct VirtqUsedElem {
    pub id: u32,
    pub len: u32,
}

#[repr(C, align(4))]
pub struct VirtqUsed {
    pub flags: u16,
    pub idx: u16,
    pub ring: [VirtqUsedElem; 256],
    pub avail_event: u16,
}

pub struct VirtQueue {
    pub desc: *mut [VirtqDesc; 256],
    pub avail: *mut VirtqAvail,
    pub used: *mut VirtqUsed,
    pub num: u16,
    pub last_used_idx: u16,
    pub free_head: u16,
    pub num_free: u16,
}

impl VirtQueue {
    pub fn new(desc_addr: usize, num: u16) -> Self {
        let desc = desc_addr as *mut [VirtqDesc; 256];
        
        // Available ring: 緊接在 descriptor table 之後
        // descriptor table size = num * 16 bytes
        let avail_offset = (num as usize) * 16;
        let avail = (desc_addr + avail_offset) as *mut VirtqAvail;
        
        // Used ring: 需要頁對齊
        // avail ring size = 6 + num * 2 bytes (flags, idx, ring[], used_event)
        let avail_size = 6 + (num as usize) * 2;
        // 對齊到 4 字節
        let avail_size_aligned = (avail_size + 3) & !3;
        let used = (desc_addr + avail_offset + avail_size_aligned) as *mut VirtqUsed;

        // 初始化 free list
        unsafe {
            for i in 0..num {
                (*desc)[i as usize].next = if i + 1 < num { i + 1 } else { 0xFFFF };
                (*desc)[i as usize].flags = 0;
                (*desc)[i as usize].addr = 0;
                (*desc)[i as usize].len = 0;
            }
            
            // 初始化 avail ring
            (*avail).flags = 0;
            (*avail).idx = 0;
            
            // 初始化 used ring
            (*used).flags = 0;
            (*used).idx = 0;
        }

        VirtQueue {
            desc,
            avail,
            used,
            num,
            last_used_idx: 0,
            free_head: 0,
            num_free: num,
        }
    }

    pub fn alloc_desc(&mut self) -> Option<u16> {
        if self.num_free == 0 {
            return None;
        }

        let desc_idx = self.free_head;
        unsafe {
            self.free_head = (*self.desc)[desc_idx as usize].next;
        }
        self.num_free -= 1;

        Some(desc_idx)
    }

    pub fn free_desc(&mut self, desc_idx: u16) {
        unsafe {
            (*self.desc)[desc_idx as usize].next = self.free_head;
        }
        self.free_head = desc_idx;
        self.num_free += 1;
    }

    pub fn add_buf(&mut self, buf_addr: u64, buf_len: u32, flags: u16) -> Option<u16> {
        let desc_idx = self.alloc_desc()?;

        unsafe {
            (*self.desc)[desc_idx as usize].addr = buf_addr;
            (*self.desc)[desc_idx as usize].len = buf_len;
            (*self.desc)[desc_idx as usize].flags = flags;
            (*self.desc)[desc_idx as usize].next = 0xFFFF;

            let avail_idx = read_volatile(&(*self.avail).idx);
            write_volatile(
                &mut (*self.avail).ring[(avail_idx % 256) as usize],
                desc_idx,
            );
            write_volatile(&mut (*self.avail).idx, avail_idx.wrapping_add(1));
        }

        Some(desc_idx)
    }

    pub fn get_used(&mut self) -> Option<(u16, u32)> {
        unsafe {
            let used_idx = read_volatile(&(*self.used).idx);

            if self.last_used_idx == used_idx {
                return None;
            }

            let last_used = &(*self.used).ring[(self.last_used_idx % 256) as usize];
            let desc_idx = read_volatile(&last_used.id) as u16;
            let len = read_volatile(&last_used.len);

            self.last_used_idx = self.last_used_idx.wrapping_add(1);

            Some((desc_idx, len))
        }
    }
}

pub const VIRTQ_DESC_F_WRITE: u16 = 2;