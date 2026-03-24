# Memory Management Research for x86_64 Rust Kernel

This document outlines the research and architectural recommendations for implementing memory management in an x86_64 Rust kernel using the `bootloader` (v0.9.23) crate.

## 1. Global Descriptor Table (GDT) and Task State Segment (TSS)

In 64-bit mode, segmentation is mostly inactive, but the GDT is still required for:
- Switching between kernel and user mode.
- Loading a Task State Segment (TSS).

### Task State Segment (TSS)
The TSS is essential for **stack switching**. When an interrupt occurs (e.g., a Double Fault), the CPU can automatically switch to a known good stack defined in the TSS. This prevents a triple fault if the original stack was corrupted (e.g., due to stack overflow).

#### Interrupt Stack Table (IST)
The TSS contains an IST, which is a list of 7 pointers to "known good" stacks. We typically dedicate one IST entry for the Double Fault handler.

### Implementation Snippet
Using the `x86_64` crate:

```rust
use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use lazy_static::lazy_static;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            #[repr(C, align(16))]
            struct Stack([u8; STACK_SIZE]);
            static STACK: Stack = Stack([0; STACK_SIZE]);

            let stack_start = VirtAddr::from_ptr(core::ptr::addr_of!(STACK));
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss
    };
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code_selector, tss_selector })
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, Segment};
    
    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
```

---

## 2. Physical Frame Allocation

Physical memory is divided into 4096-byte pages called **frames**. The kernel needs a way to track which frames are used and which are free.

### Allocation Strategies

| Strategy | Pros | Cons |
| :--- | :--- | :--- |
| **Bitmap** | Compact, easy to implement. | Slow searches for large memory; hard to find contiguous blocks. |
| **Linked List** | O(1) allocation/deallocation. | Metadata overhead in every free frame; difficult to allocate contiguous frames. |
| **Buddy Allocator**| Fast, supports power-of-two contiguous blocks. | Complex implementation; internal fragmentation. |

### Recommendation: BootInfo Frame Allocator
For an initial implementation using the `bootloader` crate, we can use the `MemoryMap` provided in `BootInfo` to create a simple frame allocator that tracks the next available frame.

```rust
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::structures::paging::{PhysFrame, Size4KiB, FrameAllocator};

pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator { memory_map, next: 0 }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
```

---

## 3. Virtual Memory Paging and Recursive Mapping

x86_64 uses a 4-level page table structure to map virtual addresses to physical addresses.

### Recursive Mapping vs. Physical Memory Mapping

1. **Recursive Mapping**: One entry in the Level 4 table points back to the Level 4 table itself.
   - **Pro**: Doesn't require extra physical memory for the map.
   - **Con**: Occupies a large chunk of virtual address space; slightly confusing to implement.

2. **Physical Memory Mapping**: The entire physical memory is mapped to a high virtual address (e.g., `0x0000_8000_0000_0000`).
   - **Pro**: Simple and clean; recommended for modern kernels.
   - **Bootloader Support**: `bootloader` 0.9.23 provides `physical_memory_offset` in `BootInfo`.

### Implementation with `OffsetPageTable`
The `x86_64` crate provides `OffsetPageTable`, which simplifies page table manipulation when physical memory is mapped at an offset.

```rust
use x86_64::{structures::paging::OffsetPageTable, VirtAddr};

pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) 
    -> &'static mut PageTable 
{
    use x86_64::registers::control::Cr3;
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    &mut *page_table_ptr
}
```

---

## 4. Heap Allocator and the `alloc` Crate

To enable dynamic memory allocation (`Box`, `Vec`, `String`), the kernel must implement the `GlobalAlloc` trait.

### Allocator Choices

- **Linked List Allocator**: Simple, fits in small memory, but prone to fragmentation.
- **Slab Allocator**: Fast for fixed-size allocations, avoids external fragmentation.
- **Buddy Allocator**: Good for varied allocation sizes.

### Enabling the `alloc` Crate
1. Add `extern crate alloc;` to `main.rs`.
2. Define a `#[global_allocator]`.

### Example: Basic Linked List Allocator
```rust
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
        };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }

    Ok(())
}
```

## Summary of Architectural Recommendations

1. **Use `x86_64` crate**: It provides high-quality abstractions for GDT, TSS, and Paging.
2. **Implement TSS**: Early in boot, set up a TSS with an IST for Double Faults to ensure system stability during crashes.
3. **Physical Memory Offset**: Utilize the `physical_memory_offset` provided by `bootloader` to map virtual memory. This is significantly easier than recursive paging.
4. **Gradual Allocator Growth**: Start with a fixed-size heap (e.g., 100 KiB) using `linked_list_allocator` and migrate to a `Slab` or `Buddy` allocator as the kernel matures.
