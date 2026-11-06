//! System V shared memory segments for tables (`LoadMode::Shm`, `--shm`,
//! as vcube's `--shm`): the first run copies a table file into a segment,
//! which stays in memory after the process exits (until reboot or `ipcrm`);
//! later runs attach it read-only and start at once.
//!
//! A segment holds the table file's bytes as a hugetlbfs file does: the data
//! at offset 0 (so the search sees the same alignment), and the file's
//! trailer in the segment's last 4 KiB, with `segment-*` lines added. The
//! trailer is written last, after the data is in and its checksum checked:
//! a segment without one is still loading, or its load was interrupted. The
//! key is legacy ASCII "co" (retained for compatibility), then k, then base (`key`: 0x636f0b0a for h11
//! base 10), so every run of a table finds the same segment; every attach
//! compares the segment's trailer with the table file's (k, base, size,
//! distribution, checksum), so a segment of another table under that key is
//! never used.
//!
//! A new segment gets 1 GiB pages, else 2 MiB, else 4 KiB pages (as
//! `ShmPages` allows). Huge pages come from the kernel's hugetlb pool and
//! need free pages there and the permission (`vm.hugetlb_shm_group`); a size
//! that cannot be had is skipped with a note that says how to get it.
//! Linux only: elsewhere `LoadMode::Shm` fails and `Auto` skips segments.
//!
//! Measured on h7 (3.62 GiB, the 48-core server): a later run attaches the
//! segment in 0.00 s on 1 GiB pages (0.6 s on 4 KiB pages, mapping 0.95
//! million pages; 0.00 s with `--prefault 0`), where copying the file from
//! a ~140 MB/s disk takes 28 s; making it from a file in memory takes 0.10 s
//! on 1 GiB pages and 1.9 s on 4 KiB pages, its checksum check included. The
//! search on a segment is as fast as on the same pages otherwise (1T, 6
//! pairs each: equal to a hugetlbfs map on 1 GiB pages and to a `/dev/shm`
//! map on 4 KiB pages); 4 KiB pages are 27% slower at 1T and 32% at 48T
//! than 1 GiB pages.

use super::file::{self, LoadOptions, TableError};
use super::Memory;

/// The first two bytes of every key: legacy ASCII "co" (retained for compatibility).
pub const KEY_PREFIX: u32 = 0x636f_0000;

/// The segment key of table hk with `base`: `KEY_PREFIX`, then k and base,
/// a byte each (`ipcs -m` shows 0x636f0709 for h7 base 9).
#[must_use]
pub fn key(k: u32, base: u8) -> u32 {
    KEY_PREFIX | (k & 0xFF) << 8 | u32::from(base)
}

/// A key as `ipcs` shows it (`0x636f0709`) or in decimal; not 0
/// (`IPC_PRIVATE`, which makes a new segment every time).
#[must_use]
pub fn parse_key(text: &str) -> Option<u32> {
    let key = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => text.parse().ok()?,
    };
    (key != 0).then_some(key)
}

/// The page sizes a new segment may get, tried largest first: 1 GiB and 2
/// MiB pages from the kernel's hugetlb pool, and normal (4 KiB) pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShmPages(u8);

/// Normal pages (the `4k` of `ShmPages`).
pub const NORMAL_PAGE: u64 = 1 << 12;

impl ShmPages {
    /// All three sizes.
    pub const ALL: ShmPages = ShmPages(0b111);
    /// Each size, largest first, with its name in `parse`.
    const SIZES: [(u64, &'static str); 3] = [(1 << 30, "1g"), (1 << 21, "2m"), (NORMAL_PAGE, "4k")];

    /// The allowed page sizes in bytes, largest first.
    pub fn sizes(self) -> impl Iterator<Item = u64> {
        (0..Self::SIZES.len())
            .filter(move |&i| self.0 & (1 << i) != 0)
            .map(|i| Self::SIZES[i].0)
    }

    /// A comma-separated list of `1g`, `2m` and `4k` (any order, at least
    /// one), or `all`.
    #[must_use]
    pub fn parse(text: &str) -> Option<ShmPages> {
        if text.eq_ignore_ascii_case("all") {
            return Some(ShmPages::ALL);
        }
        let mut bits = 0;
        for word in text.split(',') {
            let word = word.trim();
            let i = Self::SIZES
                .iter()
                .position(|(_, name)| word.eq_ignore_ascii_case(name))?;
            bits |= 1 << i;
        }
        (bits != 0).then_some(ShmPages(bits))
    }
}

impl Default for ShmPages {
    fn default() -> ShmPages {
        ShmPages::ALL
    }
}

/// A page size for messages: "1 GiB", "2 MiB", "4 KiB".
#[must_use]
pub fn page_name(page: u64) -> String {
    match page {
        p if p >= 1 << 30 => format!("{} GiB", p >> 30),
        p if p >= 1 << 20 => format!("{} MiB", p >> 20),
        p => format!("{} KiB", p >> 10),
    }
}

/// Bytes of a segment for `data_bytes` of table data on pages of `page`
/// bytes: the data and a trailer, in whole pages (so that the segment's
/// length is that of its mapping).
#[must_use]
pub fn segment_bytes(data_bytes: u64, page: u64) -> u64 {
    (data_bytes + file::TRAILER_BYTES).next_multiple_of(page)
}

/// A table's segment, as `open` found or made it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub key: u32,
    /// Its id (`ipcs -m`'s shmid).
    pub id: i32,
    /// Its size: the data and the trailer, in whole pages.
    pub bytes: u64,
    /// Its page size: 1 GiB, 2 MiB or 4 KiB.
    pub page_bytes: u64,
    /// Made (and filled from the table file) by this process, not found.
    pub created: bool,
    /// The table file it was filled from.
    pub source: String,
    /// The process that made it.
    pub creator: i32,
}

/// What `remove` removed.
#[derive(Clone, Debug)]
pub struct Removed {
    pub id: i32,
    pub bytes: u64,
    /// The table it held, for messages.
    pub held: String,
    /// Processes that still have it attached (it is freed when they detach).
    pub attached: u64,
}

/// Open the table file `path` from its segment: attach it if it holds this
/// table; else, with `create` (`LoadMode::Shm`), make it from the file.
/// Without `create` (`Auto`) every problem is a note and `None`: the caller
/// loads the file as before.
///
/// # Errors
/// With `create`: if the table has no checksum, if the segment holds
/// another table, is being loaded by another process (`ShmLoading`, try
/// again) or cannot be made (memory, kernel limits), or the copy's checksum
/// is wrong.
pub(crate) fn open(
    path: &str,
    options: &LoadOptions,
    create: bool,
    notes: &mut Vec<String>,
) -> Result<Option<(Memory, Segment)>, TableError> {
    match linux::open(path, options, create, notes) {
        Err(error) if !create => {
            notes.push(format!("shared memory not used: {error}"));
            Ok(None)
        }
        result => result,
    }
}

/// Whether the table file `path` has its segment loaded (under its key,
/// holding this table): `LoadMode::Auto` and `Shm` attach it, so a load
/// needs no memory.
#[must_use]
pub fn loaded(path: &str, options: &LoadOptions) -> bool {
    linux::loaded(path, options)
}

/// The page size a new segment of the table file `path` would get, as far
/// as can be told before making it (the kernel's segment limits, the
/// permission and pool pages for huge pages; not the memory for 4 KiB
/// pages, which the caller checks: `cli::preflight`), or `None` if none.
#[must_use]
pub fn new_segment_page(path: &str, options: &LoadOptions) -> Option<u64> {
    linux::new_segment_page(path, options)
}

/// Remove segment `key` (`IPC_RMID`; its memory is freed once no process has
/// it attached). Returns what it held, or `None` if there is none.
///
/// # Errors
/// If it cannot be removed (it belongs to another user), or off Linux.
pub fn remove(key: u32) -> Result<Option<Removed>, TableError> {
    linux::remove(key)
}

#[cfg(not(all(target_os = "linux", not(miri))))]
mod linux {
    use super::{LoadOptions, Memory, Removed, Segment, TableError};

