//! Pruning tables: cocsep data and the big H48 table.

use crate::coord::*;
use crate::simd::Cube;
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// cocsep data
// ---------------------------------------------------------------------------

/// `data[cocsep] = class << 16 | ttrep << 8 | depth`.
pub struct Cocsep {
    pub data: Vec<u32>,
    /// Representative cube (corners only meaningful) per class.
    pub reps: Vec<Cube>,
    /// Bitmask of symmetries fixing the representative's cocsep, per class.
    pub selfsim: Vec<u64>,
    /// Depth distribution of the 153090 cocsep coordinates.
    pub distribution: Vec<u64>,
    /// Hot-path copy of `data` packed into 3 bytes per entry (25% less L2
    /// footprint): `class << 10 | ttrep << 4 | depth` (+1 pad byte), on a
    /// transparent huge page (one TLB entry instead of ~113).
    packed: Mem,
}

/// Packed cocsep entry: (class, ttrep, depth).
#[inline(always)]
pub fn cp_class(d: u32) -> usize {
    ((d >> 10) & 0xFFF) as usize
}
#[inline(always)]
pub fn cp_ttrep(d: u32) -> usize {
    ((d >> 4) & 0x3F) as usize
}
#[inline(always)]
pub fn cp_depth(d: u32) -> u8 {
    (d & 15) as u8
}

#[inline(always)]
pub fn cd_class(d: u32) -> usize {
    (d >> 16) as usize
}
#[inline(always)]
pub fn cd_ttrep(d: u32) -> usize {
    ((d >> 8) & 0xFF) as usize
}
#[inline(always)]
pub fn cd_depth(d: u32) -> u8 {
    (d & 0xFF) as u8
}

impl Cocsep {
    pub fn generate() -> Cocsep {
        // BFS over the cocsep coordinate, keeping a representative cube.
        let mut depth = vec![0xFFu8; N_COCSEP];
        let mut repcube = vec![Cube::solved(); N_COCSEP];
        let mut order = Vec::with_capacity(N_COCSEP);
        let s = cocsep(Cube::solved());
        depth[s] = 0;
        order.push(s);
        let mut head = 0;
        while head < order.len() {
            let i = order[head];
            head += 1;
            for m in 0..18 {
                let n = repcube[i].apply_move(m);
                let j = cocsep(n);
                if depth[j] == 0xFF {
                    depth[j] = depth[i] + 1;
                    repcube[j] = n;
                    order.push(j);
                }
            }
        }
        assert_eq!(order.len(), N_COCSEP);
        let mut distribution = vec![0u64; 16];
        for &d in &depth {
            distribution[d as usize] += 1;
        }
        let mut class = vec![u32::MAX; N_COCSEP];
        let mut ttrep = vec![0u8; N_COCSEP];
        let mut reps = Vec::new();
        let mut selfsim = Vec::new();
        let inv = crate::simd::SYM_INV;
        for &i in &order {
            if class[i] != u32::MAX {
                continue;
            }
            let n = reps.len() as u32;
            let r = repcube[i];
            let mut ss = 0u64;
            for t in 0..48 {
                let d = r.transform(t);
                let j = cocsep(d);
                if class[j] == u32::MAX {
                    class[j] = n;
                    ttrep[j] = inv[t];
                }
                if j == i {
                    ss |= 1 << t;
                }
            }
            debug_assert_eq!(ttrep[i], 0);
            reps.push(r);
            selfsim.push(ss);
        }
        assert_eq!(reps.len(), N_CLASSES);
        let data: Vec<u32> = (0..N_COCSEP)
            .map(|i| (class[i] << 16) | ((ttrep[i] as u32) << 8) | depth[i] as u32)
            .collect();
        // 2 MiB: the kernel aligns it so that it gets one huge page.
        let packed = Mem::anon(2 << 20);
        let pb = unsafe { std::slice::from_raw_parts_mut(packed.ptr(), 3 * N_COCSEP + 1) };
        for i in 0..N_COCSEP {
            assert!(depth[i] < 16);
            let p = (class[i] << 10) | ((ttrep[i] as u32) << 4) | depth[i] as u32;
            pb[3 * i..3 * i + 3].copy_from_slice(&p.to_le_bytes()[..3]);
        }
        Cocsep {
            data,
            reps,
            selfsim,
            distribution,
            packed,
        }
    }

