#!/bin/bash

qemu-system-riscv64 \
  -machine virt \
  -m 128M \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/release/risc_rt_os

# 按 Ctrl-A 然後按 X 退出 QEMU