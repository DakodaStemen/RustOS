use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use crate::gdt;
use lazy_static::lazy_static;
use pic8259::ChainedPics;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = 40;

// SAFETY: ChainedPics::new is const unsafe but does no I/O at construction time.
// Hardware I/O happens in initialize() inside init_idt().
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
    // initialize() unmasks PIC IRQ lines — don't enable CPU interrupts until
    // after the heap is ready or any allocating interrupt handler will blow up.
    unsafe { PICS.lock().initialize() };
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    use crate::vga_buffer::{Color, ColorCode, panic_write_string};
    let color = ColorCode::from_colors(Color::Yellow, Color::Black);
    unsafe { panic_write_string("EXCEPTION: BREAKPOINT", 4, 0, color); }
    let _ = stack_frame;
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame, _error_code: u64) -> !
{
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;
    use crate::vga_buffer::{Color, ColorCode, panic_write_string};

    let color = ColorCode::from_colors(Color::Red, Color::Black);
    unsafe {
        panic_write_string("PAGE FAULT", 2, 0, color);
        if error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
            panic_write_string("CAUSE: PROTECTION VIOLATION", 3, 0, color);
        } else {
            panic_write_string("CAUSE: PAGE NOT PRESENT", 3, 0, color);
        }
    }
    let _ = (stack_frame, Cr2::read());
    panic!("PAGE FAULT");
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
