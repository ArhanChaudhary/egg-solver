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

/// A worker thread of `on_threads` panicked. (Only where panics unwind, as
/// in tests: the release profile aborts on a panic.)
#[derive(Debug, thiserror::Error)]
#[error("a worker thread panicked")]
pub struct WorkerPanicked;

/// `work(t)` for t in `0..threads` (at least one): t = 0 on the calling
/// thread, the others on scoped threads (or the `set_runner` threads). A
/// share whose thread cannot be started runs on the calling thread instead.
/// Returns the results in order of t.
///
/// # Errors
/// If a worker thread panicked.
pub fn on_threads<R: Send>(
    threads: usize,
    work: impl Fn(usize) -> R + Sync,
) -> Result<Vec<R>, WorkerPanicked> {
    let work = &work;
    if let Some(runner) = RUNNER.get() {
        let results: Vec<Mutex<Option<R>>> =
            (0..threads.max(1)).map(|_| Mutex::new(None)).collect();
        runner(results.len(), &|t| {
            if let Some(slot) = results.get(t) {
                *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(work(t));
            }
        });
        // (A share that the runner did not run runs here.)
        return Ok((0..)
            .zip(results)
            .map(|(t, r)| {
                let result = r.into_inner().unwrap_or_else(PoisonError::into_inner);
                result.unwrap_or_else(|| work(t))
            })
            .collect());
    }
    std::thread::scope(|s| {
        let spawned: Vec<_> = (1..threads.max(1))
            .map(|t| {
                (
                    t,
                    std::thread::Builder::new().spawn_scoped(s, move || work(t)),
                )
            })
            .collect();
        let first = work(0);
        // Join every worker before propagating an error. A short-circuiting
        // Result collection would leave later handles to the scope, whose
        // automatic joins would panic if another worker also failed.
        let joined: Vec<_> = spawned
            .into_iter()
            .map(|(t, spawned)| match spawned {
                Ok(handle) => handle.join().map_err(|_| WorkerPanicked),
                Err(_) => Ok(work(t)),
            })
            .collect();
        std::iter::once(Ok(first)).chain(joined).collect()
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn on_threads_runs_each_share_once_in_order() {
        for threads in [0, 1, 2, 5] {
            let results = super::on_threads(threads, |t| t * 10).unwrap();
            let want: Vec<usize> = (0..threads.max(1)).map(|t| t * 10).collect();
            assert_eq!(results, want);
        }
    }

    #[test]
    #[cfg(panic = "unwind")]
    fn on_threads_joins_all_failed_workers_before_returning_error() {
        let result = super::on_threads(3, |thread| {
            assert_eq!(thread, 0, "worker failure for error propagation test");
            thread
        });
        assert!(result.is_err());
    }
}
