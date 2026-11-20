/*
 * Memory layout of the kernel image on the QEMU `virt` machine.
 *
 * RAM starts at 0x4000_0000. For a bare-metal ELF image QEMU puts a device
 * tree blob of 1 MiB at the very start of RAM, but only if the image leaves
 * that much room. This kernel does not read the blob; it starts 1 MiB in so
 * that a later version can.
 *
 *   0x4010_0000  .text     entry code first, then everything else
 *                .rodata
 *                .data
 *                .bss      zeroed by the entry code
 *                .stack    64 KiB boot stack, grows down from __stack_top
 *                heap      4 MiB, handed to the allocator in milestone 3
 */

ENTRY(_start)

BOOT_STACK_SIZE = 64K;
HEAP_SIZE       = 4M;

SECTIONS
{
    . = 0x40100000;

    .text : {
        /* The entry point must be the first code in the image. */
        KEEP(*(.text.boot))
        *(.text .text.*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K) {
        *(.data .data.*)
    }

    /* The entry code clears .bss eight bytes at a time, so both ends are
       aligned to 16 bytes. */
    .bss (NOLOAD) : ALIGN(16) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(16);
        __bss_end = .;
    }

    /* The stack is outside .bss: nothing reads it before writing it, so
       there is no need to spend boot time on clearing it. */
    .stack (NOLOAD) : ALIGN(4K) {
        __stack_bottom = .;
        . += BOOT_STACK_SIZE;
        __stack_top = .;
    }

    . = ALIGN(4K);
    __heap_start = .;
    . += HEAP_SIZE;
    __heap_end = .;

    ASSERT(__heap_end <= 0x48000000, "the kernel image does not fit in 128 MiB of RAM")

    /DISCARD/ : {
        *(.comment)
        *(.note .note.*)
        *(.eh_frame .eh_frame_hdr)
    }
}
