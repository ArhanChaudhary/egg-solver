//! Table files: where a table's `k` and `base` are stored, and how its data
//! is brought into memory.
//!
//! A table file holds the table data at offset 0 (so that it can be mapped
//! as is), followed by a 4 KiB text trailer in the last 4 KiB of the file:
//! a magic line, then `key value` lines (`layout`, `k`, `base`, `bytes`, and
//! the code `distribution` and `checksum` of the data when known), padded
//! with NUL bytes. Unknown keys are ignored, except that a `layout` other
//! than `h48` (the index layout of `coord`) or `h48-qtm` is refused: a
//! variant table (say with a parity bit in place of an eo bit) must not be
//! read as an h48 one. `h48-qtm` is the same layout with quarter-turn-metric
//! distances (`metric`): a distinct layout, so that readers without QTM
//! refuse those tables (their values are not lower bounds on HTM distances).
//! Files on hugetlbfs are whole 1 GiB pages; their trailer sits in the
//! slack after the data, so tables made before trailers existed can be
//! stamped in place (`gen stamp`). Tables without a trailer are described by
//! a `<path>.meta` sidecar (`k base`, or `k base qtm`), or, for tables on hugetlbfs (and
//! legacy `/mnt/huge` paths), by `tables/<name>.hugemeta` in the build folder. A table can
//! also be loaded into a shared memory segment that outlives the process
//! (`LoadMode::Shm`, see `tables::shm`).

pub use super::shm::ShmPages;
use crate::metric::Metric;
use std::io::{Read, Seek, SeekFrom, Write};

/// Size of the trailer at the end of a table file.
pub const TRAILER_BYTES: u64 = 4096;
/// `TRAILER_BYTES` as a length.
const TRAILER_LEN: usize = 4096;
const _: () = assert!(TRAILER_LEN as u64 == TRAILER_BYTES);
const MAGIC: &str = "egg-solver h48 table v1\n";
const LEGACY_MAGIC: &str = "cubeopt h48 table v1\n";
/// The index layout of the tables this code reads and writes.
pub const LAYOUT: &str = "h48";
/// `LAYOUT` with QTM distances (see the module docs).
pub const LAYOUT_QTM: &str = "h48-qtm";

/// Page size of hugetlbfs mounts used for tables (files there are whole pages).
pub const HUGE_PAGE: u64 = 1 << 30;

