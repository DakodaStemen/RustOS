//! In-kernel tests, run with `cargo test` (boots QEMU via `bootimage runner`).
use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};

#[test_case]
fn breakpoint_exception_does_not_crash() {
    x86_64::instructions::interrupts::int3();
}

#[test_case]
fn heap_is_at_least_one_mib() {
    assert!(crate::allocator::HEAP_SIZE >= 1024 * 1024);
    let (_, _, total) = crate::allocator::stats();
    assert!(total >= 1024 * 1024);
}

#[test_case]
fn box_allocation() {
    let b = Box::new(41);
    assert_eq!(*b, 41);
}

#[test_case]
fn large_vec() {
    let n = 1000;
    let v: Vec<u64> = (0..n).collect();
    assert_eq!(v.iter().sum::<u64>(), (n - 1) * n / 2);
}

#[test_case]
fn many_boxes_reuse_memory() {
    // Would exhaust the heap if freed blocks were not reused.
    for i in 0..50_000u64 {
        let b = Box::new(i);
        assert_eq!(*b, i);
    }
}

#[test_case]
fn allocation_bigger_than_old_heap() {
    // The old heap was 100 KiB; 512 KiB only fits after the heap growth.
    let v = alloc::vec![7u8; 512 * 1024];
    assert_eq!(v[512 * 1024 - 1], 7);
}

#[test_case]
fn btreemap_works() {
    let mut m = BTreeMap::new();
    for i in 0..100 {
        m.insert(i, i * 2);
    }
    assert_eq!(m[&50], 100);
}

#[test_case]
fn executor_runs_spawned_task() {
    use crate::task::{executor::Executor, Task};
    use core::sync::atomic::{AtomicUsize, Ordering};
    static DONE: AtomicUsize = AtomicUsize::new(0);

    let mut ex = Executor::new();
    ex.spawn(Task::new(async { DONE.fetch_add(1, Ordering::SeqCst); }));
    ex.spawn(Task::new(async { DONE.fetch_add(10, Ordering::SeqCst); }));
    ex.run_ready_tasks();
    assert_eq!(DONE.load(Ordering::SeqCst), 11);
}

#[test_case]
fn executor_wakes_pending_task() {
    use crate::task::{executor::Executor, Task};
    use core::{future::Future, pin::Pin, task::{Context, Poll, Waker}};
    use core::sync::atomic::{AtomicUsize, Ordering};
    use spin::Mutex;

    // Future that is Pending once, stashes its waker, then is Ready.
    static WAKER: Mutex<Option<Waker>> = Mutex::new(None);
    static STAGE: AtomicUsize = AtomicUsize::new(0);
    struct YieldOnce(bool);
    impl Future for YieldOnce {
        type Output = ();
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                return Poll::Ready(());
            }
            self.0 = true;
            *WAKER.lock() = Some(cx.waker().clone());
            Poll::Pending
        }
    }

    let mut ex = Executor::new();
    ex.spawn(Task::new(async {
        STAGE.store(1, Ordering::SeqCst);
        YieldOnce(false).await;
        STAGE.store(2, Ordering::SeqCst);
    }));
    ex.run_ready_tasks();
    assert_eq!(STAGE.load(Ordering::SeqCst), 1); // parked, not re-polled
    ex.run_ready_tasks();
    assert_eq!(STAGE.load(Ordering::SeqCst), 1); // still parked: no spurious poll
    WAKER.lock().take().unwrap().wake();
    ex.run_ready_tasks();
    assert_eq!(STAGE.load(Ordering::SeqCst), 2);
}
