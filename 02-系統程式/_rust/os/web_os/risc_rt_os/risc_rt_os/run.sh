#!/bin/bash

echo "Starting RISC-V Web Server with Network Support..."
echo ""

qemu-system-riscv64 \
  -machine virt \
  -m 128M \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/release/risc_rt_os \
  -netdev user,id=net0,hostfwd=tcp::8080-:8080 \
  -device virtio-net-device,netdev=net0,mac=52:54:00:12:34:56

# 按 Ctrl-A 然後按 X 退出 QEMU
# 在另一個終端訪問: curl http://localhost:8080