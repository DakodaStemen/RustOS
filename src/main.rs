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
/// We attempt to write panic information to the VGA buffer using a lock-free
/// approach to avoid deadlock if the panic occurred while holding the WRITER lock.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use vga_buffer::{Color, ColorCode, panic_write_string};
    
    // Try to write panic message to VGA buffer using lock-free approach
    // This avoids deadlock if panic occurred while WRITER lock is held.
    // We use a lock-free write function that directly accesses the VGA buffer
    // without going through the Mutex, preventing deadlock scenarios.
    let color_code = ColorCode::from_colors(Color::Red, Color::Black);
    
    // Write basic panic message (lock-free, avoids deadlock)
    // SAFETY: panic_write_string is safe to call from panic handler because:
    // 1. Panics are single-threaded (no concurrent access from other threads)
    // 2. VGA buffer at 0xb8000 is always valid in bootloader context
    // 3. We're already in a panic state, so avoiding deadlock is critical
    // 4. The function performs bounds checking to prevent out-of-bounds access
    unsafe {
        // Write "PANIC" message to the first row
        panic_write_string("PANIC!", 0, 0, color_code);
        
        // Try to write file name if location is available (safe UTF-8 truncation)
        if let Some(location) = info.location() {
            let file = location.file();
            // Truncate file name to fit on screen (max 40 characters to leave room for "Line: " at column 40)
            // We must truncate at a UTF-8 character boundary to avoid panicking
            // if the truncation point lands in the middle of a multi-byte character.
            // Layout: File name (cols 0-39), "Line: " (cols 40-45), line number (cols 46+)
            let max_chars = 40;
            
            // Find the byte position after the max_chars-th character
            // This ensures we truncate at a valid UTF-8 character boundary
            let mut safe_byte_len = 0;
            let mut char_count = 0;
            
            for (byte_pos, ch) in file.char_indices() {
                if char_count >= max_chars {
                    // We've found max_chars characters, truncate before this one
                    safe_byte_len = byte_pos;
                    break;
                }
                // Move past this character
                safe_byte_len = byte_pos + ch.len_utf8();
                char_count += 1;
            }
            
            // If we didn't reach max_chars, safe_byte_len is already at the end
            // safe_byte_len is now guaranteed to be at a character boundary
            
            if safe_byte_len > 0 && safe_byte_len <= file.len() {
                // safe_byte_len is guaranteed to be at a UTF-8 character boundary
                // because we calculated it by iterating through characters with char_indices()
                // and using ch.len_utf8() to find the end of each character.
                // Therefore, &file[..safe_byte_len] is safe and won't panic.
                let file_slice = &file[..safe_byte_len];
                panic_write_string(file_slice, 1, 0, color_code);
            }
            
            // Write line number as simple string (limited formatting)
            let line = location.line();
            if line < 10000 {
                // Simple approach: write line number digits manually
                let mut line_buf = [b'0'; 5];
                let mut num = line;
                let mut idx = 4;
                
                if num == 0 {
                    line_buf[idx] = b'0';
                    idx -= 1;
                } else {
                    while num > 0 && idx > 0 {
                        line_buf[idx] = b'0' + (num % 10) as u8;
                        num /= 10;
                        idx -= 1;
                    }
                }
                
                let line_str = core::str::from_utf8(&line_buf[idx + 1..]).unwrap_or("?");
                panic_write_string("Line: ", 1, 40, color_code);
                panic_write_string(line_str, 1, 46, color_code);
            }
        }
    }
    
    // Infinite loop - kernel is halted
    loop {
        core::hint::spin_loop();
    }
}

