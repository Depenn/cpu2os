
```
(py310) cccimac@cccimacdeiMac eos1-v0.5-usermode % ./run.sh
-----------------------------------
   EOS with User Mode & Syscalls   
-----------------------------------
[OS] Switching to User Mode...
Task 1 [User]: count = 0, addr = 0x800281a8
Task 1 [User]: count = 1, addr = 0x800281a8
Task 2 [User]: count = 0, addr = 0x8002c1a8
Task 2 [User]: count = 1, addr = 0x8002c1a8
Task 1 [User]: count = 2, addr = 0x800281a8
Task 2 [User]: count = 2, addr = 0x8002c1a8
Task 1 [User]: count = 3, addr = 0x800281a8
Task 2 [User]: count = 3, addr = 0x8002c1a8
Task 1 [User]: count = 4, addr = 0x800281a8
Task 2 [User]: count = 4, addr = 0x8002c1a8
Task 1 [User]: count = 5, addr = 0x800281a8
QEMU: Terminated
```
