#![no_std]
#![no_main]

mod http;

use core::fmt::Write;
use panic_halt as _;
use riscv_rt::entry;

// UART 結構
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

// 模擬的網路請求
const MOCK_REQUESTS: &[&str] = &[
    "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n",
    "GET /about HTTP/1.1\r\nHost: localhost\r\n\r\n",
    "GET /api HTTP/1.1\r\nHost: localhost\r\n\r\n",
    "GET /notfound HTTP/1.1\r\nHost: localhost\r\n\r\n",
];

#[entry]
fn main() -> ! {
    let mut uart = Uart::new(0x10000000);
    
    let _ = writeln!(uart, "========================================");
    let _ = writeln!(uart, "RISC-V 64-bit Web Server");
    let _ = writeln!(uart, "========================================");
    let _ = writeln!(uart, "Starting HTTP Server on port 8080...");
    let _ = writeln!(uart, "========================================");
    let _ = writeln!(uart, "");
    
    // 模擬處理 HTTP 請求
    for (i, request) in MOCK_REQUESTS.iter().enumerate() {
        let _ = writeln!(uart, "[Request #{}]", i + 1);
        let _ = writeln!(uart, "Received: {}", request.lines().next().unwrap_or(""));
        
        // 解析請求
        if let Some(path) = http::parse_request(request.as_bytes()) {
            let _ = writeln!(uart, "Path: {}", path);
            
            // 生成響應
            let response = match path {
                "/" => {
                    http::HttpResponse::new().ok(
                        "<html>\
                         <head><title>RISC-V Web Server</title></head>\
                         <body>\
                         <h1>Welcome to RISC-V Web Server!</h1>\
                         <p>This is a bare-metal web server running on RISC-V 64-bit.</p>\
                         <ul>\
                         <li><a href='/'>Home</a></li>\
                         <li><a href='/about'>About</a></li>\
                         <li><a href='/api'>API</a></li>\
                         </ul>\
                         </body>\
                         </html>"
                    )
                },
                "/about" => {
                    http::HttpResponse::new().ok(
                        "<html>\
                         <head><title>About</title></head>\
                         <body>\
                         <h1>About This Server</h1>\
                         <p>Built with Rust and riscv-rt</p>\
                         <p>Running on QEMU RISC-V 64-bit</p>\
                         <p><a href='/'>Back to Home</a></p>\
                         </body>\
                         </html>"
                    )
                },
                "/api" => {
                    http::HttpResponse::new().ok(
                        "{\"status\":\"ok\",\"message\":\"RISC-V API\",\"version\":\"1.0\"}"
                    )
                },
                _ => http::HttpResponse::new().not_found(),
            };
            
            // 顯示響應（實際情況會通過網路發送）
            let response_bytes = response.as_bytes();
            let response_str = core::str::from_utf8(response_bytes).unwrap_or("Invalid UTF-8");
            let _ = writeln!(uart, "Response ({} bytes):", response_bytes.len());
            let _ = writeln!(uart, "{}", response_str.lines().next().unwrap_or(""));
            let _ = writeln!(uart, "");
        }
    }
    
    let _ = writeln!(uart, "========================================");
    let _ = writeln!(uart, "Server demonstration completed!");
    let _ = writeln!(uart, "In a real implementation, this would:");
    let _ = writeln!(uart, "1. Initialize VirtIO network device");
    let _ = writeln!(uart, "2. Set up TCP/IP stack (using smoltcp)");
    let _ = writeln!(uart, "3. Listen on port 8080");
    let _ = writeln!(uart, "4. Handle real network requests");
    let _ = writeln!(uart, "========================================");
    
    loop {
        riscv::asm::wfi();
    }
}