    /// A cheap stand-in for unit tests: corner depths from a BFS cut after
    /// `depth` moves (deeper coordinates get `depth + 1`, still a lower bound
    /// and still a function of the coset), all in one class with no symmetry.
    #[cfg(test)]
    pub(crate) fn truncated(depth: u8) -> Cocsep {
        let mut depths = vec![depth + 1; N_COCSEP];
        depths[cocsep(Cube::solved())] = 0;
        let mut frontier = vec![Cube::solved()];
        for d in 1..=depth {
            let mut next = Vec::new();
            for cube in frontier {
                for m in 0..18 {
                    let child = cube.apply_move(m);
                    let coord = cocsep(child);
                    if depths[coord] > d {
                        depths[coord] = d;
                        next.push(child);
                    }
                }
            }
            frontier = next;
        }
        // Packed entries `class << 10 | ttrep << 4 | depth` with class and
        // ttrep 0: just the depth byte.
        let packed = Mem::anon(2 << 20);
        let bytes = unsafe { std::slice::from_raw_parts_mut(packed.ptr(), 3 * N_COCSEP + 1) };
        let mut distribution = vec![0u64; 16];
        for (i, &d) in depths.iter().enumerate() {
            bytes[3 * i] = d;
            distribution[usize::from(d)] += 1;
        }
        Cocsep {
            data: depths.iter().map(|&d| u32::from(d)).collect(),
            reps: vec![Cube::solved(); N_CLASSES],
            selfsim: vec![1; N_CLASSES],
            distribution,
            packed,
        }
    }

    /// Base of the packed entries (see `packed`).
    pub fn packed_ptr(&self) -> *const u8 {
        self.packed.ptr()
    }

    /// Packed entry of cocsep coordinate `i` (see `cp_*`).
    #[inline(always)]
    pub fn packed(&self, i: usize) -> u32 {
        debug_assert!(i < N_COCSEP);
        unsafe { (self.packed.ptr().add(3 * i) as *const u32).read_unaligned() }
    }

    /// Full h11 key of a cube.
    #[inline(always)]
    pub fn key11(&self, c: Cube) -> u64 {
        let d = unsafe { *self.data.get_unchecked(cocsep(c)) };
        let e = c.transform_edges(cd_ttrep(d));
        h11_key(cd_class(d), e)
    }

    /// hk table index of a cube.
    #[inline(always)]
    pub fn index(&self, c: Cube, k: u32) -> u64 {
        reduce_key(self.key11(c), k)
    }
}

// ---------------------------------------------------------------------------
// Memory backing for huge tables
// ---------------------------------------------------------------------------

pub struct Mem {
    ptr: *mut u8,
    len: usize,
}
unsafe impl Send for Mem {}
unsafe impl Sync for Mem {}

impl Mem {
    /// Anonymous memory, transparent huge pages requested.
    pub fn anon(len: usize) -> Mem {
        // (Miri supports only these flags, and no `madvise`.)
        #[cfg(not(miri))]
        let flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE;
        #[cfg(miri)]
        let flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS;
        unsafe {
            let p = libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                flags,
                -1,
                0,
            );
            assert!(p != libc::MAP_FAILED, "mmap failed");
            #[cfg(not(miri))]
            libc::madvise(p, len, libc::MADV_HUGEPAGE);
            Mem {
                ptr: p as *mut u8,
                len,
            }
        }
    }

    /// Shared mapping of a file (e.g. on hugetlbfs, which gives 1GB pages).
    /// If `create`, the file is created/truncated to `len` and mapped
    /// writable, else mapped read-only.
    pub fn file(path: &str, len: usize, create: bool) -> std::io::Result<Mem> {
        use std::os::unix::io::AsRawFd;
        // (Opened tables are mapped read-only: a stray write faults instead
        // of corrupting the shared table file for every process.)
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(create)
            .create(create)
            .truncate(false)
            .open(path)?;
        if create {
            // hugetlbfs needs the size rounded up to the page size.
            let page = 1usize << 30;
            let rounded = if path.contains("huge") {
                len.div_ceil(page) * page
            } else {
                len
            };
            f.set_len(rounded as u64)?;
        }
        let flen = f.metadata()?.len() as usize;
        if flen < len {
            return Err(std::io::Error::other(format!(
                "table file {path} too small: {flen} < {len}"
            )));
        }
        unsafe {
            let p = libc::mmap(
                std::ptr::null_mut(),
                flen,
                if create {
                    libc::PROT_READ | libc::PROT_WRITE
                } else {
                    libc::PROT_READ
                },
                libc::MAP_SHARED | libc::MAP_POPULATE,
                f.as_raw_fd(),
                0,
            );
            if p == libc::MAP_FAILED {
                return Err(std::io::Error::last_os_error());
            }
            libc::madvise(p, flen, libc::MADV_HUGEPAGE);
            Ok(Mem {
                ptr: p as *mut u8,
                len: flen,
            })
        }
    }

    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn words(&self) -> &[AtomicU64] {
        unsafe { std::slice::from_raw_parts(self.ptr as *const AtomicU64, self.len / 8) }
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for Mem {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, self.len);
        }
    }
}

