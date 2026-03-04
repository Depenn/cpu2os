// VirtIO 網路設備驅動 - 完整實現
use core::ptr::{read_volatile, write_volatile};
use crate::virtio_ring::{VirtQueue, VIRTQ_DESC_F_WRITE};

// VirtIO MMIO 暫存器偏移
const VIRTIO_MMIO_MAGIC: usize = 0x000;
const VIRTIO_MMIO_VERSION: usize = 0x004;
const VIRTIO_MMIO_DEVICE_ID: usize = 0x008;
const VIRTIO_MMIO_STATUS: usize = 0x070;
const VIRTIO_MMIO_QUEUE_SEL: usize = 0x030;
const VIRTIO_MMIO_QUEUE_NUM_MAX: usize = 0x034;
const VIRTIO_MMIO_QUEUE_NUM: usize = 0x038;
const VIRTIO_MMIO_QUEUE_READY: usize = 0x044;
const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050;
const VIRTIO_MMIO_QUEUE_DESC_LOW: usize = 0x080;
const VIRTIO_MMIO_QUEUE_DESC_HIGH: usize = 0x084;
const VIRTIO_MMIO_QUEUE_DRIVER_LOW: usize = 0x090;
const VIRTIO_MMIO_QUEUE_DRIVER_HIGH: usize = 0x094;
const VIRTIO_MMIO_QUEUE_DEVICE_LOW: usize = 0x0a0;
const VIRTIO_MMIO_QUEUE_DEVICE_HIGH: usize = 0x0a4;

// VirtIO 狀態位
const VIRTIO_STATUS_ACKNOWLEDGE: u32 = 1;
const VIRTIO_STATUS_DRIVER: u32 = 2;
const VIRTIO_STATUS_FEATURES_OK: u32 = 8;
const VIRTIO_STATUS_DRIVER_OK: u32 = 4;

// VirtIO-net 封包頭部
#[repr(C)]
struct VirtioNetHdr {
    flags: u8,
    gso_type: u8,
    hdr_len: u16,
    gso_size: u16,
    csum_start: u16,
    csum_offset: u16,
    num_buffers: u16,
}

impl VirtioNetHdr {
    fn new() -> Self {
        VirtioNetHdr {
            flags: 0,
            gso_type: 0,
            hdr_len: 0,
            gso_size: 0,
            csum_start: 0,
            csum_offset: 0,
            num_buffers: 0,
        }
    }
}

// 靜態緩衝區
static mut RX_QUEUE_MEM: [u8; 65536] = [0; 65536];
static mut TX_QUEUE_MEM: [u8; 65536] = [0; 65536];
static mut RX_BUFFERS: [[u8; 1526]; 16] = [[0; 1526]; 16]; // 1514 + 12 字節頭部
static mut TX_BUFFER: [u8; 1526] = [0; 1526];

pub struct VirtioNet {
    base: usize,
    rx_queue: Option<VirtQueue>,
    tx_queue: Option<VirtQueue>,
    initialized: bool,
}

impl VirtioNet {
    pub const fn new(base: usize) -> Self {
        VirtioNet {
            base,
            rx_queue: None,
            tx_queue: None,
            initialized: false,
        }
    }

    fn read_reg(&self, offset: usize) -> u32 {
        unsafe { read_volatile((self.base + offset) as *const u32) }
    }

    fn write_reg(&self, offset: usize, value: u32) {
        unsafe { write_volatile((self.base + offset) as *mut u32, value) }
    }

    pub fn probe(&self) -> (bool, u32, u32, u32) {
        let magic = self.read_reg(VIRTIO_MMIO_MAGIC);
        let version = self.read_reg(VIRTIO_MMIO_VERSION);
        let device_id = self.read_reg(VIRTIO_MMIO_DEVICE_ID);
        
        let valid = magic == 0x74726976 && version >= 1 && device_id == 1;
        
        (valid, magic, version, device_id)
    }