/// Why a table file (or the bytes of one in memory) cannot be read,
/// written or used. Errors of the file at `path` carry its path.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TableError {
    /// Reading or writing the file failed.
    #[error("{path}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    /// Neither a trailer nor a meta file gives the table's `k` and base.
    #[error("{path}: no table trailer, and no meta file {meta_path}")]
    NoMeta {
        path: String,
        meta_path: String,
        source: std::io::Error,
    },
    /// A meta file that is not `k base`, or with values the search cannot
    /// use (`k` above 11, a base above 12).
    #[error("bad table meta {0:?}")]
    BadMeta(String),
    /// A trailer whose text does not parse.
    #[error("table trailer: {0}")]
    MalformedTrailer(String),
    /// A trailer with values the search cannot use, or whose data size is
    /// not that of its `k`.
    #[error("{path}: bad table trailer {trailer:?}")]
    BadTrailer { path: String, trailer: Trailer },
    /// A file whose size does not match the table's `k`.
    #[error("table file {path}: {bytes} bytes, but h{k} needs {needed}")]
    WrongSize {
        path: String,
        bytes: u64,
        k: u32,
        needed: u64,
    },
    /// A table larger than the address space (h7 and up on wasm32).
    #[error("h{0} is larger than the address space")]
    TooLarge(u32),
    /// `write_trailer`: no room after the data.
    #[error("{path}: {bytes} bytes, no room for a trailer after {data_bytes} bytes of data")]
    NoRoom {
        path: String,
        bytes: u64,
        data_bytes: u64,
    },
    /// `write_trailer`: the trailer does not read back as written.
    #[error("{path}: trailer reads back as {read:?}, wrote {wrote:?}")]
    ReadBack {
        path: String,
        read: Option<Box<Trailer>>,
        wrote: Box<Trailer>,
    },
    /// Table bytes in memory (`H48Table::from_file_memory`) shorter than a
    /// trailer.
    #[error("{0} bytes: not a table file")]
    NotATableFile(u64),
    /// Table bytes in memory without a trailer at the end.
    #[error("no table trailer at the end")]
    NoTrailer,
    /// Table bytes in memory with an unusable trailer (as `BadTrailer`).
    #[error("bad table trailer {0:?}")]
    UnusableTrailer(Trailer),
    /// Table bytes in memory of another size than their trailer's `k`
    /// needs.
    #[error("{bytes} bytes, but h{k} needs {needed} and a trailer")]
    WrongMemorySize { bytes: u64, k: u32, needed: u64 },
    /// The data's checksum is not the trailer's.
    #[error("checksum {got:016x}, the trailer says {want:016x}: the data is damaged")]
    Checksum { got: u64, want: u64 },
    /// `LoadMode::Shm` off Linux.
    #[error("shared memory segments (--load shm) are only supported on Linux")]
    ShmUnsupported,
    /// `LoadMode::Shm` of a table without a checksum, which could not be told
    /// from another table in a segment.
    #[error(
        "{path}: its trailer has no checksum, which a shared memory segment needs to tell \
         this table from another (`gen stamp` adds one)"
    )]
    ShmNoChecksum { path: String },
    /// The segment under the table's key holds another table (say an older
    /// one of the same k and base): never used.
    #[error(
        "shared memory segment {key:#010x} holds {held}, not the table of {path} ({wanted}); \
         remove it ({hint}) and load again"
    )]
    ShmOtherTable {
        key: u32,
        path: String,
        held: String,
        wanted: String,
        hint: String,
    },
    /// Another process is loading the segment (try again later).
    #[error("shared memory segment {key:#010x} is being loaded by process {pid}")]
    ShmLoading { key: u32, pid: i32 },
    /// The segment cannot be made or used; the message says why and how to
    /// fix it (memory, kernel limits, an interrupted load).
    #[error("shared memory segment {key:#010x}: {message}")]
    Shm { key: u32, message: String },
    /// A shared memory call failed.
    #[error("shared memory segment {key:#010x}: {call}")]
    ShmCall {
        key: u32,
        call: &'static str,
        source: std::io::Error,
    },
    /// Table memory cannot be allocated (or mapped).
    #[error("cannot allocate {bytes} bytes of table memory")]
    Alloc {
        bytes: usize,
        source: std::io::Error,
    },
    /// Memory larger than a slice can be on this target (2 GiB on 32-bit
    /// targets).
    #[error("{0} bytes: larger than a slice can be on this target")]
    TooLargeForSlice(usize),
    /// A worker thread panicked (only where panics unwind, as in tests).
    #[error(transparent)]
    Worker(#[from] crate::parallel::WorkerPanicked),
    /// An environment variable set to a value it cannot have.
    #[error("{var}={value:?}: expected {expected}")]
    BadEnv {
        var: &'static str,
        value: String,
        expected: &'static str,
    },
    /// The cocsep table does not come out as it must (a bug in the cube
    /// model).
    #[error("cocsep table: {what} {found}, expected {expected} (a bug in the cube model)")]
    CocsepModel {
        what: &'static str,
        found: usize,
        expected: usize,
    },
    /// Table generation asked for something out of range.
    #[error("table generation: {0}")]
    Generate(String),
    /// A search in a metric whose distances the table's values do not bound
    /// (QTM tables cannot bound an HTM search).
    #[error("{table} table values are not lower bounds on {search} distances")]
    WrongMetric {
        search: crate::metric::Metric,
        table: crate::metric::Metric,
    },
}

/// `TableError::Io` of the file at `path`.
pub(crate) fn io_error(path: &str) -> impl FnOnce(std::io::Error) -> TableError + '_ {
    move |source| TableError::Io {
        path: path.to_string(),
        source,
    }
}

/// What a table's trailer says about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trailer {
    pub k: u32,
    pub base: u8,
    /// Bytes of table data at the start of the file.
    pub data_bytes: u64,
    /// Number of real entries with each 2-bit code, if recorded.
    pub distribution: Option<[u64; 4]>,
    /// `checksum` of the data, if recorded.
    pub checksum: Option<u64>,
    /// The metric of the distances (`layout`).
    pub metric: Metric,
}