// ---------------------------------------------------------------------------
// H48 table (cube48opt layout): 128-byte rows of 512 2-bit slots, one row
// per (class, msplit, eoK); the row is indexed by eslot. Each 16-byte group
// (64 slots) reserves slots 0-1 for a 4-bit minimum over the group.
// ---------------------------------------------------------------------------

pub const ROW_BYTES: u64 = 128;
pub const WORDS_PER_ROW: u64 = 16;

pub fn h48_rows(k: u32) -> u64 {
    (N_CLASSES * N_MSPLIT) as u64 * (1u64 << k)
}
/// Size of the index space (including unused slots).
pub fn h48_slots(k: u32) -> u64 {
    h48_rows(k) * N_SLOTS as u64
}
pub fn h48_bytes(k: u32) -> u64 {
    h48_rows(k) * ROW_BYTES
}

/// Default base value per h (cube48opt: 7 for h0, 8 for h2-3, 9 for h4-7, 10 above).
pub fn default_base(k: u32) -> u8 {
    [7, 8, 8, 8, 9, 9, 9, 9, 10, 10, 10, 10][k as usize]
}

/// u64 word index and bit shift of an entry.
#[inline(always)]
pub fn entry_pos(idx: u64) -> (usize, u32) {
    ((idx >> 5) as usize, ((idx & 31) * 2) as u32)
}

/// Mask of the low bits of the 2-bit slots of word `w` that are real entries.
#[inline(always)]
pub fn entry_lo_mask(w: u64) -> u64 {
    let mut m = 0x5555_5555_5555_5555u64;
    if w & 1 == 0 {
        m &= !0x5; // slots 0, 1: group minimum
    }
    if w % WORDS_PER_ROW == WORDS_PER_ROW - 1 {
        m &= !(1 << 62); // slot 511 unused
    }
    m
}

#[derive(Clone, Copy, Debug)]
pub struct H48Meta {
    pub k: u32,
    pub base: u8,
}

pub struct H48Table {
    pub mem: Mem,
    pub meta: H48Meta,
}

impl H48Table {
    #[inline(always)]
    pub fn ptr(&self) -> *const u8 {
        self.mem.ptr()
    }

    /// Returns (2-bit code, group minimum).
    #[inline(always)]
    pub fn get(&self, idx: u64) -> (u8, u8) {
        unsafe {
            let p = self.mem.ptr() as *const u64;
            let w = (idx >> 5) as usize;
            let x = *p.add(w);
            let m = *p.add(w & !1) & 15;
            (((x >> ((idx & 31) * 2)) & 3) as u8, m as u8)
        }
    }

    #[inline(always)]
    pub fn prefetch(&self, idx: u64) {
        unsafe {
            let a = self.mem.ptr().add((idx >> 2) as usize);
            // NTA: each line is read once; less L2 pollution (cocsep data).
            std::arch::x86_64::_mm_prefetch(a as *const i8, std::arch::x86_64::_MM_HINT_NTA);
        }
    }

