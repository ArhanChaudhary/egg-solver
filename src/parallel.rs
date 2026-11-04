//! Parallel loops that also run where there are no threads (WebAssembly
//! without the threads proposal): the calling thread does one share of the
//! work itself, so one thread spawns nothing.

/// `work(t)` for t in `0..threads` (at least one): t = 0 on the calling
/// thread, the others on scoped threads. Returns the results in order of t.
///
/// # Panics
/// If a spawned worker panics (or cannot be spawned).
pub fn on_threads<R: Send>(threads: usize, work: impl Fn(usize) -> R + Sync) -> Vec<R> {
    let work = &work;
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
