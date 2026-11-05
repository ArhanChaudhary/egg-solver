//! Parallel loops that also run where there are no threads (WebAssembly
//! without the threads proposal): the calling thread does one share of the
//! work itself, so one thread spawns nothing. A host can also run them on
//! threads of its own (`set_runner`).

use std::sync::{Mutex, OnceLock, PoisonError};

/// Runs `work(t)` for every t in `0..n`, t = 0 on the calling thread, and
/// returns once all have returned.
pub type Runner = fn(n: usize, work: &(dyn Fn(usize) + Sync));

static RUNNER: OnceLock<Runner> = OnceLock::new();

/// Have `on_threads` use `runner` instead of spawning threads for each call.
/// The wasm threads build installs a pool of threads that never exit (in
/// wasi-libc, a thread could stay stuck in its exit; see `wasm/src/threads.rs`).
/// Returns false if a runner was set already.
pub fn set_runner(runner: Runner) -> bool {
    RUNNER.set(runner).is_ok()
}

/// `work(t)` for t in `0..threads` (at least one): t = 0 on the calling
/// thread, the others on scoped threads (or the `set_runner` threads).
/// Returns the results in order of t.
///
/// # Panics
/// If a spawned worker panics (or cannot be spawned).
pub fn on_threads<R: Send>(threads: usize, work: impl Fn(usize) -> R + Sync) -> Vec<R> {
    let work = &work;
    if let Some(runner) = RUNNER.get() {
        let results: Vec<Mutex<Option<R>>> =
            (0..threads.max(1)).map(|_| Mutex::new(None)).collect();
        runner(results.len(), &|t| {
            *results[t].lock().unwrap_or_else(PoisonError::into_inner) = Some(work(t));
        });
        return results
            .into_iter()
            .map(|r| {
                let result = r.into_inner().unwrap_or_else(PoisonError::into_inner);
                result.expect("the runner ran every share")
            })
            .collect();
    }
    std::thread::scope(|s| {
        let spawned: Vec<_> = (1..threads.max(1))
            .map(|t| s.spawn(move || work(t)))
            .collect();
        let first = work(0);
        std::iter::once(first)
            .chain(spawned.into_iter().map(|h| h.join().unwrap()))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn on_threads_runs_each_share_once_in_order() {
        for threads in [0, 1, 2, 5] {
            let results = super::on_threads(threads, |t| t * 10);
            let want: Vec<usize> = (0..threads.max(1)).map(|t| t * 10).collect();
            assert_eq!(results, want);
        }
    }
}
