//! Checks before long work (user request: "Do this for the program itself
//! instead of failing when its deep into a run"): that the file system of a
//! table file to be written has room for it (on hugetlbfs: free huge pages),
//! and that there is memory for what a run allocates. A run that cannot
//! finish then fails at once, with exit status 2 and the numbers. `check` is
//! the decision; the rest measures (`statvfs` on unix; hugetlbfs,
//! `MemAvailable` and cgroup v2 memory limits on Linux; nothing is checked
//! elsewhere).

use super::BadInput;

/// Bytes in GiB, for messages.
#[expect(clippy::cast_precision_loss, reason = "a report")]
fn gib(bytes: u64) -> f64 {
    bytes as f64 / f64::from(1u32 << 30)
}

/// The margin on top of a file: 1% and 64 MiB (file system metadata, and
/// the trailer's room after rounding).
#[must_use]
pub fn file_margin(bytes: u64) -> u64 {
    bytes / 100 + (64 << 20)
}

/// The margin on top of a run's memory: 256 MiB (thread stacks, buffers).
pub const MEMORY_MARGIN: u64 = 256 << 20;

/// What a generator needs besides its tables: the cocsep table, the
/// depth-first prefix levels and hash set (~200 MB), buffers.
pub const GENERATOR_OVERHEAD: u64 = 300 << 20;

/// The numbers of a failed `check`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Short {
    pub needed: u64,
    pub margin: u64,
    pub available: u64,
}

/// The decision of every check: whether `needed` bytes and `margin` more fit
/// in `available`.
///
/// # Errors
/// The numbers, if they do not.
pub fn check(needed: u64, margin: u64, available: u64) -> Result<(), Short> {
    if needed
        .checked_add(margin)
        .is_some_and(|want| want <= available)
    {
        Ok(())
    } else {
        Err(Short {
            needed,
            margin,
            available,
        })
    }
}

/// `CUBEOPT_PREFLIGHT_LIMIT` (bytes; for tests, undocumented): every
/// measured amount is at most this, to try the failures without a full disk.
fn limit(available: u64) -> u64 {
    std::env::var("CUBEOPT_PREFLIGHT_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .map_or(available, |limit: u64| available.min(limit))
}

/// The room on the file system where a file goes.
#[derive(Clone, Debug)]
pub struct Room {
    /// The file's folder (for messages).
    pub folder: String,
    /// The file system's device (files on one device share its room).
    pub device: u64,
    /// Bytes available: free space, or free huge pages on hugetlbfs, plus
    /// what the file itself has now (it is overwritten).
    pub available: u64,
    /// The page size, on hugetlbfs.
    pub huge_page: Option<u64>,
    /// tmpfs (/dev/shm): its files take memory too.
    pub tmpfs: bool,
}

/// The folder of `path` ("." for a bare name).
#[cfg(any(unix, test))]
fn folder(path: &str) -> String {
    match std::path::Path::new(path).parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.to_string_lossy().into_owned(),
        _ => ".".to_string(),
    }
}

/// The room for a file at `path`, or `None` where it cannot be measured.
///
/// # Errors
/// If the file's folder does not exist.
#[cfg(unix)]
pub fn room(path: &str) -> anyhow::Result<Option<Room>> {
    use std::os::unix::fs::MetadataExt as _;
    let folder = folder(path);
    let dir = std::fs::metadata(&folder)
        .map_err(|e| BadInput(format!("cannot write {path}: its folder {folder}: {e}")))?;
    let c_folder = std::ffi::CString::new(folder.as_str())?;
    // SAFETY: a NUL-terminated path and a valid out-pointer.
    let st = unsafe {
        let mut st: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c_folder.as_ptr(), &raw mut st) != 0 {
            return Ok(None);
        }
        st
    };
    #[allow(
        clippy::unnecessary_cast,
        clippy::cast_lossless,
        reason = "the field types vary by target"
    )]
    let mut available = (st.f_bavail as u64).saturating_mul(st.f_frsize as u64);
    let (huge_page, tmpfs) = file_system(&c_folder, &mut available);
    // (An existing file is truncated or reused: its blocks count.)
    let existing = std::fs::metadata(path).map_or(0, |m| m.blocks().saturating_mul(512));
    Ok(Some(Room {
        folder,
        device: dir.dev(),
        available: limit(available.saturating_add(existing)),
        huge_page,
        tmpfs,
    }))
}

