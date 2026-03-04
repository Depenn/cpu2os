#!/bin/bash
set -e

# 使用 nightly 工具鏈以支持 build-std
cargo +nightly build --release -Z build-std=core

echo "Build complete!"
echo "Run with: qemu-system-riscv64 -machine virt -nographic -bios none -kernel target/riscv64gc-unknown-none-elf/release/risc_rt_os"