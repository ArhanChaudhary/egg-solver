//! What the binaries (`solve`, `bench`, `gen`, `egg-solver`) share, with the
//! `cli` feature: logging (`-v`, `RUST_LOG`), how a table is loaded
//! (`--load`, `--prefault`, `--shm`) and what it got (page sizes), shell
//! completions and exit statuses. All of it runs before or after a search,
//! never during one. (JSON is printed by the binaries: `serde_json`'s
//! `PartialEq` impls would break type inference in the library's tests.)

pub mod preflight;

use crate::tables::file::{LoadMode, LoadOptions, ShmPages};
use crate::tables::shm::{self, Segment};
use crate::tables::{h48_bytes, H48Table, LoadReport, TableError};
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
    /// transparent huge pages), `shm`: attach its shared memory segment,
    /// made from the file first if there is none (Linux; the segment stays
    /// until reboot or `solve --shm-remove`, see README.md "Shared memory"),
    /// or `auto`: the table's segment if one is there, else map on
    /// hugetlbfs, /dev/shm and other systems than Linux, else copy.
    #[arg(
        long,
        value_name = "auto|map|copy|shm",
        env = crate::env::name("EGG_SOLVER_LOAD"),
        default_value = "auto",
        value_parser = parse_load_mode,
        display_order = 90
    )]
    pub load: LoadMode,
    /// The same as `--load shm` (and overrides it).
    #[arg(long, display_order = 91)]
    pub shm: bool,
    /// With `map` and `shm`: read (map) the whole table at start (1), or each
    /// page on first use (0).
    #[arg(
        long,
        value_name = "0|1",
        env = crate::env::name("EGG_SOLVER_PREFAULT"),
        default_value = "1",
        value_parser = parse_prefault,
        action = clap::ArgAction::Set,
        display_order = 92
    )]
    pub prefault: bool,
    /// With `shm`: the page sizes a new segment may get, tried largest first
    /// (1g and 2m pages come from the kernel's hugetlb pool).
    #[arg(
        long,
        value_name = "LIST",
        env = crate::env::name("EGG_SOLVER_SHM_PAGES"),
        default_value = "1g,2m,4k",
        value_parser = parse_shm_pages,
        display_order = 93
    )]
    pub shm_pages: ShmPages,
    /// The shared memory segment's key [default: 0x636f (ASCII "co"), then
    /// the table's k and base, a byte each: 0x636f0709 for h7 base 9].
    #[arg(
        long,
        value_name = "KEY",
        env = crate::env::name("EGG_SOLVER_SHM_KEY"),
        value_parser = parse_shm_key,
        display_order = 94
    )]
    pub shm_key: Option<u32>,
}

/// `--load` (an empty `EGG_SOLVER_LOAD` is `auto`, as before clap).
fn parse_load_mode(text: &str) -> Result<LoadMode, String> {
    match text {
        "auto" | "" => Ok(LoadMode::Auto),
        "map" => Ok(LoadMode::Map),
        "copy" => Ok(LoadMode::Copy),
        "shm" => Ok(LoadMode::Shm),
        _ => Err("expected auto, map, copy or shm".to_string()),
    }
}

/// `--shm-pages`.
fn parse_shm_pages(text: &str) -> Result<ShmPages, String> {
    ShmPages::parse(text).ok_or_else(|| "expected a list of 1g, 2m and 4k, e.g. 2m,4k".to_string())
}

/// `--shm-key`.
fn parse_shm_key(text: &str) -> Result<u32, String> {
    shm::parse_key(text).ok_or_else(|| "expected a key other than 0, e.g. 0x636f0709".to_string())
}