impl Trailer {
    /// The trailer as stored: text lines padded with NUL bytes (the text is
    /// far shorter than the trailer).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        use std::fmt::Write as _;
        let mut text = String::with_capacity(TRAILER_LEN);
        text.push_str(MAGIC);
        let layout = if self.metric.is_qtm() {
            LAYOUT_QTM
        } else {
            LAYOUT
        };
        let _ = writeln!(text, "layout {layout}");
        let _ = writeln!(text, "k {}", self.k);
        let _ = writeln!(text, "base {}", self.base);
        let _ = writeln!(text, "bytes {}", self.data_bytes);
        if let Some(d) = self.distribution {
            let _ = writeln!(text, "distribution {} {} {} {}", d[0], d[1], d[2], d[3]);
        }
        if let Some(c) = self.checksum {
            let _ = writeln!(text, "checksum {c:016x}");
        }
        let mut bytes = text.into_bytes();
        bytes.resize(TRAILER_LEN, 0);
        bytes
    }

    /// Parse a trailer. Returns `Ok(None)` if `bytes` does not start with the
    /// magic line (a file without a trailer).
    ///
    /// # Errors
    /// `MalformedTrailer` if the magic line is there but the rest is
    /// malformed.
    pub fn parse(bytes: &[u8]) -> Result<Option<Trailer>, TableError> {
        let Some(rest) = bytes
            .strip_prefix(MAGIC.as_bytes())
            .or_else(|| bytes.strip_prefix(LEGACY_MAGIC.as_bytes()))
        else {
            return Ok(None);
        };
        let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
        let invalid = |what: &str| TableError::MalformedTrailer(what.to_string());
        let text = std::str::from_utf8(&rest[..end]).map_err(|_| invalid("not text"))?;
        let (mut k, mut base, mut data_bytes) = (None, None, None);
        let (mut distribution, mut checksum) = (None, None);
        let mut metric = Metric::Htm;
        for line in text.lines() {
            let mut words = line.split_whitespace();
            let key = words.next().unwrap_or("");
            let values: Vec<&str> = words.collect();
            let number = |i: usize| -> Result<u64, TableError> {
                values
                    .get(i)
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| invalid(line))
            };
            match key {
                "layout" if values == [LAYOUT] => metric = Metric::Htm,
                "layout" if values == [LAYOUT_QTM] => metric = Metric::Qtm,
                "layout" => {
                    return Err(invalid(&format!("unsupported layout {values:?}")));
                }
                "k" => k = Some(number(0)?),
                "base" => base = Some(number(0)?),
                "bytes" => data_bytes = Some(number(0)?),
                "distribution" => {
                    distribution = Some([number(0)?, number(1)?, number(2)?, number(3)?]);
                }
                "checksum" => {
                    let v = values.first().ok_or_else(|| invalid(line))?;
                    checksum = Some(u64::from_str_radix(v, 16).map_err(|_| invalid(line))?);
                }
                // Unknown keys are allowed (later versions may add some).
                _ => {}
            }
        }
        let (Some(k), Some(base), Some(data_bytes)) = (k, base, data_bytes) else {
            return Err(invalid("k, base or bytes missing"));
        };
        let k = u32::try_from(k).map_err(|_| invalid("k"))?;
        let base = u8::try_from(base).map_err(|_| invalid("base"))?;
        Ok(Some(Trailer {
            k,
            base,
            data_bytes,
            distribution,
            checksum,
            metric,
        }))
    }
}

/// Whether `path` is on a hugetlbfs mount (whose files are whole huge pages
/// and can only be written through a mapping).
#[must_use]
pub fn is_hugetlbfs(path: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        // The statfs magic of hugetlbfs.
        const HUGETLBFS_MAGIC: i64 = 0x9584_58f6;
        let Ok(c_path) = std::ffi::CString::new(path) else {
            return false;
        };
        // A missing file: judge by its folder (the file may be about to be
        // created there).
        let dir = std::path::Path::new(path)
            .parent()
            .and_then(|p| std::ffi::CString::new(p.as_os_str().as_encoded_bytes()).ok());
        for p in [Some(c_path), dir].into_iter().flatten() {
            // SAFETY: `p` is a NUL-terminated path, `st` a valid out-pointer.
            unsafe {
                let mut st: libc::statfs = std::mem::zeroed();
                if libc::statfs(p.as_ptr(), &raw mut st) == 0 {
                    #[allow(clippy::unnecessary_cast, reason = "f_type's type varies by target")]
                    return st.f_type as i64 == HUGETLBFS_MAGIC;
                }
            }
        }
        false
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        false
    }
}

/// File size of a new table of `data_bytes` with its trailer.
#[must_use]
pub fn file_bytes(data_bytes: u64, hugetlbfs: bool) -> u64 {
    let len = data_bytes + TRAILER_BYTES;
    if hugetlbfs {
        len.div_ceil(HUGE_PAGE) * HUGE_PAGE
    } else {
        len
    }
}

/// Read the trailer of a table file, if it has one.
///
/// # Errors
/// If the file cannot be read, or has a malformed trailer.
pub fn read_trailer(path: &str) -> Result<Option<Trailer>, TableError> {
    let read = || -> std::io::Result<Option<Vec<u8>>> {
        let f = std::fs::File::open(path)?;
        let len = f.metadata()?.len();
        if len < TRAILER_BYTES {
            return Ok(None);
        }
        let mut buf = vec![0u8; TRAILER_LEN];
        (&f).seek(SeekFrom::Start(len - TRAILER_BYTES))?;
        (&f).read_exact(&mut buf)?;
        Ok(Some(buf))
    };
    match read().map_err(io_error(path))? {
        Some(buf) => Trailer::parse(&buf),
        None => Ok(None),
    }
}

