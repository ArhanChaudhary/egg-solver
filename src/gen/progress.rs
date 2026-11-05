//! Progress of the table generators. Each finished step is reported as a
//! line, printed to stderr unless a `Reporter` is installed; the `gen`
//! binary installs one on a terminal, which also draws a progress bar of
//! the current step. Reports cost an atomic load each, per chunk of work.

use std::fmt::Arguments;
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
