use crate::print;
use crossbeam_queue::ArrayQueue;
use core::{pin::Pin, task::{Context, Poll}};
use futures_util::stream::Stream;
use futures_util::stream::StreamExt;
use futures_util::task::AtomicWaker;
use pc_keyboard::{layouts, HandleControl, Keyboard, ScancodeSet1};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref SCANCODE_QUEUE: ArrayQueue<u8> = ArrayQueue::new(100);
}

static WAKER: AtomicWaker = AtomicWaker::new();

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        // Register waker first so we don't miss a notification between
        // the queue check and returning Pending
        WAKER.register(cx.waker());

        match SCANCODE_QUEUE.pop() {
            Some(scancode) => Poll::Ready(Some(scancode)),
            None => Poll::Pending,
        }
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    pc_keyboard::DecodedKey::Unicode(character) => print!("{}", character),
                    pc_keyboard::DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}

pub fn add_scancode(scancode: u8) {
    let _ = SCANCODE_QUEUE.push(scancode);
    WAKER.wake();
}