/// Write `trailer` into the last 4 KiB of an existing table file, then read
/// it back.
///
/// # Errors
/// If the file is too small to hold the data and a trailer after it (the
/// data region is never written), or on any I/O error, or if the trailer
/// does not read back as written.
/// # Safety
/// Nobody may truncate the file or access its trailer during this call. On
/// hugetlbfs this function writes through a temporary shared mapping.
pub unsafe fn write_trailer(path: &str, trailer: &Trailer) -> Result<(), TableError> {
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(io_error(path))?;
    let len = f.metadata().map_err(io_error(path))?.len();
    if trailer
        .data_bytes
        .checked_add(TRAILER_BYTES)
        .is_none_or(|needed| len < needed)
    {
        return Err(TableError::NoRoom {
            path: path.to_string(),
            bytes: len,
            data_bytes: trailer.data_bytes,
        });
    }
    let offset = len - TRAILER_BYTES;
    let bytes = trailer.to_bytes();
    #[cfg(target_os = "linux")]
    if is_hugetlbfs(path) {
        // hugetlbfs has no write(): map the last page and copy the trailer in.
        let page = HUGE_PAGE.min(len);
        let page_start = len - page;
        // (A page is at most 1 GiB, and the offset within it smaller: they
        // fit in `usize` on any target.)
        let as_usize = |x: u64| usize::try_from(x).map_err(std::io::Error::other);
        let as_offset = |x: u64| libc::off_t::try_from(x).map_err(std::io::Error::other);
        let (page_len, in_page, file_offset) = (|| {
            Ok::<_, std::io::Error>((
                as_usize(page)?,
                as_usize(offset - page_start)?,
                as_offset(page_start)?,
            ))
        })()
        .map_err(io_error(path))?;
        // SAFETY: a fresh shared mapping of the file's last page; only the
        // trailer bytes (past the data, checked above) are written.
        unsafe {
            use std::os::unix::io::AsRawFd;
            let p = libc::mmap(
                std::ptr::null_mut(),
                page_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                f.as_raw_fd(),
                file_offset,
            );
            if p == libc::MAP_FAILED {
                return Err(io_error(path)(std::io::Error::last_os_error()));
            }
            if p.is_null() {
                libc::munmap(p, page_len);
                return Err(io_error(path)(std::io::Error::other(
                    "mapping returned a null address",
                )));
            }
            let dst = p.cast::<u8>().add(in_page);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
            libc::munmap(p, page_len);
        }
        return read_back(path, trailer);
    }
    let write = || -> std::io::Result<()> {
        (&f).seek(SeekFrom::Start(offset))?;
        (&f).write_all(&bytes)?;
        f.sync_data()
    };
    write().map_err(io_error(path))?;
    read_back(path, trailer)
}

/// Check that the trailer of `path` reads back as `trailer`.
fn read_back(path: &str, trailer: &Trailer) -> Result<(), TableError> {
    match read_trailer(path)? {
        Some(t) if t == *trailer => Ok(()),
        read => Err(TableError::ReadBack {
            path: path.to_string(),
            read: read.map(Box::new),
            wrote: Box::new(*trailer),
        }),
    }
}

/// Bytes per chunk of `checksum`.
pub const CHECKSUM_CHUNK: usize = 1 << 20;

fn mix(h: u64, x: u64) -> u64 {
    (h ^ x).wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(29)
}

/// Hash of each `CHECKSUM_CHUNK` bytes of `data` (the last chunk may be
/// shorter), in parallel.
///
/// # Errors
/// If a worker thread panicked.
pub fn chunk_hashes(
    data: &[u8],
    threads: usize,
) -> Result<Vec<u64>, crate::parallel::WorkerPanicked> {
    fn chunk_hash(chunk: &[u8]) -> u64 {
        // Four independent lanes (no single dependency chain), then folded.
        let mut lanes = [1u64, 2, 3, 4];
        let (words, rest) = chunk.as_chunks::<32>();
        for w in words {
            for (lane, x) in lanes.iter_mut().zip(w.as_chunks::<8>().0) {
                *lane = mix(*lane, u64::from_le_bytes(*x));
            }
        }
        let mut h = lanes.iter().fold(chunk.len() as u64, |h, &l| mix(h, l));
        for &b in rest {
            h = mix(h, u64::from(b));
        }
        h
    }
    let n = data.len().div_ceil(CHECKSUM_CHUNK);
    let threads = threads.clamp(1, n.max(1));
    let per = n.div_ceil(threads);
    Ok(crate::parallel::on_threads(threads, |t| {
        data.chunks(CHECKSUM_CHUNK)
            .skip(t * per)
            .take(per)
            .map(chunk_hash)
            .collect::<Vec<u64>>()
    })?
    .concat())
}

/// The checksum of `len` bytes of data from its `chunk_hashes` (so that a
/// table generated in parts aligned to chunks needs no second pass).
#[must_use]
pub fn combine_hashes(len: u64, hashes: &[u64]) -> u64 {
    hashes.iter().fold(len, |h, &x| mix(h, x))
}

/// Checksum of table data: a hash of each 1 MiB chunk (in parallel), then of
/// the chunk hashes in order. Detects corruption (bit flips, truncation),
/// not tampering.
///
/// # Errors
/// If a worker thread panicked.
pub fn checksum(data: &[u8], threads: usize) -> Result<u64, crate::parallel::WorkerPanicked> {
    Ok(combine_hashes(
        data.len() as u64,
        &chunk_hashes(data, threads)?,
    ))
}

/// How `H48Table::open` brings a table file into memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadMode {
    /// The table's shared memory segment if there is one that holds it
    /// (`Shm`, Linux); else `Map` for files on hugetlbfs or `/dev/shm`
    /// (already in memory) and on other unix systems; `Copy` for other files
    /// on Linux (anonymous memory gets transparent huge pages reliably, the
    /// page cache may not) and where there are no file mappings (WASI,
    /// Windows).
    Auto,
    /// Map the file read-only (shared with other processes using it).
    Map,
    /// Copy the file into anonymous memory (transparent huge pages where
    /// available), then drop it from the page cache.
    Copy,
    /// Attach the table's shared memory segment, made from the file first if
    /// there is none (Linux; see `tables::shm`). `Auto` also uses a segment
    /// that is there, if it holds this file's table.
    Shm,
}

