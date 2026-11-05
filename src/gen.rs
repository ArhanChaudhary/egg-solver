//! Breadth-first generation of H48 tables.
//!
//! The BFS (breadth-first search: all positions at depth 1, then depth 2,
//! ...) runs on a true coset coordinate (h0 or h11) with 2 bits per entry:
//! `3` = unvisited, `2` = reached in the current level, `1` = frontier,
//! `0` = older. After each level a sequential pass relabels `1 -> 0` and
//! `2 -> 1`, records per-group minima, and updates derived tables (hk for
//! k < 11 are obtained by taking, for each hk entry, the minimum over the
//! h11 entries it covers). The BFS stops after level `base + 2`, at which
//! point the codes are exactly the final ones: `0` = `<= base`,
//! `1` = `base + 1`, `2` = `base + 2`, `3` = `>= base + 3`.
//!
//! Each level runs forward (expand the frontier's neighbors) while the
//! frontier is small, and backward (search each unvisited entry's
//! neighbors for a frontier entry) once most entries are reached (not
//! measured separately).

pub mod collapse;
pub mod dfs;
pub mod progress;

use crate::coord::{cocsep, cube_from_edge_key, h11_key, reduce_key, split_index};
use crate::cubie::Move;
use crate::simd::Cube;
use crate::tables::{
    entry_class, entry_low_bits, entry_position, entry_sym_to_rep, h48_bytes, h48_rows, Cocsep,
    H48Meta, Memory, WORDS_PER_ROW,
};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

/// A table generation: the table (`k`, `base`) and the derived tables to
/// fill along.
pub struct Generator<'a> {
    pub corners: &'a Cocsep,
    /// 0 or 11 (a true coordinate: every neighbor of an entry is known).
    pub k: u32,
    pub base: u8,
    pub threads: usize,
    /// Derived tables (`k` < the main table's) and their memory.
    pub derived: Vec<(H48Meta, &'a Memory)>,
}

/// A cube of table entry `index`.
#[inline(always)]
fn decode_index(corners: &Cocsep, k: u32, index: u64) -> Cube {
    let (class, edge_key) = split_index(index, k);
    cube_from_edge_key(corners.representatives[class], edge_key)
}

/// The 2-bit code of an entry.
#[inline(always)]
fn code_at(words: &[AtomicU64], index: u64) -> u64 {
    let (w, shift) = entry_position(index);
    (unsafe { words.get_unchecked(w) }.load(Relaxed) >> shift) & 3
}

/// Transition an entry from 3 (unvisited) to `2` if it is unvisited.
#[inline(always)]
fn mark_new(words: &[AtomicU64], index: u64) {
    let (w, shift) = entry_position(index);
    let word = unsafe { words.get_unchecked(w) };
    // (Load first: most neighbors are visited, and a plain load does not
    // take the cache line exclusive.)
    if (word.load(Relaxed) >> shift) & 3 == 3 {
        word.fetch_and(!(1u64 << shift), Relaxed);
    }
}

/// Atomically lower the 4-bit group minimum of the group containing word `w`.
fn lower_group_min(words: &[AtomicU64], w: usize, depth: u64) {
    let _ = words[w & !1].try_update(Relaxed, Relaxed, |bits| {
        (bits & 15 > depth).then_some((bits & !15) | depth)
    });
}

/// Atomically lower the code of a derived-table entry and its group minimum.
fn lower_derived(words: &[AtomicU64], index: u64, code: u64, depth: u64) {
    let (w, shift) = entry_position(index);
    let _ = words[w].try_update(Relaxed, Relaxed, |bits| {
        ((bits >> shift) & 3 > code).then_some((bits & !(3u64 << shift)) | (code << shift))
    });
    lower_group_min(words, w, depth);
}

