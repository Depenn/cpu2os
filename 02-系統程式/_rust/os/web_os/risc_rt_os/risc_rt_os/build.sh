#!/bin/bash
set -e

# 使用 nightly 工具鏈以支持 build-std，同時編譯 core 和 alloc
cargo +nightly build --release -Z build-std=core,alloc

echo "Build complete!"
echo "Run with: qemu-system-riscv64 -machine virt -nographic -bios none -kernel target/riscv64gc-unknown-none-elf/release/risc_rt_os"