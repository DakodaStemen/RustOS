#![no_std]
#![no_main]

use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;

mod vga_buffer;

// Compile-time assertions to ensure buffer constants are valid
const _: () = {
    assert!(vga_buffer::BUFFER_HEIGHT > 0, "Buffer height must be > 0");
    assert!(vga_buffer::BUFFER_WIDTH > 0, "Buffer width must be > 0");
    // VGA buffer address should be aligned (not strictly required but good practice)
    // 0xb8000 is naturally aligned for our use case
};

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    use vga_buffer::{WRITER, Color};
    use core::fmt::Write;

    // Initialize VGA writer - this is the first access to the static WRITER.
    // At this point, the bootloader has set up memory and we're in a valid context.
    // The VGA buffer at 0xb8000 is guaranteed to be accessible.
    let mut writer = WRITER.lock();
    
    // Clear the screen by writing newlines
    writer.set_color(Color::Black, Color::Black);
    for _ in 0..vga_buffer::BUFFER_HEIGHT {
        writer.write_string("\n");
    }
    
    // Set color to yellow on black for the smiley
    writer.set_color(Color::Yellow, Color::Black);
    
    // Center the smiley on the screen
    // VGA buffer is 80 columns wide, so center is around column 40
    // We'll write some newlines to center vertically, then spaces to center horizontally
    for _ in 0..10 {
        writer.write_string("\n");
    }
    
    // Center horizontally (approximately 35 spaces for 80-width screen)
    // This positions us at column 35, leaving room for the smiley and text
    for _ in 0..35 {
        writer.write_byte(b' ');
    }
    
    // Write the smiley face using IBM extended ASCII character 0x01 (☺)
    // This character is valid in Code Page 437 (VGA text mode character set)
    writer.write_byte(0x01);
    writer.write_string("\n");
    
    // Add some text below the smiley
    for _ in 0..35 {
        writer.write_byte(b' ');
    }
    writer.write_string("Hello from Rust OS!");
    
    // Release the lock before entering infinite loop
    drop(writer);
    
    // Infinite loop to keep kernel running
    loop {
        // Use a hint to prevent the compiler from optimizing away the loop
        core::hint::spin_loop();
    }
}

/// Panic handler for the kernel.
///
/// This function is called when a panic occurs. In a bare-metal environment,
/// we can't unwind the stack or exit gracefully, so we loop forever.
///
/// # Safety
///
/// This function must never return (diverging function), which is enforced by
/// the `-> !` return type. The infinite loop prevents the function from returning.
///
/// We attempt to write panic information to the VGA buffer, but if that fails
/// (e.g., if the panic occurred during VGA initialization), we just loop.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Try to write panic message to VGA buffer for debugging
    // Note: We use lock() which will spin forever if the lock is held.
    // This is acceptable in a panic handler since we're going to loop forever anyway.
    // If the panic occurred before VGA was initialized, this might not work,
    // but it's better than doing nothing.
    use vga_buffer::{WRITER, Color};
    use core::fmt::Write;
    
    // Attempt to get the lock and write panic info
    // If this panics or deadlocks, we'll just loop forever (which we do anyway)
    let mut writer = WRITER.lock();
    writer.set_color(Color::Red, Color::Black);
    let _ = write!(writer, "PANIC: ");
    if let Some(location) = info.location() {
        let _ = write!(writer, "{}:{}:{}", 
            location.file(), 
            location.line(), 
            location.column());
    }
    if let Some(message) = info.message() {
        let _ = write!(writer, " - {:?}", message);
    }
    drop(writer);
    
    // Infinite loop - kernel is halted
    loop {
        core::hint::spin_loop();
    }
}