    /// Distribution of the 2-bit codes over real entries.
    pub fn distribution(&self) -> [u64; 4] {
        let words_total = h48_bytes(self.meta.k) / 8;
        let nt = std::thread::available_parallelism()
            .map(|x| x.get())
            .unwrap_or(1) as u64;
        let per = words_total.div_ceil(nt);
        let words = self.mem.words();
        let mut tot = [0u64; 4];
        std::thread::scope(|s| {
            let hs: Vec<_> = (0..nt)
                .map(|t| {
                    s.spawn(move || {
                        let mut d = [0u64; 4];
                        for w in t * per..((t + 1) * per).min(words_total) {
                            let x = words[w as usize].load(Ordering::Relaxed);
                            let m = entry_lo_mask(w);
                            let lo = x & m;
                            let hi = (x >> 1) & m;
                            d[3] += (lo & hi).count_ones() as u64;
                            d[2] += (!lo & hi).count_ones() as u64;
                            d[1] += (lo & !hi).count_ones() as u64;
                            d[0] += (m & !lo & !hi).count_ones() as u64;
                        }
                        d
                    })
                })
                .collect();
            for h in hs {
                let d = h.join().unwrap();
                for i in 0..4 {
                    tot[i] += d[i];
                }
            }
        });
        tot
    }
}

/// The source tree this binary was built from, whose `tables/` holds the
/// `.hugemeta` files of tables on hugetlbfs (which only holds page-sized
/// files). Stored in a fixed-size buffer so that the binary's layout does not
/// depend on the length of the build path: otherwise builds from different
/// worktrees place all code at different offsets, which alone moves benchmark
/// results by 1-2%.
static BUILD_DIR: [u8; 256] = {
    let path = env!("CARGO_MANIFEST_DIR").as_bytes();
    assert!(path.len() < 256, "build path too long");
    let mut buf = [0u8; 256];
    let mut i = 0;
    while i < path.len() {
        buf[i] = path[i];
        i += 1;
    }
    buf
};

/// [`BUILD_DIR`] as a string (read through `black_box`, so the padded buffer
/// is what the binary contains, not a folded copy of the path).
fn build_dir() -> String {
    let buf: &[u8; 256] = std::hint::black_box(&BUILD_DIR);
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..len]).into_owned()
}

pub fn meta_path(path: &str) -> String {
    if path.starts_with("/mnt/huge") {
        let name = std::path::Path::new(path)
            .file_name()
            .unwrap()
            .to_string_lossy();
        format!("{}/tables/{name}.hugemeta", build_dir())
    } else {
        format!("{path}.meta")
    }
}

pub fn write_meta(path: &str, m: &H48Meta) -> std::io::Result<()> {
    std::fs::write(meta_path(path), format!("{} {}\n", m.k, m.base))
}

/// Read a table's `k` and `base`.
///
/// # Errors
/// If the meta file cannot be read, or does not hold `k <= 11` and a base
/// of at most 12: the search relies on bounds below 16 (`base + 3` and
/// 4-bit group minima) and on the index layout of `k`.
pub fn read_meta(path: &str) -> std::io::Result<H48Meta> {
    let s = std::fs::read_to_string(meta_path(path))?;
    let invalid = || {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("bad table meta {s:?}"),
        )
    };
    let v: Vec<u32> = s
        .split_whitespace()
        .map(|x| x.parse().map_err(|_| invalid()))
        .collect::<Result<_, _>>()?;
    match v[..] {
        [k, base] if k <= 11 && base <= 12 => Ok(H48Meta {
            k,
            base: u8::try_from(base).unwrap(),
        }),
        _ => Err(invalid()),
    }
}

impl H48Table {
    /// Open a table file. Files on hugetlbfs are mapped directly (1GB pages);
    /// other files are copied into anonymous huge-page memory.
    pub fn open(path: &str) -> std::io::Result<H48Table> {
        let meta = read_meta(path)?;
        let len = h48_bytes(meta.k) as usize;
        // A file of another `k` (a stale or wrong meta) would give wrong,
        // possibly too high, bounds: its size must match (hugetlbfs files
        // are rounded up to whole 1 GiB pages).
        let flen = std::fs::metadata(path)?.len();
        if flen < len as u64 || flen >= len as u64 + (1 << 30) {
            return Err(std::io::Error::other(format!(
                "table file {path}: {flen} bytes, but h{} needs {len}",
                meta.k
            )));
        }
        if path.contains("huge") || path.starts_with("/dev/shm") {
            let mem = Mem::file(path, len, false)?;
            return Ok(H48Table { mem, meta });
        }
        let mem = Mem::anon(len);
        let f = std::fs::File::open(path)?;
        let nt = 16usize;
        let per = len.div_ceil(nt);
        std::thread::scope(|s| {
            for t in 0..nt {
                let f = &f;
                let mem = &mem;
                s.spawn(move || {
                    use std::os::unix::fs::FileExt;
                    let start = t * per;
                    let end = ((t + 1) * per).min(len);
                    if start >= end {
                        return;
                    }
                    let buf = unsafe {
                        std::slice::from_raw_parts_mut(mem.ptr().add(start), end - start)
                    };
                    f.read_exact_at(buf, start as u64).expect("read table");
                });
            }
        });
        Ok(H48Table { mem, meta })
    }

