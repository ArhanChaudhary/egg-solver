//! IDA* optimal solver using the H48 table.

use crate::coord::*;
use crate::cubie::{inverse_move, CubieCube};
use crate::simd::Cube;
use crate::tables::*;

pub const ALL_MOVES: u32 = (1 << 18) - 1;
/// Quarter turns only (moves with m % 3 != 1).
pub const NO_HALF: u32 = 0b101_101_101_101_101_101;

/// Moves allowed after a move on `face` (on the same side of the search).
const fn mk_allowed() -> [u32; 6] {
    let mut r = [0u32; 6];
    let mut f = 0;
    while f < 6 {
        let mut m = ALL_MOVES & !(7 << (3 * f));
        if f % 2 == 1 {
            m &= !(7 << (3 * (f - 1)));
        }
        r[f] = m;
        f += 1;
    }
    r
}
pub static ALLOWED_AFTER: [u32; 6] = mk_allowed();

pub struct Solver<'a> {
    pub cs: &'a Cocsep,
    pub table: &'a H48Table,
    /// `table.ptr()`, cached (saves a dependent load per prefetch/lookup).
    tptr: *const u8,
    k: u32,
    pub(crate) base: u8,
}
// `tptr` points into `table`, which is shared read-only.
unsafe impl Send for Solver<'_> {}
unsafe impl Sync for Solver<'_> {}

#[derive(Default, Clone, Copy, Debug)]
pub struct Stats {
    pub nodes: u64,
    pub lookups: u64,
}

/// A canonical move prefix searched as a unit; jobs are re-ordered between
/// iterations by `stat`, an estimate of how close their subtree got to
/// solved (sum of 2^(12-h) over evaluated nodes), so that in the last
/// iteration the solution tends to be found early.
#[derive(Clone, Copy)]
struct Job {
    moves: [u8; 4],
    len: u8,
    c: Cube,
    ci: Cube,
    stat: u64,
    /// Lower bounds of the job root (normal, inverse); computed once.
    ln: u8,
    li: u8,
}

/// Face weight compensating for D/L/B prefixes having fewer continuations.
#[inline(always)]
fn job_weight(j: &Job) -> u64 {
    if (j.moves[j.len as usize - 1] / 3) % 2 == 0 {
        47525
    } else {
        58206
    }
}

struct Ctx {
    moves: [u8; 32],
    nmoves: usize,
    premoves: [u8; 32],
    npremoves: usize,
    stats: Stats,
    heur: u64,
}

#[derive(Clone, Copy)]
struct Kid {
    c: Cube,
    i: Cube,
    idx: u64,
    m: u8,
    pn: u8,
    pi: u8,
    live: bool,
}

const UNKNOWN: u8 = 0xFF;
const FOUND: u8 = 0xFE;

