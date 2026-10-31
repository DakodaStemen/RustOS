//! Tiny line-oriented shell driven by the async keyboard task.
use crate::{allocator, interrupts, memory, print, println, qemu, vga_buffer};
use alloc::{string::String, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;

const PROMPT: &str = "rustos> ";
const MAX_LINE: usize = 70;
const MAX_HISTORY: usize = 16;

/// Line buffer plus command history (oldest first).
pub struct Editor {
    line: String,
    history: Vec<String>,
    /// `history.len()` means "editing a fresh line"; lower = browsing history.
    cursor: usize,
    /// The unsent line stashed when the user starts browsing history.
    draft: String,
}

impl Editor {
    pub const fn new() -> Self {
        Editor { line: String::new(), history: Vec::new(), cursor: 0, draft: String::new() }
    }

    pub fn line(&self) -> &str {
        &self.line
    }

    pub fn push(&mut self, c: char) -> bool {
        if self.line.len() < MAX_LINE {
            self.line.push(c);
            true
        } else {
            false
        }
    }

    pub fn backspace(&mut self) -> bool {
        self.line.pop().is_some()
    }

    /// Finish the line: record it in history (skipping blanks and immediate
    /// duplicates) and return it.
    pub fn submit(&mut self) -> String {
        let line = core::mem::take(&mut self.line);
        let trimmed = line.trim();
        if !trimmed.is_empty() && self.history.last().map(String::as_str) != Some(trimmed) {
            if self.history.len() == MAX_HISTORY {
                self.history.remove(0);
            }
            self.history.push(String::from(trimmed));
        }
        self.cursor = self.history.len();
        self.draft.clear();
        line
    }

    /// Move to an older entry. Returns true if the line changed.
    pub fn history_up(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        if self.cursor == self.history.len() {
            self.draft = self.line.clone();
        }
        self.cursor -= 1;
        self.line = self.history[self.cursor].clone();
        true
    }

    /// Move to a newer entry (or back to the draft). Returns true if changed.
    pub fn history_down(&mut self) -> bool {
        if self.cursor >= self.history.len() {
            return false;
        }
        self.cursor += 1;
        self.line = if self.cursor == self.history.len() {
            core::mem::take(&mut self.draft)
        } else {
            self.history[self.cursor].clone()
        };
        true
    }
}

lazy_static! {
    static ref EDITOR: Mutex<Editor> = Mutex::new(Editor::new());
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
            let line = EDITOR.lock().submit();
            run(line.trim());
            print!("{}", PROMPT);
        }
        '\u{8}' => {
            if EDITOR.lock().backspace() {
                print!("\u{8}");
            }
        }
        c if (' '..='~').contains(&c) => {
            if EDITOR.lock().push(c) {
                print!("{}", c);
            }
        }
        _ => {}
    }
}

pub fn handle_key(key: pc_keyboard::KeyCode) {
    use pc_keyboard::KeyCode;
    let mut ed = EDITOR.lock();
    let old_len = ed.line().len();
    let changed = match key {
        KeyCode::ArrowUp => ed.history_up(),
        KeyCode::ArrowDown => ed.history_down(),
        _ => false,
    };
    if changed {
        // Erase the old text, then print the replacement.
        for _ in 0..old_len {
            print!("\u{8}");
        }
        print!("{}", ed.line());
    }
}

fn run(line: &str) {
    let (cmd, rest) = match line.split_once(' ') {
        Some((c, r)) => (c, r.trim()),
        None => (line, ""),
    };
    match cmd {
        "" => {}
        "help" => println!("commands: help ticks uptime mem echo clear version reboot shutdown"),
        "ticks" => println!("timer ticks: {}", interrupts::ticks()),
        "uptime" => println!("uptime: {} ms", interrupts::uptime_ms()),
        "mem" => {
            let (used, free, total) = allocator::stats();
            let (usable, frames) = memory::frame_stats();
            println!("heap: {} KiB used, {} KiB free, {} KiB total", used / 1024, free / 1024, total / 1024);
            println!("ram:  {} KiB usable, {} KiB of frames allocated", usable / 1024, frames / 1024);
        }
        "echo" => println!("{}", rest),
        "clear" => vga_buffer::clear_screen(),
        "version" => println!("RustOS {}", env!("CARGO_PKG_VERSION")),
        "reboot" => {
            println!("rebooting...");
            qemu::reboot();
            println!("reboot failed");
        }
        "shutdown" => {
            println!("shutting down...");
            qemu::power_off();
            println!("power-off not supported here; safe to turn off the machine");
        }
        other => println!("unknown command: {}", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_line(ed: &mut Editor, s: &str) {
        for c in s.chars() {
            ed.push(c);
        }
        ed.submit();
    }

    #[test_case]
    fn history_walks_up_and_down() {
        let mut ed = Editor::new();
        type_line(&mut ed, "one");
        type_line(&mut ed, "two");
        ed.push('x');
        assert!(ed.history_up());
        assert_eq!(ed.line(), "two");
        assert!(ed.history_up());
        assert_eq!(ed.line(), "one");
        assert!(!ed.history_up());
        assert!(ed.history_down());
        assert_eq!(ed.line(), "two");
        assert!(ed.history_down());
        assert_eq!(ed.line(), "x"); // draft restored
        assert!(!ed.history_down());
    }

    #[test_case]
    fn history_skips_blank_and_duplicates() {
        let mut ed = Editor::new();
        type_line(&mut ed, "a");
        type_line(&mut ed, "a");
        type_line(&mut ed, "   ");
        assert!(ed.history_up());
        assert_eq!(ed.line(), "a");
        assert!(!ed.history_up());
    }

    #[test_case]
    fn history_is_bounded() {
        let mut ed = Editor::new();
        for i in 0..(MAX_HISTORY + 5) {
            type_line(&mut ed, &alloc::format!("cmd{}", i));
        }
        let mut n = 0;
        while ed.history_up() {
            n += 1;
        }
        assert_eq!(n, MAX_HISTORY);
    }
}
