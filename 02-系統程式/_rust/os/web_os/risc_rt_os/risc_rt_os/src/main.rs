#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

mod allocator;
mod http;
mod virtio_ring;
mod virtio;
mod network;

use core::fmt::Write;
use panic_halt as _;
use riscv_rt::entry;
use smoltcp::iface::{SocketSet, SocketStorage};
use smoltcp::socket::tcp;
use smoltcp::storage::RingBuffer;

struct Uart {
    base: usize,
}

impl Uart {
    const fn new(base: usize) -> Self {
        Uart { base }
    }

    fn putc(&self, c: u8) {
        unsafe {
            core::ptr::write_volatile(self.base as *mut u8, c);
        }
    }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.putc(byte);
        }
        Ok(())
    }
}

// VirtIO 網路設備的可能 MMIO 基地址（QEMU virt 機器）
const VIRTIO_BASES: [usize; 8] = [
    0x10001000, // VirtIO device 0
    0x10002000, // VirtIO device 1
    0x10003000, // VirtIO device 2
    0x10004000, // VirtIO device 3
    0x10005000, // VirtIO device 4
    0x10006000, // VirtIO device 5
    0x10007000, // VirtIO device 6
    0x10008000, // VirtIO device 7
];

// 靜態緩衝區
static mut TCP_RX_DATA: [u8; 2048] = [0; 2048];
static mut TCP_TX_DATA: [u8; 2048] = [0; 2048];
static mut SOCKET_STORAGE: [SocketStorage<'static>; 1] = [SocketStorage::EMPTY; 1];

#[entry]
fn main() -> ! {
    let mut uart = Uart::new(0x10000000);
    
    let _ = writeln!(uart, "========================================");
    let _ = writeln!(uart, "  RISC-V Web Server with Real Network");
    let _ = writeln!(uart, "========================================");
    let _ = writeln!(uart, "");
    
    // 初始化 VirtIO 網路設備
    let _ = writeln!(uart, "[1/4] Initializing VirtIO network device...");
    let _ = writeln!(uart, "      Scanning for VirtIO devices...");
    
    let mut virtio_device = None;
    
    for (i, &base) in VIRTIO_BASES.iter().enumerate() {
        let test_device = virtio::VirtioNet::new(base);
        let (valid, magic, version, device_id) = test_device.probe();
        
        let _ = writeln!(uart, "      [{}] 0x{:08x}: magic=0x{:08x}, ver={}, dev_id={}", 
                        i, base, magic, version, device_id);
        
        if valid {
            let _ = writeln!(uart, "          ✓ Found VirtIO network device!");
            virtio_device = network::VirtioDevice::new(base);
            break;
        }
    }
    
    let virtio_device = match virtio_device {
        Some(dev) => {
            let _ = writeln!(uart, "      ✓ VirtIO network device initialized");
            dev
        }
        None => {
            let _ = writeln!(uart, "      ✗ No VirtIO network device found");
            let _ = writeln!(uart, "");
            let _ = writeln!(uart, "Make sure QEMU is started with:");
            let _ = writeln!(uart, "  -netdev user,id=net0");
            let _ = writeln!(uart, "  -device virtio-net-device,netdev=net0");
            let _ = writeln!(uart, "");
            let _ = writeln!(uart, "Running in demo mode without real network...");
            
            // 降級到演示模式
            run_demo_mode(&mut uart);
        }
    };
    
    // 創建網路堆棧
    let _ = writeln!(uart, "[2/4] Setting up TCP/IP stack...");
    let mut network_stack = network::NetworkStack::new(virtio_device);
    let _ = writeln!(uart, "      ✓ IP: 10.0.2.15/24");
    let _ = writeln!(uart, "      ✓ Gateway: 10.0.2.2");
    
    // 創建 TCP socket
    let _ = writeln!(uart, "[3/4] Creating TCP socket...");
    
    unsafe {
        let tcp_rx_buffer = RingBuffer::new(&mut TCP_RX_DATA[..]);
        let tcp_tx_buffer = RingBuffer::new(&mut TCP_TX_DATA[..]);
        let tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);
        
        let mut sockets = SocketSet::new(&mut SOCKET_STORAGE[..]);
        let tcp_handle = sockets.add(tcp_socket);
        
        let _ = writeln!(uart, "      ✓ TCP socket created");
        
        // 監聽端口 8080
        let _ = writeln!(uart, "[4/4] Starting HTTP server on port 8080...");
        {
            let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);
            socket.listen(8080).unwrap();
        }
        let _ = writeln!(uart, "      ✓ Server listening on 10.0.2.15:8080");
        
        let _ = writeln!(uart, "");
        let _ = writeln!(uart, "========================================");
        let _ = writeln!(uart, "  Server is ready!");
        let _ = writeln!(uart, "  Access from host: http://localhost:8080");
        let _ = writeln!(uart, "========================================");
        let _ = writeln!(uart, "");
        
        // 主循環
        let mut counter = 0u64;
        loop {
            // 更新網路堆棧
            let timestamp = smoltcp::time::Instant::from_millis(counter as i64);
            network_stack.poll(timestamp, &mut sockets);
            
            // 處理 TCP 連接
            let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);
            
            if socket.can_recv() {
                let _ = writeln!(uart, "[Connection] Received request");
                
                // 讀取請求
                let mut buffer = [0u8; 2048];
                if let Ok(size) = socket.recv_slice(&mut buffer) {
                    let request = core::str::from_utf8(&buffer[..size]).unwrap_or("");
                    let _ = writeln!(uart, "  Request: {}", request.lines().next().unwrap_or(""));
                    
                    // 生成響應
                    let response = if let Some(path) = http::parse_request(&buffer[..size]) {
                        match path {
                            "/" => http::HttpResponse::new().ok("<h1>RISC-V Web Server</h1><p>Running with real TCP/IP!</p>"),
                            _ => http::HttpResponse::new().not_found(),
                        }
                    } else {
                        http::HttpResponse::new().not_found()
                    };
                    
                    // 發送響應
                    if socket.can_send() {
                        let _ = socket.send_slice(response.as_bytes());
                        let _ = writeln!(uart, "  Response sent ({} bytes)", response.as_bytes().len());
                    }
                    
                    socket.close();
                }
            }
            
            counter = counter.wrapping_add(1);
            
            // 每秒輸出一次狀態
            if counter % 1000000 == 0 {
                let _ = writeln!(uart, "[Status] Server running... ({}s)", counter / 1000000);
            }
        }
    }
}

fn run_demo_mode(uart: &mut Uart) -> ! {
    let _ = writeln!(uart, "");
    let _ = writeln!(uart, "Running demo mode...");
    
    let test_requests = [
        "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "GET /about HTTP/1.1\r\nHost: localhost\r\n\r\n",
    ];
    
    for (i, request) in test_requests.iter().enumerate() {
        let _ = writeln!(uart, "\n[Demo Request #{}]", i + 1);
        let _ = writeln!(uart, ">>> {}", request.lines().next().unwrap_or(""));
        
        if let Some(path) = http::parse_request(request.as_bytes()) {
            let response = match path {
                "/" => http::HttpResponse::new().ok("<h1>Demo Mode</h1>"),
                _ => http::HttpResponse::new().not_found(),
            };
            let _ = writeln!(uart, "<<< {} bytes", response.as_bytes().len());
        }
    }
    
    loop {
        riscv::asm::wfi();
    }
}