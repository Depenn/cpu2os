use super::frame::alloc_frame;

#[allow(dead_code)]
pub const PTE_V: usize = 1 << 0;
pub const PTE_R: usize = 1 << 1;
pub const PTE_W: usize = 1 << 2;
pub const PTE_X: usize = 1 << 3;
pub const PTE_U: usize = 1 << 4;
#[allow(dead_code)]
pub const PTE_G: usize = 1 << 5;
#[allow(dead_code)]
pub const PTE_A: usize = 1 << 6;
#[allow(dead_code)]
pub const PTE_D: usize = 1 << 7;

// 全域分頁表指標
pub static mut KERNEL_PAGE_TABLE: *mut PageTable = core::ptr::null_mut();

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(pub usize);

impl PageTableEntry {
    pub fn is_valid(&self) -> bool { (self.0 & PTE_V) != 0 }
    pub fn ppn(&self) -> usize { (self.0 >> 10) & ((1 << 44) - 1) }
    pub fn set_next_table(&mut self, ppn: usize) { self.0 = (ppn << 10) | PTE_V; }
    pub fn set_entry(&mut self, ppn: usize, flags: usize) { self.0 = (ppn << 10) | flags | PTE_V; }
}

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

pub unsafe fn map(root: &mut PageTable, vaddr: usize, paddr: usize, flags: usize) {
    let vpn2 = (vaddr >> 30) & 0x1FF;
    let vpn1 = (vaddr >> 21) & 0x1FF;
    let vpn0 = (vaddr >> 12) & 0x1FF;

    let mut pte = &mut root.entries[vpn2];
    let mut next_table: *mut PageTable;

    if !pte.is_valid() {
        let frame = alloc_frame();
        if frame == 0 { panic!("Map: Out of memory L1"); }
        pte.set_next_table(frame >> 12);
    }
    next_table = (pte.ppn() << 12) as *mut PageTable;
    let table1 = unsafe { &mut *next_table };

    pte = &mut table1.entries[vpn1];
    if !pte.is_valid() {
        let frame = alloc_frame();
        if frame == 0 { panic!("Map: Out of memory L0"); }
        pte.set_next_table(frame >> 12);
    }
    next_table = (pte.ppn() << 12) as *mut PageTable;
    let table0 = unsafe { &mut *next_table };

    pte = &mut table0.entries[vpn0];
    pte.set_entry(paddr >> 12, flags);
}

pub unsafe fn translate(root: &PageTable, vaddr: usize) -> Option<usize> {
    let vpn2 = (vaddr >> 30) & 0x1FF;
    let vpn1 = (vaddr >> 21) & 0x1FF;
    let vpn0 = (vaddr >> 12) & 0x1FF;

    let pte2 = &root.entries[vpn2];
    if !pte2.is_valid() { return None; }
    
    // [修正] 移除分號，讓它回傳 Reference
    let table1 = unsafe { &*((pte2.ppn() << 12) as *const PageTable) };

    let pte1 = &table1.entries[vpn1];
    if !pte1.is_valid() { return None; }
    
    // [修正] 移除分號
    let table0 = unsafe { &*((pte1.ppn() << 12) as *const PageTable) };

    let pte0 = &table0.entries[vpn0];
    if !pte0.is_valid() { return None; }

    Some(pte0.ppn() << 12)
}