# RustOS Development Roadmap

## Phase 1: System Stability (GDT, TSS, IDT, Exception Handling)
**Objective**: Establish a stable foundation for the kernel by handling CPU exceptions and setting up safe stack switching.
1. **Dependencies**: Add `x86_64` and `lazy_static` to `Cargo.toml`.
2. **GDT & TSS**: Create `src/gdt.rs`. Define a Task State Segment (TSS) with an Interrupt Stack Table (IST) for the Double Fault handler. Initialize the Global Descriptor Table (GDT).
3. **IDT & Exceptions**: Create `src/interrupts.rs`. Initialize the Interrupt Descriptor Table (IDT). Implement handlers for Breakpoint and Double Fault exceptions.
4. **Integration**: Update `src/main.rs` to initialize the GDT and IDT during boot. Verify by triggering a breakpoint exception (`x86_64::instructions::interrupts::int3()`).

## Phase 2: Dynamic Memory (Paging, Frame Allocator, Heap Allocator)
**Objective**: Enable dynamic memory allocation (`alloc` crate) by managing physical frames and setting up virtual memory.
1. **Dependencies**: Add `linked_list_allocator` and enable the `alloc` crate.
2. **Paging**: Create `src/memory.rs`. Initialize a recursive or physical memory offset page table.
3. **Frame Allocator**: Implement a `BootInfoFrameAllocator` using the memory map provided by the `bootloader`.
4. **Heap Allocation**: Create `src/allocator.rs`. Initialize a basic linked-list or bump allocator. Test `Vec` and `Box` creation in `kernel_main`.

## Phase 3: Interactivity (PIC initialization, Timer IRQ, PS/2 Keyboard Driver)
**Objective**: Allow the kernel to respond to hardware interrupts, specifically keyboard input and a system timer.
1. **Dependencies**: Add `pic8259` and `pc-keyboard`.
2. **PIC Initialization**: Remap the 8259 PIC interrupts to IDT vectors 32-47 to avoid conflicts with CPU exceptions.
3. **Timer Interrupt**: Implement an IDT handler for the Timer (IRQ 0).
4. **Keyboard Driver**: Implement an IDT handler for the Keyboard (IRQ 1). Read scan codes from I/O port `0x60` and translate them to ASCII using `pc-keyboard`. Output typed characters to the VGA buffer.

## Phase 4: Concurrency (Async/Await executor, basic preemptive multitasking)
**Objective**: Implement cooperative and basic preemptive multitasking.
1. **Dependencies**: Add `crossbeam-queue` or minimal `futures-util` components if necessary.
2. **Async Tasks**: Create `src/task/mod.rs` and `src/task/keyboard.rs` to handle keyboard input asynchronously.
3. **Executor**: Create a basic `SimpleExecutor` to poll futures cooperatively.
4. **Preemption (Stretch Goal)**: Integrate the hardware timer interrupt to force context switches between distinct threads using a Round-Robin scheduler.