/// Options of `H48Table::open_with`.
#[derive(Clone, Copy, Debug)]
pub struct LoadOptions {
    pub mode: LoadMode,
    /// With `Map`: fault the whole table in at open (a sequential read), not
    /// on first use (random reads, slow until the table is cached). With
    /// `Shm`: map all of the segment's pages at attach.
    pub prefault: bool,
    /// With `Shm`: the page sizes a new segment may get.
    pub shm_pages: ShmPages,
    /// The segment's key [default: `shm::key` of the table's k and base].
    pub shm_key: Option<u32>,
}

impl Default for LoadOptions {
    fn default() -> LoadOptions {
        LoadOptions {
            mode: LoadMode::Auto,
            prefault: true,
            shm_pages: ShmPages::ALL,
            shm_key: None,
        }
    }
}

impl LoadOptions {
    /// Options from the environment: `EGG_SOLVER_LOAD` = `auto` | `map` |
    /// `copy` | `shm`, `EGG_SOLVER_PREFAULT` = `1` | `0`, `EGG_SOLVER_SHM_PAGES`
    /// (`ShmPages::parse`) and `EGG_SOLVER_SHM_KEY` (`shm::parse_key`).
    ///
    /// # Errors
    /// If a variable is set to another value.
    pub fn from_env() -> Result<LoadOptions, TableError> {
        let mut o = LoadOptions::default();
        let bad = |var, value: String, expected| TableError::BadEnv {
            var,
            value,
            expected,
        };
        if let Ok(v) = crate::env::var("EGG_SOLVER_LOAD") {
            o.mode = match v.as_str() {
                "auto" | "" => LoadMode::Auto,
                "map" => LoadMode::Map,
                "copy" => LoadMode::Copy,
                "shm" => LoadMode::Shm,
                _ => return Err(bad("EGG_SOLVER_LOAD", v, "auto, map, copy or shm")),
            };
        }
        if let Ok(v) = crate::env::var("EGG_SOLVER_SHM_PAGES") {
            o.shm_pages = match ShmPages::parse(&v) {
                Some(pages) => pages,
                None => return Err(bad("EGG_SOLVER_SHM_PAGES", v, "e.g. 1g,2m,4k")),
            };
        }
        if let Ok(v) = crate::env::var("EGG_SOLVER_SHM_KEY") {
            o.shm_key = match super::shm::parse_key(&v) {
                Some(key) => Some(key),
                None => return Err(bad("EGG_SOLVER_SHM_KEY", v, "e.g. 0x636f0709")),
            };
        }
        if let Ok(v) = crate::env::var("EGG_SOLVER_PREFAULT") {
            o.prefault = match v.as_str() {
                "1" | "" => true,
                "0" => false,
                _ => return Err(bad("EGG_SOLVER_PREFAULT", v, "0 or 1")),
            };
        }
        Ok(o)
    }

    /// The mode used for the file at `path` (resolves `Auto`, unless a
    /// segment of the table is there: see `H48Table::open_reporting`).
    #[must_use]
    pub fn mode_for(&self, path: &str) -> LoadMode {
        match self.mode {
            LoadMode::Auto => {
                let in_memory = path.starts_with("/dev/shm") || is_hugetlbfs(path);
                if cfg!(unix) && (!cfg!(target_os = "linux") || in_memory) {
                    LoadMode::Map
                } else {
                    LoadMode::Copy
                }
            }
            m => m,
        }
    }
}

/// Where `bench tune` saves a table's best lane counts. Hugetlbfs cannot
/// write small files: keep tuning beside the metadata on regular storage.
pub(crate) fn tune_path(table: &str) -> String {
    if is_hugetlbfs(table) {
        format!("{}.tune", super::meta_path(table))
    } else {
        format!("{table}.tune")
    }
}

/// The first line of a tune file made by `bench tune` as it measures now:
/// the search speed of `solve`'s two modes. (The files of the previous
/// `bench tune`, which counted the cubes that a stream finished in a few
/// seconds, have no such line and are not used: on a small table few cubes
/// finish in that time, and the best lane count was close to chance.)
const TUNE_HEADER: &str = "# egg-solver tune 2";

/// How `solve` runs, as far as the best lane count goes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TuneMode {
    /// Many cubes in progress at once (`solve -c 0`, the default).
    Batch,
    /// One cube at a time on all threads (`solve -c 1`, a single scramble).
    OneCube,
}

impl TuneMode {
    /// The mode of `solve -c concurrency`.
    #[must_use]
    pub fn of_concurrency(concurrency: usize) -> TuneMode {
        if concurrency == 1 {
            TuneMode::OneCube
        } else {
            TuneMode::Batch
        }
    }

    /// The word that starts the mode's lines in a tune file (and names it
    /// in `bench tune`'s output).
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            TuneMode::Batch => "batch",
            TuneMode::OneCube => "one",
        }
    }
}