impl<'a> Solver<'a> {
    pub fn new(cs: &'a Cocsep, table: &'a H48Table) -> Solver<'a> {
        Solver {
            cs,
            table,
            tptr: table.ptr(),
            k: table.meta.k,
            base: table.meta.base,
        }
    }

    /// (table index, corner lower bound)
    #[inline(always)]
    pub(crate) fn index(&self, c: Cube) -> (u64, u8) {
        let d = self.cs.packed(cocsep(c));
        let parts = edge_parts_sym(c, cp_ttrep(d));
        (index_from_parts(cp_class(d), parts, self.k), cp_depth(d))
    }

    #[inline(always)]
    pub(crate) fn value(&self, idx: u64, cdepth: u8) -> u8 {
        // `H48Table::get` through the cached pointer.
        let (code, min) = unsafe {
            let p = self.tptr as *const u64;
            let w = (idx >> 5) as usize;
            let x = *p.add(w);
            let m = *p.add(w & !1) & 15;
            (((x >> ((idx & 31) * 2)) & 3) as u8, m as u8)
        };
        if code == 0 {
            min.max(cdepth)
        } else {
            self.base + code
        }
    }

    /// Prefetch table entry `idx` (`H48Table::prefetch`).
    #[inline(always)]
    pub(crate) fn prefetch(&self, idx: u64) {
        unsafe {
            let a = self.tptr.add((idx >> 2) as usize);
            // NTA: each line is read once; less L2 pollution (cocsep data).
            std::arch::x86_64::_mm_prefetch(a as *const i8, std::arch::x86_64::_MM_HINT_NTA);
        }
    }

    /// Lower bound for a single cube (no pipelining).
    pub fn lower_bound(&self, c: Cube) -> u8 {
        let (idx, cd) = self.index(c);
        self.value(idx, cd)
    }

    /// Length of the job prefixes for an iteration with target `d`.
    pub(crate) fn job_len(&self, d: u8) -> usize {
        let top = self.base + 3;
        if d > top {
            ((d - top) as usize).min(4)
        } else {
            0
        }
    }

    /// Extend jobs by one move; children inherit the parent's stat.
    fn extend_jobs(jobs: &[Job]) -> Vec<Job> {
        let mut out = Vec::with_capacity(jobs.len() * 15);
        for j in jobs {
            let mm = if j.len == 0 {
                ALL_MOVES
            } else {
                ALLOWED_AFTER[(j.moves[j.len as usize - 1] / 3) as usize]
            };
            let mut mmv = mm;
            while mmv != 0 {
                let m = mmv.trailing_zeros() as usize;
                mmv &= mmv - 1;
                let mut n = *j;
                n.moves[n.len as usize] = m as u8;
                n.len += 1;
                n.c = j.c.apply_move(m);
                n.ci = j.ci.premove_inv(m);
                out.push(n);
            }
        }
        out
    }

    /// Compute the root bounds of jobs, prefetching in batches.
    fn job_bounds(&self, jobs: &mut [Job]) {
        for chunk in jobs.chunks_mut(32) {
            let mut idx = [(0u64, 0u8); 64];
            for (n, j) in chunk.iter().enumerate() {
                idx[2 * n] = self.index(j.c);
                idx[2 * n + 1] = self.index(j.ci);
                self.prefetch(idx[2 * n].0);
                self.prefetch(idx[2 * n + 1].0);
            }
            for (n, j) in chunk.iter_mut().enumerate() {
                j.ln = self.value(idx[2 * n].0, idx[2 * n].1);
                j.li = self.value(idx[2 * n + 1].0, idx[2 * n + 1].1);
            }
        }
    }

    /// Find one optimal solution.
    pub fn solve(&self, cube: &CubieCube) -> (Vec<u8>, Stats) {
        let c = Cube::from_cubie(cube);
        if c.is_solved() {
            return (vec![], Stats::default());
        }
        let ci = c.inverse();
        let ln = self.lower_bound(c);
        let li = self.lower_bound(ci);
        let mut ctx = Ctx {
            moves: [0; 32],
            nmoves: 0,
            premoves: [0; 32],
            npremoves: 0,
            stats: Stats::default(),
            heur: 0,
        };
        let mut jobs = vec![Job {
            moves: [0; 4],
            len: 0,
            c,
            ci,
            stat: 0,
            ln,
            li,
        }];
        let mut target = ln.max(li).max(1);
        loop {
            let jl = self.job_len(target);
            while (jobs[0].len as usize) < jl {
                jobs = Self::extend_jobs(&jobs);
                self.job_bounds(&mut jobs);
            }
            if jl > 0 {
                // stable sort keeps the canonical order among equal keys
                jobs.sort_by_key(|j| std::cmp::Reverse(j.stat * job_weight(j)));
            }
            for j in jobs.iter_mut() {
                let r = target - j.len;
                let (jn, ji) = (j.ln, j.li);
                ctx.heur = 0;
                if jn > r || ji > r || (j.len > 0 && j.c.is_solved()) {
                    j.stat = 0;
                    continue;
                }
                let mut mm_n = if j.len == 0 {
                    ALL_MOVES
                } else {
                    ALLOWED_AFTER[(j.moves[j.len as usize - 1] / 3) as usize]
                };
                let mut mm_i = ALL_MOVES;
                if ji == r {
                    mm_n &= NO_HALF;
                }
                if jn == r {
                    mm_i &= NO_HALF;
                }
                ctx.moves[..j.len as usize].copy_from_slice(&j.moves[..j.len as usize]);
                ctx.nmoves = j.len as usize;
                ctx.npremoves = 0;
                let found = self.dfs(&mut ctx, j.c, j.ci, jn, ji, mm_n, mm_i, r) == FOUND;
                j.stat = ctx.heur;
                if found {
                    let mut sol: Vec<u8> = ctx.moves[..ctx.nmoves].to_vec();
                    for k in (0..ctx.npremoves).rev() {
                        sol.push(inverse_move(ctx.premoves[k]));
                    }
                    return (sol, ctx.stats);
                }
            }
            target += 1;
        }
    }

    /// `r` = number of moves remaining (> 0). Bounds `ln`, `li` are <= r.
    ///
    /// Returns `FOUND`, or a lower bound on the distance of this node learned
    /// from its children (0 if nothing was learned). Every child value is a
    /// lower bound on the child's true distance, and the true distance
    /// changes by at most one per move, so (a) a child value `v` bounds this
    /// node by `v - 1` and (b) bounds same-face siblings (one move apart) by
    /// `v - 1` as well.
    #[allow(clippy::too_many_arguments)]
    fn dfs(
        &self,
        ctx: &mut Ctx,
        c: Cube,
        ci: Cube,
        ln: u8,
        li: u8,
        mm_n: u32,
        mm_i: u32,
        r: u8,
    ) -> u8 {
        if r == 1 {
            let mut mm = mm_n & mm_i;
            while mm != 0 {
                let m = mm.trailing_zeros() as usize;
                mm &= mm - 1;
                ctx.stats.nodes += 1;
                if c.apply_move(m).is_solved() {
                    ctx.moves[ctx.nmoves] = m as u8;
                    ctx.nmoves += 1;
                    return FOUND;
                }
            }
            return 0;
        }
        let r1 = r - 1;
        let normal = mm_n.count_ones() <= mm_i.count_ones();
        // On the chosen side, (a, b) = (cube moved, cube premoved).
        let (a, b, mm, la_same) = if normal {
            (c, ci, mm_n, li)
        } else {
            (ci, c, mm_i, ln)
        };
        // Best known lower bound per face over that face's children.
        let mut fmax = [0u8; 6];
        let mut kids = [Kid {
            c: a,
            i: b,
            idx: 0,
            m: 0,
            pn: 0,
            pi: 0,
            live: false,
        }; 18];
        let mut nk = 0;
        let mut mmv = mm;
        while mmv != 0 {
            let m = mmv.trailing_zeros() as usize;
            mmv &= mmv - 1;
            // A half turn on this side does not change the other side's coset.
            let pi = if m % 3 == 1 { la_same } else { UNKNOWN };
            if pi != UNKNOWN && pi > r1 {
                fmax[m / 3] = fmax[m / 3].max(pi);
                continue;
            }
            ctx.stats.nodes += 1;
            kids[nk] = Kid {
                c: a.apply_move(m),
                i: b.premove_inv(m),
                idx: 0,
                m: m as u8,
                pn: 0,
                pi,
                live: true,
            };
            nk += 1;
        }
        let maxval = self.base + 3;
        if r1 <= maxval {
            // Stage 1: inverse-side index + prefetch.
            for k in kids[..nk].iter_mut() {
                if k.pi == UNKNOWN {
                    let (idx, cd) = self.index(k.i);
                    if cd > r1 {
                        k.live = false;
                        let f = (k.m / 3) as usize;
                        fmax[f] = fmax[f].max(cd);
                        continue;
                    }
                    k.idx = idx;
                    k.pi = cd; // temporarily the corner bound
                    self.prefetch(idx);
                    k.pn = UNKNOWN; // marks "pi pending"
                }
            }
            // Stage 2: inverse-side value, normal-side index + prefetch.
            for k in kids[..nk].iter_mut() {
                if !k.live {
                    continue;
                }
                let f = (k.m / 3) as usize;
                if k.pn == UNKNOWN {
                    ctx.stats.lookups += 1;
                    k.pi = self.value(k.idx, k.pi);
                    if k.pi > r1 {
                        k.live = false;
                        fmax[f] = fmax[f].max(k.pi);
                        continue;
                    }
                }
                let (idx, cd) = self.index(k.c);
                if cd > r1 {
                    k.live = false;
                    fmax[f] = fmax[f].max(cd);
                    continue;
                }
                k.idx = idx;
                k.pn = cd;
                self.prefetch(idx);
            }
            // Stage 3: normal-side value.
            for k in kids[..nk].iter_mut() {
                if !k.live {
                    continue;
                }
                ctx.stats.lookups += 1;
                k.pn = self.value(k.idx, k.pn);
                ctx.heur += (4096u64 >> k.pi.min(63)) + (4096u64 >> k.pn.min(63));
                let f = (k.m / 3) as usize;
                fmax[f] = fmax[f].max(k.pn).max(k.pi);
                if k.pn > r1 {
                    k.live = false;
                }
            }
        } else {
            for k in kids[..nk].iter_mut() {
                k.pn = 0;
                if k.pi == UNKNOWN {
                    k.pi = 0;
                }
            }
        }
        // Lower bound for this node from all children.
        let mut node_lb = fmax.iter().copied().max().unwrap_or(0).saturating_sub(1);
        if node_lb > r {
            return node_lb;
        }
        for k in kids[..nk].iter() {
            let face = (k.m / 3) as usize;
            // Pruned directly, or by a same-face sibling that is >= 2 over.
            if !k.live || fmax[face] > r1 + 1 {
                continue;
            }
            // Solved before the target depth: a shorter solution exists.
            if k.c.is_solved() {
                continue;
            }
            let (mut nmm_n, mut nmm_i, nc, nci, nln, nli);
            if normal {
                nmm_n = ALLOWED_AFTER[face];
                nmm_i = mm_i;
                nc = k.c;
                nci = k.i;
                nln = k.pn;
                nli = k.pi;
                ctx.moves[ctx.nmoves] = k.m;
                ctx.nmoves += 1;
            } else {
                nmm_n = mm_n;
                nmm_i = ALLOWED_AFTER[face];
                nc = k.i;
                nci = k.c;
                nln = k.pi;
                nli = k.pn;
                ctx.premoves[ctx.npremoves] = k.m;
                ctx.npremoves += 1;
            }
            if nli == r1 {
                nmm_n &= NO_HALF;
            }
            if nln == r1 {
                nmm_i &= NO_HALF;
            }
            let res = self.dfs(ctx, nc, nci, nln, nli, nmm_n, nmm_i, r1);
            if res == FOUND {
                return FOUND;
            }
            if normal {
                ctx.nmoves -= 1;
            } else {
                ctx.npremoves -= 1;
            }
            // Bound learned for the child: prunes its same-face siblings and
            // possibly this whole node.
            fmax[face] = fmax[face].max(res);
            node_lb = node_lb.max(res.saturating_sub(1));
            if node_lb > r {
                return node_lb;
            }
        }
        node_lb
    }
}
