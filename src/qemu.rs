//! Exit QEMU via the `isa-debug-exit` device (port 0xf4) and power off.
use x86_64::instructions::port::Port;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
#[cfg_attr(not(test), allow(dead_code))]
pub enum QemuExitCode {
    /// QEMU process exit status is `(0x10 << 1) | 1 = 33`.
    Success = 0x10,
    /// QEMU process exit status is `(0x11 << 1) | 1 = 35`.
    Failed = 0x11,
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn exit_qemu(code: QemuExitCode) -> ! {
    unsafe { Port::new(0xf4).write(code as u32) };
    loop {
        x86_64::instructions::hlt();
    }
}

/// ACPI/PM power-off as implemented by QEMU (i440fx: 0x604, older: 0xB004).
/// Returns only if the emulator ignored both writes.
pub fn power_off() {
    unsafe {
        Port::<u16>::new(0x604).write(0x2000);
        Port::<u16>::new(0xB004).write(0x2000);
    }
}

/// Reset the CPU by pulsing the keyboard controller's reset line.
/// Returns only if the reset was ignored.
pub fn reboot() {
    unsafe { Port::<u8>::new(0x64).write(0xFE) };
}