/// Save the best lane count per mode and thread count (after the header,
/// `mode threads lanes` lines).
///
/// # Errors
/// If the file cannot be written (e.g. the table's folder is read-only).
pub fn save_tuned_lanes(table: &str, best: &[(TuneMode, usize, usize)]) -> std::io::Result<String> {
    use std::fmt::Write as _;
    let mut text = format!("{TUNE_HEADER}\n");
    for (mode, threads, lanes) in best {
        let _ = writeln!(text, "{} {threads} {lanes}", mode.word());
    }
    let path = tune_path(table);
    std::fs::write(&path, text)?;
    Ok(path)
}

/// Lanes per thread when not tuned (`bench tune`): `lanes::DEFAULT_LANES`,
/// 24, for every table. `solve`'s search (the work-sharing mode) was
/// fastest with 24 on every table measured, `-c 1` and `-c 0` alike. At 1
/// thread on Zen 5 (EPYC 9V45, 2026-11-24, 17- and 18-move sets, seconds
/// with 24, 32, 40, 48 lanes, `-c 1`): h6 4.84, 4.84, 4.96, 5.11; h7 2.69,
/// 2.73, 2.79, 2.88; h8 9.57, 9.73, 9.87, 10.43; h9 5.11, 5.20, 5.34, 5.59
/// (16 lanes: 11 to 17% slower, 8: 65 to 76%). At 48 threads there, 24 won
/// every batch screen of h6 to h10 (docs/performance/PARALLELISM.md). On an
/// Apple M2 (h6, h7; 1, 4 and 8 threads) 16 to 48 lanes are within 1.5% of
/// each other, 40 is 2 to 3% slower at 8 threads, and 8 lanes 20% slower.
/// (Until then: 40 lanes up to h8 and 32 for h9 and h10, from `bench
/// stream`'s search, where 40 lanes were up to 10% faster than 24 on h7:
/// its lanes are smaller than the work-sharing mode's.)
#[must_use]
pub fn untuned_lanes(_k: u32) -> usize {
    crate::lanes::DEFAULT_LANES
}

/// Lanes per thread of `bench stream` (independent cubes, one per lane)
/// for a table of `k`, unless asked otherwise: `bench tune` measures the
/// work-sharing mode, not this one. Smaller tables gain from more lanes
/// (measured, solves/s, 24 -> 40 lanes): h7 +10% at 1T (2.017 -> 2.217)
/// and at 8 threads (15.43 -> 17.07), also on 1 GiB pages (2.017 -> 2.283
/// at 1T: not a TLB effect); h8 equal at 1T, +8.9% at 8 threads (26.73 ->
/// 29.10); h6 equal (+4.5% at 1T, -1 to -5% at 8 threads). h11 keeps
/// `lanes::DEFAULT_LANES`, 24 (tuned at 1 and 48 threads; 40 is 3.4%
/// slower at 8 threads, 197.9 -> 191.2). h9 and h10 (no RAM for them
/// then) get 32, in between.
#[must_use]
pub fn untuned_stream_lanes(k: u32) -> usize {
    match k {
        0..=8 => 40,
        9 | 10 => 32,
        _ => crate::lanes::DEFAULT_LANES,
    }
}

/// The entries (`threads`, `lanes`) of `mode` in the text of a tune file;
/// none if the text is not of this `bench tune` (`TUNE_HEADER`).
fn tuned_entries(text: &str, mode: TuneMode) -> Vec<(usize, usize)> {
    let mut lines = text.lines();
    if lines.next() != Some(TUNE_HEADER) {
        return Vec::new();
    }
    lines
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            if words.next()? != mode.word() {
                return None;
            }
            let mut numbers = words.map(|word| word.parse::<usize>().ok());
            Some((numbers.next()??, numbers.next()??))
        })
        .filter(|&(threads, lanes)| threads > 0 && lanes > 0)
        .collect()
}

/// Of `entries` (`threads`, `lanes`), the lanes of the thread count
/// nearest to `threads` as a ratio (4 threads are nearer to 8 than to 1;
/// of two as near, the larger count's).
fn nearest_lanes(entries: &[(usize, usize)], threads: usize) -> Option<usize> {
    // (The ratio of the larger to the smaller of two counts, as a fraction.)
    let ratio = |t: usize| (t.max(threads) as u128, t.min(threads).max(1) as u128);
    entries
        .iter()
        .min_by(|&&(a, _), &&(b, _)| {
            let ((a_top, a_bottom), (b_top, b_bottom)) = (ratio(a), ratio(b));
            (a_top * b_bottom).cmp(&(b_top * a_bottom)).then(b.cmp(&a))
        })
        .map(|&(_, lanes)| lanes)
}

/// Whether the table at `path` has a tune file that `default_lanes` uses
/// (made by this `bench tune`, with an entry).
#[must_use]
pub fn is_tuned(path: &str) -> bool {
    std::fs::read_to_string(tune_path(path))
        .is_ok_and(|text| !tuned_entries(&text, TuneMode::Batch).is_empty())
}

