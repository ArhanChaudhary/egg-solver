//! What the binaries (`solve`, `bench`, `gen`, `cubeopt`) share, with the
//! `cli` feature: logging (`-v`, `RUST_LOG`), how a table is loaded
//! (`--load`, `--prefault`) and what it got (page sizes), shell completions
//! and exit statuses. All of it runs before or after a search, never during
//! one. (JSON is printed by the binaries: `serde_json`'s `PartialEq` impls
//! would break type inference in the library's tests.)

use crate::tables::file::{LoadMode, LoadOptions};
use crate::tables::{h48_bytes, H48Table, TableError};
use std::io::Write as _;
use std::process::ExitCode;
use std::time::Instant;

/// `-v`: diagnostics on stderr.
#[derive(clap::Args, Clone, Copy, Debug)]
pub struct Verbosity {
    /// Log diagnostics to stderr (table loading and pages, threads, lanes);
    /// -vv for more. `RUST_LOG` (e.g. `debug`) overrides it.
    #[arg(short, long, action = clap::ArgAction::Count, global = true, display_order = 101)]
    pub verbose: u8,
}

impl Verbosity {
    /// Start logging to stderr: warnings only by default (nothing in normal
    /// runs, so that stderr is as before), more with each `-v`.
    pub fn init_logging(self) {
        let level = match self.verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };
        let start = Instant::now();
        env_logger::Builder::new()
            .filter_level(level)
            .parse_default_env()
            .format(move |out, record| {
                writeln!(
                    out,
                    "[{:8.3}s {} {}] {}",
                    start.elapsed().as_secs_f64(),
                    record.level(),
                    record.target(),
                    record.args()
                )
            })
            .init();
    }
}

/// The value parser of a count of at least 1 (threads, lanes).
#[must_use]
pub fn positive() -> clap::builder::RangedU64ValueParser<usize> {
    clap::builder::RangedU64ValueParser::new().range(1..)
}

/// How a table file is brought into memory.
#[derive(clap::Args, Clone, Copy, Debug)]
pub struct LoadArgs {
    /// How to load the table: `map` the file, `copy` it into memory (with
    /// transparent huge pages), or `auto`: map on hugetlbfs, /dev/shm and
    /// other systems than Linux, else copy.
    #[arg(
        long,
        value_name = "auto|map|copy",
        env = "CUBEOPT_LOAD",
        default_value = "auto",
        value_parser = parse_load_mode,
        display_order = 90
    )]
    pub load: LoadMode,
    /// With `map`: read the whole table at start (1), or each page on first
    /// use (0).
    #[arg(
        long,
        value_name = "0|1",
        env = "CUBEOPT_PREFAULT",
        default_value = "1",
        value_parser = parse_prefault,
        action = clap::ArgAction::Set,
        display_order = 91
    )]
    pub prefault: bool,
}

/// `--load` (an empty `CUBEOPT_LOAD` is `auto`, as before clap).
fn parse_load_mode(text: &str) -> Result<LoadMode, String> {
    match text {
        "auto" | "" => Ok(LoadMode::Auto),
        "map" => Ok(LoadMode::Map),
        "copy" => Ok(LoadMode::Copy),
        _ => Err("expected auto, map or copy".to_string()),
    }
}

/// `--prefault` (an empty `CUBEOPT_PREFAULT` is 1, as before clap).
fn parse_prefault(text: &str) -> Result<bool, String> {
    match text {
        "1" | "" => Ok(true),
        "0" => Ok(false),
        _ => Err("expected 0 or 1".to_string()),
    }
}

impl LoadArgs {
    /// The options for `H48Table::open_with`.
    #[must_use]
    pub fn options(self) -> LoadOptions {
        LoadOptions {
            mode: self.load,
            prefault: self.prefault,
        }
    }

    /// Open the table at `path`, logging how it is loaded and what pages it
    /// got.
    ///
    /// # Errors
    /// As `H48Table::open_with`, as a `BadInput` (exit status 2, as `solve`
    /// gave before).
    pub fn open(self, path: &str) -> anyhow::Result<H48Table> {
        self.open_logged(path).map_err(|error| {
            // (Most messages name the file already.)
            let what = if error.to_string().contains(path) {
                "cannot open table".to_string()
            } else {
                format!("cannot open table {path}")
            };
            anyhow::Error::new(error).context(BadInput(what))
        })
    }

