OUTPUT_ARCH(loongarch)
ENTRY(_start)

BASE_ADDRESS = 0x10000;
USER_HEAP_SIZE = 32768;

SECTIONS
{
    . = BASE_ADDRESS;
    . = ALIGN(4K);
    .text ALIGN(4K): {
        *(.text.entry)
        *(.text .text.*)
    }
    . = ALIGN(4K);
    .rodata ALIGN(4K): {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);
    .data ALIGN(4K): {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(4K);
    .got ALIGN(4K): {
        *(.got .got.*)
    }
    . = ALIGN(4K);
    .bss ALIGN(4K): {
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        PROVIDE(HEAP_START = .);  /* 定义 HEAP_START 符号 */
        . += USER_HEAP_SIZE;      /* 为 HEAP_SPACE 预留空间 */
        PROVIDE(HEAP_END = .);    /* 定义 HEAP_END 符号 */
    }
    /DISCARD/ : {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.debug*)
        *(.rela*)
        *(.dynamic*)
    }
}