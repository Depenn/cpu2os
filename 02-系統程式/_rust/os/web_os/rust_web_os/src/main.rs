#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use core::ptr::NonNull;
use riscv_rt::entry;
use panic_halt as _;

use virtio_drivers::{Hal, BufferDirection};
use virtio_drivers::transport::mmio::{MmioTransport, VirtIOHeader};
use virtio_drivers::device::net::VirtIONet;

use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::socket::tcp;
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address};
use smoltcp::phy::{Device, RxToken, TxToken};

// --- UART Console (Safe & Robust) ---
mod uart {
    use core::fmt::{self, Write};
    
    const UART0: *mut u8 = 0x1000_0000 as *mut u8;
    const UART0_LSR: *mut u8 = 0x1000_0005 as *mut u8;

    pub struct Uart;

    impl Uart {
        pub fn putc(&self, c: u8) {
            unsafe {
                // 等待 UART TX 緩衝區為空 (LSR bit 5)
                // 在 QEMU 其實可以省略，但為了穩健性加上
                while (UART0_LSR.read_volatile() & 0x20) == 0 {}
                UART0.write_volatile(c);
            }
        }
    }

    impl Write for Uart {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for c in s.bytes() {
                // 處理換行符號，讓輸出更好看
                if c == b'\n' {
                    self.putc(b'\r');
                }
                self.putc(c);
            }
            Ok(())
        }
    }

    pub fn print(args: fmt::Arguments) {
        let mut uart = Uart;
        uart.write_fmt(args).unwrap();
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::uart::print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

// --- Heap Memory ---
#[global_allocator]
static HEAP: embedded_alloc::Heap = embedded_alloc::Heap::empty();

// --- HAL ---
struct SystemHal;

unsafe impl Hal for SystemHal {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (usize, NonNull<u8>) {
        static mut HEAP_PTR: usize = 0;
        unsafe {
            if HEAP_PTR == 0 {
                HEAP_PTR = 0x8400_0000;
            }
            let paddr = HEAP_PTR;
            HEAP_PTR += pages * 4096;
            let vaddr = NonNull::new(paddr as *mut u8).expect("DMA alloc failed");
            core::ptr::write_bytes(vaddr.as_ptr(), 0, pages * 4096);
            (paddr, vaddr)
        }
    }

    unsafe fn dma_dealloc(_paddr: usize, _vaddr: NonNull<u8>, _pages: usize) -> i32 { 0 }
    unsafe fn mmio_phys_to_virt(paddr: usize, _size: usize) -> NonNull<u8> {
        NonNull::new(paddr as *mut u8).unwrap()
    }
    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> usize {
        buffer.as_ptr() as *mut u8 as usize
    }
    unsafe fn unshare(_paddr: usize, _buffer: NonNull<[u8]>, _direction: BufferDirection) {}
}

const VIRTIO0: usize = 0x10001000;
const QUEUE_SIZE: usize = 32;
type VirtioNetDevice = VirtIONet<SystemHal, MmioTransport, QUEUE_SIZE>;

// --- Trap Handler (除錯用) ---
// 當發生錯誤(如存取非法記憶體)時，CPU會跳到這裡
#[export_name = "ExceptionHandler"]
fn custom_exception_handler(_trap_frame: &riscv_rt::TrapFrame) -> ! {
    // 直接操作 UART 避免依賴其他部分
    let uart = 0x1000_0000 as *mut u8;
    unsafe { 
        *uart = b'T'; *uart = b'R'; *uart = b'A'; *uart = b'P'; *uart = b'!'; *uart = b'\n';
    }
    loop {}
}

// --- Entry Point ---
#[entry]
fn main() -> ! {
    // 1. 極早期測試：直接寫入 UART 證明活著
    unsafe {
        *(0x1000_0000 as *mut u8) = b'!'; 
        *(0x1000_0000 as *mut u8) = b'\n';
    }

    println!(">>> Hello! OS is starting...");

    // 2. 初始化 Heap
    {
        use core::ptr;
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024 * 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { 
            let heap_start = ptr::addr_of_mut!(HEAP_MEM) as usize;
            HEAP.init(heap_start, HEAP_SIZE);
        }
        println!(">>> Heap Initialized.");
    }

    // 3. 初始化 VirtIO
    println!(">>> Init VirtIO...");
    let header_ptr = NonNull::new(VIRTIO0 as *mut VirtIOHeader).unwrap();
    let transport = unsafe { MmioTransport::new(header_ptr).expect("Failed transport") };
    
    let net = VirtioNetDevice::new(transport, 2048).expect("Failed VirtIO Net");
    
    let mac = net.mac_address();
    println!(">>> MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", 
             mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);

    // 4. 設定網路
    let mut config = Config::new(EthernetAddress::from_bytes(&mac).into());
    config.random_seed = 0x1234;

    let mut device = VirtIoWrapper { driver: net };
    let mut iface = Interface::new(config, &mut device, Instant::from_millis(0));

    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs.push(IpCidr::new(Ipv4Address::new(10, 0, 2, 15).into(), 24)).unwrap();
    });
    iface.routes_mut().add_default_ipv4_route(Ipv4Address::new(10, 0, 2, 2)).unwrap();

    println!(">>> IP Configured. Listening on port 80.");

    let mut sockets = SocketSet::new(vec![]);
    let tcp_socket = tcp::Socket::new(
        tcp::SocketBuffer::new(vec![0; 4096]),
        tcp::SocketBuffer::new(vec![0; 4096])
    );
    let tcp_handle = sockets.add(tcp_socket);

    let mut counter: u64 = 0;
    
    loop {
        counter += 1;
        let timestamp = Instant::from_millis((counter / 1000) as i64);

        iface.poll(timestamp, &mut device, &mut sockets);

        let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);
        
        if !socket.is_open() {
            socket.listen(80).ok();
        }

        if socket.can_recv() {
            let mut buffer = [0u8; 1024];
            if let Ok(size) = socket.recv_slice(&mut buffer) {
                let request = core::str::from_utf8(&buffer[..size]).unwrap_or("");
                if request.starts_with("GET /") {
                    println!(">>> Got Request!");
                    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<h1>It Works!</h1>";
                    socket.send_slice(response.as_bytes()).ok();
                    socket.close();
                }
            }
        }
    }
}

