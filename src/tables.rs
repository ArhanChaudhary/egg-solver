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
        let data = (0..N_COCSEP)
            .map(|i| (class[i] << 16) | ((ttrep[i] as u32) << 8) | depth[i] as u32)
            .collect();
        Cocsep {
            data,
            reps,
            selfsim,
            distribution,
        }
    }

    /// Full h11 index of a cube.
    #[inline(always)]
    pub fn index11(&self, c: Cube) -> u64 {
        let d = unsafe { *self.data.get_unchecked(cocsep(c)) };
        let e = c.transform_edges(cd_ttrep(d));
        cd_class(d) as u64 * (N_ESEP * N_EO) as u64 + edge_coord(e)
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
        unsafe {
            let p = libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            );
            assert!(p != libc::MAP_FAILED, "mmap failed");
            libc::madvise(p, len, libc::MADV_HUGEPAGE);
            Mem {
                ptr: p as *mut u8,
                len,
            }
        }
    }

    /// Shared mapping of a file (e.g. on hugetlbfs, which gives 1GB pages).
    /// If `create`, the file is created/truncated to `len`.
    pub fn file(path: &str, len: usize, create: bool) -> std::io::Result<Mem> {
        use std::os::unix::io::AsRawFd;
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
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
                libc::PROT_READ | libc::PROT_WRITE,
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
// H48 table: 64-byte lines of 254 2-bit entries + a 4-bit line minimum.
// ---------------------------------------------------------------------------

pub const LINE_ENTRIES: u64 = 254;
pub const LINE_BYTES: u64 = 64;

pub fn h48_entries(k: u32) -> u64 {
    (N_CLASSES * N_ESEP) as u64 * (1u64 << k)
}
pub fn h48_lines(k: u32) -> u64 {
    h48_entries(k).div_ceil(LINE_ENTRIES)
}
pub fn h48_bytes(k: u32) -> u64 {
    h48_lines(k) * LINE_BYTES
}

/// Default base value per h (from nissy, tuned experimentally there).
pub fn default_base(k: u32) -> u8 {
    [8, 8, 8, 8, 9, 9, 9, 9, 10, 10, 10, 10][k as usize]
}

/// Word index and bit shift of an entry.
#[inline(always)]
pub fn entry_pos(idx: u64) -> (usize, u32) {
    let line = idx / LINE_ENTRIES;
    let pos = idx - line * LINE_ENTRIES;
    ((line * 8 + pos / 32) as usize, ((pos % 32) * 2) as u32)
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

    /// Returns (2-bit code, line minimum).
    #[inline(always)]
    pub fn get(&self, idx: u64) -> (u8, u8) {
        let line = idx / LINE_ENTRIES;
        let pos = idx - line * LINE_ENTRIES;
        unsafe {
            let lp = self.mem.ptr().add((line * LINE_BYTES) as usize);
            let b = *lp.add((pos >> 2) as usize);
            let m = *lp.add(63) >> 4;
            ((b >> ((pos & 3) * 2)) & 3, m)
        }
    }

    #[inline(always)]
    pub fn prefetch(&self, idx: u64) {
        let line = idx / LINE_ENTRIES;
        unsafe {
            let lp = self.mem.ptr().add((line * LINE_BYTES) as usize);
            std::arch::x86_64::_mm_prefetch(lp as *const i8, std::arch::x86_64::_MM_HINT_T0);
        }
    }

    /// Distribution of the 2-bit codes.
    pub fn distribution(&self) -> [u64; 4] {
        let lines = h48_lines(self.meta.k);
        let entries = h48_entries(self.meta.k);
        let nt = std::thread::available_parallelism()
            .map(|x| x.get())
            .unwrap_or(1) as u64;
        let per = lines.div_ceil(nt);
        let words = self.mem.words();
        let mut tot = [0u64; 4];
        std::thread::scope(|s| {
            let hs: Vec<_> = (0..nt)
                .map(|t| {
                    s.spawn(move || {
                        let mut d = [0u64; 4];
                        for line in t * per..((t + 1) * per).min(lines) {
                            for w in 0..8 {
                                let x = words[(line * 8 + w) as usize].load(Ordering::Relaxed);
                                let n = if w == 7 { 30 } else { 32 };
                                for e in 0..n {
                                    let idx = line * LINE_ENTRIES + w * 32 + e;
                                    if idx < entries {
                                        d[((x >> (2 * e)) & 3) as usize] += 1;
                                    }
                                }
                            }
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

/// Metadata sidecar path. hugetlbfs does not support `write(2)`, so for
/// tables there the sidecar lives in the repository's `tables/` directory.
pub fn meta_path(path: &str) -> String {
    if path.starts_with("/mnt/huge") {
        let name = std::path::Path::new(path)
            .file_name()
            .unwrap()
            .to_string_lossy();
        format!("{}/tables/{name}.hugemeta", env!("CARGO_MANIFEST_DIR"))
    } else {
        format!("{path}.meta")
    }
}

pub fn write_meta(path: &str, m: &H48Meta) -> std::io::Result<()> {
    std::fs::write(meta_path(path), format!("{} {}\n", m.k, m.base))
}

pub fn read_meta(path: &str) -> std::io::Result<H48Meta> {
    let s = std::fs::read_to_string(meta_path(path))?;
    let v: Vec<u32> = s.split_whitespace().map(|x| x.parse().unwrap()).collect();
    Ok(H48Meta {
        k: v[0],
        base: v[1] as u8,
    })
}

impl H48Table {
    /// Open a table file. Files on hugetlbfs are mapped directly (1GB pages);
    /// other files are copied into anonymous huge-page memory.
    pub fn open(path: &str) -> std::io::Result<H48Table> {
        let meta = read_meta(path)?;
        let len = h48_bytes(meta.k) as usize;
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

    #[test]
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
            // ttrep maps the cube's corners onto the representative.
            let r = cs.reps[cd_class(d0)];
            assert_eq!(cocsep(c.transform(cd_ttrep(d0))), cocsep(r));
        }
    }
}