    pub(super) fn open(
        _: &str,
        _: &LoadOptions,
        create: bool,
        _: &mut Vec<String>,
    ) -> Result<Option<(Memory, Segment)>, TableError> {
        if create {
            Err(TableError::ShmUnsupported)
        } else {
            Ok(None)
        }
    }

    pub(super) fn remove(_: u32) -> Result<Option<Removed>, TableError> {
        Err(TableError::ShmUnsupported)
    }

    pub(super) fn loaded(_: &str, _: &LoadOptions) -> bool {
        false
    }

    pub(super) fn new_segment_page(_: &str, _: &LoadOptions) -> Option<u64> {
        None
    }
}

#[cfg(all(target_os = "linux", not(miri)))]
mod linux {
    use super::{
        file, key, page_name, segment_bytes, LoadOptions, Memory, Removed, Segment, TableError,
        NORMAL_PAGE,
    };
    use crate::tables::file::Trailer;
    use std::io;
    use std::sync::atomic::{AtomicU8, Ordering};

    /// Trailer keys a segment adds to the table file's trailer.
    const PAGE_KEY: &str = "segment-page-bytes";
    const SOURCE_KEY: &str = "segment-source";

    /// A segment's trailer: the table file's, and the segment's own lines.
    #[derive(Debug, PartialEq, Eq)]
    pub(super) struct Header {
        pub(super) trailer: Trailer,
        pub(super) page_bytes: Option<u64>,
        pub(super) source: String,
    }

    /// The last 4 KiB of a segment: the table file's trailer, then the
    /// segment's page size and source file (keys `Trailer::parse` ignores).
    pub(super) fn header_bytes(trailer: &Trailer, page: u64, source: &str) -> Vec<u8> {
        let mut bytes = trailer.to_bytes();
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        // (One line of at most 2000 bytes: the trailer text is far shorter
        // than its 4 KiB.)
        let source = source.replace(char::is_control, "?");
        let source = &source[..source.floor_char_boundary(2000)];
        let extra = format!("{PAGE_KEY} {page}\n{SOURCE_KEY} {source}\n");
        bytes[end..end + extra.len()].copy_from_slice(extra.as_bytes());
        bytes
    }

