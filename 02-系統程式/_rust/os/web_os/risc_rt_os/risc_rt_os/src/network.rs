// 網路協定棧封裝 - 支持真實收發（帶調試）
use crate::virtio::VirtioNet;
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};
use core::fmt::Write;

// 簡單的調試輸出
struct DebugUart;

impl DebugUart {
    fn putc(c: u8) {
        unsafe {
            core::ptr::write_volatile(0x10000000 as *mut u8, c);
        }
    }
    
    fn puts(s: &str) {
        for b in s.bytes() {
            Self::putc(b);
        }
    }
}

impl Write for DebugUart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        DebugUart::puts(s);
        Ok(())
    }
}

// VirtIO 設備包裝器
pub struct VirtioDevice {
    virtio: VirtioNet,
    rx_buffer: [u8; 1514],
    rx_len: usize,
    rx_count: usize,
    tx_count: usize,
}

impl VirtioDevice {
    pub fn new(base: usize) -> Option<Self> {
        let mut virtio = VirtioNet::new(base);
        if virtio.init() {
            Some(VirtioDevice {
                virtio,
                rx_buffer: [0; 1514],
                rx_len: 0,
                rx_count: 0,
                tx_count: 0,
            })
        } else {
            None
        }
    }

    fn try_receive(&mut self) -> bool {
        if let Some(len) = self.virtio.receive(&mut self.rx_buffer) {
            self.rx_len = len;
            self.rx_count += 1;
            
            let mut uart = DebugUart;
            let _ = writeln!(uart, "[NET] RX packet #{}: {} bytes", self.rx_count, len);
            
            // 顯示前幾個字節（以太網頭）
            if len >= 14 {
                let _ = write!(uart, "      Eth: ");
                for i in 0..14.min(len) {
                    let _ = write!(uart, "{:02x} ", self.rx_buffer[i]);
                }
                let _ = writeln!(uart, "");
            }
            
            true
        } else {
            false
        }
    }
}

// RxToken 實現
pub struct VirtioRxToken {
    buffer: [u8; 1514],
    length: usize,
}

impl RxToken for VirtioRxToken {
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut uart = DebugUart;
        let _ = writeln!(uart, "[NET] RxToken consuming {} bytes", self.length);
        f(&mut self.buffer[..self.length])
    }
}

// TxToken 實現
pub struct VirtioTxToken<'a> {
    device: &'a mut VirtioNet,
    tx_count: &'a mut usize,
}

impl<'a> TxToken for VirtioTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = [0u8; 1514];
        let result = f(&mut buffer[..len]);
        
        *self.tx_count += 1;
        
        let mut uart = DebugUart;
        let _ = writeln!(uart, "[NET] TX packet #{}: {} bytes", self.tx_count, len);
        
        // 顯示前幾個字節
        if len >= 14 {
            let _ = write!(uart, "      Eth: ");
            for i in 0..14.min(len) {
                let _ = write!(uart, "{:02x} ", buffer[i]);
            }
            let _ = writeln!(uart, "");
        }
        
        // 真正發送封包
        if self.device.send(&buffer[..len]) {
            let _ = writeln!(uart, "[NET] TX sent successfully");
        } else {
            let _ = writeln!(uart, "[NET] TX FAILED!");
        }
        
        result
    }
}

impl Device for VirtioDevice {
    type RxToken<'a> = VirtioRxToken where Self: 'a;
    type TxToken<'a> = VirtioTxToken<'a> where Self: 'a;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        static mut RX_CALL_COUNT: usize = 0;
        unsafe {
            RX_CALL_COUNT += 1;
            if RX_CALL_COUNT % 100000 == 0 {
                let mut uart = DebugUart;
                let _ = writeln!(uart, "[NET] Device::receive() called {} times", RX_CALL_COUNT);
            }
        }
        
        // 嘗試從設備接收封包
        if self.try_receive() {
            let rx_token = VirtioRxToken {
                buffer: self.rx_buffer,
                length: self.rx_len,
            };
            
            let tx_token = VirtioTxToken {
                device: &mut self.virtio,
                tx_count: &mut self.tx_count,
            };
            
            Some((rx_token, tx_token))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        static mut TX_CALL_COUNT: usize = 0;
        unsafe {
            TX_CALL_COUNT += 1;
            if TX_CALL_COUNT % 100000 == 0 {
                let mut uart = DebugUart;
                let _ = writeln!(uart, "[NET] Device::transmit() called {} times", TX_CALL_COUNT);
            }
        }
        
        Some(VirtioTxToken {
            device: &mut self.virtio,
            tx_count: &mut self.tx_count,
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1514;
        caps.medium = Medium::Ethernet;
        caps
    }
}

// 網路堆棧管理器
pub struct NetworkStack {
    pub iface: Interface,
    pub device: VirtioDevice,
}

impl NetworkStack {
    pub fn new(mut device: VirtioDevice) -> Self {
        let mut uart = DebugUart;
        let _ = writeln!(uart, "[NET] Configuring network stack...");
        
        // 配置網路接口
        let mac_addr = EthernetAddress([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
        let ip_addr = IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24);
        
        let _ = writeln!(uart, "[NET] MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", 
                        mac_addr.0[0], mac_addr.0[1], mac_addr.0[2], 
                        mac_addr.0[3], mac_addr.0[4], mac_addr.0[5]);
        
        let config = Config::new(mac_addr.into());
        let mut iface = Interface::new(config, &mut device, Instant::from_millis(0));
        
        iface.update_ip_addrs(|ip_addrs| {
            ip_addrs.push(ip_addr).unwrap();
        });

        // 設置默認路由
        iface
            .routes_mut()
            .add_default_ipv4_route(Ipv4Address::new(10, 0, 2, 2))
            .unwrap();

        let _ = writeln!(uart, "[NET] Network stack configured");

        NetworkStack {
            iface,
            device,
        }
    }

    pub fn poll(&mut self, timestamp: Instant, sockets: &mut SocketSet) {
        static mut POLL_COUNT: usize = 0;
        unsafe {
            POLL_COUNT += 1;
            if POLL_COUNT % 1000000 == 0 {
                let mut uart = DebugUart;
                let _ = writeln!(uart, "[NET] poll() called {} times", POLL_COUNT);
            }
        }
        
        let _ = self.iface.poll(timestamp, &mut self.device, sockets);
    }
}