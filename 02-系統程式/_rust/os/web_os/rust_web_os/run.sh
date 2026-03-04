#!/bin/sh
set -e

#!/bin/sh
set -e

qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -kernel target/riscv64gc-unknown-none-elf/release/riscv-web-os