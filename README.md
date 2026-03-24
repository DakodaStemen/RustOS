# RustOS

A minimal, from-scratch x86_64 operating system kernel written entirely in Rust. RustOS boots directly from BIOS, manages physical memory, runs async tasks in the kernel, and echoes keyboard input — all without an underlying OS, standard library, or runtime beneath it.

[![Build Status](https://github.com/DakodaStemen/RustOS/workflows/Build/badge.svg)](https://github.com/DakodaStemen/RustOS/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-nightly-brightgreen.svg)](https://www.rust-lang.org/)

---

## Table of Contents

- [What It Is](#what-it-is)
- [Architecture Overview](#architecture-overview)
- [Phase 1 — Exceptions and Interrupts](#phase-1--exceptions-and-interrupts)
- [Phase 2 — Dynamic Memory](#phase-2--dynamic-memory)
- [Phase 3 — Hardware Input](#phase-3--hardware-input)
- [Phase 4 — Async/Await in the Kernel](#phase-4--asyncawait-in-the-kernel)
- [The no_std Constraint](#the-no_std-constraint)
- [VGA Text Buffer](#vga-text-buffer)
- [Project Layout](#project-layout)
- [Getting Started](#getting-started)
- [Running](#running)
- [Running Tests](#running-tests)
- [Roadmap](#roadmap)
- [References](#references)
- [License](#license)

---

## What It Is

RustOS is a bare-metal x86_64 kernel that builds on the work of Philipp Oppermann's [Writing an OS in Rust](https://os.phil-opp.com/) series, extended with a cooperative async executor and keyboard-driven async I/O. It exists as a learning platform for low-level systems programming in Rust: how hardware works when there is nothing beneath you, how memory safety guarantees survive the absence of a standard library, and how Rust's type system can be used to make unsafe operations tractable and auditable.

The kernel:
- Boots from a BIOS-compatible bootable disk image produced by `bootimage`
- Sets up exception and interrupt handlers via an IDT
- Initializes a heap allocator to enable dynamic allocation in the kernel
- Runs a cooperative async executor that polls `Future`s natively
- Processes PS/2 keyboard input asynchronously using a lock-free queue

---

## Architecture Overview

```
┌──────────────────────────────────────┐
│              main.rs                 │  entry point, init sequence
├──────────────┬───────────────────────┤
│  interrupts  │  gdt / memory         │  hardware interface layer
├──────────────┴───────────────────────┤
│           allocator.rs               │  heap (linked_list_allocator)
├──────────────────────────────────────┤
│     task/ (executor + keyboard)      │  async runtime
├──────────────────────────────────────┤
│           vga_buffer.rs              │  output
└──────────────────────────────────────┘
```

Initialization order (strictly sequential at boot):

1. **GDT + TSS** — required before any interrupt can be handled safely
2. **IDT** — catches exceptions; double-fault handler uses a dedicated IST stack from the TSS
3. **PIC remapping** — moves hardware interrupt vectors above the CPU exception range
4. **Heap initialization** — maps physical frames and creates the heap region
5. **Keyboard interrupt enable** — hardware interrupts begin flowing
6. **Async executor launch** — runs the keyboard task loop

---

## Phase 1 — Exceptions and Interrupts

### GDT and TSS

The Global Descriptor Table defines memory segments visible to the CPU. In 64-bit mode most segmentation is inactive, but the GDT still matters for two things: privilege levels (ring 0 vs ring 3) and the Task State Segment selector.

The TSS holds the Interrupt Stack Tables (IST) — a set of known-good stack pointers the CPU switches to when handling specific exceptions. This is essential for the double-fault handler: a double fault can occur because the stack is corrupt or overflowed. Without a separate IST stack, the handler itself triple-faults, causing an unrecoverable reset. RustOS assigns IST slot 0 to the double-fault handler.

### IDT

The Interrupt Descriptor Table maps interrupt vectors to handler functions. RustOS installs handlers for:

| Vector | Exception | Handler action |
|--------|-----------|----------------|
| 3 | Breakpoint | Print register state, continue |
| 8 | Double fault | Print state, halt (diverging) |
| 14 | Page fault | Print fault address and error code, halt |
| 32 | Timer (PIT) | Acknowledge PIC, return |
| 33 | PS/2 Keyboard | Read scancode, push to queue, acknowledge PIC |

Handler functions are `extern "x86-interrupt"` functions. The `x86-interrupt` calling convention instructs the Rust compiler to preserve all caller-saved registers, including the segment registers pushed by the CPU, and to end the handler with `iretq` instead of a normal return.

---

## Phase 2 — Dynamic Memory

### Paging

x86_64 uses four-level paging: PML4 → PDPT → PD → PT → physical frame. Each level is a 4096-byte table of 512 eight-byte entries. The bootloader maps all physical memory at a fixed virtual offset (configured via `bootloader = { map_physical_memory = true }`), so any physical address can be accessed by adding the offset.

`memory.rs` implements a `BootInfoFrameAllocator` that walks the memory map provided by the bootloader and hands out free physical frames on request.

### Heap

The heap region is a contiguous range of virtual addresses backed by physical frames allocated at init time. `allocator.rs` maps a 100 KiB region and hands it to the `linked_list_allocator` crate, which implements a standard free-list allocator. Once the heap is live, `alloc` crate types work in the kernel:

```rust
use alloc::{boxed::Box, vec::Vec, rc::Rc};

let heap_value = Box::new(41);               // heap allocation
let mut v: Vec<u32> = Vec::new();            // growable vector
let rc = Rc::new(42);                        // reference-counted pointer
```

This is demonstrated in `main.rs` at boot to confirm the allocator is functional before the async executor starts.

---

## Phase 3 — Hardware Input

### PIC Remapping

The 8259 PIC is the legacy interrupt controller on x86. By default it routes hardware IRQs to CPU vectors 0–15, which collide with CPU exceptions (vectors 0–31). RustOS remaps the PIC to vectors 32–47 using the standard initialization sequence, placing hardware interrupts safely above the exception range.

```
IRQ 0 (PIT timer)    → vector 32
IRQ 1 (keyboard)     → vector 33
...
IRQ 7                → vector 39
IRQ 8 (RTC)          → vector 40
...
IRQ 15               → vector 47
```

After handling a hardware interrupt, the kernel sends an End-of-Interrupt (EOI) command to the PIC. Without EOI, the PIC stops delivering further interrupts of that line.

### PS/2 Keyboard Driver

The keyboard controller sits at I/O port `0x60`. When a key is pressed or released, it asserts IRQ 1, causing the CPU to invoke the keyboard interrupt handler. The handler:

1. Reads the raw scancode byte from port `0x60`
2. Pushes it into a lock-free `crossbeam_queue::ArrayQueue` (capacity 100)
3. Wakes the async keyboard task (via an `AtomicWaker`)
4. Sends EOI to the PIC

Scancodes are translated to `DecodedKey` values using the `pc-keyboard` crate in the async task, not in the interrupt handler. The interrupt handler does as little work as possible to minimize time spent with interrupts disabled.

---

## Phase 4 — Async/Await in the Kernel

### The Problem

A cooperative executor cannot call `thread::sleep` or `std::sync::Mutex` — there are no threads and no OS syscalls. The kernel needs a way to wait for hardware events without spinning.

### The Solution

`task/executor.rs` implements a `SimpleExecutor` that polls a set of `Future`s in a cooperative loop. Futures yield via `Poll::Pending` when they have nothing to do (e.g., waiting for the next keypress). They are woken by a waker stored in an `AtomicWaker`, which the interrupt handler calls when new data arrives.

```
interrupt fires → push scancode to queue → wake AtomicWaker
         ↓
executor polls keyboard task → task reads from queue → processes keypress
```

The keyboard task is an infinite async loop:

```rust
async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(/* ... */);
    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(c) => print!("{}", c),
                    DecodedKey::RawKey(k) => print!("{:?}", k),
                }
            }
        }
    }
}
```

`ScancodeStream::poll_next` checks the queue. If empty, it registers the waker and returns `Poll::Pending`. The executor then runs other tasks (or halts with `hlt`) until the interrupt handler wakes it.

---

## The `no_std` Constraint

`#![no_std]` at the crate root disables the Rust standard library. This means:
- No heap by default (`Box`, `Vec`, `String` require explicit allocator setup)
- No OS-level I/O (no `println!` — must write directly to the VGA buffer)
- No threads or synchronization primitives from `std::sync`
- No `panic!` handler — must define one explicitly
- No process model, no `main` function signature — must use `#[no_mangle] pub extern "C" fn _start()`

`#![no_main]` is also required because the standard entry point setup is OS-specific and unavailable.

The `alloc` crate (a subset of `std` that provides heap types) is re-enabled explicitly after the allocator is initialized.

---

## VGA Text Buffer

The VGA text buffer is a 25×80 grid of characters mapped at physical address `0xb8000`. Each character is two bytes: a code point byte and an attribute byte (foreground color, background color, blink). RustOS maps this as a `volatile` memory region using the `volatile` crate to prevent the compiler from optimizing away writes.

`vga_buffer.rs` implements a `Writer` that manages a cursor position, handles newlines, and scrolls the buffer when the cursor reaches the bottom row. A global `WRITER` protected by a `spin::Mutex` (spinlock — no OS blocking primitives available) implements `core::fmt::Write`, enabling the `print!` and `println!` macros.

---

## Project Layout

```
RustOS/
├── src/
│   ├── main.rs            Entry point, init sequence, async executor launch
│   ├── gdt.rs             GDT, TSS, IST stack setup
│   ├── interrupts.rs      IDT installation, all exception/IRQ handlers
│   ├── memory.rs          Physical frame allocator, page table mapping
│   ├── allocator.rs       Heap initialization, linked_list_allocator wrapper
│   ├── vga_buffer.rs      VGA text driver, global Writer, print!/println! macros
│   └── task/
│       ├── mod.rs          Task and TaskId types
│       ├── executor.rs     SimpleExecutor: poll loop, AtomicWaker wakeup
│       └── keyboard.rs     ScancodeStream, async keyboard processing loop
├── Cargo.toml
├── rust-toolchain.toml    Pins exact nightly version
├── Makefile               make run, make test
├── docs/
│   ├── ARCHITECTURE.md    Detailed architecture documentation
│   └── ROADMAP.md         Planned features and extension points
├── research/              Notes on interrupts, memory management, multitasking
├── CONTRIBUTING.md
├── TROUBLESHOOTING.md
└── LICENSE
```

---

## Getting Started

The repo includes a `rust-toolchain.toml` that pins the exact nightly version. Running any `cargo` command automatically installs and activates it via `rustup` — no manual `rustup override` needed.

```bash
# Install remaining components
rustup component add rust-src llvm-tools-preview

# Install the bootimage tool
cargo install bootimage --version "^0.10"

# Install QEMU (Debian/Ubuntu)
sudo apt install qemu-system-x86
# macOS
brew install qemu
```

---

## Running

```bash
# Build and run in QEMU (via Makefile)
make run

# Or manually:
cargo bootimage
qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/boot-bios-rust-os.img
```

At boot you should see:

```
Hello World!
[allocated heap values]
[async keyboard echo ready]
```

Type on the keyboard — characters appear on screen. The async executor is running.

---

## Running Tests

```bash
make test
# or
cargo test
```

Tests run in QEMU with a test runner that communicates pass/fail via I/O port `0xf4`. The bootloader handles the harness; individual tests use the `#[test_case]` attribute.

---

## Roadmap

| Feature | Status |
|---------|--------|
| GDT + TSS | Done |
| IDT (exceptions + hardware IRQs) | Done |
| Physical frame allocator | Done |
| Heap allocator | Done |
| PS/2 keyboard (async) | Done |
| Cooperative async executor | Done |
| Preemptive multitasking (APIC timer) | Planned |
| Virtual filesystem (FAT32 or custom) | Planned |
| Userspace (ring 3 + syscall interface) | Planned |
| RTL8139 ethernet driver | Planned |
| Basic TCP/IP stack | Planned |

---

## References

- Philipp Oppermann, [*Writing an OS in Rust*](https://os.phil-opp.com/) — the foundational series this kernel builds on
- OSDev Wiki: [https://wiki.osdev.org](https://wiki.osdev.org)
- Intel 64 and IA-32 Architectures Software Developer's Manual
- Peter Gutmann, [*Secure Deletion of Data from Magnetic and Solid-State Memory*](https://www.usenix.org/legacy/publications/library/proceedings/sec96/full_papers/gutmann/index.html), USENIX Security 1996

---

## License

MIT License — see [LICENSE](LICENSE).

Copyright (c) 2025 Dakoda Stemen
