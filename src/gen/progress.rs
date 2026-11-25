//! Progress of the table generators. Each finished step is reported as a
//! line, printed to stderr unless a `Reporter` is installed; the `gen`
//! binary installs one on a terminal, which also draws a progress bar of
//! the current step. Reports cost an atomic load each, per chunk of work.

use std::fmt::Arguments;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::OnceLock;

/// Receives the generators' progress (from any thread).
pub trait Reporter: Send + Sync {
    /// A finished step, as the line printed without a reporter.
    fn line(&self, line: &str);
    /// A step (`name`) of `total` units starts.
    fn start(&self, name: &str, total: u64);
    /// `units` more units of the current step are done.
    fn advance(&self, units: u64);
}

static REPORTER: OnceLock<Box<dyn Reporter>> = OnceLock::new();

/// Send all progress to `reporter` from now on. Returns false (and drops
/// it) if one is installed already.
pub fn install(reporter: Box<dyn Reporter>) -> bool {
    REPORTER.set(reporter).is_ok()
}

/// Whether a reporter is installed.
#[must_use]
pub fn installed() -> bool {
    REPORTER.get().is_some()
}

/// Report a finished step: to the reporter, else to stderr.
pub fn line(line: Arguments<'_>) {
    match REPORTER.get() {
        Some(reporter) => reporter.line(&line.to_string()),
        None => eprintln!("{line}"),
    }
}

/// Start a step of `total` units (for the reporter only).
pub fn start(name: Arguments<'_>, total: u64) {
    if let Some(reporter) = REPORTER.get() {
        reporter.start(&name.to_string(), total);
    }
}

/// `units` more units of the current step are done (for the reporter only).
pub fn advance(units: u64) {
    if let Some(reporter) = REPORTER.get() {
        reporter.advance(units);
    }
}

/// A reporter that keeps one number for a progress bar: how far a whole
/// generation is, in permille, as an atomic that another thread can read
/// (a page reads it in the WebAssembly module's shared memory while the
/// module's thread generates).
///
/// A step's share of the whole is told by its name, with shares measured
/// on h4 tables (2026-11-25, 8 threads, seconds: HTM base 8: prefix 0.6,
/// then two passes of 0.7 and 3.2; QTM base 10: 7.7, then 8.6 and 22.3):
/// - `prefix depth D of P` (`gen::dfs`'s prefix): the first 160
///   permille, each level ten times the one before;
/// - `... depth D of L` (the BFS of h0 and h11): everything, each level
///   eight times the one before;
/// - any other name: a pass of `gen::dfs`: the first up to 350, the
///   second to the end.
#[derive(Default)]
pub struct Overall {
    permille: AtomicU32,
    /// The current step's share (permille from and to) and units.
    from: AtomicU32,
    to: AtomicU32,
    total: AtomicU64,
    done: AtomicU64,
    /// `gen::dfs` passes started.
    passes: AtomicU32,
}

/// The prefix's share of a `gen::dfs` generation, and where its first pass
/// ends (permille).
const PREFIX_END: f64 = 160.0;
const FIRST_PASS_END: u32 = 350;

/// `(index, count)` of a step named `... <word> D of L`.
fn level(name: &str, word: &str) -> Option<(u32, u32)> {
    let (_, rest) = name.split_once(word)?;
    let (index, count) = rest.trim().split_once(" of ")?;
    let (index, count) = (index.parse().ok()?, count.trim().parse().ok()?);
    (1..=count).contains(&index).then_some((index, count))
}

/// Where level `index` of `count` ends, of `whole` permille, when each
/// level is `ratio` times the one before.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "0..=whole, at most 1000"
)]
fn level_end(index: u32, count: u32, ratio: f64, whole: f64) -> u32 {
    // (ratio^index - 1) / (ratio^count - 1), with the powers scaled down.
    let part = (ratio.powf(f64::from(index) - f64::from(count)) - ratio.powf(-f64::from(count)))
        / (1.0 - ratio.powf(-f64::from(count)));
    (whole * part.clamp(0.0, 1.0)).round() as u32
}

impl Overall {
    /// Nothing done yet.
    #[must_use]
    pub const fn new() -> Self {
        Overall {
            permille: AtomicU32::new(0),
            from: AtomicU32::new(0),
            to: AtomicU32::new(0),
            total: AtomicU64::new(0),
            done: AtomicU64::new(0),
            passes: AtomicU32::new(0),
        }
    }