    /// An all-zero table (every bound is then the corner depth), for unit
    /// tests with `Cocsep::truncated`.
    #[cfg(test)]
    pub(crate) fn zeroed(k: u32, base: u8) -> H48Table {
        H48Table {
            mem: Mem::anon(usize::try_from(h48_bytes(k)).unwrap()),
            meta: H48Meta { k, base },
        }
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let len = h48_bytes(self.meta.k) as usize;
        std::fs::write(path, &self.mem.bytes()[..len])?;
        write_meta(path, &self.meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cubie::*;

    /// Tables open only with a valid meta and a file of the size its `k`
    /// needs (a wrong meta would give wrong, possibly too high, bounds).
    #[test]
    #[cfg_attr(miri, ignore = "files")]
    fn table_meta_and_size_are_checked() {
        let dir = std::env::temp_dir().join(format!("cubeopt-meta-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("h0").to_string_lossy().into_owned();
        let len = h48_bytes(0);
        std::fs::write(&path, vec![0u8; usize::try_from(len).unwrap()]).unwrap();
        let open_with = |meta: &str| {
            std::fs::write(meta_path(&path), meta).unwrap();
            H48Table::open(&path).map(|t| (t.meta.k, t.meta.base))
        };
        assert_eq!(open_with("0 8\n").unwrap(), (0, 8));
        for bad in ["0 13", "12 8", "0", "0 8 1", "x 8", "0 -1", ""] {
            assert!(open_with(bad).is_err(), "meta {bad:?}");
        }
        // The file of an h0 table is too small for h1.
        assert!(open_with("1 8").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A table file opened for solving (hugetlbfs or /dev/shm) is mapped
    /// read-only.
    #[test]
    #[cfg_attr(miri, ignore = "files")]
    fn opened_table_file_is_read_only() {
        let path = format!("/dev/shm/cubeopt-test-{}-h0", std::process::id());
        let len = usize::try_from(h48_bytes(0)).unwrap();
        std::fs::write(&path, vec![0u8; len]).unwrap();
        std::fs::write(meta_path(&path), "0 8\n").unwrap();
        let table = H48Table::open(&path).unwrap();
        let start = table.ptr() as usize;
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
        let perms = maps
            .lines()
            .find(|l| l.starts_with(&format!("{start:x}-")))
            .map(|l| l.split_whitespace().nth(1).unwrap().to_string());
        assert_eq!(perms.as_deref(), Some("r--s"), "{maps}");
        drop(table);
        std::fs::remove_file(meta_path(&path)).unwrap();
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    #[cfg_attr(miri, ignore = "Cocsep::generate is too slow under Miri")]
    fn cocsep_classes_and_distribution() {
        let cs = Cocsep::generate();
        assert_eq!(cs.reps.len(), 3393);
        assert_eq!(
            &cs.distribution[..10],
            &[1, 6, 63, 468, 3068, 15438, 53814, 71352, 8784, 96]
        );
        // The cocsep class and depth are invariant under symmetry.
        let mut rng = Rng(7);
        for _ in 0..200 {
            let c = Cube::from_cubie(&CubieCube::random(&mut rng));
            let d0 = cs.data[cocsep(c)];
            for t in 0..48 {
                let d1 = cs.data[cocsep(c.transform(t))];
                assert_eq!(cd_class(d0), cd_class(d1));
                assert_eq!(cd_depth(d0), cd_depth(d1));
            }
            let p = cs.packed(cocsep(c));
            assert_eq!(cp_class(p), cd_class(d0));
            assert_eq!(cp_ttrep(p), cd_ttrep(d0));
            assert_eq!(cp_depth(p), cd_depth(d0));
            // ttrep maps the cube's corners onto the representative.
            let r = cs.reps[cd_class(d0)];
            assert_eq!(cocsep(c.transform(cd_ttrep(d0))), cocsep(r));
        }
    }
}
