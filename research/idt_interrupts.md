# Implementation Research: Interrupt Descriptor Table (IDT) for x86_64 Rust Kernel

This document outlines the research and architectural recommendations for implementing an Interrupt Descriptor Table (IDT) in a minimal x86_64 Rust kernel.

## 1. Necessary Crates

To implement interrupts efficiently and safely in Rust, the following crates are recommended:

- **`x86_64`**: The core crate for x86_64 architecture support. It provides high-level abstractions for the IDT, GDT, and CPU registers.
- **`pic8259`**: A crate for managing the legacy 8259 Programmable Interrupt Controller (PIC). Essential for mapping hardware interrupts (like keyboard or timer) to the CPU.
- **`spin`**: Provides spinlocks and synchronization primitives, necessary for safe global access to the IDT and PICs in a `no_std` environment.
- **`lazy_static`** or **`concurrency_once_cell`**: Used to safely initialize global static variables like the IDT at runtime.

### Cargo.toml Snippet
```toml
[dependencies]
x86_64 = "0.14.2"
pic8259 = "0.10.1"
spin = "0.9.8"
lazy_static = { version = "1.4.0", features = ["spin_no_std"] }
```

## 2. IDT Data Structures

The x86_64 architecture uses a 16-byte entry format for the IDT. The `x86_64` crate abstracts this into the `InterruptDescriptorTable` struct.

### Core Structures:
- **`InterruptDescriptorTable`**: A table containing 256 entries. The first 32 entries are reserved for CPU exceptions.
- **`Entry<F>`**: Represents a single entry in the IDT.
- **`InterruptStackFrame`**: Automatically pushed by the CPU onto the stack when an interrupt occurs. It contains the instruction pointer (`rip`), code segment (`cs`), CPU flags (`rflags`), stack pointer (`rsp`), and stack segment (`ss`).

### Calling Convention:
Interrupt handlers MUST use the `extern "x86-interrupt"` calling convention. This requires the `#![feature(abi_x86_interrupt)]` unstable feature in Rust.

```rust
#![feature(abi_x86_interrupt)]

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: InterruptStackFrame) 
{
    // Handle breakpoint
}
```

## 3. Handling Common Exceptions

### Breakpoint (Vector 3)
Used for debugging. It is a "trap" exception, meaning the instruction pointer points to the instruction *after* the one that caused it.

### Double Fault (Vector 8)
Occurs when the CPU fails to invoke an exception handler. A common cause is a stack overflow.
- **Critical Requirement**: To handle Double Faults reliably, you should use an **Interrupt Stack Table (IST)** entry in the Global Descriptor Table (GDT). This switches to a known good stack when a Double Fault occurs, preventing a triple fault (which causes a system reset).

### Page Fault (Vector 14)
Occurs when the CPU tries to access unmapped or protected memory.
- **Error Code**: Unlike the Breakpoint handler, the Page Fault handler receives a `PageFaultErrorCode`.
- **CR2 Register**: The `CR2` register contains the virtual address that caused the fault.

```rust
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let accessed_address = x86_64::registers::control::Cr2::read();
    // Handle page fault...
}
```

## 4. Mapping Hardware Interrupts (PIC vs APIC)

### 8259 PIC (Legacy)
The PIC maps hardware IRQs to CPU vectors. By default, IRQ 0-7 are mapped to vectors 0-7, which conflicts with CPU exceptions.
- **Remapping**: You must remap the PIC to a safe range, typically starting at vector 32 (0x20).
- **Master/Slave**: Two PICs are chained. The Master handles IRQ 0-7, and the Slave handles IRQ 8-15.
- **End of Interrupt (EOI)**: Handlers must send an EOI signal to the PIC(s) to acknowledge receipt, otherwise the PIC will stop sending interrupts.

```rust
use pic8259::ChainedPics;
use spin::Mutex;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { 
    ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) 
});
```

### APIC (Modern)
The Advanced Programmable Interrupt Controller (APIC) is more complex but supports multi-core (SMP) and MSI.
- **Local APIC (LAPIC)**: Per-core controller for timers and local interrupts.
- **I/O APIC**: Routes external interrupts to LAPICs.
- **Recommendation**: For initial development, start with the 8259 PIC. For a production-ready kernel or SMP support, implement ACPI parsing and transition to the APIC.

## 5. Architectural Recommendations

1.  **Static IDT**: Use `lazy_static!` to define a global IDT. Load it during kernel initialization using `idt.load()`.
2.  **GDT and IST**: Implement a GDT with at least a kernel code segment and an IST for the Double Fault handler.
3.  **Atomic Handlers**: Keep interrupt handlers as short and fast as possible. Avoid complex logic or blocking operations inside handlers.
4.  **Interrupt Enable/Disable**: Use `x86_64::instructions::interrupts::enable()` to start receiving hardware interrupts after the IDT and PIC are initialized.

---
*Created by Gemini CLI - Research Task*