impl Generator<'_> {
    /// Call `f` with the index of `cube` and all indices equivalent to it by
    /// the self-symmetries of its corner class (the same coset, stored once
    /// per symmetric variant).
    #[inline(always)]
    fn for_each_index(&self, cube: Cube, mut f: impl FnMut(u64)) {
        let entry = unsafe { *self.corners.entries.get_unchecked(cocsep(cube)) };
        let class = entry_class(entry);
        let edges = cube.transform_edges(entry_sym_to_rep(entry));
        f(reduce_key(h11_key(class, edges), self.k));
        let mut syms = unsafe { *self.corners.self_symmetries.get_unchecked(class) } & !1;
        while syms != 0 {
            let sym = syms.trailing_zeros() as usize;
            syms ^= syms.isolate_lowest_one();
            f(reduce_key(
                h11_key(class, edges.transform_edges(sym)),
                self.k,
            ));
        }
    }

    #[inline(always)]
    fn index(&self, cube: Cube) -> u64 {
        self.corners.index(cube, self.k)
    }

    /// Run `f(first_word, end_word)` over chunks of whole rows in parallel.
    fn for_word_chunks(&self, words: usize, f: impl Fn(usize, usize) + Sync) {
        let chunk = WORDS_PER_ROW << 12;
        let next = AtomicUsize::new(0);
        crate::parallel::on_threads(self.threads, |_| loop {
            let start = next.fetch_add(chunk, Relaxed);
            if start >= words {
                break;
            }
            f(start, (start + chunk).min(words));
        });
    }

    /// Run the BFS, filling `memory` and the derived tables. Returns the
    /// number of entries per depth.
    #[must_use]
    pub fn run(&self, memory: &Memory) -> Vec<u64> {
        let n_words = table_words(self.k);
        let words = memory.words();
        let t0 = Instant::now();
        // init: all entries 3, all group minima 15
        self.for_word_chunks(n_words, |first, end| {
            for word in &words[first..end] {
                word.store(u64::MAX, Relaxed);
            }
        });
        for (meta, memory) in &self.derived {
            let derived_words = memory.words();
            self.for_word_chunks(table_words(meta.k), |first, end| {
                for word in &derived_words[first..end] {
                    word.store(u64::MAX, Relaxed);
                }
            });
        }
        // depth 0
        let mut start = Vec::new();
        self.for_each_index(Cube::solved(), |i| start.push(i));
        start.sort_unstable();
        start.dedup();
        for &i in &start {
            let (w, shift) = entry_position(i);
            if code_at(words, i) == 3 {
                words[w].fetch_and(!(2u64 << shift), Relaxed); // 3 -> 1
            }
            lower_group_min(words, w, 0);
            for (meta, memory) in &self.derived {
                lower_derived(memory.words(), reduce_index(i, self.k, meta.k), 0, 0);
            }
        }
        let entries: u64 = h48_rows(self.k) * 495;
        let mut counts = vec![start.len() as u64];
        let mut total = start.len() as u64;
        progress::line(format_args!("[gen h{}] depth 0: {} entries", self.k, total));
        let last = u64::from(self.base) + 2;
        // (Progress: each level's expansion and finishing pass, by chunks.)
        let chunks = n_words.div_ceil(WORDS_PER_ROW << 12) as u64;
        for depth in 1..=last {
            progress::start(
                format_args!("h{} depth {depth} of {last}", self.k),
                2 * chunks,
            );
            let frontier = counts[counts.len() - 1];
            let unvisited = entries - total;
            // Forward costs ~18 neighbors per frontier entry, backward up
            // to 18 lookups per unvisited one, but stops at the first hit.
            let forward = frontier.saturating_mul(18) < unvisited.saturating_mul(4);
            let level_start = Instant::now();
            if forward {
                self.for_word_chunks(n_words, |first, end| {
                    self.expand_forward(words, first, end);
                    progress::advance(1);
                });
            } else {
                self.for_word_chunks(n_words, |first, end| {
                    self.expand_backward(words, first, end);
                    progress::advance(1);
                });
            }
            let step_seconds = level_start.elapsed().as_secs_f64();
            let relabel = depth != last;
            let new_entries = AtomicU64::new(0);
            self.for_word_chunks(n_words, |first, end| {
                let count = self.finish_level(words, first, end, depth, relabel);
                new_entries.fetch_add(count, Relaxed);
                progress::advance(1);
            });
            let count = new_entries.load(Relaxed);
            counts.push(count);
            total += count;
            progress::line(format_args!(
                "[gen h{}] depth {depth}: {count} entries ({}, {step_seconds:.1}s step, {:.1}s total)",
                self.k,
                if forward { "forward" } else { "backward" },
                t0.elapsed().as_secs_f64()
            ));
        }
        progress::line(format_args!(
            "[gen h{}] >= {}: {} entries",
            self.k,
            last + 1,
            entries - total
        ));
        counts.push(entries - total);
        counts
    }

    /// Mark the unvisited neighbors of the frontier (code 1) entries in
    /// words `first..end` as new (code 2).
    fn expand_forward(&self, words: &[AtomicU64], first: usize, end: usize) {
        for (w, word) in (first as u64..).zip(&words[first..end]) {
            let bits = word.load(Relaxed);
            let mask = entry_low_bits(w);
            let lo = bits & mask;
            let hi = (bits >> 1) & mask;
            let mut frontier = lo & !hi; // code 1
            while frontier != 0 {
                let bit = u64::from(frontier.trailing_zeros());
                frontier ^= frontier.isolate_lowest_one();
                let cube = decode_index(self.corners, self.k, w * 32 + bit / 2);
                for mv in Move::ALL {
                    let neighbor = cube.apply_move(mv);
                    self.for_each_index(neighbor, |i| mark_new(words, i));
                }
            }
        }
    }

    /// Mark the unvisited (code 3) entries in words `first..end` that have a
    /// frontier neighbor as new (code 2). Entries go in batches of 8: their
    /// 144 neighbor lookups are prefetched first.
    fn expand_backward(&self, words: &[AtomicU64], first: usize, end: usize) {
        const BATCH: usize = 8;
        let mut pending: [u64; BATCH] = [0; BATCH];
        let mut n_pending = 0;
        let mut neighbors = [[0u64; 18]; BATCH];
        let flush = |pending: &[u64], neighbors: &mut [[u64; 18]; BATCH]| {
            for (p, &index) in pending.iter().enumerate() {
                let cube = decode_index(self.corners, self.k, index);
                for (&mv, slot) in Move::ALL.iter().zip(neighbors[p].iter_mut()) {
                    let i = self.index(cube.apply_move(mv));
                    *slot = i;
                    let (w, _) = entry_position(i);
                    crate::prefetch::read(words.as_ptr().wrapping_add(w));
                }
            }
            for (p, &index) in pending.iter().enumerate() {
                if neighbors[p].iter().any(|&i| code_at(words, i) == 1) {
                    let (w, shift) = entry_position(index);
                    words[w].fetch_and(!(1u64 << shift), Relaxed); // 3 -> 2
                }
            }
        };
        for (w, word) in (first as u64..).zip(&words[first..end]) {
            let bits = word.load(Relaxed);
            let mut unvisited = bits & (bits >> 1) & entry_low_bits(w); // code 3
            while unvisited != 0 {
                let bit = u64::from(unvisited.trailing_zeros());
                unvisited ^= unvisited.isolate_lowest_one();
                pending[n_pending] = w * 32 + bit / 2;
                n_pending += 1;
                if n_pending == BATCH {
                    flush(&pending, &mut neighbors);
                    n_pending = 0;
                }
            }
        }
        flush(&pending[..n_pending], &mut neighbors);
    }

    /// Count the new (code 2) entries in words `first..end`, update group
    /// minima and derived tables, and optionally relabel (1 -> 0, 2 -> 1).
    fn finish_level(
        &self,
        words: &[AtomicU64],
        first: usize,
        end: usize,
        depth: u64,
        relabel: bool,
    ) -> u64 {
        let mut count = 0;
        // Last derived index updated per derived table: neighboring h11
        // entries often share one.
        let mut last = [u64::MAX; 8];
        for wi in first..end {
            let w = wi as u64;
            let bits = words[wi].load(Relaxed);
            let mask = entry_low_bits(w);
            let lo = bits & mask;
            let hi = (bits >> 1) & mask;
            let mut new = hi & !lo; // code 2
            if new != 0 {
                count += u64::from(new.count_ones());
                lower_group_min(words, wi, depth);
                while new != 0 && !self.derived.is_empty() {
                    let bit = u64::from(new.trailing_zeros());
                    new ^= new.isolate_lowest_one();
                    let index = w * 32 + bit / 2;
                    for (last, (meta, memory)) in last.iter_mut().zip(&self.derived) {
                        let j = reduce_index(index, self.k, meta.k);
                        if j == *last {
                            continue;
                        }
                        *last = j;
                        let code = depth.saturating_sub(u64::from(meta.base)).min(3);
                        lower_derived(memory.words(), j, code, depth);
                    }
                }
            }
            if relabel {
                // reload: lower_group_min may have changed this word
                let bits = words[wi].load(Relaxed);
                let keep = !(mask | (mask << 1));
                words[wi].store((bits & keep) | hi | ((hi & lo) << 1), Relaxed);
            }
        }
        count
    }
}

/// Number of `u64` words of an hk table.
fn table_words(k: u32) -> usize {
    usize::try_from(h48_bytes(k) / 8).expect("64-bit usize")
}

/// Map an index of the hk table to the index of the h`dk` table (dk <= k).
#[inline(always)]
fn reduce_index(index: u64, k: u32, dk: u32) -> u64 {
    ((index >> (k - dk)) & !0x1FF) | (index & 0x1FF)
}
