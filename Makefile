.DEFAULT_GOAL := help
.PHONY: help setup lint test test-kernel ci all build run run-curses check clean

help:
	@echo "make setup        install the pinned nightly (rust-toolchain.toml) and bootimage"
	@echo "make build        build the bootable image"
	@echo "make run          build and run in QEMU"
	@echo "make run-curses   run in QEMU with a curses display"
	@echo "make lint         cargo check and clippy (rustfmt is advisory)"
	@echo "make test         boot the release image headless in QEMU and drive the shell"
	@echo "make test-kernel  run the in-kernel #[test_case] suite (cargo test)"
	@echo "make ci           lint + build + test-kernel + test"
	@echo "make clean        remove build artifacts"

# Idempotent: rustup show is a no-op once the toolchain is installed.
setup:
	rustup show
	command -v cargo-bootimage >/dev/null || cargo install bootimage --locked

all: build

# Build the bootable image
build:
	cargo bootimage --release
	@echo "Image: target/x86_64-unknown-none/release/bootimage-rust-os.bin"

run: build
	qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/bootimage-rust-os.bin

run-curses: build
	qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/bootimage-rust-os.bin -display curses

check:
	cargo check --target x86_64-unknown-none

# clippy warnings are reported but do not fail the build; legacy code still has a few.
# The rustfmt check is advisory: existing files predate it and are not mass-reformatted.
lint: check
	cargo clippy
	-cargo fmt --check >/dev/null 2>&1 || echo "rustfmt: differences found (advisory)"

# Boot the release image headless in QEMU and drive the shell (tests/qemu_test.py)
test: build
	python3 tests/qemu_test.py

# Run the in-kernel #[test_case] suite (boots QEMU, exits via isa-debug-exit)
test-kernel:
	cargo test

ci: lint build test-kernel test

clean:
	cargo clean