/// Non-unix: not measured.
///
/// # Errors
/// Never.
#[cfg(not(unix))]
pub fn room(_: &str) -> anyhow::Result<Option<Room>> {
    Ok(None)
}

/// The huge page size of a hugetlbfs folder (and `available` set to its free
/// pages, within the mount's quota if it has one), and whether it is tmpfs.
#[cfg(target_os = "linux")]
fn file_system(folder: &std::ffi::CStr, available: &mut u64) -> (Option<u64>, bool) {
    const HUGETLBFS_MAGIC: i64 = 0x9584_58f6;
    const TMPFS_MAGIC: i64 = 0x0102_1994;
    // SAFETY: a NUL-terminated path and a valid out-pointer.
    let st = unsafe {
        let mut st: libc::statfs = std::mem::zeroed();
        if libc::statfs(folder.as_ptr(), &raw mut st) != 0 {
            return (None, false);
        }
        st
    };
    #[allow(clippy::unnecessary_cast, reason = "f_type's type varies by target")]
    let kind = st.f_type as i64;
    if kind != HUGETLBFS_MAGIC {
        return (None, kind == TMPFS_MAGIC);
    }
    // (f_bsize is signed.)
    let page = u64::try_from(st.f_bsize).unwrap_or(0);
    #[allow(
        clippy::unnecessary_cast,
        clippy::cast_lossless,
        reason = "the field types vary by target"
    )]
    let (quota_pages, quota_free) = (st.f_blocks as u64, st.f_bavail as u64);
    // Free pages of that size, less those reserved by mappings.
    let pool = format!("/sys/kernel/mm/hugepages/hugepages-{}kB", page >> 10);
    let read = |name: &str| -> u64 {
        std::fs::read_to_string(format!("{pool}/{name}"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    };
    let mut pages = read("free_hugepages").saturating_sub(read("resv_hugepages"));
    if quota_pages > 0 {
        pages = pages.min(quota_free);
    }
    *available = pages.saturating_mul(page);
    (Some(page), false)
}

/// Other unix systems: no hugetlbfs, no tmpfs check.
#[cfg(all(unix, not(target_os = "linux")))]
fn file_system(_: &std::ffi::CStr, _: &mut u64) -> (Option<u64>, bool) {
    (None, false)
}

/// Memory available (bytes), where known (Linux): `MemAvailable`, and no
/// more than this process's cgroup limits leave (`memory.max` less
/// `memory.current`, cgroup v2, at each level up to the root: a run over
/// it is killed, e.g. under `systemd-run -p MemoryMax=`).
#[must_use]
pub fn memory_available() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|l| l.starts_with("MemAvailable:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    let available = kib.saturating_mul(1024);
    Some(limit(
        cgroup_room().map_or(available, |room| room.min(available)),
    ))
}

/// What the cgroup v2 memory limits of this process leave, if any is set.
fn cgroup_room() -> Option<u64> {
    let cgroups = std::path::Path::new("/sys/fs/cgroup");
    let text = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let own = text.lines().find_map(|l| l.strip_prefix("0::"))?;
    let mut dir = cgroups.join(own.trim_start_matches('/'));
    let mut left_min: Option<u64> = None;
    while dir.starts_with(cgroups) && dir != cgroups {
        let read = |name: &str| -> Option<u64> {
            std::fs::read_to_string(dir.join(name))
                .ok()?
                .trim()
                .parse()
                .ok()
        };
        // ("max": no limit at this level.)
        if let (Some(max), Some(current)) = (read("memory.max"), read("memory.current")) {
            let left = max.saturating_sub(current);
            left_min = Some(left_min.map_or(left, |m| m.min(left)));
        }
        dir.pop();
    }
    left_min
}

/// Check that the table files `outputs` (name for messages, path, data
/// bytes) fit where they go: files on one file system together (hugetlbfs
/// files are whole 1 GiB pages, as `tables::file::file_bytes` sizes them).
/// Logs what was checked. Returns each output's room (`None`: not
/// measured).
///
/// # Errors
/// A `BadInput` with the numbers if they do not fit (or a folder is
/// missing).
pub fn files(outputs: &[(&str, &str, u64)], hint: &str) -> anyhow::Result<Vec<Option<Room>>> {
    let mut each = Vec::new();
    let mut rooms: Vec<(Room, u64, Vec<&str>)> = Vec::new();
    for &(name, path, data_bytes) in outputs {
        let measured = room(path)?;
        each.push(measured.clone());
        let Some(room) = measured else {
            log::info!("preflight: {path}: room not measured on this system");
            continue;
        };
        let bytes = crate::tables::file::file_bytes(data_bytes, room.huge_page.is_some());
        match rooms.iter_mut().find(|(r, _, _)| r.device == room.device) {
            Some((_, total, names)) => {
                *total += bytes;
                names.push(name);
            }
            None => rooms.push((room, bytes, vec![name])),
        }
    }
    for (room, bytes, names) in &rooms {
        let what = names.join(" and ");
        let (kind, margin) = match room.huge_page {
            Some(page) => (format!("hugetlbfs, {} MiB pages, free", page >> 20), 0),
            None => ("free".to_string(), file_margin(*bytes)),
        };
        let place = &room.folder;
        if let Err(short) = check(*bytes, margin, room.available) {
            return Err(BadInput(format!(
                "{place} has {:.1} GiB {kind}, {what} needs {:.1} GiB (+ {:.1} GiB margin); {hint} \
                 (--force skips this check)",
                gib(short.available),
                gib(short.needed),
                gib(short.margin)
            ))
            .into());
        }
        log::info!(
            "preflight: {place}: {:.1} GiB {kind}, {what} needs {:.1} GiB (+ {:.1} GiB): ok",
            gib(room.available),
            gib(*bytes),
            gib(margin)
        );
    }
    Ok(each)
}

/// Check that `needed` bytes of memory (and `MEMORY_MARGIN`) are available
/// for `what`. Logs what was checked.
///
/// # Errors
/// A `BadInput` with the numbers if they are not.
pub fn memory(what: &str, needed: u64, hint: &str) -> anyhow::Result<()> {
    let Some(available) = memory_available() else {
        log::info!("preflight: memory not measured on this system");
        return Ok(());
    };
    if let Err(short) = check(needed, MEMORY_MARGIN, available) {
        return Err(BadInput(format!(
            "{:.1} GiB of memory available (MemAvailable, within the cgroup's memory.max), \
             {what} needs {:.1} GiB (+ {:.2} GiB margin); {hint}",
            gib(short.available),
            gib(short.needed),
            gib(short.margin)
        ))
        .into());
    }
    log::info!(
        "preflight: memory: {:.1} GiB available, {what} needs {:.1} GiB (+ {:.2} GiB): ok",
        gib(available),
        gib(needed),
        gib(MEMORY_MARGIN)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{check, file_margin, folder, Short};

    #[test]
    fn needs_and_margins() {
        assert_eq!(check(100, 10, 110), Ok(()));
        let short = Short {
            needed: 100,
            margin: 10,
            available: 109,
        };
        assert_eq!(check(100, 10, 109), Err(short));
        assert_eq!(check(0, 0, 0), Ok(()));
        // No overflow at the extremes.
        assert!(check(u64::MAX, 1, u64::MAX).is_err());
        // h11 (58 GiB) on a disk with 18.2 GiB free, and with 60 GiB.
        let h11 = 62_271_225_856 + 4096;
        assert!(check(h11, file_margin(h11), 19_541_180_416).is_err());
        assert!(check(h11, file_margin(h11), 64_424_509_440).is_ok());
        // The margin is 1% and 64 MiB.
        assert_eq!(file_margin(0), 64 << 20);
        assert_eq!(file_margin(100 << 20), (1 << 20) + (64 << 20));
    }

    #[test]
    fn folders() {
        assert_eq!(folder("h8"), ".");
        assert_eq!(folder("tables/h8"), "tables");
        assert_eq!(folder("/mnt/huge/h11"), "/mnt/huge");
    }
}
