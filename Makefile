.PHONY: all build run run-curses clean check test test-kernel help

# Default target
all: build

# Build the bootable image
build:
	@echo "Building bootable image..."
	cargo bootimage --release
	@echo "Build complete! Image: target/x86_64-unknown-none/release/bootimage-rust-os.bin"

# Build and run in QEMU
run: build
	@echo "Starting QEMU..."
	qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/bootimage-rust-os.bin

# Boot the release image headless in QEMU and drive the shell (tests/qemu_test.py)
test: build
	python3 tests/qemu_test.py

# Run the in-kernel #[test_case] suite (boots QEMU, exits via isa-debug-exit)
test-kernel:
	cargo test

# Run with curses display (better for terminal recording)

run-curses: build
	@echo "Starting QEMU with curses display..."
	qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/bootimage-rust-os.bin -display curses

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
	@echo "  make test    - Boot in headless QEMU and run the shell integration test"
	@echo "  make test-kernel - Run in-kernel #[test_case] tests (cargo test)"
	@echo "  make run-curses - Run in QEMU with curses display"
	@echo "  make check   - Run cargo check"
	@echo "  make clean   - Clean build artifacts"
	@echo "  make help    - Show this help message"

