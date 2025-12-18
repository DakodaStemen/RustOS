# Architecture Documentation

This document explains the architecture and design decisions of the RustTest bare-metal kernel.

## Boot Process

### 1. BIOS Boot

When the computer starts, the BIOS:
1. Performs Power-On Self-Test (POST)
2. Searches for bootable devices
3. Loads the bootloader from the first sector of the disk
4. Transfers control to the bootloader

### 2. Bootloader (bootloader crate)

The `bootloader` crate (version 0.11) provides:
- **BIOS Bootloader**: Handles the initial boot process
- **Memory Setup**: Sets up paging, stack, and memory layout
- **Kernel Loading**: Loads our kernel into memory
- **Entry Point**: Calls our `kernel_main` function

The bootloader uses the `entry_point!` macro to define the kernel entry:

```rust
entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // Kernel code here
}
```

### 3. Kernel Entry

When `kernel_main` is called:
- CPU is in 64-bit mode
- Paging is enabled
- Stack is set up by bootloader
- Interrupts are disabled
- We have access to the full address space

## Memory Layout

### VGA Text Buffer

The VGA text buffer is a memory-mapped I/O region at address `0xb8000`:

```
Address: 0xb8000
Size:    4000 bytes (80 columns × 25 rows × 2 bytes per character)
Layout:  [character_byte, color_byte] pairs
```

Each screen character consists of:
- **ASCII byte** (0-255): The character to display
- **Color byte**: Foreground (lower 4 bits) + Background (upper 4 bits)

### Why 0xb8000?

This is the standard VGA text mode buffer address in x86 systems:
- **Historical**: VGA cards mapped text buffer to this address
- **Standardized**: All x86 systems use this address for compatibility
- **Always Available**: Present even in early boot stages

### Memory Safety

Accessing `0xb8000` requires `unsafe` because:
1. It's a raw pointer dereference
2. Rust can't verify the address is valid
3. We're bypassing Rust's memory safety

However, it's **safe** because:
- The address is guaranteed to exist in x86_64 systems
- The bootloader ensures we're in a valid memory context
- We use `Volatile<T>` to prevent compiler optimizations
- We only access it through safe wrapper methods

## Target: x86_64-unknown-none

### What does "unknown-none" mean?

- **x86_64**: 64-bit x86 architecture
- **unknown**: No specific vendor/OS
- **none**: No operating system

This target means:
- No standard library (`std`)
- No C runtime
- No OS abstractions
- Direct hardware access only

### Why this target?

For bare-metal programming, we need:
- **No OS dependencies**: Can't rely on OS services
- **Full control**: Direct hardware access
- **Minimal runtime**: Only `core` library available
- **Custom entry point**: No `main()` function

## Entry Point

### The `entry_point!` Macro

The bootloader provides a macro that:
1. Marks the function as the kernel entry point
2. Ensures the function isn't optimized away
3. Sets up the correct calling convention
4. Provides `BootInfo` parameter with memory layout

```rust
entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // Never returns (diverging function)
}
```

### Why `-> !` (Never Type)?

The kernel entry point never returns because:
- There's nowhere to return to (no OS to return to)
- The kernel runs indefinitely
- Returning would cause undefined behavior

## VGA Driver Architecture

### Writer Pattern

The VGA driver uses a Writer struct that:
- Manages cursor position
- Handles text wrapping
- Implements scrolling
- Provides color control

### Volatile Memory

All VGA writes use `Volatile<T>` because:
- **Compiler Optimizations**: Without volatile, compiler might optimize away writes
- **Memory-Mapped I/O**: VGA buffer is hardware, not regular memory
- **Side Effects**: Writes have visible effects (screen updates)

Example:
```rust
self.buffer.chars[row][col].write(ScreenChar { ... });
```

The `.write()` method ensures the compiler doesn't optimize this away.

### Global Writer

The global `WRITER` uses `spin::Mutex` because:
- **Thread Safety**: Allows safe access from multiple contexts (future: interrupts)
- **No Heap**: `spin::Mutex` doesn't require heap allocation
- **Lock-Free**: Uses atomic operations, no OS mutex needed

## Panic Handling

### Why Loop Forever?

In a bare-metal environment:
- **No OS**: Can't exit or abort
- **No Unwinding**: Stack unwinding requires runtime support
- **Debugging**: Infinite loop allows inspection of state

### Panic Handler Implementation

```rust
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Try to write to VGA for debugging
    // Then loop forever
    loop { core::hint::spin_loop(); }
}
```

The panic handler:
1. Attempts to write error info to VGA
2. Loops forever to prevent undefined behavior
3. Uses `spin_loop()` hint to prevent CPU from overheating

## Code Page 437

VGA text mode uses IBM Code Page 437, not UTF-8:
- **Limited Character Set**: Only 256 characters
- **Extended ASCII**: Characters 0x01-0xFF have specific meanings
- **Smiley Face**: Character 0x01 (☺) is valid in Code Page 437

Our `write_string()` method filters characters:
- Allows: `0x20..=0x7e` (printable ASCII)
- Allows: `\n` (newline)
- Replaces others with: `0xfe` (■ block character)

## Design Decisions

### Why `#![no_std]`?

- **No OS**: Standard library requires OS services
- **Minimal Runtime**: Only need `core` library
- **Full Control**: Direct hardware access

### Why `#![no_main]`?

- **Custom Entry**: Bootloader provides entry point
- **No C Runtime**: Standard `main()` requires C runtime
- **Kernel Entry**: `kernel_main` is our entry point

### Why Static WRITER?

- **Global Access**: Needed from multiple places
- **Early Initialization**: Available before heap (if we had one)
- **Thread Safety**: `spin::Mutex` provides synchronization

## Future Enhancements

Potential additions:
- **Interrupts**: IDT setup for hardware interrupts
- **Keyboard Input**: PS/2 controller driver
- **Heap Allocator**: Enable `Vec`, `String`, etc.
- **Multitasking**: Task switching and scheduling
- **File System**: Simple file system support

## References

- [Writing an OS in Rust](https://os.phil-opp.com/)
- [bootloader crate documentation](https://docs.rs/bootloader/)
- [x86_64-unknown-none target](https://doc.rust-lang.org/nightly/rustc/platform-support/x86_64-unknown-none.html)
- [VGA Text Mode](https://en.wikipedia.org/wiki/VGA_text_mode)

