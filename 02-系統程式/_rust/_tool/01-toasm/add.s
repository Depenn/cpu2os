	.build_version macos, 11, 0
	.section	__TEXT,__text,regular,pure_instructions
	.globl	_add
	.p2align	2
_add:
	.cfi_startproc
	add	w0, w1, w0
	ret
	.cfi_endproc

.subsections_via_symbols