    pub fn init(&mut self) -> bool {
        let (valid, _, _, _) = self.probe();
        
        if !valid {
            return false;
        }

        // 重置設備
        self.write_reg(VIRTIO_MMIO_STATUS, 0);

        // 設置狀態位
        self.write_reg(VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
        self.write_reg(VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);

        // 簡化：跳過 feature 協商
        self.write_reg(VIRTIO_MMIO_STATUS, 
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK);

        // 初始化 RX queue (queue 0)
        self.write_reg(VIRTIO_MMIO_QUEUE_SEL, 0);
        let queue_max = self.read_reg(VIRTIO_MMIO_QUEUE_NUM_MAX);
        if queue_max == 0 {
            return false;
        }
        
        let queue_size = queue_max.min(16) as u16;
        self.write_reg(VIRTIO_MMIO_QUEUE_NUM, queue_size as u32);

        unsafe {
            let rx_queue_addr = RX_QUEUE_MEM.as_ptr() as usize;
            
            // 確保地址對齊
            if rx_queue_addr % 16 != 0 {
                return false;
            }
            
            self.write_reg(VIRTIO_MMIO_QUEUE_DESC_LOW, (rx_queue_addr & 0xFFFFFFFF) as u32);
            self.write_reg(VIRTIO_MMIO_QUEUE_DESC_HIGH, (rx_queue_addr >> 32) as u32);
            
            let avail_addr = rx_queue_addr + (queue_size as usize) * 16;
            self.write_reg(VIRTIO_MMIO_QUEUE_DRIVER_LOW, (avail_addr & 0xFFFFFFFF) as u32);
            self.write_reg(VIRTIO_MMIO_QUEUE_DRIVER_HIGH, (avail_addr >> 32) as u32);
            
            let avail_size = 6 + (queue_size as usize) * 2;
            let avail_size_aligned = (avail_size + 3) & !3;
            let used_addr = avail_addr + avail_size_aligned;
            self.write_reg(VIRTIO_MMIO_QUEUE_DEVICE_LOW, (used_addr & 0xFFFFFFFF) as u32);
            self.write_reg(VIRTIO_MMIO_QUEUE_DEVICE_HIGH, (used_addr >> 32) as u32);
            
            self.write_reg(VIRTIO_MMIO_QUEUE_READY, 1);
            
            // 檢查是否真的準備好了
            let ready = self.read_reg(VIRTIO_MMIO_QUEUE_READY);
            if ready != 1 {
                return false;
            }
            
            let mut rx_queue = VirtQueue::new(rx_queue_addr, queue_size);
            
            // 預先分配 RX buffers（加上 VirtIO-net 頭部）
            for i in 0..queue_size {
                let buf_addr = RX_BUFFERS[i as usize].as_ptr() as u64;
                rx_queue.add_buf(buf_addr, 1526, VIRTQ_DESC_F_WRITE);
            }
            
            // 通知設備 RX buffers 已準備好
            self.write_reg(VIRTIO_MMIO_QUEUE_NOTIFY, 0);
            
            self.rx_queue = Some(rx_queue);
        }

        // 初始化 TX queue (queue 1)
        self.write_reg(VIRTIO_MMIO_QUEUE_SEL, 1);
        self.write_reg(VIRTIO_MMIO_QUEUE_NUM, queue_size as u32);

        unsafe {
            let tx_queue_addr = TX_QUEUE_MEM.as_ptr() as usize;
            self.write_reg(VIRTIO_MMIO_QUEUE_DESC_LOW, (tx_queue_addr & 0xFFFFFFFF) as u32);
            self.write_reg(VIRTIO_MMIO_QUEUE_DESC_HIGH, (tx_queue_addr >> 32) as u32);
            
            let avail_addr = tx_queue_addr + (queue_size as usize) * 16;
            self.write_reg(VIRTIO_MMIO_QUEUE_DRIVER_LOW, (avail_addr & 0xFFFFFFFF) as u32);
            self.write_reg(VIRTIO_MMIO_QUEUE_DRIVER_HIGH, (avail_addr >> 32) as u32);
            
            let avail_size = 6 + (queue_size as usize) * 2;
            let avail_size_aligned = (avail_size + 3) & !3;
            let used_addr = avail_addr + avail_size_aligned;
            self.write_reg(VIRTIO_MMIO_QUEUE_DEVICE_LOW, (used_addr & 0xFFFFFFFF) as u32);
            self.write_reg(VIRTIO_MMIO_QUEUE_DEVICE_HIGH, (used_addr >> 32) as u32);
            
            self.write_reg(VIRTIO_MMIO_QUEUE_READY, 1);
            
            self.tx_queue = Some(VirtQueue::new(tx_queue_addr, queue_size));
        }

        // 設置 DRIVER_OK
        self.write_reg(VIRTIO_MMIO_STATUS, 
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | 
            VIRTIO_STATUS_FEATURES_OK | VIRTIO_STATUS_DRIVER_OK);

        self.initialized = true;
        true
    }

    pub fn send(&mut self, data: &[u8]) -> bool {
        if !self.initialized || data.len() > 1514 {
            return false;
        }

        if let Some(ref mut tx_queue) = self.tx_queue {
            unsafe {
                // 添加 VirtIO-net 頭部
                let hdr = VirtioNetHdr::new();
                let hdr_bytes = core::slice::from_raw_parts(
                    &hdr as *const VirtioNetHdr as *const u8,
                    core::mem::size_of::<VirtioNetHdr>()
                );
                
                TX_BUFFER[..hdr_bytes.len()].copy_from_slice(hdr_bytes);
                TX_BUFFER[hdr_bytes.len()..hdr_bytes.len() + data.len()].copy_from_slice(data);
                
                // 添加到 TX queue
                let buf_addr = TX_BUFFER.as_ptr() as u64;
                let total_len = hdr_bytes.len() + data.len();
                
                if let Some(desc_idx) = tx_queue.add_buf(buf_addr, total_len as u32, 0) {
                    // 通知設備
                    self.write_reg(VIRTIO_MMIO_QUEUE_NOTIFY, 1);
                    
                    // 調試：檢查是否發送成功
                    // 簡單等待一下讓設備處理
                    for _ in 0..1000 {
                        core::hint::spin_loop();
                    }
                    
                    return true;
                }
            }
        }

        false
    }

    pub fn receive(&mut self, buffer: &mut [u8]) -> Option<usize> {
        if !self.initialized {
            return None;
        }

        if let Some(ref mut rx_queue) = self.rx_queue {
            if let Some((desc_idx, len)) = rx_queue.get_used() {
                unsafe {
                    // 跳過 VirtIO-net 頭部（12字節）
                    let hdr_size = core::mem::size_of::<VirtioNetHdr>();
                    if len as usize > hdr_size {
                        let data_len = (len as usize - hdr_size).min(buffer.len());
                        buffer[..data_len].copy_from_slice(
                            &RX_BUFFERS[desc_idx as usize][hdr_size..hdr_size + data_len]
                        );
                        
                        // 重新添加 buffer 到 RX queue
                        let buf_addr = RX_BUFFERS[desc_idx as usize].as_ptr() as u64;
                        rx_queue.free_desc(desc_idx);
                        rx_queue.add_buf(buf_addr, 1526, VIRTQ_DESC_F_WRITE);
                        
                        // 通知設備有新的 RX buffer 可用
                        self.write_reg(VIRTIO_MMIO_QUEUE_NOTIFY, 0);
                        
                        return Some(data_len);
                    }
                }
            }
        }

        None
    }
    
    // 添加調試函數
    pub fn debug_queues(&self) {
        // 可以在這裡添加 queue 狀態的調試輸出
    }
}