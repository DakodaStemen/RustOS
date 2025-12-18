# 🦀 RustTest - Bare-Metal OS Kernel

[![Build Status](https://github.com/DakodaStemen/RustOS/workflows/Build/badge.svg)](https://github.com/DakodaStemen/RustOS/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-nightly-brightgreen.svg)](https://www.rust-lang.org/)

A minimal, bootable x86_64 operating system kernel written in Rust. This project demonstrates bare-metal programming with `#![no_std]`, direct hardware access, and VGA text mode output.

## 🎯 What is This?

This is a **freestanding** Rust kernel that boots on x86_64 hardware (or QEMU). It bypasses the standard library and directly interacts with hardware:

- **No Standard Library**: Uses only `core` library (`#![no_std]`)
- **Direct Hardware Access**: Writes directly to VGA text buffer at `0xb8000`
- **Bootloader Integration**: Uses `bootloader` crate for BIOS boot
- **Memory Safety**: Demonstrates safe Rust patterns for unsafe hardware operations

## 🚀 Quick Start

### Prerequisites

```bash
# Install Rust nightly
rustup toolchain install nightly
rustup override set nightly

# Install target
rustup target add x86_64-unknown-none

# Install LLVM tools
rustup component add llvm-tools-preview

# Install bootimage
cargo install bootimage --version "^0.11"
```

### One-Command Run

```bash
make run
```

Or manually:

```bash
# Build the bootable image
cargo bootimage

# Run in QEMU
qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/boot-bios-RustTest.img
```

## 📸 Demo

![Kernel Boot Demo](demo.gif)

*The kernel boots and displays a smiley face (☺) with "Hello from Rust OS!" message*

To record your own demo:
```bash
# Using QEMU with curses (for terminal recording)
qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/boot-bios-RustTest.img -display curses

# Or use asciinema
asciinema rec demo.cast
```

## ✨ Features

- ✅ **Bare-Metal Boot**: Boots from BIOS using bootloader crate
- ✅ **VGA Text Mode**: Direct memory-mapped I/O to VGA buffer
- ✅ **Safe Unsafe Code**: Well-documented unsafe blocks with safety justifications
- ✅ **Panic Handling**: Custom panic handler with VGA output for debugging
- ✅ **Volatile Memory**: Prevents compiler optimizations on hardware writes
- ✅ **No Heap**: Stack-only allocations, no allocator required

## 🏗️ Architecture

This kernel demonstrates several key concepts:

- **Entry Point**: Uses `bootloader::entry_point!` macro to define kernel entry
- **Memory Layout**: VGA text buffer at `0xb8000`, stack provided by bootloader
- **No Standard Library**: All code uses only `core` library
- **Hardware Abstraction**: VGA driver with Writer pattern

For detailed architecture documentation, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## 📁 Project Structure

```
RustTest/
├── src/
│   ├── main.rs          # Kernel entry point and initialization
│   └── vga_buffer.rs    # VGA text mode driver
├── .cargo/
│   └── config.toml     # Build target configuration
├── Cargo.toml          # Project manifest
├── rust-toolchain.toml # Nightly toolchain specification
├── Makefile            # Build automation
└── docs/
    └── ARCHITECTURE.md # Technical documentation
```

## 🛠️ Development

### Build Commands

```bash
make build    # Build bootable image
make run      # Build and run in QEMU
make check     # Run cargo check
make clean     # Clean build artifacts
make test      # Run with curses display
```

### Testing Locally

1. Build the kernel:
   ```bash
   cargo bootimage
   ```

2. Run in QEMU:
   ```bash
   qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/boot-bios-RustTest.img
   ```

3. Exit QEMU: Press `Ctrl+Alt+G` to release mouse, then `Ctrl+C` or close window

## 📚 Documentation

- [Architecture Documentation](docs/ARCHITECTURE.md) - Deep dive into kernel design
- [Troubleshooting Guide](TROUBLESHOOTING.md) - Common errors and solutions
- [Contributing Guide](CONTRIBUTING.md) - How to contribute

## 🤝 Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Good First Issues

- Add keyboard input support
- Implement text scrolling
- Add color schemes
- Create animated smiley
- Add border around text

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Writing an OS in Rust](https://os.phil-opp.com/) by Philipp Oppermann
- [bootloader](https://github.com/rust-osdev/bootloader) crate
- Rust embedded working group

## 🔗 Links

- **Repository**: [GitHub](https://github.com/DakodaStemen/RustOS)
- **Issues**: [Report a bug](https://github.com/DakodaStemen/RustOS/issues)
- **Discussions**: [GitHub Discussions](https://github.com/DakodaStemen/RustOS/discussions)

