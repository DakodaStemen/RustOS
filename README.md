# RustOS - Advanced Bare-Metal Microkernel

[![Build Status](https://github.com/DakodaStemen/RustOS/workflows/Build/badge.svg)](https://github.com/DakodaStemen/RustOS/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-nightly-brightgreen.svg)](https://www.rust-lang.org/)

**RustOS** is a robust, feature-complete foundational x86_64 operating system kernel written entirely in Rust. It bypasses the standard library (`#![no_std]`) to interact directly with bare-metal hardware, demonstrating modern OS development principles with unparalleled memory safety and concurrency.

## Mission Statement
To provide a minimal yet deeply sophisticated microkernel architecture that bridges the gap between a simple bootable image and a fully concurrent, memory-safe operating system. RustOS serves as an educational and foundational platform for bare-metal Rust development.

## Key Technical Pillars

- **Memory Safety First**: Built with Rust's strict compiler guarantees. Unsafe blocks are deeply isolated, constrained, and documented.
- **Modern Concurrency**: First-class async/await support natively in the kernel without an underlying OS framework.
- **Hardware Direct**: Direct VGA buffer MMIO, 8259 PIC remapping, and raw I/O port interactions.

## Architecture & Development Phases

The project has been aggressively developed through 4 core architectural phases:

### Phase 1: System Stability (Exceptions & Interrupts)
- **GDT & TSS**: Global Descriptor Table and Task State Segment implemented to provide known-good stacks (Interrupt Stack Tables) for fatal exceptions.
- **IDT**: Interrupt Descriptor Table catches CPU exceptions (e.g., Page Faults, Double Faults, Breakpoints) to prevent triple-fault boot-looping.

### Phase 2: Dynamic Memory (Paging & Heap)
- **Paging**: Utilizes a physical memory offset strategy to safely map virtual addresses to physical frames.
- **Frame Allocator**: Custom bootinfo-based frame allocator to manage physical RAM.
- **Heap Allocator**: Integrated `linked_list_allocator` to enable Rust's `alloc` crate, supporting `Box`, `Vec`, `String`, and `Rc` directly in the kernel.

### Phase 3: Interactivity (Hardware Input)
- **PIC Remapping**: 8259 Programmable Interrupt Controller remapped to avoid collisions with CPU exceptions.
- **PS/2 Keyboard**: Interrupt-driven keyboard driver reading raw scan codes from I/O port `0x60` and translating them using `pc-keyboard`.
- **System Timer**: Hardware PIT (Programmable Interval Timer) configured for base clock ticks.

### Phase 4: Concurrency (Async/Await)
- **Cooperative Executor**: Custom `SimpleExecutor` built to poll Rust `Future`s natively.
- **Async I/O**: Keyboard inputs are streamed into a lock-free `crossbeam_queue::ArrayQueue` and processed asynchronously without blocking the kernel or spending excessive time in interrupt contexts.

## Getting Started

### Prerequisites

The repo contains a `rust-toolchain.toml` file that pins the exact nightly version. Running any `cargo` command will automatically install and use the correct toolchain — no manual `rustup default` needed.

Install the remaining required components:

```bash
# Install target & compiler source
rustup component add rust-src
rustup component add llvm-tools-preview

# Install bootimage tool
cargo install bootimage --version "^0.10"
```

### Building & Running

You can run the OS seamlessly via QEMU using the provided `Makefile`.

```bash
# Build the bootable image and run it in QEMU
make run
```

Or, manually using cargo:

```bash
# Build the OS
cargo bootimage

# Run in QEMU
qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/boot-bios-rust-os.img
```

## Demo Features
- Boots directly from BIOS.
- Dynamically allocates a Vector and Reference Counted pointer (`Rc`) to prove heap stability.
- Listens to asynchronous keyboard events and echoes typed characters instantly to the VGA buffer.

## Future Roadmap
- [ ] **Preemptive Multitasking**: Upgrade the executor to preempt threads using the APIC/PIT timer ticks.
- [ ] **Filesystem**: Simple FAT32 or custom ext2 driver support.
- [ ] **Userspace**: Ring 3 privilege transitions and system calls.
- [ ] **Networking**: Basic RTL8139 ethernet driver and TCP/IP stack.

## Contributing
Contributions, issues, and feature requests are welcome!
Check out our [Contributing Guide](CONTRIBUTING.md) and the [Architecture Deep Dive](docs/ARCHITECTURE.md).

## License
This project is licensed under the [MIT License](LICENSE).

## Acknowledgments
Heavily inspired by Philipp Oppermann's exceptional [Writing an OS in Rust](https://os.phil-opp.com/) series.
