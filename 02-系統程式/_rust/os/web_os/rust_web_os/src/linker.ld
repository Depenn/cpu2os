OUTPUT_ARCH(riscv)
ENTRY(_start)

/*
 * OpenSBI occupies 0x8000_0000
 * S-mode kernel starts at 0x8020_0000
 */
MEMORY
{
  /* 避開 OpenSBI (前 2MB) */
  RAM (wxa) : ORIGIN = 0x80200000, LENGTH = 126M
}

/* Required by riscv-rt */
PROVIDE(_max_hart_id = 0);
PROVIDE(_hart_stack_size = 2K);
PROVIDE(_stack_start = ORIGIN(RAM) + LENGTH(RAM));

SECTIONS
{
  /* -------------------------------------------------- */
  /* Text */
  /* -------------------------------------------------- */
  .text : {
    PROVIDE(_stext = .);
    *(.text.init)
    *(.text .text.*)
    PROVIDE(_etext = .);
  } > RAM

  /* -------------------------------------------------- */
  /* Read-only data */
  /* -------------------------------------------------- */
  .rodata : {
    . = ALIGN(4);
    *(.rodata .rodata.*)
  } > RAM

  /* -------------------------------------------------- */
  /* Data */
  /* -------------------------------------------------- */
  .data : {
    . = ALIGN(8);
    PROVIDE(_sdata = .);
    *(.data .data.*)
    PROVIDE(_edata = .);
  } > RAM

  /* Load address of .data */
  PROVIDE(_sidata = LOADADDR(.data));

  /* -------------------------------------------------- */
  /* Global Pointer (★ 關鍵 ★)
   * 必須放在 small data / bss 中央附近
   */
  . = ALIGN(8);
  PROVIDE(__global_pointer$ = . + 0x800);

  /* -------------------------------------------------- */
  /* BSS */
  /* -------------------------------------------------- */
  .bss (NOLOAD) : {
    . = ALIGN(8);
    PROVIDE(_sbss = .);
    *(.bss .bss.*)
    *(COMMON)
    PROVIDE(_ebss = .);
  } > RAM

  /* -------------------------------------------------- */
  /* Discard */
  /* -------------------------------------------------- */
  /DISCARD/ : {
    *(.eh_frame)
    *(.eh_frame_hdr)
  }
}