/// `--prefault` (an empty `EGG_SOLVER_PREFAULT` is 1, as before clap).
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
            mode: if self.shm { LoadMode::Shm } else { self.load },
            prefault: self.prefault,
            shm_pages: self.shm_pages,
            shm_key: self.shm_key,
        }
    }

    /// Open the table at `path`, logging how it is loaded and what pages it
    /// got.
    ///
    /// # Errors
    /// As `H48Table::open_with`, as a `BadInput` (exit status 2, as `solve`
    /// gave before).
    /// # Safety
    /// Shared table storage must remain unchanged as required by `H48Table::open_with`.
    pub unsafe fn open(self, path: &str) -> anyhow::Result<H48Table> {
        // SAFETY: inherited from this method's shared-storage contract.
        unsafe { self.open_reported(path) }.map(|(table, _)| table)
    }

    /// `open`, and how the table was loaded.
    ///
    /// # Errors
    /// As `open`.
    /// # Safety
    /// Shared table storage must remain unchanged as required by `H48Table::open_with`.
    pub unsafe fn open_reported(self, path: &str) -> anyhow::Result<(H48Table, LoadReport)> {
        // SAFETY: the shared-storage contract includes the preflight inspection.
        unsafe { self.preflight(path) }?;
        // SAFETY: inherited from this method's shared-storage contract.
        unsafe { self.open_logged(path) }.map_err(|error| {
            // (Most messages name the file already.)
            let what = if error.to_string().contains(path) {
                "cannot open table".to_string()
            } else {
                format!("cannot open table {path}")
            };
            anyhow::Error::new(error).context(BadInput(what))
        })
    }

    /// Before the table is loaded: that the memory for it is there
    /// (`preflight`), for a copy (`--load copy`, or `auto` off hugetlbfs and
    /// /dev/shm on Linux) and for a new shared memory segment that only 4
    /// KiB pages are left for. Nothing is needed when `auto` or `--shm`
    /// attaches the table's segment, or for a segment on huge pages (pool
    /// pages, which `tables::shm` checks itself).
    unsafe fn preflight(self, path: &str) -> anyhow::Result<()> {
        let options = self.options();
        let mode = options.mode_for(path);
        if !matches!(mode, LoadMode::Copy | LoadMode::Shm) {
            return Ok(());
        }
        // (A table that does not open fails in `open_logged`.)
        let Ok(meta) = crate::tables::read_meta(path) else {
            return Ok(());
        };
        // SAFETY: inherited from the loading API's shared-storage contract.
        if matches!(options.mode, LoadMode::Auto | LoadMode::Shm)
            && unsafe { shm::loaded(path, &options) }
        {
            return Ok(());
        }
        if mode == LoadMode::Copy {
            return preflight::memory(
                &format!("copying table {path} (h{}) into memory", meta.k),
                h48_bytes(meta.k),
                "use a copy on hugetlbfs (mapped, not copied: --load map) or a smaller table",
            );
        }
        if shm::new_segment_page(path, &options) == Some(shm::NORMAL_PAGE) {
            preflight::memory(
                &format!(
                    "a shared memory segment of table {path} (h{}) on 4 KiB pages",
                    meta.k
                ),
                shm::segment_bytes(h48_bytes(meta.k), shm::NORMAL_PAGE),
                "allow huge pages (README.md \"Shared memory\") or use a smaller table",
            )?;
        }
        Ok(())
    }

    /// `open_reported`, with the table's own error.
    unsafe fn open_logged(self, path: &str) -> Result<(H48Table, LoadReport), TableError> {
        let options = self.options();
        let mode = options.mode_for(path);
        log::info!(
            "table {path}: {mode:?}{}{}",
            if options.mode == LoadMode::Auto && cfg!(target_os = "linux") {
                " (or the table's shared memory segment, if there)"
            } else {
                ""
            },
            if matches!(mode, LoadMode::Map | LoadMode::Shm) && !options.prefault {
                ", pages read on first use"
            } else {
                ""
            }
        );
        let start = Instant::now();
        let mut waiting = false;
        let (table, report) = loop {
            // SAFETY: inherited from this method's shared-storage contract.
            match unsafe { H48Table::open_reporting(path, &options) } {
                // (Another process is filling the segment: wait for it.)
                Err(TableError::ShmLoading { key, pid }) => {
                    if !waiting {
                        log::warn!(
                            "table {path}: shared memory segment {key:#010x} is being loaded by \
                             process {pid}: waiting for it"
                        );
                        waiting = true;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                result => break result?,
            }
        };
        for note in &report.notes {
            log::info!("table {path}: {note}");
        }
        let len = usize::try_from(h48_bytes(table.meta.k)).unwrap_or(usize::MAX);
        log::info!(
            "table {path}: h{} base {}, {:.2} GiB, {:?}, loaded in {:.2}s",
            table.meta.k,
            table.meta.base,
            gib(len),
            report.mode,
            start.elapsed().as_secs_f64()
        );
        if let Some(segment) = &report.segment {
            log_segment(path, segment, &report.notes, options.shm_pages);
        }
        if log::log_enabled!(log::Level::Info) {
            log::info!(
                "table {path}: {}",
                page_report(table.memory.ptr().addr(), len)
            );
        }
        Ok((table, report))
    }

    /// `solve --shm-load`: make the table's shared memory segment from the
    /// file (or check the one there) and print it, without solving.
    ///
    /// # Errors
    /// As `open` with `--load shm`.
    /// # Safety
    /// Shared table storage must remain unchanged as required by `H48Table::open_with`.
    pub unsafe fn shm_load(self, path: &str) -> anyhow::Result<ExitCode> {
        let args = LoadArgs { shm: true, ..self };
        let start = Instant::now();
        // SAFETY: inherited from this method's shared-storage contract.
        let (table, report) = unsafe { args.open_reported(path) }?;
        let seconds = start.elapsed().as_secs_f64();
        let segment = report
            .segment
            .ok_or_else(|| anyhow::anyhow!("no segment"))?;
        let how = if segment.created {
            format!("made from {path} in {seconds:.2}s")
        } else {
            format!(
                "already loaded (from {} by process {})",
                segment.source, segment.creator
            )
        };
        println!(
            "segment {:#010x} shmid {}: h{} base {}, {:.2} GiB on {} pages, {how}",
            segment.key,
            segment.id,
            table.meta.k,
            table.meta.base,
            gib(usize::try_from(segment.bytes).unwrap_or(usize::MAX)),
            shm::page_name(segment.page_bytes),
        );
        Ok(ExitCode::SUCCESS)
    }

    /// `solve --shm-remove`: remove the table's shared memory segment (as
    /// `ipcrm -M KEY`; the key from the table's k and base, or `--shm-key`).
    ///
    /// # Errors
    /// If the table's k and base cannot be read, or the segment cannot be
    /// removed (another user's).
    ///
    /// # Safety
    /// The segment must follow [`shm::remove`]'s publication contract.
    pub unsafe fn shm_remove(self, path: &str) -> anyhow::Result<ExitCode> {
        let key = if let Some(key) = self.shm_key {
            key
        } else {
            let meta = crate::tables::read_meta(path).map_err(|error| {
                anyhow::Error::new(error).context(BadInput(format!("cannot read table {path}")))
            })?;
            shm::key(meta.k, meta.base)
        };
        // SAFETY: required by this method.
        match unsafe { shm::remove(key) }? {
            Some(removed) => {
                let still = match removed.attached {
                    0 => String::new(),
                    n => format!(" (attached by {n} processes: freed when they detach)"),
                };
                println!(
                    "removed segment {key:#010x} shmid {}: {}, {:.2} GiB{still}",
                    removed.id,
                    removed.held,
                    gib(usize::try_from(removed.bytes).unwrap_or(usize::MAX))
                );
            }
            None => println!("no segment {key:#010x} for {path}"),
        }
        Ok(ExitCode::SUCCESS)
    }
}

/// Log the table's shared memory segment: its page size, and whether it was
/// made or found; warn if it is on 4 KiB pages although huge pages were
/// allowed (much slower).
fn log_segment(path: &str, segment: &Segment, notes: &[String], allowed: ShmPages) {
    let key = segment.key;
    let how = if segment.created {
        "made from the file".to_string()
    } else {
        format!(
            "attached (made by process {} from {})",
            segment.creator, segment.source
        )
    };
    log::info!(
        "table {path}: shared memory segment {key:#010x} (shmid {}), {:.2} GiB on {} pages: {how}",
        segment.id,
        gib(usize::try_from(segment.bytes).unwrap_or(usize::MAX)),
        shm::page_name(segment.page_bytes)
    );
    if segment.page_bytes == shm::NORMAL_PAGE && allowed.sizes().any(|page| page > shm::NORMAL_PAGE)
    {
        let fix = if segment.created {
            notes.join("; ")
        } else {
            format!(
                "to move it to huge pages, remove it (`solve --shm-remove {path}`) and load it \
                 again once they can be had"
            )
        };
        log::warn!(
            "table {path}: shared memory segment {key:#010x} is on 4 KiB pages, much slower \
             than huge pages (h7: 27% fewer solves/s at 1 thread, 32% at 48, than on 1 GiB \
             pages; README.md \"Shared memory\"); {fix}"
        );
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
/// `asked`, else from its tune file (`bench tune`, see
/// `tables::file::tune_path`), else the untuned
/// default for `k` (`tables::file::default_lanes`). Logs where it came from.
#[must_use]
pub fn lanes(asked: Option<usize>, path: &str, k: u32, threads: usize) -> usize {
    let lanes = asked.unwrap_or_else(|| crate::tables::file::default_lanes(path, k)(threads));
    let source = if asked.is_some() {
        "as asked"
    } else if std::path::Path::new(&crate::tables::file::tune_path(path)).exists() {
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