    /// The `Header` in a segment's last 4 KiB, or `None` if there is no
    /// trailer there.
    pub(super) fn parse_header(bytes: &[u8]) -> Result<Option<Header>, TableError> {
        let Some(trailer) = Trailer::parse(bytes)? else {
            return Ok(None);
        };
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        let text = String::from_utf8_lossy(&bytes[..end]);
        let value = |key: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(key)?.strip_prefix(' '))
                .map(str::to_string)
        };
        Ok(Some(Header {
            trailer,
            page_bytes: value(PAGE_KEY).and_then(|v| v.parse().ok()),
            source: value(SOURCE_KEY).unwrap_or_else(|| "?".to_string()),
        }))
    }

    /// A table for messages: "h7 base 9, checksum a8f0d2657a36a775".
    fn describe(trailer: &Trailer) -> String {
        match trailer.checksum {
            Some(c) => format!("h{} base {}, checksum {c:016x}", trailer.k, trailer.base),
            None => format!("h{} base {}, no checksum", trailer.k, trailer.base),
        }
    }

    /// How to remove segment `key` of the table at `path`.
    fn remove_hint(path: &str, key: u32) -> String {
        format!("`solve --shm-remove {path}` or `ipcrm -M {key:#010x}`")
    }

    /// `shmget`'s page size field (linux/shm.h `SHM_HUGE_SHIFT`): log2 of the
    /// size, from bit 26.
    const SHM_HUGE_SHIFT: u32 = 26;

    fn call_error(key: u32, call: &'static str) -> impl FnOnce(io::Error) -> TableError {
        move |source| TableError::ShmCall { key, call, source }
    }

    fn is(error: &io::Error, code: i32) -> bool {
        error.raw_os_error() == Some(code)
    }

    pub(super) fn shmget(key: u32, bytes: u64, flags: i32) -> io::Result<i32> {
        let bytes = usize::try_from(bytes).map_err(io::Error::other)?;
        // SAFETY: no pointers.
        let id = unsafe { libc::shmget(key.cast_signed(), bytes, flags) };
        if id < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(id)
        }
    }

    pub(super) fn stat(id: i32) -> io::Result<libc::shmid_ds> {
        // SAFETY: a valid out-pointer.
        unsafe {
            let mut ds: libc::shmid_ds = std::mem::zeroed();
            if libc::shmctl(id, libc::IPC_STAT, &raw mut ds) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(ds)
        }
    }

    /// `IPC_RMID`: the segment goes once no process has it attached.
    fn mark_removed(id: i32) -> io::Result<()> {
        // SAFETY: no buffer is read for IPC_RMID.
        if unsafe { libc::shmctl(id, libc::IPC_RMID, std::ptr::null_mut()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Whether process `pid` exists.
    fn alive(pid: i32) -> bool {
        // SAFETY: signal 0 only checks.
        let sent = unsafe { libc::kill(pid, 0) } == 0;
        sent || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }

    /// An attachment of a segment, detached on drop (unless made `Memory`).
    struct Attachment {
        ptr: *mut u8,
        ds: libc::shmid_ds,
    }

    impl Attachment {
        /// Attach segment `id`, read-only or writable (`ds` is its state
        /// before: `shm_nattch` does not count this attachment).
        fn new(id: i32, read_only: bool) -> io::Result<Attachment> {
            let ds = stat(id)?;
            let flags = if read_only { libc::SHM_RDONLY } else { 0 };
            // SAFETY: the kernel picks the address.
            let ptr = unsafe { libc::shmat(id, std::ptr::null(), flags) };
            if ptr.addr() == usize::MAX {
                return Err(io::Error::last_os_error());
            }
            Ok(Attachment {
                ptr: ptr.cast(),
                ds,
            })
        }

        fn bytes(&self) -> u64 {
            self.ds.shm_segsz as u64
        }

        /// Its last 4 KiB (the trailer, once loaded).
        fn last_page(&self) -> Option<Vec<u8>> {
            let page = usize::try_from(file::TRAILER_BYTES).ok()?;
            let offset = usize::try_from(self.bytes()).ok()?.checked_sub(page)?;
            let mut bytes = vec![0u8; page];
            // SAFETY: the last 4 KiB of the attached segment. Its first byte
            // is written last, with Release (`write_trailer`): a reader that
            // sees it sees the rest, and the data.
            unsafe {
                let start = self.ptr.add(offset);
                bytes[0] = AtomicU8::from_ptr(start).load(Ordering::Acquire);
                std::ptr::copy_nonoverlapping(start.add(1), bytes.as_mut_ptr().add(1), page - 1);
            }
            Some(bytes)
        }

        /// The attachment as `Memory`, unmapped on drop (`munmap` detaches a
        /// segment as `shmdt` does; the length is the mapping's, as segments
        /// are whole pages).
        fn into_memory(self) -> io::Result<Memory> {
            let len = usize::try_from(self.bytes()).map_err(io::Error::other)?;
            let memory = Memory { ptr: self.ptr, len };
            std::mem::forget(self);
            Ok(memory)
        }
    }

    impl Drop for Attachment {
        fn drop(&mut self) {
            // SAFETY: our attachment, detached once.
            unsafe {
                libc::shmdt(self.ptr.cast());
            }
        }
    }

    /// Write `trailer` into the last 4 KiB of a writable segment, its first
    /// byte last (with Release: a reader that sees it, `Attachment::last_page`,
    /// sees the rest and the data).
    fn write_trailer(segment: &Memory, trailer: &[u8]) {
        let offset = segment.len() - trailer.len();
        // SAFETY: the last 4 KiB of our writable attachment.
        unsafe {
            let start = segment.ptr().add(offset);
            std::ptr::copy_nonoverlapping(trailer.as_ptr().add(1), start.add(1), trailer.len() - 1);
            AtomicU8::from_ptr(start).store(trailer[0], Ordering::Release);
        }
    }

    /// What a segment holds, for the table of `trailer`.
    enum Found {
        /// This table, loaded.
        Same(Header),
        /// No trailer yet: being loaded by its maker, which is running.
        Loading(i32),
        /// No trailer, and its maker is gone: the load was interrupted.
        Interrupted(i32),
        /// Another table, or no table this code reads.
        Other(String),
    }

    fn examine(attachment: &Attachment, trailer: &Trailer) -> Found {
        let bytes = attachment.bytes();
        let creator = attachment.ds.shm_cpid;
        match attachment.last_page().map(|page| parse_header(&page)) {
            None => Found::Other(format!("a segment of {bytes} bytes, too small for a table")),
            Some(Err(error)) => {
                Found::Other(format!("a segment with an unreadable trailer ({error})"))
            }
            Some(Ok(None)) if alive(creator) => Found::Loading(creator),
            Some(Ok(None)) => Found::Interrupted(creator),
            Some(Ok(Some(header))) => {
                let whole = |page: u64| {
                    page > 0
                        && bytes.is_multiple_of(page)
                        && bytes >= header.trailer.data_bytes + file::TRAILER_BYTES
                };
                if header.trailer == *trailer && header.page_bytes.is_some_and(whole) {
                    Found::Same(header)
                } else {
                    Found::Other(format!(
                        "{} (loaded from {})",
                        describe(&header.trailer),
                        header.source
                    ))
                }
            }
        }
    }

    pub(super) fn open(
        path: &str,
        options: &LoadOptions,
        create: bool,
        notes: &mut Vec<String>,
    ) -> Result<Option<(Memory, Segment)>, TableError> {
        let Some(trailer) = file::read_trailer(path)?.filter(|t| t.checksum.is_some()) else {
            return if create {
                Err(TableError::ShmNoChecksum {
                    path: path.to_string(),
                })
            } else {
                Ok(None)
            };
        };
        let key = options
            .shm_key
            .unwrap_or_else(|| key(trailer.k, trailer.base));
        // (A few rounds: other processes may make or remove it meanwhile.)
        for _ in 0..4 {
            let id = match shmget(key, 0, 0) {
                Ok(id) => id,
                Err(e) if is(&e, libc::ENOENT) && create => {
                    match make(key, path, &trailer, options, notes)? {
                        Some(made) => return Ok(Some(made)),
                        None => continue,
                    }
                }
                Err(e) if is(&e, libc::ENOENT) => return Ok(None),
                Err(e) => return Err(call_error(key, "shmget")(e)),
            };
            let attachment = match Attachment::new(id, true) {
                Ok(attachment) => attachment,
                // (Removed meanwhile.)
                Err(e) if is(&e, libc::EIDRM) || is(&e, libc::EINVAL) => continue,
                Err(e) => return Err(call_error(key, "shmat")(e)),
            };
            match examine(&attachment, &trailer) {
                Found::Same(header) => {
                    let segment = Segment {
                        key,
                        id,
                        bytes: attachment.bytes(),
                        page_bytes: header.page_bytes.unwrap_or(NORMAL_PAGE),
                        created: false,
                        source: header.source,
                        creator: attachment.ds.shm_cpid,
                    };
                    let memory = attachment.into_memory().map_err(call_error(key, "shmat"))?;
                    if options.prefault {
                        memory.prefault().map_err(call_error(key, "prefault"))?;
                    }
                    return Ok(Some((memory, segment)));
                }
                Found::Loading(pid) => return Err(TableError::ShmLoading { key, pid }),
                // (Nobody else has it attached, and its maker is gone: no
                // process can be filling it.)
                Found::Interrupted(pid) if create && attachment.ds.shm_nattch == 0 => {
                    drop(attachment);
                    mark_removed(id).map_err(call_error(key, "shmctl(IPC_RMID)"))?;
                    notes.push(format!(
                        "removed segment {key:#010x}: its load by process {pid} was interrupted"
                    ));
                }
                Found::Interrupted(pid) => {
                    return Err(TableError::Shm {
                        key,
                        message: format!(
                            "has no trailer: its load by process {pid} was interrupted; \
                             remove it ({})",
                            remove_hint(path, key)
                        ),
                    });
                }
                Found::Other(held) => {
                    return Err(TableError::ShmOtherTable {
                        key,
                        path: path.to_string(),
                        held,
                        wanted: describe(&trailer),
                        hint: remove_hint(path, key),
                    });
                }
            }
        }
        Err(TableError::Shm {
            key,
            message: "made and removed by other processes again and again; try once more".into(),
        })
    }

    pub(super) fn loaded(path: &str, options: &LoadOptions) -> bool {
        let Ok(Some(trailer)) = file::read_trailer(path) else {
            return false;
        };
        let key = options
            .shm_key
            .unwrap_or_else(|| key(trailer.k, trailer.base));
        trailer.checksum.is_some()
            && shmget(key, 0, 0)
                .and_then(|id| Attachment::new(id, true))
                .is_ok_and(|attachment| matches!(examine(&attachment, &trailer), Found::Same(_)))
    }

    pub(super) fn new_segment_page(path: &str, options: &LoadOptions) -> Option<u64> {
        let trailer = file::read_trailer(path).ok()??;
        options
            .shm_pages
            .sizes()
            .find(|&page| preflight(page, segment_bytes(trailer.data_bytes, page)).is_ok())
    }

    /// Removes a segment on drop, unless disarmed: a failed load leaves none.
    struct RemoveOnDrop(Option<i32>);

    impl RemoveOnDrop {
        fn disarm(mut self) {
            self.0 = None;
        }
    }

    impl Drop for RemoveOnDrop {
        fn drop(&mut self) {
            if let Some(id) = self.0 {
                let _ = mark_removed(id);
            }
        }
    }

    /// Make segment `key` from the table file: on the first page size that
    /// can be had, the data copied in and its checksum checked, then the
    /// trailer. Returns it attached read-only, or `None` if another process
    /// made it first.
    fn make(
        key: u32,
        path: &str,
        trailer: &Trailer,
        options: &LoadOptions,
        notes: &mut Vec<String>,
    ) -> Result<Option<(Memory, Segment)>, TableError> {
        let mut made = None;
        for page in options.shm_pages.sizes() {
            let bytes = segment_bytes(trailer.data_bytes, page);
            let huge = huge_page(page);
            // Preflight: fail (or skip a page size) now, not deep into the copy.
            if let Err(why) = preflight(page, bytes) {
                if huge {
                    notes.push(why);
                    continue;
                }
                return Err(TableError::Shm { key, message: why });
            }
            let page_flags = if huge {
                libc::SHM_HUGETLB | (page.trailing_zeros() << SHM_HUGE_SHIFT).cast_signed()
            } else {
                0
            };
            match shmget(
                key,
                bytes,
                libc::IPC_CREAT | libc::IPC_EXCL | 0o600 | page_flags,
            ) {
                Ok(id) => {
                    made = Some((id, page, bytes));
                    break;
                }
                Err(e) if is(&e, libc::EEXIST) => return Ok(None),
                Err(e) if huge => notes.push(huge_page_failure(page, &e)),
                Err(e) => return Err(call_error(key, "shmget")(e)),
            }
        }
        let Some((id, page, bytes)) = made else {
            return Err(TableError::Shm {
                key,
                message: format!("no allowed page size can be had: {}", notes.join("; ")),
            });
        };
        let guard = RemoveOnDrop(Some(id));
        let writable = Attachment::new(id, false)
            .map_err(call_error(key, "shmat"))?
            .into_memory()
            .map_err(call_error(key, "shmat"))?;
        if !huge_page(page) {
            // Transparent huge pages, where shared memory gets them
            // (/sys/kernel/mm/transparent_hugepage/shmem_enabled: advise).
            // SAFETY: advice on our own attachment.
            unsafe {
                libc::madvise(writable.ptr().cast(), writable.len(), libc::MADV_HUGEPAGE);
            }
        }
        let table_file = std::fs::File::open(path).map_err(file::io_error(path))?;
        let data_len =
            usize::try_from(trailer.data_bytes).map_err(|_| TableError::TooLarge(trailer.k))?;
        super::super::read_parallel(&table_file, &writable, data_len)
            .map_err(file::io_error(path))?;
        // The copy is the file's table, and the file is not damaged.
        let data = (writable.bytes()?.get(..data_len)).ok_or(TableError::TooLarge(trailer.k))?;
        let got = file::checksum(data, crate::thread_count())?;
        let want = trailer.checksum.unwrap_or_default();
        if got != want {
            return Err(TableError::Checksum { got, want });
        }
        write_trailer(&writable, &header_bytes(trailer, page, path));
        // Read-only from now on (`ipcs -m` shows perms 400 once loaded, 600
        // while loading).
        let mut ds = stat(id).map_err(call_error(key, "shmctl(IPC_STAT)"))?;
        ds.shm_perm.mode = 0o400;
        // SAFETY: a valid buffer.
        if unsafe { libc::shmctl(id, libc::IPC_SET, &raw mut ds) } != 0 {
            return Err(call_error(key, "shmctl(IPC_SET)")(
                io::Error::last_os_error(),
            ));
        }
        drop(writable);
        let read_only = Attachment::new(id, true).map_err(call_error(key, "shmat"))?;
        guard.disarm();
        let segment = Segment {
            key,
            id,
            bytes,
            page_bytes: page,
            created: true,
            source: path.to_string(),
            creator: std::process::id().cast_signed(),
        };
        let memory = read_only.into_memory().map_err(call_error(key, "shmat"))?;
        if options.prefault {
            memory.prefault().map_err(call_error(key, "prefault"))?;
        }
        Ok(Some((memory, segment)))
    }

    fn huge_page(page: u64) -> bool {
        page != NORMAL_PAGE
    }

    /// A number in a /proc or /sys file.
    fn read_number(path: &str) -> Option<u64> {
        std::fs::read_to_string(path).ok()?.trim().parse().ok()
    }

    /// Room for a segment of `bytes` on pages of `page` bytes: the kernel's
    /// segment limits, then for huge pages the permission and free pages in
    /// the hugetlb pool. (Free memory for 4 KiB pages is the caller's check:
    /// `cli::preflight`.)
    fn preflight(page: u64, bytes: u64) -> Result<(), String> {
        kernel_limits(bytes)?;
        if !huge_page(page) {
            return Ok(());
        }
        // The permission first, as the kernel checks it first.
        if let Some(permission) = no_permission() {
            return Err(format!("{} pages: {permission}", page_name(page)));
        }
        pool_room(page, bytes)
    }

    /// `kernel.shmmax` (bytes per segment), `kernel.shmall` (pages in all
    /// segments) and `kernel.shmmni` (segments).
    fn kernel_limits(bytes: u64) -> Result<(), String> {
        if let Some(max) = read_number("/proc/sys/kernel/shmmax").filter(|&max| bytes > max) {
            return Err(format!(
                "kernel.shmmax is {max} bytes, below the segment's {bytes}: raise it with \
                 `sudo sysctl kernel.shmmax={bytes}`"
            ));
        }
        // SAFETY: no pointers.
        let system_page =
            u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).unwrap_or(4096);
        let (count, used) = segments_in_use(system_page);
        let pages = bytes.div_ceil(system_page);
        if let Some(all) = read_number("/proc/sys/kernel/shmall").filter(|&all| used + pages > all)
        {
            return Err(format!(
                "kernel.shmall is {all} pages of {} KiB, other segments use {used} and this one \
                 needs {pages}: raise it with `sudo sysctl kernel.shmall={}`",
                system_page >> 10,
                used + pages
            ));
        }
        if let Some(most) = read_number("/proc/sys/kernel/shmmni").filter(|&most| count >= most) {
            return Err(format!(
                "kernel.shmmni: all {most} segments are in use (`ipcs -m`)"
            ));
        }
        Ok(())
    }

    /// Segments and the system pages they take, from /proc/sysvipc/shm.
    fn segments_in_use(system_page: u64) -> (u64, u64) {
        let text = std::fs::read_to_string("/proc/sysvipc/shm").unwrap_or_default();
        let sizes = text
            .lines()
            .skip(1)
            .filter_map(|line| line.split_whitespace().nth(3)?.parse::<u64>().ok());
        sizes.fold((0, 0), |(count, pages), size| {
            (count + 1, pages + size.div_ceil(system_page))
        })
    }

    /// Free pages of `page` bytes in the hugetlb pool (free less reserved,
    /// plus what overcommit allows) for a segment of `bytes`.
    fn pool_room(page: u64, bytes: u64) -> Result<(), String> {
        let pool = format!("/sys/kernel/mm/hugepages/hugepages-{}kB", page >> 10);
        let read = |what: &str| read_number(&format!("{pool}/{what}"));
        let Some(total) = read("nr_hugepages") else {
            return Err(format!(
                "{} pages: this kernel has none ({pool} is missing)",
                page_name(page)
            ));
        };
        let unused = |have: Option<u64>, taken: Option<u64>| {
            have.unwrap_or(0).saturating_sub(taken.unwrap_or(0))
        };
        let free = unused(read("free_hugepages"), read("resv_hugepages"))
            + unused(read("nr_overcommit_hugepages"), read("surplus_hugepages"));
        pool_verdict(page, bytes, total, free)
    }

    /// Whether `free` pages of the pool of `total` pages of `page` bytes
    /// hold a segment of `bytes`; if not, how to add them.
    pub(super) fn pool_verdict(page: u64, bytes: u64, total: u64, free: u64) -> Result<(), String> {
        let needed = bytes / page;
        if needed <= free {
            return Ok(());
        }
        Err(format!(
            "{} pages: {needed} needed, {free} free in the pool ({total} in all); to add them: \
             `echo {} | sudo tee /sys/kernel/mm/hugepages/hugepages-{}kB/nr_hugepages`",
            page_name(page),
            total + needed - free,
            page >> 10
        ))
    }

    /// Why `shmget` of `bytes` on huge pages of `page` bytes failed, and how
    /// to fix it.
    pub(super) fn huge_page_failure(page: u64, error: &io::Error) -> String {
        let name = page_name(page);
        match error.raw_os_error() {
            Some(libc::EPERM) => {
                let group = read_number("/proc/sys/vm/hugetlb_shm_group").unwrap_or(0);
                // SAFETY: no pointers.
                let gid = unsafe { libc::getegid() };
                format!("{name} pages: EPERM: {}", permission_message(group, gid))
            }
            Some(libc::ENOMEM) => format!(
                "{name} pages: the pool has too few free pages (ENOMEM; see \
                 /sys/kernel/mm/hugepages/hugepages-{}kB)",
                page >> 10
            ),
            _ => format!("{name} pages: shmget: {error}"),
        }
    }

    /// Why this process may not make huge-page segments, if it may not
    /// (the kernel allows them to the group `vm.hugetlb_shm_group` and to
    /// `CAP_IPC_LOCK`).
    fn no_permission() -> Option<String> {
        // (`EGG_SOLVER_SHM_DENY_HUGE`, for tests, undocumented: as if it were
        // missing, without asking the kernel.)
        let simulated = crate::env::var_os("EGG_SOLVER_SHM_DENY_HUGE").is_some();
        let group = read_number("/proc/sys/vm/hugetlb_shm_group");
        if simulated {
            // SAFETY: no pointers.
            let message = permission_message(group.unwrap_or(0), unsafe { libc::getegid() });
            return Some(format!("{message} (simulated: EGG_SOLVER_SHM_DENY_HUGE)"));
        }
        let group = group?;
        // SAFETY: no pointers; then a buffer of the size asked for.
        let (gid, groups) = unsafe {
            let n = usize::try_from(libc::getgroups(0, std::ptr::null_mut())).unwrap_or(0);
            let mut groups = vec![0; n];
            let got = libc::getgroups(i32::try_from(n).unwrap_or(0), groups.as_mut_ptr());
            groups.truncate(usize::try_from(got).unwrap_or(0));
            (libc::getegid(), groups)
        };
        let in_group = std::iter::once(gid)
            .chain(groups)
            .any(|g| u64::from(g) == group);
        // `CAP_IPC_LOCK` is capability 14 of the effective set.
        let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        let ipc_lock = status
            .lines()
            .find_map(|l| l.strip_prefix("CapEff:"))
            .and_then(|caps| u64::from_str_radix(caps.trim(), 16).ok())
            .is_some_and(|caps| caps & (1 << 14) != 0);
        (!in_group && !ipc_lock).then(|| permission_message(group, gid))
    }

    /// How to get the permission for huge-page segments.
    pub(super) fn permission_message(group: u64, gid: u32) -> String {
        format!(
            "no permission: huge-page segments need a group of yours in \
             vm.hugetlb_shm_group (now {group}) or CAP_IPC_LOCK; to allow your group: \
             `sudo sysctl vm.hugetlb_shm_group={gid}`"
        )
    }

    pub(super) fn remove(key: u32) -> Result<Option<Removed>, TableError> {
        let id = match shmget(key, 0, 0) {
            Ok(id) => id,
            Err(e) if is(&e, libc::ENOENT) => return Ok(None),
            Err(e) => return Err(call_error(key, "shmget")(e)),
        };
        let ds = stat(id).map_err(call_error(key, "shmctl(IPC_STAT)"))?;
        let held = match Attachment::new(id, true) {
            Ok(attachment) => match attachment.last_page().map(|page| parse_header(&page)) {
                Some(Ok(Some(header))) => describe(&header.trailer),
                Some(Ok(None)) => "no table yet (loading, or its load was interrupted)".into(),
                _ => "no table".into(),
            },
            Err(e) => format!("unknown ({e})"),
        };
        mark_removed(id).map_err(call_error(key, "shmctl(IPC_RMID)"))?;
        #[allow(
            clippy::unnecessary_cast,
            reason = "shmatt_t is 32-bit on 32-bit targets"
        )]
        let attached = ds.shm_nattch as u64;
        Ok(Some(Removed {
            id,
            bytes: ds.shm_segsz as u64,
            held,
            attached,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{key, parse_key, segment_bytes, ShmPages, NORMAL_PAGE};

    /// Keys are "co", k and base; `ShmPages` lists are any order of 1g, 2m
    /// and 4k, tried largest first.
    #[test]
    fn keys_and_page_lists() {
        assert_eq!(key(11, 10), 0x636f_0b0a);
        assert_eq!(key(7, 9), 0x636f_0709);
        assert_eq!(parse_key("0x636f0709"), Some(0x636f_0709));
        assert_eq!(parse_key("1668220681"), Some(0x636f_0709));
        for bad in ["0", "0x0", "x", "0x100000000", "-1", ""] {
            assert_eq!(parse_key(bad), None, "{bad}");
        }
        let sizes = |text: &str| ShmPages::parse(text).map(|p| p.sizes().collect::<Vec<u64>>());
        let all = vec![1 << 30, 1 << 21, NORMAL_PAGE];
        assert_eq!(sizes("1g,2m,4k"), Some(all.clone()));
        assert_eq!(sizes("4K, 1G,2m"), Some(all.clone()));
        assert_eq!(sizes("all"), Some(all));
        assert_eq!(sizes("4k,2m"), Some(vec![1 << 21, NORMAL_PAGE]));
        for bad in ["", "1g,", "8k", "2m 4k"] {
            assert_eq!(sizes(bad), None, "{bad}");
        }
        // Data and trailer in whole pages.
        assert_eq!(segment_bytes(30_401_280, NORMAL_PAGE), 30_408_704);
        assert_eq!(segment_bytes(3_891_363_840, 1 << 30), 4 << 30);
        assert_eq!(segment_bytes((1 << 30) - 4096, 1 << 30), 1 << 30);
    }
}

/// Segments made, attached, refused and removed, on 4 KiB pages (and 2 MiB
/// ones where the pool has them; never 1 GiB pages: the pool of those is
/// left alone). Each test's segment has its own key and is removed at the
/// end, also when the test fails.
#[cfg(all(test, target_os = "linux", not(miri)))]
mod segment_tests {
    use super::linux::{
        header_bytes, huge_page_failure, parse_header, permission_message, pool_verdict, shmget,
        stat,
    };
    use super::{remove, segment_bytes, ShmPages, NORMAL_PAGE};
    use crate::tables::file::{self, LoadMode, LoadOptions, TableError, Trailer};
    use crate::tables::{h48_bytes, scratch_path, H48Table};

    // The interrupted-loader test forks. Its child inherits every live SysV
    // mapping in this process, which would perturb another test's nattch checks.
    static SEGMENT_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn segment_test_guard() -> std::sync::MutexGuard<'static, ()> {
        SEGMENT_TESTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Test `n`'s key in this process (0x40000000 and up, below "co"
    /// keys), and its segment removed on drop.
    struct TestKey(u32);

    impl TestKey {
        fn new(n: u32) -> TestKey {
            assert!(n < 64);
            let key = 0x4000_0000 | std::process::id() << 6 | n;
            // (A segment left by a crashed earlier process of this id.)
            let _ = remove(key);
            TestKey(key)
        }

        fn options(&self, mode: LoadMode, pages: &str) -> LoadOptions {
            LoadOptions {
                mode,
                shm_pages: ShmPages::parse(pages).unwrap(),
                shm_key: Some(self.0),
                ..LoadOptions::default()
            }
        }
    }

    impl Drop for TestKey {
        fn drop(&mut self) {
            let _ = remove(self.0);
        }
    }

    /// An h0 table file (made-up data: segments do not look at it, only at
    /// its trailer and checksum), removed on drop.
    struct TableFile {
        path: String,
        data: Vec<u8>,
        trailer: Trailer,
    }

    impl TableFile {
        fn new(name: &str, seed: u8) -> TableFile {
            let len = usize::try_from(h48_bytes(0)).unwrap();
            let data: Vec<u8> = (0..len)
                .map(|i| u8::try_from(i % 251).unwrap() ^ seed)
                .collect();
            let trailer = Trailer {
                k: 0,
                base: 8,
                data_bytes: len as u64,
                distribution: None,
                checksum: Some(file::checksum(&data, 4).unwrap()),
            };
            let path = scratch_path(name).to_string_lossy().into_owned();
            std::fs::write(&path, [&data[..], &trailer.to_bytes()].concat()).unwrap();
            TableFile {
                path,
                data,
                trailer,
            }
        }

        fn open(
            &self,
            options: &LoadOptions,
        ) -> Result<(H48Table, crate::tables::LoadReport), TableError> {
            H48Table::open_reporting(&self.path, options)
        }
    }

    impl Drop for TableFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn attached(id: i32) -> u64 {
        #[allow(
            clippy::unnecessary_cast,
            reason = "shmatt_t is 32-bit on 32-bit targets"
        )]
        let attached = stat(id).unwrap().shm_nattch as u64;
        attached
    }

    /// `Shm` makes the segment once (read-only, the file's bytes), later
    /// opens attach it (`Auto` too); every detach is counted; `remove`
    /// removes it, and `Auto` then loads the file.
    #[test]
    fn segment_is_made_once_then_attached() {
        let _serial = segment_test_guard();
        let key = TestKey::new(0);
        let table = TableFile::new("shm-made", 0);
        let (first, report) = table.open(&key.options(LoadMode::Shm, "4k")).unwrap();
        let segment = report.segment.unwrap();
        assert_eq!(report.mode, LoadMode::Shm);
        assert!(segment.created);
        assert_eq!(segment.key, key.0);
        assert_eq!(segment.page_bytes, NORMAL_PAGE);
        assert_eq!(segment.bytes, segment_bytes(h48_bytes(0), NORMAL_PAGE));
        assert_eq!(segment.source, table.path);
        assert_eq!((first.meta.k, first.meta.base), (0, 8));
        assert_eq!(
            &first.memory.bytes().unwrap()[..table.data.len()],
            &table.data[..]
        );
        let end = &first.memory.bytes().unwrap()[first.memory.len() - 4096..];
        let header = parse_header(end).unwrap().unwrap();
        assert_eq!(header.trailer, table.trailer);
        assert_eq!(header.page_bytes, Some(NORMAL_PAGE));
        let ds = stat(segment.id).unwrap();
        assert_eq!(ds.shm_perm.mode & 0o777, 0o400, "read-only once loaded");
        assert_eq!(attached(segment.id), 1);
        for mode in [LoadMode::Shm, LoadMode::Auto] {
            let (again, report) = table.open(&key.options(mode, "4k")).unwrap();
            let found = report.segment.unwrap();
            assert!(!found.created);
            assert_eq!(
                (found.id, found.bytes, found.page_bytes),
                (segment.id, segment.bytes, NORMAL_PAGE)
            );
            assert_eq!(report.mode, LoadMode::Shm);
            assert_eq!(
                &again.memory.bytes().unwrap()[..table.data.len()],
                &table.data[..]
            );
            assert_eq!(attached(segment.id), 2);
        }
        // Explicit map and copy loads never use a segment.
        for mode in [LoadMode::Map, LoadMode::Copy] {
            let (_, report) = table.open(&key.options(mode, "4k")).unwrap();
            assert_eq!((report.mode, report.segment), (mode, None));
        }
        drop(first);
        assert_eq!(attached(segment.id), 0, "dropping the table detaches");
        let removed = remove(key.0).unwrap().unwrap();
        assert_eq!(removed.id, segment.id);
        assert!(
            removed.held.starts_with("h0 base 8, checksum"),
            "{}",
            removed.held
        );
        assert!(remove(key.0).unwrap().is_none());
        let (_, report) = table.open(&key.options(LoadMode::Auto, "4k")).unwrap();
        assert_eq!(report.segment, None);
        assert!(report.notes.is_empty(), "{:?}", report.notes);
    }

    /// A segment of another table under the key (another checksum) is
    /// refused by `Shm` and passed over by `Auto`, which loads the file.
    #[test]
    fn another_table_under_the_key_is_never_used() {
        let _serial = segment_test_guard();
        let key = TestKey::new(1);
        let first = TableFile::new("shm-first", 0);
        let second = TableFile::new("shm-second", 1);
        let (_attached, _) = first.open(&key.options(LoadMode::Shm, "4k")).unwrap();
        let error = second
            .open(&key.options(LoadMode::Shm, "4k"))
            .err()
            .unwrap();
        assert!(matches!(error, TableError::ShmOtherTable { .. }), "{error}");
        assert!(error.to_string().contains("--shm-remove"), "{error}");
        let (table, report) = second.open(&key.options(LoadMode::Auto, "4k")).unwrap();
        assert_eq!(report.segment, None);
        assert!(
            report.notes.iter().any(|n| n.contains("holds h0 base 8")),
            "{:?}",
            report.notes
        );
        assert_eq!(
            &table.memory.bytes().unwrap()[..second.data.len()],
            &second.data[..]
        );
    }

    /// A segment without a trailer is being loaded (its maker runs: `Shm`
    /// says so, to try again; `Auto` loads the file) or was interrupted (its
    /// maker is gone: `Shm` removes it and loads anew).
    #[test]
    fn unfinished_segments_are_found() {
        let _serial = segment_test_guard();
        let key = TestKey::new(2);
        let table = TableFile::new("shm-unfinished", 0);
        let bytes = segment_bytes(h48_bytes(0), NORMAL_PAGE);
        let flags = libc::IPC_CREAT | libc::IPC_EXCL | 0o600;
        let id = shmget(key.0, bytes, flags).unwrap();
        let error = table.open(&key.options(LoadMode::Shm, "4k")).err().unwrap();
        let me = std::process::id().cast_signed();
        assert!(
            matches!(error, TableError::ShmLoading { pid, .. } if pid == me),
            "{error}"
        );
        let (_, report) = table.open(&key.options(LoadMode::Auto, "4k")).unwrap();
        assert_eq!(report.segment, None);
        assert!(
            report.notes.iter().any(|n| n.contains("being loaded")),
            "{:?}",
            report.notes
        );
        assert!(remove(key.0).unwrap().is_some());
        assert!(stat(id).is_err());
        // Made by a child process that exits without loading it.
        // SAFETY: the child only makes the segment and exits (no allocation,
        // no locks).
        unsafe {
            let child = libc::fork();
            if child == 0 {
                let ok =
                    libc::shmget(key.0.cast_signed(), usize::try_from(bytes).unwrap(), flags) >= 0;
                libc::_exit(i32::from(!ok));
            }
            let mut status = 0;
            assert_eq!(libc::waitpid(child, &raw mut status, 0), child);
            assert_eq!(status, 0);
        }
        let (loaded, report) = table.open(&key.options(LoadMode::Shm, "4k")).unwrap();
        assert!(report.segment.unwrap().created);
        assert!(
            report.notes.iter().any(|n| n.contains("was interrupted")),
            "{:?}",
            report.notes
        );
        assert_eq!(
            &loaded.memory.bytes().unwrap()[..table.data.len()],
            &table.data[..]
        );
    }

    /// Without huge pages to be had, a segment falls back to the next size
    /// with a note that says why; a table without a checksum gets none.
    #[test]
    fn huge_pages_fall_back_with_a_note() {
        let _serial = segment_test_guard();
        let key = TestKey::new(3);
        let table = TableFile::new("shm-pages", 0);
        let (_, report) = table.open(&key.options(LoadMode::Shm, "2m,4k")).unwrap();
        let segment = report.segment.unwrap();
        if segment.page_bytes == NORMAL_PAGE {
            assert!(
                report.notes.iter().any(|n| n.starts_with("2 MiB pages: ")),
                "{:?}",
                report.notes
            );
        } else {
            assert_eq!(segment.page_bytes, 1 << 21);
            assert_eq!(segment.bytes % (1 << 21), 0);
        }
        drop(key);
        let key = TestKey::new(4);
        let no_checksum = TableFile::new("shm-no-checksum", 0);
        let trailer = Trailer {
            checksum: None,
            ..no_checksum.trailer
        };
        file::write_trailer(&no_checksum.path, &trailer).unwrap();
        let error = no_checksum
            .open(&key.options(LoadMode::Shm, "4k"))
            .err()
            .unwrap();
        assert!(matches!(error, TableError::ShmNoChecksum { .. }), "{error}");
        let (_, report) = no_checksum
            .open(&key.options(LoadMode::Auto, "4k"))
            .unwrap();
        assert_eq!(report.segment, None);
    }

    /// The preflight verdicts and messages: pool pages and the permission,
    /// with how to fix them.
    #[test]
    fn preflight_verdicts() {
        let _serial = segment_test_guard();
        // h7 on 1 GiB pages: 4 pages.
        let h7 = segment_bytes(h48_bytes(7), 1 << 30);
        assert!(pool_verdict(1 << 30, h7, 70, 4).is_ok());
        let short = pool_verdict(1 << 30, h7, 70, 1).unwrap_err();
        assert!(
            short.starts_with("1 GiB pages: 4 needed, 1 free in the pool (70 in all)"),
            "{short}"
        );
        assert!(
            short.contains(
                "echo 73 | sudo tee /sys/kernel/mm/hugepages/hugepages-1048576kB/nr_hugepages"
            ),
            "{short}"
        );
        let denied = huge_page_failure(1 << 30, &std::io::Error::from_raw_os_error(libc::EPERM));
        assert!(
            denied.starts_with("1 GiB pages: EPERM: no permission"),
            "{denied}"
        );
        assert!(
            denied.contains("vm.hugetlb_shm_group") && denied.contains("CAP_IPC_LOCK"),
            "{denied}"
        );
        let message = permission_message(0, 1000);
        assert!(
            message.ends_with("`sudo sysctl vm.hugetlb_shm_group=1000`"),
            "{message}"
        );
    }

    /// A segment's trailer is the file's, with its page size and source.
    #[test]
    fn segment_trailer_round_trip() {
        let _serial = segment_test_guard();
        let trailer = Trailer {
            k: 7,
            base: 9,
            data_bytes: h48_bytes(7),
            distribution: Some([1, 2, 3, 4]),
            checksum: Some(0xa8f0_d265_7a36_a775),
        };
        let bytes = header_bytes(&trailer, 1 << 21, "/mnt/huge/h7\nx");
        assert_eq!(bytes.len(), 4096);
        assert_eq!(Trailer::parse(&bytes).unwrap(), Some(trailer));
        let header = parse_header(&bytes).unwrap().unwrap();
        assert_eq!(
            (header.trailer, header.page_bytes),
            (trailer, Some(1 << 21))
        );
        assert_eq!(header.source, "/mnt/huge/h7?x");
        assert!(parse_header(&[0u8; 4096]).unwrap().is_none());
    }
}
