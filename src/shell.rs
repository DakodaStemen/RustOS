//! Tiny line-oriented shell driven by the async keyboard task.
use crate::{interrupts, print, println, vga_buffer};
use alloc::string::String;
use lazy_static::lazy_static;
use spin::Mutex;

const PROMPT: &str = "rustos> ";

lazy_static! {
    static ref LINE: Mutex<String> = Mutex::new(String::new());
}

pub fn init() {
    println!();
    println!("Type 'help' for commands.");
    print!("{}", PROMPT);
}

pub fn handle_char(c: char) {
    match c {
        '\n' | '\r' => {
            println!();
            let line = core::mem::take(&mut *LINE.lock());
            run(line.trim());
            print!("{}", PROMPT);
        }
        '\u{8}' => {
            if LINE.lock().pop().is_some() {
                print!("\u{8}");
            }
        }
        c if (' '..='~').contains(&c) => {
            let mut l = LINE.lock();
            if l.len() < 70 {
                l.push(c);
                print!("{}", c);
            }
        }
        _ => {}
    }
}

fn run(line: &str) {
    let (cmd, rest) = match line.split_once(' ') {
        Some((c, r)) => (c, r.trim()),
        None => (line, ""),
    };
    match cmd {
        "" => {}
        "help" => println!("commands: help ticks uptime echo clear version"),
        "ticks" => println!("timer ticks: {}", interrupts::ticks()),
        "uptime" => println!("uptime: {} ms", interrupts::uptime_ms()),
        "echo" => println!("{}", rest),
        "clear" => vga_buffer::clear_screen(),
        "version" => println!("RustOS {}", env!("CARGO_PKG_VERSION")),
        other => println!("unknown command: {}", other),
    }
}
