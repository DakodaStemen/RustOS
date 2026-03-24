use volatile::Volatile;
use core::fmt;

pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)]
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

struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: *mut Buffer,
}

// SAFETY: all access goes through spin::Mutex<Writer>, raw pointer is not Send by default.
unsafe impl Send for Writer {}

impl Writer {
    pub fn new() -> Writer {
        Writer {
            column_position: 0,
            color_code: ColorCode::from_colors(Color::Yellow, Color::Black),
            // SAFETY: 0xb8000 is the VGA text buffer, always valid in bootloader context.
            buffer: 0xb8000 as *mut Buffer,
        }
    }

    #[allow(dead_code)]
    pub fn set_color(&mut self, foreground: Color, background: Color) {
        self.color_code = ColorCode::from_colors(foreground, background);
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;
                let color_code = self.color_code;
                unsafe { &mut (*self.buffer).chars[row][col] }.write(ScreenChar {
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
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = unsafe { (*self.buffer).chars[row][col].read() };
                unsafe { &mut (*self.buffer).chars[row - 1][col] }.write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            unsafe { &mut (*self.buffer).chars[row][col] }.write(blank);
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
use lazy_static::lazy_static;

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer::new());
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Write directly to VGA memory without acquiring the WRITER lock.
///
/// Used in panic/exception handlers where the lock may already be held.
/// Uses write_volatile through a raw pointer to avoid aliasing the WRITER's buffer.
///
/// # Safety
/// Caller must ensure `row` < BUFFER_HEIGHT and `col` < BUFFER_WIDTH.
pub unsafe fn panic_write_string(s: &str, row: usize, col: usize, color_code: ColorCode) {
    if row >= BUFFER_HEIGHT || col >= BUFFER_WIDTH {
        return;
    }

    // Cast directly to the underlying 2D array — Volatile<ScreenChar> is repr(transparent).
    let buffer_ptr = 0xb8000 as *mut [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT];

    let mut current_col = col;
    for byte in s.bytes() {
        if current_col >= BUFFER_WIDTH {
            break;
        }

        let char_byte = match byte {
            0x20..=0x7e | b'\n' => byte,
            _ => 0xfe,
        };

        if char_byte == b'\n' {
            break;
        }

        let screen_char = ScreenChar {
            ascii_character: char_byte,
            color_code,
        };

        let char_ptr = core::ptr::addr_of_mut!((*buffer_ptr)[row][current_col]);
        core::ptr::write_volatile(char_ptr, screen_char);

        current_col += 1;
    }
}
