qemu-system-riscv64 \
    -machine virt \
    -cpu rv64 \
    -smp 1 \
    -m 128M \
    -nographic \
    -bios default \
    -kernel target/riscv64gc-unknown-none-elf/release/riscv-web-os \
    -netdev user,id=net0,hostfwd=tcp::8080-:80 \
    -device virtio-net-device,netdev=net0