//! Threads that never exit, for the threads build: the `parallel::Runner`
//! that `_initialize` installs, so that solves and table generation start no
//! threads after the first ones.
//!
//! With a thread per share and call, a thread that had ended in Rust could
//! stay stuck in wasi-libc's thread exit and hold its worker forever (under
//! load, `solve`s right after one another then found no idle worker: 6 in
//! 200 runs of `wasm/test.mjs`). wasi-libc's exit frees a detached thread's
//! stack before its last steps, so a thread created at that moment can reuse
//! the memory under it. Threads that never exit avoid the window.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

/// The job being run, and the threads.
struct State {
    /// Bumped for each job; a thread runs each one once.
    generation: u64,
    /// The job: shares `1..shares` run on threads 1.. (share 0 on the caller).
    shares: usize,
    work: Option<&'static (dyn Fn(usize) + Sync)>,
    /// Shares not yet finished.
    remaining: usize,
    /// Threads started so far (numbered from 1).
    threads: usize,
}

static STATE: Mutex<State> = Mutex::new(State {
    generation: 0,
    shares: 0,
    work: None,
    remaining: 0,
    threads: 0,
});
/// Signalled when a job starts, and when its last share ends.
static STARTED: Condvar = Condvar::new();
static FINISHED: Condvar = Condvar::new();

fn lock_state() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `parallel::Runner`: `work(t)` for t in `0..n`, share 0 here and the others
/// on the pool's threads (started as needed, never ended). Shares without a
/// thread (one that could not be started), and all of them when called
/// again while a job runs (`on_threads` inside `on_threads`: not done by
/// this crate), run here.
pub fn run(n: usize, work: &(dyn Fn(usize) + Sync)) {
    let mut state = lock_state();
    if n <= 1 || state.work.is_some() {
        drop(state);
        (0..n.max(1)).for_each(work);
        return;
    }
    // SAFETY: the pool's threads use `work` only until `remaining` is 0,
    // which this function waits for before it returns.
    let work_static: &'static (dyn Fn(usize) + Sync) =
        unsafe { std::mem::transmute::<&(dyn Fn(usize) + Sync), _>(work) };
    while state.threads < n - 1 {
        let (index, seen) = (state.threads + 1, state.generation);
        if std::thread::Builder::new()
            .spawn(move || serve(index, seen))
            .is_err()
        {
            break;
        }
        state.threads = index;
    }
    // Shares 1..=pooled run on the pool, the rest here.
    let pooled = state.threads.min(n - 1);
    state.generation += 1;
    state.shares = pooled + 1;
    state.work = Some(work_static);
    state.remaining = pooled;
    STARTED.notify_all();
    drop(state);
    work(0);
    (pooled + 1..n).for_each(work);
    let mut state = lock_state();
    while state.remaining > 0 {
        state = FINISHED.wait(state).unwrap_or_else(PoisonError::into_inner);
    }
    state.work = None;
}

/// Thread `index` of the pool: runs share `index` of each job that has one.
fn serve(index: usize, mut seen: u64) {
    loop {
        let job = {
            let mut state = lock_state();
            while state.generation == seen {
                state = STARTED.wait(state).unwrap_or_else(PoisonError::into_inner);
            }
            seen = state.generation;
            (index < state.shares).then_some(state.work).flatten()
        };
        if let Some(work) = job {
            work(index);
            let mut state = lock_state();
            state.remaining -= 1;
            if state.remaining == 0 {
                FINISHED.notify_all();
            }
        }
    }
}