/// The default lanes per thread for `threads` threads in `mode` on the
/// table at `path`: from `<path>.tune` (`bench tune`: the entry of the
/// mode, or else of the batch mode, at the nearest thread count), else
/// `untuned_lanes(k)`.
pub fn default_lanes(path: &str, k: u32) -> impl Fn(usize, TuneMode) -> usize {
    let text = std::fs::read_to_string(tune_path(path)).unwrap_or_default();
    let batch = tuned_entries(&text, TuneMode::Batch);
    let one = tuned_entries(&text, TuneMode::OneCube);
    move |threads, mode| {
        let of_mode = match mode {
            TuneMode::OneCube => nearest_lanes(&one, threads),
            TuneMode::Batch => None,
        };
        of_mode
            .or_else(|| nearest_lanes(&batch, threads))
            .unwrap_or_else(|| untuned_lanes(k))
    }
}

#[cfg(test)]
mod tests {
    use super::{nearest_lanes, tuned_entries, TuneMode, TUNE_HEADER};

    /// A tune file: the entries of each mode; lines of other kinds and the
    /// files of the previous `bench tune` (no header) are not used.
    #[test]
    fn tune_files_are_read_by_mode() {
        let text =
            format!("{TUNE_HEADER}\nbatch 1 24\nbatch 16 12\none 1 32\nlater 3 4\none x 8\n");
        assert_eq!(tuned_entries(&text, TuneMode::Batch), [(1, 24), (16, 12)]);
        assert_eq!(tuned_entries(&text, TuneMode::OneCube), [(1, 32)]);
        // (As the previous `bench tune` wrote them, and one with zeros.)
        assert_eq!(tuned_entries("1 48\n8 24\n", TuneMode::Batch), []);
        let zeros = format!("{TUNE_HEADER}\nbatch 0 24\nbatch 4 0\n");
        assert_eq!(tuned_entries(&zeros, TuneMode::Batch), []);
        assert_eq!(tuned_entries("", TuneMode::Batch), []);
    }

    /// The entry of the nearest thread count is the nearest as a ratio.
    #[test]
    fn nearest_thread_count_is_by_ratio() {
        let entries = [(1, 10), (8, 20), (16, 30)];
        let lanes = |threads| nearest_lanes(&entries, threads);
        assert_eq!(lanes(1), Some(10));
        assert_eq!(lanes(2), Some(10));
        // (4 is 4 times 1 and half of 8.)
        assert_eq!(lanes(4), Some(20));
        assert_eq!(lanes(8), Some(20));
        assert_eq!(lanes(12), Some(30));
        assert_eq!(lanes(16), Some(30));
        assert_eq!(lanes(1000), Some(30));
        // As near to both: the larger count's.
        assert_eq!(nearest_lanes(&[(1, 10), (16, 30)], 4), Some(30));
        assert_eq!(nearest_lanes(&[(16, 30), (1, 10)], 4), Some(30));
        assert_eq!(nearest_lanes(&[], 4), None);
        assert_eq!(
            nearest_lanes(&[(usize::MAX, 7), (1, 9)], usize::MAX),
            Some(7)
        );
    }

