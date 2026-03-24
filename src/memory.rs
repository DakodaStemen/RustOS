use core::sync::atomic::{AtomicBool, Ordering};
use x86_64::{
    structures::paging::{
        PageTable, OffsetPageTable, FrameAllocator, PhysFrame, Size4KiB,
    },
    VirtAddr, PhysAddr,
};
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

static MEMORY_INIT_CALLED: AtomicBool = AtomicBool::new(false);

/// Initialize a new OffsetPageTable.
///
/// # Safety
/// The caller must guarantee that all physical memory is mapped at
/// `physical_memory_offset`. Must only be called once.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    assert!(
        !MEMORY_INIT_CALLED.swap(true, Ordering::AcqRel),
        "memory::init must only be called once"
    );
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

/// A FrameAllocator backed by the bootloader's memory map.
/// Tracks position with a cursor so each allocation is O(1) amortized.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    region_index: usize,
    next_frame: u64,
}

impl BootInfoFrameAllocator {
    /// # Safety
    /// The caller must guarantee the memory map is valid and all USABLE
    /// frames are actually free.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        let mut alloc = BootInfoFrameAllocator {
            memory_map,
            region_index: 0,
            next_frame: 0,
        };
        alloc.advance_to_usable();
        alloc
    }

    fn advance_to_usable(&mut self) {
        let regions: &[_] = self.memory_map;
        while self.region_index < regions.len() {
            if regions[self.region_index].region_type == MemoryRegionType::Usable {
                self.next_frame = regions[self.region_index].range.start_frame_number;
                return;
            }
            self.region_index += 1;
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let regions: &[_] = self.memory_map;

        while self.region_index < regions.len() {
            let region = &regions[self.region_index];
            if region.region_type == MemoryRegionType::Usable
                && self.next_frame < region.range.end_frame_number
            {
                let addr = PhysAddr::new(self.next_frame * 4096);
                let frame = PhysFrame::containing_address(addr);
                self.next_frame += 1;
                return Some(frame);
            }
            self.region_index += 1;
            if self.region_index < regions.len() {
                self.next_frame = regions[self.region_index].range.start_frame_number;
            }
        }

        None
    }
}
