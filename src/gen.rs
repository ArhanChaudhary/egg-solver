//! Breadth-first generation of H48 tables.
//!
//! The BFS runs on a true coset coordinate (h0 or h11) with 2 bits per entry:
//! `3` = unvisited, `2` = reached in the current level, `1` = frontier,
//! `0` = older. After each level a sequential pass relabels `1 -> 0` and
//! `2 -> 1`, records per-line minima, and updates derived tables (hk for
//! k < 11 are obtained by taking, for each hk entry, the minimum over the
//! h11 entries it covers). The BFS stops after level `base + 2`, at which
//! point the codes are exactly the final ones: `0` = `<= base`,
//! `1` = `base + 1`, `2` = `base + 2`, `3` = `>= base + 3`.

use crate::coord::*;
use crate::simd::Cube;
use crate::tables::*;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::Instant;

const ESIZE11: u64 = (N_ESEP * N_EO) as u64;
const M55: u64 = 0x5555_5555_5555_5555;
const M55_7: u64 = 0x0555_5555_5555_5555; // word 7 has 30 entries
const NIBBLE_SHIFT: u32 = 60;

pub struct Gen<'a> {
    pub cs: &'a Cocsep,
    pub k: u32,
    pub base: u8,
    pub threads: usize,
    pub derived: Vec<(H48Meta, &'a Mem)>,
}

#[inline(always)]
fn decode(cs: &Cocsep, k: u32, idx: u64) -> Cube {
    let (class, ec) = if k == 11 {
        (idx / ESIZE11, idx % ESIZE11)
    } else {
        let e = N_ESEP as u64;
        (idx / e, (idx % e) << 11)
    };
    cube_from_edge_coord(cs.reps[class as usize], ec)
}

#[inline(always)]
fn code_at(words: &[AtomicU64], idx: u64) -> u64 {
    let (w, sh) = entry_pos(idx);
    (unsafe { words.get_unchecked(w) }.load(Relaxed) >> sh) & 3
}

/// Transition an entry from 3 (unvisited) to `2` if it is unvisited.
#[inline(always)]
fn mark_new(words: &[AtomicU64], idx: u64) {
    let (w, sh) = entry_pos(idx);
    let a = unsafe { words.get_unchecked(w) };
    if (a.load(Relaxed) >> sh) & 3 == 3 {
        a.fetch_and(!(1u64 << sh), Relaxed);
    }
}

/// Atomically lower the code of a derived-table entry and its line minimum.
fn derived_update(words: &[AtomicU64], idx: u64, code: u64, depth: u64) {
    let (w, sh) = entry_pos(idx);
    let a = &words[w];
    let mut cur = a.load(Relaxed);
    loop {
        let c = (cur >> sh) & 3;
        if c <= code {
            break;
        }
        let new = (cur & !(3u64 << sh)) | (code << sh);
        match a.compare_exchange_weak(cur, new, Relaxed, Relaxed) {
            Ok(_) => break,
            Err(x) => cur = x,
        }
    }
    let wn = &words[(w & !7) + 7];
    let mut cur = wn.load(Relaxed);
    loop {
        if (cur >> NIBBLE_SHIFT) <= depth {
            break;
        }
        let new = (cur & !(0xFu64 << NIBBLE_SHIFT)) | (depth << NIBBLE_SHIFT);
        match wn.compare_exchange_weak(cur, new, Relaxed, Relaxed) {
            Ok(_) => break,
            Err(x) => cur = x,
        }
    }
}

impl<'a> Gen<'a> {
    #[inline(always)]
    fn shift(&self) -> u32 {
        11 - self.k
    }

    /// Index of a neighbor plus all indices equivalent to it by the
    /// self-symmetries of its corner class.
    #[inline(always)]
    fn for_each_index(&self, n: Cube, mut f: impl FnMut(u64)) {
        let d = unsafe { *self.cs.data.get_unchecked(cocsep(n)) };
        let class = cd_class(d) as u64;
        let e = n.transform_edges(cd_ttrep(d));
        f((class * ESIZE11 + edge_coord(e)) >> self.shift());
        let mut ss = unsafe { *self.cs.selfsim.get_unchecked(class as usize) } & !1;
        while ss != 0 {
            let t = ss.trailing_zeros() as usize;
            ss &= ss - 1;
            let e2 = e.transform_edges(t);
            f((class * ESIZE11 + edge_coord(e2)) >> self.shift());
        }
    }

