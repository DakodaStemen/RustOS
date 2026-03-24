# Research: Multitasking and User Input for x86_64 Rust Kernel

This document explores the implementation details for adding user input via the PS/2 keyboard and building a multitasking system in a bare-metal Rust environment.

---

## 1. PS/2 Keyboard Controller Driver

The PS/2 controller is the traditional way to handle keyboard and mouse input on x86 systems. Even on modern systems with USB keyboards, the firmware usually provides "Legacy USB Support," which emulates a PS/2 controller.

### Port Interface
- **Data Port (0x60)**: Read received byte (scan code) or write command/data.
- **Status Register (0x64)**: Read status of the controller (e.g., if data is available).
- **Command Register (0x64)**: Write commands to the controller.

### Interrupt-Based vs. Polling
- **Polling**: Continuously check the status register (bit 0) to see if data is available. This is inefficient as it wastes CPU cycles.
- **Interrupts**: Configure the I/O APIC (or the legacy PIC) to send an interrupt (usually IRQ 1) when a key is pressed.

### Scan Codes to ASCII
Keyboards send "scan codes," not ASCII. A key press sends a "make" code, and a key release sends a "break" code. Most systems default to Scan Code Set 1 or 2.

In Rust, it's recommended to use the `pc-keyboard` crate to handle the complexity of different scan code sets and state tracking (like Shift/Caps Lock).

```rust
// Example using pc-keyboard crate
use pc_keyboard::{layouts, HandleControl, Keyboard, ScancodeSet1, DecodedKey};

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore));
}

pub fn handle_keyboard_interrupt(scancode: u8) {
    let mut keyboard = KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }
}
```

---

## 2. Cooperative vs. Preemptive Multitasking

### Cooperative Multitasking
In cooperative multitasking, the currently running task voluntarily yields control back to the scheduler.
- **Pros**: Simple to implement, no race conditions (if single-core), no need for complex synchronization.
- **Cons**: A single malicious or buggy task can hang the entire system by never yielding.
- **Implementation in Rust**: Often implemented using `async/await` and a dedicated Executor.

### Preemptive Multitasking
In preemptive multitasking, the operating system uses a hardware timer (PIT or APIC) to interrupt the running task and switch to another.
- **Pros**: Robust against hanging tasks; provides a "smoother" feel.
- **Cons**: Requires complex context switching logic, synchronization primitives (Spinlocks, Mutexes), and careful handling of shared state to avoid race conditions.
- **Kernel Requirement**: Requires an Interrupt Descriptor Table (IDT) and a Global Descriptor Table (GDT) properly configured.

---

## 3. Context Switching (x86_64)

Context switching involves saving the current state of the CPU (registers) for one task and restoring the state of another.

### Register State to Save
On x86_64, a task's context includes:
- **General Purpose**: `rax`, `rbx`, `rcx`, `rdx`, `rsi`, `rdi`, `rbp`, `r8-r15`.
- **Special Purpose**: `rip` (Instruction Pointer), `rsp` (Stack Pointer), `rflags`.
- **FPU/SSE State**: Floating point registers should also be saved if tasks use them.

### Assembly Implementation
The core of context switching is an assembly routine that swaps stacks.

```nasm
; switch_to(old_rsp: *mut u64, new_rsp: u64)
global switch_to
switch_to:
    ; Save registers to current stack
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15
    pushfq ; Save RFLAGS

    ; Swap stacks
    mov [rdi], rsp ; Save old RSP to the address in RDI
    mov rsp, rsi    ; Load new RSP from RSI

    ; Restore registers from new stack
    popfq
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp

    ret
```

---

## 4. Simple Round-Robin Scheduler Design

A Round-Robin scheduler maintains a queue of ready tasks and gives each one a small "time slice" (quantum).

### Data Structures
- **Task Control Block (TCB)**: Stores metadata about a task.
- **Ready Queue**: A `VecDeque` or a linked list of tasks waiting for CPU time.

```rust
pub enum TaskState {
    Ready,
    Running,
    Blocked,
}

pub struct Task {
    id: TaskId,
    stack_pointer: VirtAddr,
    state: TaskState,
    // ... other metadata
}

pub struct Scheduler {
    ready_queue: VecDeque<Task>,
    current_task: Option<Task>,
}

impl Scheduler {
    pub fn schedule(&mut self) {
        if let Some(mut old_task) = self.current_task.take() {
            old_task.state = TaskState::Ready;
            self.ready_queue.push_back(old_task);
        }

        if let Some(mut next_task) = self.ready_queue.pop_front() {
            next_task.state = TaskState::Running;
            let old_rsp_ptr = &mut self.current_task.as_mut().unwrap().stack_pointer as *mut VirtAddr;
            let new_rsp = next_task.stack_pointer;
            
            self.current_task = Some(next_task);
            
            unsafe {
                switch_to(old_rsp_ptr, new_rsp);
            }
        }
    }
}
```

---

## Architectural Recommendations

1.  **Start with Cooperative**: Implement a simple `async` executor first to get the feel of task management in Rust.
2.  **Separate Kernel/User Stacks**: Ensure each task has its own stack to prevent one task from corrupting another's state.
3.  **Use APIC Timer**: For preemption, the Local APIC timer is more modern and provides better precision/features than the legacy PIT.
4.  **Hardware Task State Segment (TSS)**: On x86_64, use the TSS to store the kernel stack pointer for privilege level switches (from User Mode to Kernel Mode).
5.  **Lock-Free Structures**: When possible, use lock-free queues for the scheduler to avoid deadlocks in interrupt handlers.
6.  **Scan Code Mapping**: Abstract the keyboard driver early. Use a generic `Event` system where the keyboard driver pushes events into a queue that the user-space tasks can read.
