
```
(py310) cccimac@cccimacdeiMac eos1-v0.9-elf % ./run.sh   
-----------------------------------
   EOS with ELF Loader (M-Mode)    
-----------------------------------
[OS] User Mode initialized.
Shell initialized. Type 'exec program.elf' to run.
eos> exec program.elf
Loading program.elf...
[Kernel] Loading ELF...
[Kernel] Jumping to 80200000

[UserApp] Hello, World!
[UserApp] I am running at 0x80200000
[UserApp] Calculation: 10 + 20 = 30

[Trap caught] mcause=2, mepc=80200084
User App terminated. Rebooting shell...
Shell initialized. Type 'exec program.elf' to run.
eos> ls
 - hello.txt
 - secret.txt
 - program.elf
eos> help
ls, cat <file>, exec <file>
eos> cat hello.txt
Hello! This is a text file stored in the Kernel.
Rust OS is fun!
eos> QEMU: Terminated
```
