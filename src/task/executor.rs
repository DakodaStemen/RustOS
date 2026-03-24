use super::Task;
use alloc::collections::VecDeque;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

pub struct SimpleExecutor {
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    pub fn new() -> SimpleExecutor {
        SimpleExecutor {
            task_queue: VecDeque::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task)
    }

    pub fn run(&mut self) {
        loop {
            let queue_len = self.task_queue.len();
            if queue_len == 0 {
                break;
            }

            for _ in 0..queue_len {
                if let Some(mut task) = self.task_queue.pop_front() {
                    let waker = dummy_waker();
                    let mut context = Context::from_waker(&waker);
                    match task.poll(&mut context) {
                        Poll::Ready(()) => {}
                        Poll::Pending => self.task_queue.push_back(task),
                    }
                }
            }

            // Atomically enable interrupts and halt — avoids missing a wakeup
            // that arrives between the end of the poll loop and the halt.
            x86_64::instructions::interrupts::enable_and_hlt();
        }
    }
}

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(core::ptr::null(), vtable)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}
