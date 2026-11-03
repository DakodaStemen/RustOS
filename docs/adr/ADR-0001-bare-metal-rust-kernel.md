# ADR-0001: Bare-metal x86_64 kernel in Rust with bootloader 0.9

| Field | Value |
| :---- | :---- |
| Status | Accepted |
| Date | 2026-11-03 |
| Deciders | @DakodaStemen |

## Context

The project is a small educational kernel (see [Writing an OS in Rust](https://os.phil-opp.com/)). It needs a `no_std` build for `x86_64-unknown-none`, a BIOS boot path, a test setup that works without hardware, and a toolchain that stays reproducible.

## Decision

- Write the kernel in Rust, `no_std`, with `panic = "abort"`.
- Boot through the `bootloader` 0.9 crate and build disk images with `bootimage`.
- Pin a nightly toolchain in `rust-toolchain.toml` (needed for `build-std` and the custom test framework).
- Test in QEMU: `cargo test` runs `#[test_case]` tests and reports through `isa-debug-exit`; `tests/qemu_test.py` drives the shell over the QEMU monitor.

## Alternatives considered

| Alternative | Why not chosen |
| :---- | :---- |
| `bootloader` 0.11 (UEFI) | Larger migration; listed as not done in the README |
| C or C++ kernel | The project's goal is memory-safe kernel code in Rust |
| Real hardware for tests | Not repeatable in CI |

## Consequences

**Positive**

- One pinned toolchain and one command (`make ci`) reproduce the build and tests.
- Tests run headless in CI.

**Negative**

- Nightly Rust: the pin needs periodic bumps.
- BIOS-only boot; no UEFI.
