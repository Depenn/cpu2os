use core::mem::size_of;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElfHeader {
    pub magic: [u8; 4],
    pub class: u8,
    pub endian: u8,
    pub version: u8,
    pub os_abi: u8,
    pub abi_version: u8,
    pub pad: [u8; 7],
    pub type_: u16,
    pub machine: u16,
    pub version2: u32,
    pub entry: u64,
    pub phoff: u64,
    pub shoff: u64,
    pub flags: u32,
    pub ehsize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProgramHeader {
    pub type_: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

pub unsafe fn load_elf(data: &[u8]) -> Option<u64> {
    if data.len() < size_of::<ElfHeader>() {
        return None;
    }

    // [修正] 包裹 unsafe
    let header = unsafe { &*(data.as_ptr() as *const ElfHeader) };

    if header.magic != [0x7f, 0x45, 0x4c, 0x46] {
        return None;
    }

    if header.machine != 0xF3 {
        return None;
    }

    // [修正] 包裹 unsafe
    let ph_table_ptr = unsafe { data.as_ptr().add(header.phoff as usize) };
    
    for i in 0..header.phnum {
        // [修正] 包裹 unsafe
        let ph_ptr = unsafe { ph_table_ptr.add((i as usize) * (header.phentsize as usize)) };
        let ph = unsafe { &*(ph_ptr as *const ProgramHeader) };

        if ph.type_ == 1 {
            if ph.vaddr < 0x80000000 {
                return None; 
            }

            let dest = ph.vaddr as *mut u8;
            
            // [修正] 包裹 unsafe
            let src = unsafe { data.as_ptr().add(ph.offset as usize) };
            
            if ph.offset + ph.filesz > data.len() as u64 {
                return None;
            }

            if ph.filesz > 0 {
                // [修正] 包裹 unsafe
                unsafe { core::ptr::copy_nonoverlapping(src, dest, ph.filesz as usize); }
            }

            if ph.memsz > ph.filesz {
                // [修正] 包裹 unsafe
                unsafe {
                    let zero_start = dest.add(ph.filesz as usize);
                    let zero_len = (ph.memsz - ph.filesz) as usize;
                    core::ptr::write_bytes(zero_start, 0, zero_len);
                }
            }
        }
    }

    Some(header.entry)
}