    #[inline(always)]
    fn index(&self, n: Cube) -> u64 {
        let d = unsafe { *self.cs.data.get_unchecked(cocsep(n)) };
        let e = n.transform_edges(cd_ttrep(d));
        (cd_class(d) as u64 * ESIZE11 + edge_coord(e)) >> self.shift()
    }

    fn par_lines(&self, lines: u64, f: impl Fn(u64, u64) + Sync) {
        let chunk = 1u64 << 14;
        let next = AtomicU64::new(0);
        std::thread::scope(|s| {
            for _ in 0..self.threads {
                s.spawn(|| loop {
                    let start = next.fetch_add(chunk, Relaxed);
                    if start >= lines {
                        break;
                    }
                    f(start, (start + chunk).min(lines));
                });
            }
        });
    }

    pub fn run(&self, mem: &Mem) -> Vec<u64> {
        let entries = h48_entries(self.k);
        let lines = h48_lines(self.k);
        let words = mem.words();
        let t0 = Instant::now();
        // init
        self.par_lines(lines, |a, b| {
            for w in a * 8..b * 8 {
                words[w as usize].store(u64::MAX, Relaxed);
            }
        });
        for (dm, dmem) in &self.derived {
            let dl = h48_lines(dm.k);
            let dw = dmem.words();
            self.par_lines(dl, |a, b| {
                for w in a * 8..b * 8 {
                    dw[w as usize].store(u64::MAX, Relaxed);
                }
            });
        }
        // depth 0
        let solved = Cube::solved();
        let mut start = Vec::new();
        self.for_each_index(solved, |i| start.push(i));
        for &i in &start {
            let (w, sh) = entry_pos(i);
            if code_at(words, i) == 3 {
                words[w].fetch_and(!(2u64 << sh), Relaxed); // 3 -> 1
            }
            let wn = (w & !7) + 7;
            words[wn].fetch_and(!(0xFu64 << NIBBLE_SHIFT), Relaxed); // min = 0
            for (dm, dmem) in &self.derived {
                derived_update(dmem.words(), i >> (11 - dm.k), 0, 0);
            }
        }
        start.sort();
        start.dedup();
        let mut counts = vec![start.len() as u64];
        let mut total = start.len() as u64;
        eprintln!("[gen h{}] depth 0: {} entries", self.k, total);
        let last = self.base as u64 + 2;
        for d in 0..last {
            let frontier = counts[d as usize];
            let unvisited = entries - total;
            let forward = frontier.saturating_mul(18) < unvisited.saturating_mul(4);
            let tl = Instant::now();
            if forward {
                self.par_lines(lines, |a, b| self.forward_chunk(words, entries, a, b));
            } else {
                self.par_lines(lines, |a, b| self.backward_chunk(words, entries, a, b));
            }
            let tstep = tl.elapsed().as_secs_f64();
            let newd = d + 1;
            let relabel = newd != last;
            let cnt = AtomicU64::new(0);
            self.par_lines(lines, |a, b| {
                let c = self.post_pass(words, entries, a, b, newd, relabel);
                cnt.fetch_add(c, Relaxed);
            });
            let c = cnt.load(Relaxed);
            counts.push(c);
            total += c;
            eprintln!(
                "[gen h{}] depth {newd}: {c} entries ({}, {tstep:.1}s step, {:.1}s total)",
                self.k,
                if forward { "forward" } else { "backward" },
                t0.elapsed().as_secs_f64()
            );
        }
        eprintln!(
            "[gen h{}] >= {}: {} entries",
            self.k,
            last + 1,
            entries - total
        );
        counts.push(entries - total);
        counts
    }

    fn forward_chunk(&self, words: &[AtomicU64], entries: u64, a: u64, b: u64) {
        for line in a..b {
            for w in 0..8u64 {
                let x = words[(line * 8 + w) as usize].load(Relaxed);
                let m = if w == 7 { M55_7 } else { M55 };
                let lo = x & m;
                let hi = (x >> 1) & m;
                let mut f = lo & !hi; // code 1
                while f != 0 {
                    let bit = f.trailing_zeros() as u64;
                    f &= f - 1;
                    let idx = line * LINE_ENTRIES + w * 32 + bit / 2;
                    if idx >= entries {
                        continue;
                    }
                    let c = decode(self.cs, self.k, idx);
                    for mv in 0..18 {
                        let n = c.apply_move(mv);
                        self.for_each_index(n, |i| mark_new(words, i));
                    }
                }
            }
        }
    }

