/* Bare-metal link script for AETHER-ENCLAVE x86_64 simulation target.
 * Places .text, .rodata, .data, and .bss in a fixed physical layout. */
ENTRY(_start)

SECTIONS
{
    . = 0x100000;

    .text : {
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }

    /DISCARD/ : {
        *(.eh_frame*)
    }
}
