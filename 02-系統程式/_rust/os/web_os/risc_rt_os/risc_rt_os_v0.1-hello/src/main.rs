#![no_std]
#![no_main]

use core::fmt::Write;
use panic_halt as _;
use riscv_rt::entry;

// UART 結構（支援 64 位元地址）
struct Uart {
    base: usize,
}

impl Uart {
    const fn new(base: usize) -> Self {
        Uart { base }
    }

    fn putc(&self, c: u8) {
        unsafe {
            // 寫入字元到 UART 傳送暫存器
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

// 程式入口點
#[entry]
fn main() -> ! {
    // UART 基地址（QEMU virt 機器的地址）
    let mut uart = Uart::new(0x10000000);
    
    // 輸出 Hello 訊息
    let _ = writeln!(uart, "========================================");
    let _ = writeln!(uart, "Hello, RISC-V 64-bit!");
    let _ = writeln!(uart, "========================================");
    let _ = writeln!(uart, "Target: riscv64gc-unknown-none-elf");
    let _ = writeln!(uart, "This is a bare-metal Rust program.");
    let _ = writeln!(uart, "Running on 64-bit RISC-V architecture!");
    let _ = writeln!(uart, "========================================");
    let _ = writeln!(uart, "");
    
    // 印出 1 到 20
    for i in 1..=20 {
        let _ = writeln!(uart, "Count: {}", i);
    }
    
    let _ = writeln!(uart, "");
    let _ = writeln!(uart, "Program completed successfully!");
    let _ = writeln!(uart, "Entering infinite loop...");
    
    // 進入無限迴圈
    loop {
        // 使用 WFI 省電
        riscv::asm::wfi();
    }
}