    fn backward_chunk(&self, words: &[AtomicU64], entries: u64, a: u64, b: u64) {
        const BATCH: usize = 8;
        let mut pending: [u64; BATCH] = [0; BATCH];
        let mut np = 0;
        let mut nidx = [[0u64; 18]; BATCH];
        let flush = |pending: &[u64], nidx: &mut [[u64; 18]; BATCH]| {
            for (p, &idx) in pending.iter().enumerate() {
                let c = decode(self.cs, self.k, idx);
                for mv in 0..18 {
                    let i = self.index(c.apply_move(mv));
                    nidx[p][mv] = i;
                    let (w, _) = entry_pos(i);
                    unsafe {
                        std::arch::x86_64::_mm_prefetch(
                            words.as_ptr().add(w) as *const i8,
                            std::arch::x86_64::_MM_HINT_T0,
                        );
                    }
                }
            }
            for (p, &idx) in pending.iter().enumerate() {
                for mv in 0..18 {
                    if code_at(words, nidx[p][mv]) == 1 {
                        let (w, sh) = entry_pos(idx);
                        words[w].fetch_and(!(1u64 << sh), Relaxed); // 3 -> 2
                        break;
                    }
                }
            }
        };
        for line in a..b {
            for w in 0..8u64 {
                let x = words[(line * 8 + w) as usize].load(Relaxed);
                let m = if w == 7 { M55_7 } else { M55 };
                let mut f = x & (x >> 1) & m; // code 3
                while f != 0 {
                    let bit = f.trailing_zeros() as u64;
                    f &= f - 1;
                    let idx = line * LINE_ENTRIES + w * 32 + bit / 2;
                    if idx >= entries {
                        continue;
                    }
                    pending[np] = idx;
                    np += 1;
                    if np == BATCH {
                        flush(&pending, &mut nidx);
                        np = 0;
                    }
                }
            }
        }
        flush(&pending[..np], &mut nidx);
    }

    /// Count code-2 entries, update line minima and derived tables, and
    /// optionally relabel (1 -> 0, 2 -> 1).
    fn post_pass(
        &self,
        words: &[AtomicU64],
        entries: u64,
        a: u64,
        b: u64,
        depth: u64,
        relabel: bool,
    ) -> u64 {
        let mut count = 0;
        for line in a..b {
            let mut any_new = false;
            for w in 0..8u64 {
                let wi = (line * 8 + w) as usize;
                let x = words[wi].load(Relaxed);
                let m = if w == 7 { M55_7 } else { M55 };
                let lo = x & m;
                let hi = (x >> 1) & m;
                let mut newm = hi & !lo; // code 2
                                         // padding entries past the end are always 3, never 2
                if newm != 0 {
                    any_new = true;
                    count += newm.count_ones() as u64;
                    if !self.derived.is_empty() {
                        let mut last = [u64::MAX; 8];
                        while newm != 0 {
                            let bit = newm.trailing_zeros() as u64;
                            newm &= newm - 1;
                            let idx = line * LINE_ENTRIES + w * 32 + bit / 2;
                            debug_assert!(idx < entries);
                            for (di, (dm, dmem)) in self.derived.iter().enumerate() {
                                let j = idx >> (11 - dm.k);
                                if j == last[di] {
                                    continue;
                                }
                                last[di] = j;
                                let code = depth.saturating_sub(dm.base as u64).min(3);
                                derived_update(dmem.words(), j, code, depth);
                            }
                        }
                    }
                }
                if relabel {
                    let nl = hi;
                    let nh = hi & lo;
                    let keep = !(m | (m << 1));
                    words[wi].store((x & keep) | nl | (nh << 1), Relaxed);
                }
            }
            if any_new {
                let wn = &words[(line * 8 + 7) as usize];
                let x = wn.load(Relaxed);
                if x >> NIBBLE_SHIFT > depth {
                    wn.store(
                        (x & !(0xFu64 << NIBBLE_SHIFT)) | (depth << NIBBLE_SHIFT),
                        Relaxed,
                    );
                }
            }
        }
        count
    }
}
