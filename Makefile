.PHONY: all build run clean check test help

# Default target
all: build

# Build the bootable image
build:
	@echo "Building bootable image..."
	cargo bootimage
	@echo "Build complete! Image: target/x86_64-unknown-none/release/boot-bios-RustTest.img"

# Build and run in QEMU
run: build
	@echo "Starting QEMU..."
	qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/boot-bios-RustTest.img

# Run with curses display (better for terminal recording)
test: build
	@echo "Starting QEMU with curses display..."
	qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/boot-bios-RustTest.img -display curses

# Run cargo check
check:
	@echo "Running cargo check..."
	cargo check --target x86_64-unknown-none

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	@echo "Clean complete!"

# Show help
help:
	@echo "Available targets:"
	@echo "  make build  - Build bootable image"
	@echo "  make run     - Build and run in QEMU"
	@echo "  make test    - Run with curses display"
	@echo "  make check   - Run cargo check"
	@echo "  make clean   - Clean build artifacts"
	@echo "  make help    - Show this help message"

