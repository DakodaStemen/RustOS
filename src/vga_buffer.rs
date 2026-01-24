use volatile::Volatile;
use core::fmt;

pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
    
    /// Create a ColorCode from foreground and background colors.
    /// Public for use in panic handler.
    pub fn from_colors(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    /// Creates a new Writer that writes to the VGA text buffer.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it creates a raw pointer to the VGA text buffer
    /// at address 0xb8000. This is safe in the context of a bootloader/kernel because:
    ///
    /// 1. The VGA text buffer at 0xb8000 is a memory-mapped I/O region that is always
    ///    available in x86_64 systems, even in early boot stages.
    /// 2. The bootloader ensures we're in a valid memory context before calling kernel_main.
    /// 3. We use `Volatile<T>` wrapper to prevent compiler optimizations that could
    ///    eliminate writes to this memory-mapped region.
    /// 4. The buffer is only accessed through safe methods that use volatile operations.
    ///
    /// The static WRITER is initialized at compile time, but the actual memory access
    /// only occurs when `lock()` is called, which happens after kernel_main starts.
    pub fn new() -> Writer {
        Writer {
            column_position: 0,
            color_code: ColorCode::new(Color::Yellow, Color::Black),
            buffer: unsafe {
                // SAFETY: 0xb8000 is the standard VGA text buffer address in x86_64.
                // This address is guaranteed to be valid and writable in the bootloader
                // environment. We cast to *mut Buffer and immediately create a reference,
                // which is safe because Buffer is a simple struct with no invariants
                // that need to be maintained, and we only access it through Volatile<T>.
                &mut *(0xb8000 as *mut Buffer)
            },
        }
    }

    pub fn set_color(&mut self, foreground: Color, background: Color) {
        self.color_code = ColorCode::new(foreground, background);
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                // Bounds check: ensure we don't write beyond screen width
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                // Always write to the last row (bottom of screen)
                // Row is guaranteed to be in bounds: BUFFER_HEIGHT - 1 is always < BUFFER_HEIGHT
                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                // Column is now guaranteed to be in bounds after new_line() check above
                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        // Scroll all rows up by one, starting from row 1 (row 0 gets overwritten)
        // Bounds: row ranges from 1 to BUFFER_HEIGHT-1, so row-1 ranges from 0 to BUFFER_HEIGHT-2
        // Both are valid indices in the [0..BUFFER_HEIGHT) range
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                // row - 1 is safe: when row = 1, row - 1 = 0 (valid)
                // when row = BUFFER_HEIGHT - 1, row - 1 = BUFFER_HEIGHT - 2 (valid)
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        // Note: This function assumes row is in bounds. It's only called internally
        // with BUFFER_HEIGHT - 1, which is guaranteed to be valid.
        // For defensive programming, we could add a bounds check here, but it would
        // add runtime overhead. Since this is only called from new_line() with a
        // constant value, the bounds are guaranteed at compile time.
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

use spin::Mutex;

/// Global VGA text buffer writer.
///
/// This static is initialized at compile time, but the actual VGA buffer access
/// only occurs when `lock()` is called. The Writer::new() function creates a pointer
/// to 0xb8000, but doesn't dereference it until write operations occur.
///
/// # Safety
///
/// Safe to use because:
/// 1. The VGA buffer at 0xb8000 is always available in x86_64 bootloader context
/// 2. spin::Mutex provides synchronization (no heap allocation required)
/// 3. First access happens in kernel_main after bootloader has set up memory
/// 4. All buffer accesses use Volatile<T> to prevent compiler optimizations
pub static WRITER: Mutex<Writer> = Mutex::new(Writer::new());

/// Panic-safe function to write directly to VGA buffer without acquiring the lock.
///
/// This function bypasses the Mutex to avoid deadlock if a panic occurs while
/// the WRITER lock is held. It directly accesses the VGA buffer at 0xb8000.
///
/// # Safety
///
/// This function is unsafe because:
/// 1. It bypasses the Mutex synchronization
/// 2. It directly accesses the VGA buffer without lock protection
///
/// However, it's safe to use in a panic handler because:
/// 1. Panics are single-threaded (no concurrent access)
/// 2. We're already in a panic state, so avoiding deadlock is more important
/// 3. The VGA buffer at 0xb8000 is always valid in x86_64 bootloader context
///
/// # Arguments
///
/// * `s` - The string to write to the VGA buffer
/// * `row` - The row to write to (0-24)
/// * `col` - The starting column (0-79)
/// * `color_code` - The color code to use
pub unsafe fn panic_write_string(s: &str, row: usize, col: usize, color_code: ColorCode) {
    // Bounds checking to prevent out-of-bounds access
    if row >= BUFFER_HEIGHT || col >= BUFFER_WIDTH {
        return;
    }
    
    // SAFETY: 0xb8000 is the standard VGA text buffer address.
    // This is safe in panic context because we're single-threaded and
    // the buffer is always available in bootloader context.
    // The function is marked unsafe, so callers must ensure proper usage.
    //
    // We use a raw pointer to write directly to avoid creating a mutable reference
    // to the Buffer, which might already be mutably borrowed by WRITER (which caused
    // the panic or is held during panic). Aliasing &mut is UB, so we avoid it.
    let buffer_ptr = 0xb8000 as *mut ScreenChar;
    
    let mut current_col = col;
    for byte in s.bytes() {
        if current_col >= BUFFER_WIDTH {
            break;
        }
        
        // Filter valid characters (same as Writer::write_string)
        let char_byte = match byte {
            0x20..=0x7e | b'\n' => byte,
            _ => 0xfe, // Block character for invalid bytes
        };
        
        if char_byte == b'\n' {
            // Newline handling would require row management, but for panic
            // messages we'll just stop to keep it simple
            break;
        }
        
        // Calculate offset: row * WIDTH + col
        let offset = row * BUFFER_WIDTH + current_col;

        // SAFETY:
        // 1. buffer_ptr (0xb8000) is valid.
        // 2. offset is within bounds (checked at start and loop condition).
        // 3. We use write_volatile to prevent optimization.
        buffer_ptr.add(offset).write_volatile(ScreenChar {
            ascii_character: char_byte,
            color_code,
        });
        
        current_col += 1;
    }
}

