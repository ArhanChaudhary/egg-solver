//! Table files: where a table's `k` and `base` are stored, and how its data
//! is brought into memory.
//!
//! A table file holds the table data at offset 0 (so that it can be mapped
//! as is), followed by a 4 KiB text trailer in the last 4 KiB of the file:
//! a magic line, then `key value` lines (`layout`, `k`, `base`, `bytes`, and
//! the code `distribution` and `checksum` of the data when known), padded
//! with NUL bytes. Unknown keys are ignored, except that a `layout` other
//! than `h48` (the index layout of `coord`) is refused: a variant table (say
//! with a parity bit in place of an eo bit) must not be read as an h48 one. Files on hugetlbfs are whole 1 GiB pages; their trailer sits in the
//! slack after the data, so tables made before trailers existed can be
//! stamped in place (`gen stamp`). Tables without a trailer are described by
//! a `<path>.meta` sidecar (`k base`), or, for the legacy tables on
//! `/mnt/huge`, by `tables/<name>.hugemeta` in the build folder.

use std::io::{Read, Seek, SeekFrom, Write};

/// Size of the trailer at the end of a table file.
pub const TRAILER_BYTES: u64 = 4096;
const MAGIC: &str = "cubeopt h48 table v1\n";
/// The index layout of the tables this code reads and writes.
pub const LAYOUT: &str = "h48";

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
}

