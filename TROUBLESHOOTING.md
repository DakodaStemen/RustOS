# Troubleshooting Guide for Bare-Metal Rust Kernel

This guide covers common errors and their solutions when building and running the bare-metal OS kernel.

## Prerequisites

Before building, ensure you have:

1. **Rust Nightly Toolchain**
   ```bash
   rustup toolchain install nightly
   rustup override set nightly  # or use rust-toolchain.toml
   ```

2. **Target Architecture**
   ```bash
   rustup target add x86_64-unknown-none
   ```

3. **LLVM Tools**
   ```bash
   rustup component add llvm-tools-preview
   ```

4. **Bootimage Tool**
   ```bash
   cargo install bootimage --version "^0.11"
   ```

## Common Compilation Errors

### Error: `target 'x86_64-unknown-none' not found`

**Cause**: The target architecture is not installed.

**Solution**:
```bash
rustup target add x86_64-unknown-none
```

**Verification**:
```bash
rustup target list | grep x86_64-unknown-none
```

---

### Error: `could not find 'llvm-tools-preview' component`

**Cause**: LLVM tools component is not installed.

**Solution**:
```bash
rustup component add llvm-tools-preview
```

**Verification**:
```bash
rustup component list | grep llvm-tools
```

---

### Error: `failed to run custom build command for 'bootloader'`

**Cause**: The `bootimage` tool is not installed or not in PATH.

**Solution**:
```bash
cargo install bootimage --version "^0.11"
```

**Verification**:
```bash
cargo bootimage --help
```

---

### Error: `#[panic_handler] function has wrong type`

**Cause**: The panic handler signature is incorrect.

**Solution**: Ensure your panic handler has this exact signature:
```rust
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
}
```

**Note**: The return type `-> !` (never type) is required.

---

### Error: `language item required`

**Cause**: Missing required language items (panic handler, allocator, etc.).

**Solution**: 
- Ensure `#[panic_handler]` is present
- For `no_std` projects, you don't need an allocator unless using heap allocation

---

### Error: `linking with 'cc' failed`

**Cause**: Linker issues, often related to bootloader version or missing linker script.

**Solution**:
1. Verify bootloader version matches: `bootloader = "0.11"` in Cargo.toml
2. Ensure `.cargo/config.toml` has correct target configuration
3. Bootloader 0.11 should provide linker script automatically

---

### Error: `use of unstable library feature`

**Cause**: Using unstable features without nightly toolchain or missing feature flags.

**Solution**:
1. Verify nightly toolchain: `rustc --version` should show "nightly"
2. Check `rust-toolchain.toml` has `channel = "nightly"`
3. Bootloader 0.11 should handle feature flags automatically

---

## Build Errors

### Error: `cargo bootimage` fails with "command not found"

**Cause**: Bootimage tool not installed or not in PATH.

**Solution**:
```bash
cargo install bootimage
# Add ~/.cargo/bin to PATH if needed
```

---

### Error: Boot image not created

**Cause**: Build succeeded but image file missing.

**Solution**:
1. Check build output: `cargo bootimage --verbose`
2. Expected output path: `target/x86_64-unknown-none/release/boot-bios-RustTest.img`
3. Verify build completed without errors

---

## Runtime Errors (QEMU)

### Error: `QEMU: Could not open disk image`

**Cause**: Wrong file path or image not created.

**Solution**:
1. Verify image exists:
   ```bash
   ls -la target/x86_64-unknown-none/release/boot-bios-RustTest.img
   ```
2. Use absolute path or correct relative path
3. Verify file format is correct (should be raw disk image)

**Correct QEMU Command**:
```bash
qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/boot-bios-RustTest.img
```

---

### Error: QEMU boots but shows nothing / black screen

**Cause**: 
- Entry point not reached
- VGA buffer not initialized
- Compiler optimized away writes

**Solution**:
1. Verify `entry_point!` macro is used correctly
2. Ensure VGA writes use `Volatile<T>` (we use `volatile` crate)
3. Check that `0xb8000` address is correct for VGA text buffer
4. Verify panic handler is present (system might be panicking silently)

