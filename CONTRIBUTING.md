# Contributing to RustTest

Thank you for your interest in contributing! This document provides guidelines and instructions for contributing to the project.

## Getting Started

### Prerequisites

Before contributing, ensure you have:

1. **Rust Nightly Toolchain**
   ```bash
   rustup toolchain install nightly
   rustup override set nightly
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

5. **QEMU** (for testing)
   ```bash
   # Ubuntu/Debian
   sudo apt-get install qemu-system-x86
   
   # macOS
   brew install qemu
   
   # Windows
   # Download from https://www.qemu.org/download/
   ```

### Setup

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/DakodaStemen/RustOS.git
   cd RustOS
   ```

3. Verify setup:
   ```bash
   make check
   make build
   ```

## Development Workflow

### Making Changes

1. Create a branch for your feature:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes

3. Test locally:
   ```bash
   make build
   make run  # Or make test for curses display
   ```

4. Ensure code quality:
   ```bash
   cargo fmt
   cargo clippy --target x86_64-unknown-none
   ```

### Code Style

We follow standard Rust conventions:

- **Formatting**: Use `cargo fmt` to format code
- **Linting**: Fix all `cargo clippy` warnings
- **Documentation**: Document all public APIs
- **Safety Comments**: Document all `unsafe` blocks with safety justifications

#### Example Safety Comment

```rust
unsafe {
    // SAFETY: 0xb8000 is the standard VGA text buffer address in x86_64.
    // This address is guaranteed to be valid and writable in the bootloader
    // environment. We cast to *mut Buffer and immediately create a reference,
    // which is safe because Buffer is a simple struct with no invariants.
    &mut *(0xb8000 as *mut Buffer)
}
```

### Testing

Before submitting a pull request:

1. **Build Test**: Ensure code compiles
   ```bash
   cargo build --target x86_64-unknown-none --release
   ```

2. **Boot Test**: Verify kernel boots in QEMU
   ```bash
   make run
   ```

3. **Visual Test**: Check output appears correctly
   - Smiley face displays
   - Text is readable
   - Colors work as expected

## Adding Features

### Guidelines

1. **Keep it Minimal**: This is a minimal kernel - avoid unnecessary complexity
2. **Document Everything**: Add comments explaining design decisions
3. **Safety First**: All `unsafe` code must have safety comments
4. **No Heap**: Avoid heap allocations unless adding an allocator
5. **Test Thoroughly**: Test in QEMU before submitting

### Feature Ideas

Good first issues for new contributors:

- **Keyboard Input**: Add PS/2 keyboard driver
- **Text Scrolling**: Improve scrolling behavior
- **Color Schemes**: Add preset color combinations
- **Animated Smiley**: Make smiley blink or animate
- **Border**: Add border around text area
- **Clear Screen**: Add explicit clear screen function
- **Cursor**: Add visible cursor
- **Backspace**: Implement backspace functionality

### Example: Adding a New Feature

1. **Plan**: Document what you want to add
2. **Implement**: Write the code following style guidelines
3. **Test**: Verify it works in QEMU
4. **Document**: Update relevant documentation
5. **Submit**: Create a pull request

## Pull Request Process

### Before Submitting

- [ ] Code compiles without warnings
- [ ] Code is formatted with `cargo fmt`
- [ ] All `unsafe` blocks have safety comments
- [ ] Changes tested in QEMU
- [ ] Documentation updated if needed
- [ ] Commit messages are clear and descriptive

### Pull Request Template

When creating a PR, include:

1. **Description**: What does this PR do?
2. **Changes**: List of changes made
3. **Testing**: How was it tested?
4. **Screenshots**: If visual changes (optional)

Example:
```markdown
## Description
Adds keyboard input support using PS/2 controller.

## Changes
- Added PS/2 keyboard driver
- Implemented key press detection
- Added echo of typed characters to screen

## Testing
Tested in QEMU - keyboard input works correctly and characters appear on screen.
```

## Code Review

All contributions require code review. Reviewers will check:

- Code correctness and safety
- Adherence to style guidelines
- Documentation quality
- Test coverage

Be open to feedback and willing to make changes!

## Reporting Issues

### Bug Reports

When reporting a bug, include:

1. **Description**: What went wrong?
2. **Steps to Reproduce**: How to trigger the bug
3. **Expected Behavior**: What should happen?
4. **Actual Behavior**: What actually happened?
5. **Environment**: Rust version, OS, QEMU version
6. **Screenshots**: If applicable

### Feature Requests

For feature requests, include:

1. **Description**: What feature do you want?
2. **Use Case**: Why is this useful?
3. **Proposed Implementation**: How would it work? (optional)

## Questions?

- Open an issue for questions
- Check [TROUBLESHOOTING.md](../TROUBLESHOOTING.md) for common issues
- Review [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for technical details

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

Thank you for contributing! 🦀

