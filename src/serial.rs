//! COM1 (0x3F8) serial output. Everything printed to VGA is mirrored here so
//! host-side tests can read kernel output from QEMU's `-serial` backend.
use x86_64::instructions::port::Port;

const COM1: u16 = 0x3F8;

pub fn init() {
    unsafe {
        Port::<u8>::new(COM1 + 1).write(0x00); // disable UART interrupts
        Port::<u8>::new(COM1 + 3).write(0x80); // DLAB on
        Port::<u8>::new(COM1).write(0x03); // divisor 3 => 38400 baud
        Port::<u8>::new(COM1 + 1).write(0x00);
        Port::<u8>::new(COM1 + 3).write(0x03); // 8N1
        Port::<u8>::new(COM1 + 2).write(0xC7); // FIFO
        Port::<u8>::new(COM1 + 4).write(0x0B);
    }
}

pub fn write_str(s: &str) {
    let mut data = Port::<u8>::new(COM1);
    let mut status = Port::<u8>::new(COM1 + 5);
    for b in s.bytes() {
        unsafe {
            while status.read() & 0x20 == 0 {}
            data.write(b);
        }
    }
}

struct SerialWriter;

impl core::fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write_str(s);
        Ok(())
    }
}

#[doc(hidden)]
#[cfg_attr(not(test), allow(dead_code))]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    SerialWriter.write_fmt(args).ok();
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ($crate::serial::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}