**Debugging**:
- Add simple write to VGA buffer early in `kernel_main`
- Check if panic handler is being called (should show red text)

---

### Error: Screen shows garbage characters

**Cause**: 
- VGA buffer not properly initialized
- Writing invalid characters (not in Code Page 437)
- Memory corruption

**Solution**:
1. Clear screen at start of `kernel_main`
2. Filter characters: only write `0x20..=0x7e` and valid extended ASCII
3. Replace invalid characters with `0xfe` (■)
4. Verify volatile writes are not optimized away

---

### Error: QEMU hangs / freezes

**Cause**: 
- Infinite loop without proper CPU hint
- Deadlock on mutex
- Triple fault (stack overflow, invalid memory access)

**Solution**:
1. Use `core::hint::spin_loop()` in infinite loops
2. Ensure no deadlocks (release mutexes before infinite loops)
3. Check for stack overflow (minimize stack usage)
4. Verify all unsafe blocks are properly documented and safe

---

## Code-Specific Issues

### Issue: VGA writes don't appear

**Symptoms**: Code compiles and boots, but nothing on screen.

**Possible Causes**:
1. **Volatile writes optimized away**: Ensure using `Volatile<T>.write()` method
2. **Wrong buffer address**: VGA text buffer is at `0xb8000`
3. **Buffer not initialized**: Clear screen before writing
4. **Color code wrong**: Ensure foreground/background colors are set correctly

**Solution**: Verify in `src/vga_buffer.rs`:
- Using `volatile::Volatile` crate
- Calling `.write()` method on Volatile wrapper
- Address `0xb8000` is correct
- Buffer struct matches VGA layout (ScreenChar with u8 + ColorCode)

---

### Issue: Panic handler not showing messages

**Symptoms**: System hangs on panic but no error message.

**Possible Causes**:
1. Panic occurred before VGA initialization
2. Panic handler itself panicked
3. Lock is held when panic occurs

**Solution**:
- Panic handler uses `lock()` which will spin if lock is held
- This is acceptable since we loop forever anyway
- If VGA not initialized, panic handler won't be able to write

---

### Issue: Character encoding problems

**Symptoms**: Wrong characters appear, or screen corruption.

**Cause**: VGA text mode uses Code Page 437, not UTF-8.

**Solution**:
- Only use characters `0x20..=0x7e` (printable ASCII)
- Extended ASCII `0x01` (☺) and `0x02` (☻) are valid
- Replace invalid bytes with `0xfe` (■)
- See `write_string()` method for filtering logic

---

## Verification Checklist

Before reporting issues, verify:

- [ ] Rust nightly is active: `rustc --version`
- [ ] Target is installed: `rustup target list | grep x86_64-unknown-none`
- [ ] LLVM tools installed: `rustup component list | grep llvm-tools`
- [ ] Bootimage installed: `cargo bootimage --help`
- [ ] Code compiles: `cargo check --target x86_64-unknown-none`
- [ ] Build succeeds: `cargo build --target x86_64-unknown-none --release`
- [ ] Bootimage creates file: `cargo bootimage`
- [ ] QEMU can find image: Check file path

## Getting Help

If you encounter an error not listed here:

1. Check the full error message (including backtrace if available)
2. Verify all prerequisites are installed
3. Check that code matches the implementation in this repository
4. Try a clean build: `cargo clean && cargo bootimage`
5. Verify Rust and tool versions are compatible

## Memory Safety Notes

All unsafe blocks in this codebase are documented with safety comments explaining:
- Why the operation is safe
- What invariants must be maintained
- What could go wrong if misused

Key unsafe operations:
- VGA buffer pointer creation (`0xb8000` address)
- Static initialization of WRITER

These are safe because:
1. VGA buffer is always available in x86_64 bootloader context
2. Bootloader ensures valid memory before calling kernel_main
3. Volatile wrapper prevents compiler optimizations
4. Mutex provides synchronization