    /// `open`, with the table's own error.
    fn open_logged(self, path: &str) -> Result<H48Table, TableError> {
        let options = self.options();
        let mode = options.mode_for(path);
        log::info!(
            "table {path}: {mode:?}{}",
            if mode == LoadMode::Map && !options.prefault {
                ", pages read on first use"
            } else {
                ""
            }
        );
        let start = Instant::now();
        let table = H48Table::open_with(path, &options)?;
        let len = usize::try_from(h48_bytes(table.meta.k)).unwrap_or(usize::MAX);
        log::info!(
            "table {path}: h{} base {}, {:.2} GiB, loaded in {:.2}s",
            table.meta.k,
            table.meta.base,
            gib(len),
            start.elapsed().as_secs_f64()
        );
        if log::log_enabled!(log::Level::Info) {
            log::info!(
                "table {path}: {}",
                page_report(table.memory.ptr().addr(), len)
            );
        }
        Ok(table)
    }
}

/// Bytes in GiB, for messages.
#[must_use]
pub fn gib(bytes: usize) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "a report")]
    let gib = bytes as f64 / f64::from(1u32 << 30);
    gib
}

/// The page sizes of the memory at `start..start + len` (from
/// `/proc/self/smaps`): 1 GiB or 2 MiB pages (hugetlbfs), else how much of
/// it is in transparent huge pages.
#[cfg(target_os = "linux")]
fn page_report(start: usize, len: usize) -> String {
    let Ok(smaps) = std::fs::read_to_string("/proc/self/smaps") else {
        return "page sizes unknown (no /proc/self/smaps)".to_string();
    };
    let end = start.saturating_add(len);
    // (KiB, as smaps gives them.)
    let (mut page, mut resident, mut huge) = (0u64, 0u64, 0u64);
    let mut inside = false;
    for line in smaps.lines() {
        let mut words = line.split_whitespace();
        let first = words.next().unwrap_or("");
        // A mapping's header line starts with its address range.
        let range = first.split_once('-').and_then(|(a, b)| {
            Some((
                usize::from_str_radix(a, 16).ok()?,
                usize::from_str_radix(b, 16).ok()?,
            ))
        });
        if let Some((a, b)) = range {
            inside = a < end && b > start;
            continue;
        }
        let value = words.next().and_then(|v| v.parse::<u64>().ok());
        match (inside, first, value) {
            (true, "KernelPageSize:", Some(kib)) => page = page.max(kib),
            (true, "Rss:", Some(kib)) => resident += kib,
            (true, "AnonHugePages:" | "FilePmdMapped:" | "ShmemPmdMapped:", Some(kib)) => {
                huge += kib;
            }
            _ => {}
        }
    }
    match page {
        0 => "page sizes unknown (not in /proc/self/smaps)".to_string(),
        1_048_576 => "1 GiB pages (hugetlbfs)".to_string(),
        2048 => "2 MiB pages (hugetlbfs)".to_string(),
        _ => {
            #[expect(clippy::cast_precision_loss, reason = "a report")]
            let share = 100.0 * huge as f64 / resident.max(1) as f64;
            format!(
                "{page} KiB pages, {share:.0}% of {:.2} GiB resident in transparent huge pages",
                gib(usize::try_from(resident << 10).unwrap_or(usize::MAX))
            )
        }
    }
}

/// The page sizes of a memory range (unknown off Linux).
#[cfg(not(target_os = "linux"))]
fn page_report(_: usize, _: usize) -> String {
    "page sizes unknown (only reported on Linux)".to_string()
}

/// Lanes per thread for `threads` threads on the table at `path` (of `k`):
/// `asked`, else from `<path>.tune` (`bench tune`), else the untuned
/// default for `k` (`tables::file::default_lanes`). Logs where it came from.
#[must_use]
pub fn lanes(asked: Option<usize>, path: &str, k: u32, threads: usize) -> usize {
    let lanes = asked.unwrap_or_else(|| crate::tables::file::default_lanes(path, k)(threads));
    let source = if asked.is_some() {
        "as asked"
    } else if std::path::Path::new(&format!("{path}.tune")).exists() {
        "from the table's .tune file"
    } else {
        "untuned default"
    };
    log::info!("{threads} threads, {lanes} lanes per thread ({source})");
    lanes
}

/// Print the completion script of the command `C` for `shell`.
pub fn print_completions<C: clap::CommandFactory>(shell: clap_complete::Shell) {
    let mut command = C::command();
    let name = command.get_name().to_string();
    clap_complete::generate(shell, &mut command, name, &mut std::io::stdout());
}

/// An unusable input: an error with exit status 2, the status of usage
/// errors (clap's too), as the binaries gave these before clap.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct BadInput(pub String);

/// The exit status of a binary's run: its own if it ran, else (after
/// printing `name: error: cause: ...` to stderr) 2 for a `BadInput`, 1 for
/// other errors.
#[must_use]
pub fn exit_status(name: &str, result: anyhow::Result<ExitCode>) -> ExitCode {
    match result {
        Ok(status) => status,
        Err(error) => {
            eprintln!("{name}: {error:#}");
            if error.downcast_ref::<BadInput>().is_some() {
                ExitCode::from(2)
            } else {
                ExitCode::FAILURE
            }
        }
    }
}