    /// A generation starts: back to 0.
    pub fn reset(&self) {
        for word in [&self.permille, &self.from, &self.to, &self.passes] {
            word.store(0, Relaxed);
        }
        self.total.store(0, Relaxed);
        self.done.store(0, Relaxed);
    }

    /// The generation ended: 1000.
    pub fn finish(&self) {
        self.permille.store(1000, Relaxed);
    }

    /// How far the generation is, 0 to 1000 (it never goes back).
    #[must_use]
    pub fn permille(&self) -> &AtomicU32 {
        &self.permille
    }
}

impl Reporter for &'static Overall {
    fn line(&self, _: &str) {}

    fn start(&self, name: &str, total: u64) {
        let (from, to) = if let Some((index, count)) = level(name, "prefix depth ") {
            (
                level_end(index - 1, count, 10.0, PREFIX_END),
                level_end(index, count, 10.0, PREFIX_END),
            )
        } else if let Some((index, count)) = level(name, " depth ") {
            (
                level_end(index - 1, count, 8.0, 1000.0),
                level_end(index, count, 8.0, 1000.0),
            )
        } else if self.passes.fetch_add(1, Relaxed) == 0 {
            (level_end(1, 1, 10.0, PREFIX_END), FIRST_PASS_END)
        } else {
            (FIRST_PASS_END, 1000)
        };
        self.from.store(from, Relaxed);
        self.to.store(to, Relaxed);
        self.total.store(total.max(1), Relaxed);
        self.done.store(0, Relaxed);
        self.permille.fetch_max(from, Relaxed);
    }

    #[expect(clippy::cast_possible_truncation, reason = "at most 1000")]
    fn advance(&self, units: u64) {
        let total = self.total.load(Relaxed).max(1);
        let done = (self.done.fetch_add(units, Relaxed) + units).min(total);
        let (from, to) = (self.from.load(Relaxed), self.to.load(Relaxed));
        let span = u64::from(to.saturating_sub(from));
        // (`done * span` fits: both are far below 2^32.)
        let now = from + (done.saturating_mul(span) / total) as u32;
        self.permille.fetch_max(now.min(1000), Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::{Overall, Reporter};
    use std::sync::atomic::Ordering::Relaxed;

    /// A `gen::dfs` generation as its steps are reported: the prefix's
    /// levels, then two passes; the number only grows, and ends at 1000.
    #[test]
    fn overall_follows_a_dfs_generation() {
        static OVERALL: Overall = Overall::new();
        let reporter = &OVERALL;
        let read = || OVERALL.permille().load(Relaxed);
        let mut last = 0;
        let mut step = |name: &str, total: u64, at_end: u32| {
            reporter.start(name, total);
            for _ in 0..total / 64 {
                reporter.advance(64);
                assert!(read() >= last && read() <= at_end, "{name}: {}", read());
                last = read();
            }
            reporter.advance(total % 64);
            assert_eq!(read(), at_end, "{name}");
            last = at_end;
        };
        // (Each prefix level is ten times the one before: 160 / 1111 * 1, 11, 111, 1111.)
        step("prefix depth 1 of 4", 1, 0);
        step("prefix depth 2 of 4", 4, 2);
        step("prefix depth 3 of 4", 34, 16);
        step("prefix depth 4 of 4", 333, 160);
        step("h4 depth 9", 3815, 350);
        step("h4 depth 10", 3815, 1000);
        reporter.line("ignored");
        OVERALL.reset();
        assert_eq!(read(), 0);
        // Units beyond a step's total (a last partial chunk) do not pass its end.
        reporter.start("prefix depth 2 of 2", 10);
        reporter.advance(4096);
        assert_eq!(read(), 160);
        OVERALL.finish();
        assert_eq!(read(), 1000);
    }

    /// The BFS of h0 and h11: every level in the whole bar, each eight
    /// times the one before.
    #[test]
    fn overall_follows_a_bfs_generation() {
        static OVERALL: Overall = Overall::new();
        let reporter = &OVERALL;
        let read = || OVERALL.permille().load(Relaxed);
        let mut ends = Vec::new();
        for depth in 1..=10 {
            reporter.start(&format!("h0 depth {depth} of 10"), 6);
            for _ in 0..6 {
                reporter.advance(1);
            }
            ends.push(read());
        }
        assert!(ends.windows(2).all(|w| w[0] <= w[1]), "{ends:?}");
        assert_eq!(ends[9], 1000);
        // (The last level alone is 7/8 of the whole.)
        assert_eq!(ends[8], 125);
        assert!(ends[0] <= 1);
    }
}