impl Trailer {
    /// The trailer as stored: text lines padded with NUL bytes.
    ///
    /// # Panics
    /// Never: the text is far shorter than the trailer.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        use std::fmt::Write as _;
        let trailer_len = usize::try_from(TRAILER_BYTES).unwrap();
        let mut text = String::with_capacity(trailer_len);
        text.push_str(MAGIC);
        let _ = writeln!(text, "layout {LAYOUT}");
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
        assert!(bytes.len() < trailer_len);
        bytes.resize(trailer_len, 0);
        bytes
    }

    /// Parse a trailer. Returns `Ok(None)` if `bytes` does not start with the
    /// magic line (a file without a trailer).
    ///
    /// # Errors
    /// `MalformedTrailer` if the magic line is there but the rest is
    /// malformed.
    pub fn parse(bytes: &[u8]) -> Result<Option<Trailer>, TableError> {
        let Some(rest) = bytes.strip_prefix(MAGIC.as_bytes()) else {
            return Ok(None);
        };
        let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
        let invalid = |what: &str| TableError::MalformedTrailer(what.to_string());
        let text = std::str::from_utf8(&rest[..end]).map_err(|_| invalid("not text"))?;
        let (mut k, mut base, mut data_bytes) = (None, None, None);
        let (mut distribution, mut checksum) = (None, None);
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
                "layout" if values != [LAYOUT] => {
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
///
/// # Panics
/// Never (a constant conversion).
pub fn read_trailer(path: &str) -> Result<Option<Trailer>, TableError> {
    let read = || -> std::io::Result<Option<Vec<u8>>> {
        let f = std::fs::File::open(path)?;
        let len = f.metadata()?.len();
        if len < TRAILER_BYTES {
            return Ok(None);
        }
        let mut buf = vec![0u8; usize::try_from(TRAILER_BYTES).unwrap()];
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
///
/// # Panics
/// If the file is larger than the address space.
pub fn write_trailer(path: &str, trailer: &Trailer) -> Result<(), TableError> {
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(io_error(path))?;
    let len = f.metadata().map_err(io_error(path))?.len();
    if len < trailer.data_bytes + TRAILER_BYTES {
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
        // SAFETY: a fresh shared mapping of the file's last page; only the
        // trailer bytes (past the data, checked above) are written.
        unsafe {
            use std::os::unix::io::AsRawFd;
            let p = libc::mmap(
                std::ptr::null_mut(),
                usize::try_from(page).unwrap(),
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                f.as_raw_fd(),
                libc::off_t::try_from(page_start).unwrap(),
            );
            if p == libc::MAP_FAILED {
                return Err(io_error(path)(std::io::Error::last_os_error()));
            }
            let dst = p
                .cast::<u8>()
                .add(usize::try_from(offset - page_start).unwrap());
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
            libc::munmap(p, usize::try_from(page).unwrap());
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
#[must_use]
pub fn chunk_hashes(data: &[u8], threads: usize) -> Vec<u64> {
    fn chunk_hash(chunk: &[u8]) -> u64 {
        // Four independent lanes (no single dependency chain), then folded.
        let mut lanes = [1u64, 2, 3, 4];
        let (words, rest) = chunk.as_chunks::<32>();
        for w in words {
            for (i, lane) in lanes.iter_mut().enumerate() {
                let x = u64::from_le_bytes(w[8 * i..8 * i + 8].try_into().unwrap());
                *lane = mix(*lane, x);
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
    crate::parallel::on_threads(threads, |t| {
        (t * per..((t + 1) * per).min(n))
            .map(|i| {
                let start = i * CHECKSUM_CHUNK;
                chunk_hash(&data[start..(start + CHECKSUM_CHUNK).min(data.len())])
            })
            .collect::<Vec<u64>>()
    })
    .concat()
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
#[must_use]
pub fn checksum(data: &[u8], threads: usize) -> u64 {
    combine_hashes(data.len() as u64, &chunk_hashes(data, threads))
}

/// How `H48Table::open` brings a table file into memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadMode {
    /// `Map` for files on hugetlbfs or `/dev/shm` (already in memory) and on
    /// other unix systems; `Copy` for other files on Linux (anonymous memory
    /// gets transparent huge pages reliably, the page cache may not) and
    /// where there are no file mappings (WASI, Windows).
    Auto,
    /// Map the file read-only (shared with other processes using it).
    Map,
    /// Copy the file into anonymous memory (transparent huge pages where
    /// available), then drop it from the page cache.
    Copy,
}

/// Options of `H48Table::open_with`.
#[derive(Clone, Copy, Debug)]
pub struct LoadOptions {
    pub mode: LoadMode,
    /// With `Map`: fault the whole table in at open (a sequential read), not
    /// on first use (random reads, slow until the table is cached).
    pub prefault: bool,
}

impl Default for LoadOptions {
    fn default() -> LoadOptions {
        LoadOptions {
            mode: LoadMode::Auto,
            prefault: true,
        }
    }
}

impl LoadOptions {
    /// Options from the environment: `CUBEOPT_LOAD` = `auto` | `map` |
    /// `copy`, and `CUBEOPT_PREFAULT` = `1` | `0`.
    ///
    /// # Panics
    /// If a variable is set to another value.
    #[must_use]
    pub fn from_env() -> LoadOptions {
        let mut o = LoadOptions::default();
        if let Ok(v) = std::env::var("CUBEOPT_LOAD") {
            o.mode = match v.as_str() {
                "auto" | "" => LoadMode::Auto,
                "map" => LoadMode::Map,
                "copy" => LoadMode::Copy,
                _ => panic!("CUBEOPT_LOAD={v:?}: expected auto, map or copy"),
            };
        }
        if let Ok(v) = std::env::var("CUBEOPT_PREFAULT") {
            o.prefault = match v.as_str() {
                "1" | "" => true,
                "0" => false,
                _ => panic!("CUBEOPT_PREFAULT={v:?}: expected 0 or 1"),
            };
        }
        o
    }

    /// The mode used for the file at `path` (resolves `Auto`).
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

/// Where `bench tune` saves a table's best lane counts.
fn tune_path(table: &str) -> String {
    format!("{table}.tune")
}

/// Save the best lane count per thread count (`threads lanes` lines).
///
/// # Errors
/// If the file cannot be written (e.g. the table's folder is read-only).
pub fn save_tuned_lanes(table: &str, best: &[(usize, usize)]) -> std::io::Result<String> {
    use std::fmt::Write as _;
    let mut text = String::new();
    for (threads, lanes) in best {
        let _ = writeln!(text, "{threads} {lanes}");
    }
    let path = tune_path(table);
    std::fs::write(&path, text)?;
    Ok(path)
}

/// Lanes per thread for a table of `k`, when not tuned (`bench tune`).
/// Smaller tables gain from more lanes (measured, solves/s, 24 -> 40 lanes):
/// h7 +10% at 1T (2.017 -> 2.217) and at 8 threads (15.43 -> 17.07), also
/// on 1 GiB pages (2.017 -> 2.283 at 1T: not a TLB effect); h8 equal at 1T,
/// +8.6% at 8 threads (26.73 -> 29.10); h6 equal (+4.5% at 1T, -1 to -5%
/// at 8 threads). h11 keeps `lanes::DEFAULT_LANES`, 24 (tuned at 1 and 48
/// threads; 40 is 3.4% slower at 8 threads, 197.9 -> 191.2). h9 and h10
/// (no RAM for them here) get 32, in between.
#[must_use]
pub fn untuned_lanes(k: u32) -> usize {
    match k {
        0..=8 => 40,
        9 | 10 => 32,
        _ => crate::lanes::DEFAULT_LANES,
    }
}

/// The default lanes per thread for `threads` threads on the table at
/// `path`: from `<path>.tune` (`bench tune`; the entry of the nearest
/// thread count), else `untuned_lanes(k)`.
pub fn default_lanes(path: &str, k: u32) -> impl Fn(usize) -> usize {
    let tuned: Vec<(usize, usize)> = std::fs::read_to_string(tune_path(path))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut w = l.split_whitespace().map(|x| x.parse::<usize>().ok());
            Some((w.next()??, w.next()??))
        })
        .filter(|&(threads, lanes)| threads > 0 && lanes > 0)
        .collect();
    move |threads| {
        tuned
            .iter()
            .min_by_key(|&&(t, _)| t.abs_diff(threads))
            .map_or(untuned_lanes(k), |&(_, lanes)| lanes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailer_roundtrip() {
        let t = Trailer {
            k: 7,
            base: 9,
            data_bytes: 3_891_363_840,
            distribution: Some([88_529_771, 897_323_586, 6_126_261_227, 7_936_519_016]),
            checksum: Some(0x0123_4567_89ab_cdef),
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
            checksum: Some(checksum(&data, 3)),
        };
        // Too small for data plus trailer: refused, file untouched.
        std::fs::write(&path, &data).unwrap();
        assert!(write_trailer(&path, &t).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), data);
        let mut with_room = data.clone();
        with_room.resize(data.len() + usize::try_from(TRAILER_BYTES).unwrap(), 0xAA);
        std::fs::write(&path, &with_room).unwrap();
        write_trailer(&path, &t).unwrap();
        assert_eq!(read_trailer(&path).unwrap(), Some(t));
        assert_eq!(&std::fs::read(&path).unwrap()[..data.len()], &data[..]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn checksum_sees_every_byte() {
        let mut data = vec![0u8; 3 << 20 | 0x4D];
        let c0 = checksum(&data, 4);
        assert_eq!(c0, checksum(&data, 1), "independent of the thread count");
        let (a, b) = data.split_at(2 * CHECKSUM_CHUNK);
        let parts = [chunk_hashes(a, 2), chunk_hashes(b, 3)].concat();
        assert_eq!(combine_hashes(data.len() as u64, &parts), c0, "from parts");
        for pos in [0, 5, 1 << 20, (3 << 20) + 76] {
            data[pos] ^= 1;
            assert_ne!(checksum(&data, 4), c0, "flip at {pos}");
            data[pos] ^= 1;
        }
        assert_ne!(checksum(&data[1..], 4), c0);
    }
}
