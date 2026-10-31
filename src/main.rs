#![no_std]
#![no_main]
#![cfg_attr(test, allow(dead_code, unused_imports))]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
#[cfg(not(test))]
use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};

mod vga_buffer;
mod interrupts;
mod gdt;
mod memory;
mod allocator;
mod task;
mod serial;
mod shell;
mod qemu;
#[cfg(test)]
mod tests;

#[cfg(not(test))]
use task::executor::Executor;
#[cfg(not(test))]
use task::Task;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    init(boot_info);

    #[cfg(test)]
    test_main();

    #[cfg(not(test))]
    run_kernel();

    #[cfg(test)]
    loop {
        x86_64::instructions::hlt();
    }
}

/// Bring up serial, GDT/IDT, paging and the heap. Interrupts stay disabled.
fn init(boot_info: &'static BootInfo) {
    use x86_64::VirtAddr;
    use memory::BootInfoFrameAllocator;

    serial::init();
    gdt::init();
    interrupts::init_idt();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    memory::record_usable(&boot_info.memory_map);
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
}

#[cfg(not(test))]
fn run_kernel() -> ! {
    let x = Box::new(41);
    println!("heap_value at {:p}", x);

    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());

    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!("current reference count is {}", Rc::strong_count(&reference_counted));
    core::mem::drop(reference_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_reference));

    println!("RustOS booted: heap, IDT, PIC online");
    x86_64::instructions::interrupts::enable();
    shell::init();

    let mut executor = Executor::new();
    executor.spawn(Task::new(task::keyboard::print_keypresses()));
    executor.run()
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    qemu::exit_qemu(qemu::QemuExitCode::Failed);
}

#[cfg(test)]
pub trait Testable {
    fn run(&self);
}

#[cfg(test)]
impl<T: Fn()> Testable for T {
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    qemu::exit_qemu(qemu::QemuExitCode::Success);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use vga_buffer::{Color, ColorCode, panic_write_string};

    let color_code = ColorCode::from_colors(Color::Red, Color::Black);

    unsafe {
        panic_write_string("PANIC!", 0, 0, color_code);

        if let Some(location) = info.location() {
            let file = location.file();
            let max_chars = 40;

            let mut safe_byte_len = 0;
            let mut char_count = 0;

            for (byte_pos, ch) in file.char_indices() {
                if char_count >= max_chars {
                    safe_byte_len = byte_pos;
                    break;
                }
                safe_byte_len = byte_pos + ch.len_utf8();
                char_count += 1;
            }

            if safe_byte_len > 0 && safe_byte_len <= file.len() {
                panic_write_string(&file[..safe_byte_len], 1, 0, color_code);
            }

            let line = location.line();
            if line < 100000 {
                let mut digits = [b'0'; 5];
                let mut n = line;
                let len;

                if n == 0 {
                    len = 1;
                } else {
                    let mut i = 4usize;
                    loop {
                        digits[i] = b'0' + (n % 10) as u8;
                        n /= 10;
                        if n == 0 {
                            let written_len = 5 - i;
                            for j in 0..written_len {
                                digits[j] = digits[i + j];
                            }
                            len = written_len;
                            break;
                        }
                        i -= 1;
                    }
                }

                let line_str = core::str::from_utf8(&digits[..len]).unwrap_or("?");
                panic_write_string("Line: ", 1, 40, color_code);
                panic_write_string(line_str, 1, 46, color_code);
            }
        }
    }

    loop {
        core::hint::spin_loop();
    }
}