// --- VirtIO Glue ---
struct VirtIoWrapper { driver: VirtioNetDevice }
impl Device for VirtIoWrapper {
    type RxToken<'a> = MyRxToken;
    type TxToken<'a> = MyTxToken<'a>;
    fn receive(&mut self, _t: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        match self.driver.receive() {
            Ok(buf) => {
                let mut v = vec![0u8; buf.packet_len()];
                v.copy_from_slice(buf.packet());
                Some((MyRxToken(v), MyTxToken(&mut self.driver)))
            },
            _ => None
        }
    }
    fn transmit(&mut self, _t: Instant) -> Option<Self::TxToken<'_>> {
        Some(MyTxToken(&mut self.driver))
    }
    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        smoltcp::phy::DeviceCapabilities::default()
    }
}

struct MyRxToken(alloc::vec::Vec<u8>);
impl RxToken for MyRxToken {
    fn consume<R, F>(mut self, f: F) -> R where F: FnOnce(&mut [u8]) -> R { f(&mut self.0) }
}
struct MyTxToken<'a>(&'a mut VirtioNetDevice);
impl<'a> TxToken for MyTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R where F: FnOnce(&mut [u8]) -> R {
        let mut b = self.0.new_tx_buffer(len);
        let r = f(b.packet_mut());
        self.0.send(b).ok();
        r
    }
}

// --- Hooks ---
#[export_name = "_mp_hook"] pub extern "C" fn mp_hook(_: usize) -> bool { true }
#[export_name = "__pre_init"] pub extern "C" fn pre_init() {}
#[export_name = "_setup_interrupts"] pub extern "C" fn setup_interrupts() {}