    /// `default_lanes` on a table's tune file: the mode's entry, else the
    /// batch's, else the untuned default (also for a file of the previous
    /// `bench tune`).
    #[test]
    #[cfg_attr(miri, ignore = "files")]
    fn default_lanes_use_the_tune_file() {
        let dir = std::env::temp_dir().join(format!("egg-solver-tune-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let table = dir.join("h0").to_str().unwrap().to_string();
        let untuned = super::untuned_lanes(0);
        assert!(!super::is_tuned(&table));
        assert_eq!(super::default_lanes(&table, 0)(8, TuneMode::Batch), untuned);
        let best = [
            (TuneMode::Batch, 1, 12),
            (TuneMode::Batch, 8, 16),
            (TuneMode::OneCube, 8, 20),
        ];
        let saved = super::save_tuned_lanes(&table, &best).unwrap();
        assert_eq!(saved, format!("{table}.tune"));
        assert!(super::is_tuned(&table));
        let lanes = super::default_lanes(&table, 0);
        assert_eq!(lanes(1, TuneMode::Batch), 12);
        assert_eq!(lanes(6, TuneMode::Batch), 16);
        assert_eq!(lanes(1, TuneMode::OneCube), 20);
        // Only batch entries: for both modes.
        super::save_tuned_lanes(&table, &best[..2]).unwrap();
        assert_eq!(super::default_lanes(&table, 0)(8, TuneMode::OneCube), 16);
        // A file of the previous `bench tune`.
        std::fs::write(&saved, "1 48\n8 48\n").unwrap();
        assert!(!super::is_tuned(&table));
        assert_eq!(super::default_lanes(&table, 0)(8, TuneMode::Batch), untuned);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn previous_table_magic_is_accepted() {
        let trailer = super::Trailer {
            k: 0,
            base: 8,
            data_bytes: crate::tables::h48_bytes(0),
            distribution: None,
            checksum: Some(123),
            metric: Metric::Htm,
        };
        let modern = trailer.to_bytes();
        let mut previous = super::LEGACY_MAGIC.as_bytes().to_vec();
        previous.extend_from_slice(modern.strip_prefix(super::MAGIC.as_bytes()).unwrap());
        previous.resize(super::TRAILER_LEN, 0);
        assert_eq!(super::Trailer::parse(&previous).unwrap(), Some(trailer));
        assert_eq!(super::Trailer::parse(&modern).unwrap(), Some(trailer));
    }

    use super::*;

    #[test]
    fn trailer_roundtrip() {
        let t = Trailer {
            k: 7,
            base: 9,
            data_bytes: 3_891_363_840,
            distribution: Some([88_529_771, 897_323_586, 6_126_261_227, 7_936_519_016]),
            checksum: Some(0x0123_4567_89ab_cdef),
            metric: Metric::Htm,
        };
        let bytes = t.to_bytes();
        assert_eq!(bytes.len() as u64, TRAILER_BYTES);
        assert_eq!(Trailer::parse(&bytes).unwrap(), Some(t));
        let bare = Trailer {
            distribution: None,
            checksum: None,
            ..t
        };
        assert_eq!(Trailer::parse(&bare.to_bytes()).unwrap(), Some(bare));
        // No magic: no trailer. Magic but garbage: an error.
        assert_eq!(Trailer::parse(&[0u8; 64]).unwrap(), None);
        assert!(Trailer::parse(format!("{MAGIC}k 7\n").as_bytes()).is_err());
        assert!(Trailer::parse(format!("{MAGIC}k x\nbase 9\nbytes 1\n").as_bytes()).is_err());
        // Another index layout is refused, unknown keys are not.
        let other = format!("{MAGIC}layout h48-parity\nk 7\nbase 9\nbytes 1\n");
        assert!(Trailer::parse(other.as_bytes()).is_err());
        let extra = format!("{MAGIC}k 7\nbase 9\nbytes 1\nmade-by gen 2\n");
        assert_eq!(
            Trailer::parse(extra.as_bytes()).unwrap().map(|t| t.k),
            Some(7)
        );
        // QTM tables: their own layout, which round-trips; HTM tables keep
        // the bytes they had.
        let qtm = Trailer {
            metric: Metric::Qtm,
            ..t
        };
        let text = qtm.to_bytes();
        assert!(String::from_utf8_lossy(&text).contains("\nlayout h48-qtm\n"));
        assert_eq!(Trailer::parse(&text).unwrap(), Some(qtm));
        assert!(String::from_utf8_lossy(&t.to_bytes()).contains("\nlayout h48\n"));
        let qtm_text = format!("{MAGIC}layout h48-qtm\nk 7\nbase 9\nbytes 1\n");
        assert_eq!(
            Trailer::parse(qtm_text.as_bytes())
                .unwrap()
                .map(|t| t.metric),
            Some(Metric::Qtm)
        );
    }

    #[test]
    #[cfg_attr(miri, ignore = "files")]
    fn trailer_write_keeps_data() {
        let path = crate::tables::scratch_path("trailer")
            .to_string_lossy()
            .into_owned();
        let data: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
        let t = Trailer {
            k: 0,
            base: 7,
            data_bytes: data.len() as u64,
            distribution: None,
            checksum: Some(checksum(&data, 3).unwrap()),
            metric: Metric::Htm,
        };
        // Too small for data plus trailer: refused, file untouched.
        std::fs::write(&path, &data).unwrap();
        assert!(unsafe { write_trailer(&path, &t) }.is_err());
        let overflowing = Trailer {
            data_bytes: u64::MAX,
            ..t
        };
        assert!(unsafe { write_trailer(&path, &overflowing) }.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), data);
        let mut with_room = data.clone();
        with_room.resize(data.len() + usize::try_from(TRAILER_BYTES).unwrap(), 0xAA);
        std::fs::write(&path, &with_room).unwrap();
        unsafe { write_trailer(&path, &t) }.unwrap();
        assert_eq!(read_trailer(&path).unwrap(), Some(t));
        assert_eq!(&std::fs::read(&path).unwrap()[..data.len()], &data[..]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn checksum_sees_every_byte() {
        let mut data = vec![0u8; 3 << 20 | 0x4D];
        let c0 = checksum(&data, 4).unwrap();
        assert_eq!(
            c0,
            checksum(&data, 1).unwrap(),
            "independent of the thread count"
        );
        let (a, b) = data.split_at(2 * CHECKSUM_CHUNK);
        let parts = [chunk_hashes(a, 2).unwrap(), chunk_hashes(b, 3).unwrap()].concat();
        assert_eq!(combine_hashes(data.len() as u64, &parts), c0, "from parts");
        for pos in [0, 5, 1 << 20, (3 << 20) + 76] {
            data[pos] ^= 1;
            assert_ne!(checksum(&data, 4).unwrap(), c0, "flip at {pos}");
            data[pos] ^= 1;
        }
        assert_ne!(checksum(&data[1..], 4).unwrap(), c0);
    }
}
