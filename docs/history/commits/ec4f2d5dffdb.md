# NEGATIVE (c7): keep the solver's free side choice at full-length job roots; inverse-side jobs run whole

Commit `ec4f2d5dffdb8c459d0ce7c871a0cfaadb12ff84` · 2026-11-03T22:14:59+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/c7-free-side-inv-whole`

## Original experiment record

NEGATIVE (c7): keep the solver's free side choice at full-length job roots; inverse-side jobs run whole

Variant of c7-childsplit (on 78ce33e) that does not force the normal side at T=17: a job whose root
branched on the inverse side there (normal side tight, ln == r; ~10% of jobs, ~8% of the solutions)
keeps `JobX::INV`; its first split iteration runs it whole on the normal side as one first-part entry
keyed by its best child. FIRST 0.1, MERGE 8 steps, no face bias, select-based slot.

Nodes/cube (count.sh): 1000:2026 3089690 (-6.25%), 3000:777 3096967 (-4.70%); combined -5.09% vs
-5.68% for the forced normal side with the same FIRST/MERGE (3113996 / 3063220) and -6.98% for
c7-childsplit's final setting. Also tried: splitting inverse-side jobs on the inverse side (18
children; the 6 inverse half turns have no statistic): -4.8% (2026, exact slot); giving those
half turns the job's best key: -5.2%, the face's best key: -5.4%, best - 1 octave: -4.9% (2026).
(Forcing the normal side costs +1.26% nodes in main's search, all of it at T=17.)
1T at this stage (c5, paired perf windows, unpaired with main): +5.0% throughput; superseded.


---
--- a/src/lanes.rs
+++ b/src/lanes.rs
@@ -31,6 +31,17 @@

 const MAXD: usize = 26;

+/// Job length at which jobs stop being extended (`Solver::job_len`).
+const MAX_JOB_LEN: usize = 4;
+
+/// The first part of a split iteration runs about this fraction of the
+/// root children: the ones with the best statistics.
+const FIRST: f64 = 0.1;
+
+/// A first-part run takes the job's candidates whose `ck` is at most this
+/// much below its child's (fewer runs).
+const MERGE: u8 = 8;
+
 /// (128 bytes: two cache lines.)
 #[derive(Clone, Copy)]
 #[repr(align(64))]
@@ -126,6 +137,103 @@
     std::mem::swap(jobs, out);
 }

+/// Per-job state of the stream mode's child ordering (parallel to
+/// `Lane::jobs` once the jobs have their full length).
+#[derive(Clone, Copy)]
+#[repr(C, align(32))]
+struct JobX {
+    /// Quantized statistic (`quant`) of each root child's subtree in the
+    /// job's last run of that child; 0 if unknown. The children are the
+    /// moves of the side the root branches on (`INV`).
+    ck: [u8; 18],
+    /// `Node::tw` of the root (on that side) after its last run (0xFF:
+    /// unknown). Position-only, so valid in any iteration: a run of an X'
+    /// child without its X sibling takes the value from here.
+    tw: [u8; 6],
+    /// Bits 0..18: children run in iteration `done >> 24`; `INV`.
+    done: u32,
+    _pad: u32,
+}
+
+/// `JobX::done`: the root branches on the inverse side (the side its first
+/// full-length run chose; kept so that `ck` stays meaningful).
+const INV: u32 = 1 << 23;
+
+const JOBX0: JobX = JobX {
+    ck: [0; 18],
+    tw: [0xFF; 6],
+    done: 0,
+    _pad: 0,
+};
+
+/// A run entry (`Lane::runs`): key (bits 24..32), job (bits 5..24) and a
+/// child move (bits 0..5), or `RUN_ALL` for the job's children not run yet.
+const RUN_ALL: u32 = 31;
+
+/// Monotone 8-bit code of a non-negative statistic: 4 steps per octave
+/// (exponent and top two mantissa bits of the f32), 0 below 1.
+#[inline(always)]
+fn quant(x: f32) -> u8 {
+    ((x.to_bits() >> 21) as i32 - 127 * 4).clamp(0, 255) as u8
+}
+
+/// `quant` of 18 statistics.
+#[inline(always)]
+fn quant18(x: &[f32; 18]) -> [u8; 18] {
+    let mut r = [0u8; 18];
+    unsafe {
+        use std::arch::x86_64::*;
+        let v = _mm512_loadu_ps(x.as_ptr());
+        let q = _mm512_sub_epi32(
+            _mm512_srli_epi32(_mm512_castps_si512(v), 21),
+            _mm512_set1_epi32(127 * 4),
+        );
+        let q = _mm512_max_epi32(q, _mm512_setzero_si512());
+        _mm_storeu_si128(r.as_mut_ptr() as *mut __m128i, _mm512_cvtusepi32_epi8(q));
+    }
+    r[16] = quant(x[16]);
+    r[17] = quant(x[17]);
+    r
+}
+
+/// The sum of 18 statistics (a tree of additions: off the latency of a
+/// sequential sum).
+#[inline(always)]
+fn sum18(x: &[f32; 18]) -> f32 {
+    unsafe {
+        use std::arch::x86_64::*;
+        _mm512_reduce_add_ps(_mm512_loadu_ps(x.as_ptr())) + (x[16] + x[17])
+    }
+}
+
+/// Bit mask of the moves m with `ck[m] > theta`.
+#[inline(always)]
+fn ck_above(ck: &[u8; 18], theta: u8) -> u32 {
+    unsafe {
+        use std::arch::x86_64::*;
+        let v = _mm_loadu_si128(ck.as_ptr() as *const __m128i);
+        let lo = _mm_cmpgt_epu8_mask(v, _mm_set1_epi8(theta as i8)) as u32;
+        lo | ((ck[16] > theta) as u32) << 16 | ((ck[17] > theta) as u32) << 17
+    }
+}
+
+/// The largest `ck[m]` over the moves m in `mask` (0 if none).
+#[inline(always)]
+fn max18(ck: &[u8; 18], mask: u32) -> u8 {
+    let lo = unsafe {
+        use std::arch::x86_64::*;
+        let v = _mm_maskz_mov_epi8(mask as u16, _mm_loadu_si128(ck.as_ptr() as *const __m128i));
+        let v = _mm_max_epu8(v, _mm_srli_si128(v, 8));
+        let v = _mm_max_epu8(v, _mm_srli_si128(v, 4));
+        let v = _mm_max_epu8(v, _mm_srli_si128(v, 2));
+        let v = _mm_max_epu8(v, _mm_srli_si128(v, 1));
+        _mm_cvtsi128_si32(v) as u8
+    };
+    let b16 = if mask >> 16 & 1 != 0 { ck[16] } else { 0 };
+    let b17 = if mask >> 17 & 1 != 0 { ck[17] } else { 0 };
+    lo.max(b16).max(b17)
+}
+
 /// Job statistic contribution of a lookup value h: 8^(16-h). Steeper than
 /// cube48opt's 2^(12-h); measured ~2% fewer nodes (saturating beyond 8^).
 const fn mk_heur_terms() -> [f32; 64] {
@@ -174,6 +282,29 @@
     WaitOther = 1,
 }

+/// Stream mode's run state of a lane (see `Lane::start_iteration`).
+struct RunState {
+    /// Child ordering state of the jobs (stream mode, full-length jobs).
+    jx: Vec<JobX>,
+    /// Whether runs record `jx` (the jobs have their full length).
+    full: bool,
+    /// Per-child statistics of the jobs one move short of full length
+    /// (their children are the next iteration's jobs).
+    pstat: Vec<[f32; 18]>,
+    /// Part of the iteration: 0 whole jobs (`job_pos`), 1 the first part of
+    /// a split iteration (`runs`), 2 its rest (`runs`).
+    part: u8,
+    /// Run entries of the current part, in order, and scratch.
+    runs: Vec<u32>,
+    runs_tmp: Vec<u32>,
+    run_pos: usize,
+    /// First part: candidate children have `ck > theta`.
+    theta: u8,
+    /// The current run: job index and children.
+    cur_job: usize,
+    cur_mask: u32,
+}
+
 pub struct Lane {
     pub cube_index: usize,
     /// When the current cube was started.
@@ -184,6 +315,8 @@
     /// Scratch buffer for job extension and sorting.
     jobs_tmp: Vec<Job>,
     job_pos: usize,
+    /// Stream mode's run state (cold: kept out of the lane's hot lines).
+    rs: Box<RunState>,
     target: u8,
     stack: [Node; MAXD],
     sp: usize,
@@ -197,7 +330,11 @@
     k_idx: u64,
     k_cd: u8,
     k_po: u8,
+    /// Statistic of the last finished job (`Lane::shared` lanes).
     heur: f32,
+    /// Statistic per root child of the current run, by `stack[1].m`
+    /// (lookups at the root itself count for the last child pushed).
+    cheur: [f32; 18],
     pub stats: Stats,
     pub solution: Vec<u8>,
     /// Lane of `share`: `advance` returns `Step::JobDone` at the end of a
@@ -237,6 +374,18 @@
             jobs: Vec::new(),
             jobs_tmp: Vec::new(),
             job_pos: 0,
+            rs: Box::new(RunState {
+                jx: Vec::new(),
+                full: false,
+                pstat: Vec::new(),
+                part: 0,
+                runs: Vec::new(),
+                runs_tmp: Vec::new(),
+                run_pos: 0,
+                theta: 0,
+                cur_job: 0,
+                cur_mask: 0,
+            }),
             target: 0,
             stack: [n; MAXD],
             sp: 0,
@@ -249,6 +398,7 @@
             k_cd: 0,
             k_po: 0,
             heur: 0.0,
+            cheur: [0.0; 18],
             stats: Stats::default(),
             solution: Vec::new(),
             shared: false,
@@ -287,6 +437,7 @@
             stat: 0.0,
         });
         self.target = ln.max(li).max(1);
+        self.rs.full = false;
         self.start_iteration(s);
         false
     }
@@ -310,7 +461,8 @@
                 out[slot / 2].li = v;
             }
         };
-        for j in &self.jobs {
+        let use_p = self.rs.pstat.len() == self.jobs.len();
+        for (pi, j) in self.jobs.iter().enumerate() {
             let (c, ci) = self.job_cubes(j);
             let mut mm = if j.len == 0 {
                 ALL_MOVES
@@ -323,6 +475,9 @@
                 let mut n = *j;
                 n.moves[n.len as usize] = m as u8;
                 n.len += 1;
+                if use_p {
+                    n.stat = self.rs.pstat[pi][m];
+                }
                 let pos = out.len();
                 out.push(n);
                 let (idx, cd) = s.index(c.apply_move(m));
@@ -355,23 +510,345 @@
         self.jobs_tmp = std::mem::replace(&mut self.jobs, out);
     }

+    /// Start the iteration with target `self.target` (or a later one, if
+    /// it has no runs).
+    ///
+    /// An iteration on the jobs of the previous one (they have their full
+    /// length; with h11, T >= 18: the likely last iterations, and every one
+    /// but the last is exhaustive, where the order does not matter) is
+    /// split: first the root children with the best statistics from the
+    /// previous iteration, about `FIRST` of all, by decreasing statistic
+    /// across jobs (`build_first`), then each job's remaining children, by
+    /// decreasing best statistic (`build_rest`). Other iterations run whole
+    /// jobs by decreasing statistic, as solver does.
     fn start_iteration(&mut self, s: &Solver) {
         loop {
             // The stack depth stays below the target (relied on by the
             // unchecked indexing in `advance`); 20 moves always suffice.
             assert!((self.target as usize) < MAXD);
             let jl = s.job_len(self.target);
+            let mut ext = false;
             while (self.jobs[0].len as usize) < jl {
                 self.extend_jobs(s);
+                ext = true;
             }
-            if jl > 0 {
-                sort_jobs(&mut self.jobs, &mut self.jobs_tmp);
+            let was_full = self.rs.full;
+            self.rs.full = jl == MAX_JOB_LEN && !self.shared;
+            self.rs.pstat.clear();
+            if false {
+                self.rs.pstat.resize(self.jobs.len(), [0.0; 18]);
             }
-            self.job_pos = 0;
-            if self.setup_job() {
+            if self.rs.full && was_full && !ext {
+                self.build_first();
+                self.rs.part = 1;
+            } else {
+                if jl > 0 {
+                    sort_jobs(&mut self.jobs, &mut self.jobs_tmp);
+                }
+                if self.rs.full {
+                    // (Every job's entry is written in this iteration: by
+                    // its run, or as `JOBX0` if it is skipped.)
+                    self.rs.jx.resize(self.jobs.len(), JOBX0);
+                }
+                self.rs.part = 0;
+                self.job_pos = 0;
+            }
+            if self.setup_run() {
                 return;
             }
+            debug_assert!(self.iteration_covered());
+            self.target += 1;
+        }
+    }
+
+    /// The threshold on `ck` that selects about `FIRST` of all root
+    /// children (estimated from a sample of the jobs).
+    fn choose_theta(&self) -> u8 {
+        let n = self.rs.jx.len();
+        let step = if n >= 4096 { 8 } else { 1 };
+        let mut hist = [0u32; 256];
+        let mut total = 0u32;
+        for x in self.rs.jx.iter().step_by(step) {
+            for &q in &x.ck {
+                hist[q as usize] += 1;
+            }
+            total += 18;
+        }
+        total -= hist[0];
+        let limit = (total as f64 * FIRST) as u32;
+        let mut acc = 0;
+        for v in (1..256).rev() {
+            acc += hist[v];
+            if acc > limit {
+                return v as u8;
+            }
+        }
+        0
+    }
+
+    /// The first part of a split iteration: one entry per root child with
+    /// `ck > theta`, by decreasing `ck` (ties in job and move order).
+    fn build_first(&mut self) {
+        self.rs.theta = self.choose_theta();
+        let theta = self.rs.theta;
+        let n = self.rs.jx.len();
+        self.rs.runs_tmp.clear();
+        self.rs.runs_tmp.reserve(n + 64);
+        let mut cnt = [0u32; 256];
+        let mut k = 0;
+        for ji in 0..n {
+            if self.rs.jx[ji].done & INV != 0 {
+                let x = &mut self.rs.jx[ji];
+                x.done &= !INV;
+                x.tw = [0xFF; 6];
+                let key = max18(&x.ck, ALL_MOVES);
+                if key > theta {
+                    if self.rs.runs_tmp.capacity() < k + 32 {
+                        unsafe { self.rs.runs_tmp.set_len(k) };
+                        self.rs.runs_tmp.reserve(n + 64);
+                    }
+                    unsafe {
+                        self.rs
+                            .runs_tmp
+                            .as_mut_ptr()
+                            .add(k)
+                            .write((key as u32) << 24 | (ji as u32) << 5 | RUN_ALL);
+                    }
+                    k += 1;
+                }
+                // (kept for the rest's order: `ck` of a child of the side)
+                let m0 = moves_after(&self.jobs[ji]).trailing_zeros() as usize;
+                let x = &mut self.rs.jx[ji];
+                x.ck = [0; 18];
+                x.ck[m0] = key;
+                continue;
+            }
+            // SAFETY: ji < n.
+            let x = unsafe { self.rs.jx.get_unchecked(ji) };
+            let cand = ck_above(&x.ck, theta);
+            if self.rs.runs_tmp.capacity() < k + 32 {
+                // SAFETY: the first k entries were written.
+                unsafe { self.rs.runs_tmp.set_len(k) };
+                self.rs.runs_tmp.reserve(n + 64);
+            }
+            // SAFETY: k + 18 <= capacity; entries are written before
+            // `set_len` covers them.
+            unsafe {
+                use std::arch::x86_64::*;
+                let out = self.rs.runs_tmp.as_mut_ptr().add(k);
+                let ck = _mm_loadu_si128(x.ck.as_ptr() as *const __m128i);
+                let e = _mm512_or_si512(
+                    _mm512_slli_epi32(_mm512_cvtepu8_epi32(ck), 24),
+                    _mm512_add_epi32(
+                        _mm512_set1_epi32((ji as i32) << 5),
+                        _mm512_setr_epi32(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15),
+                    ),
+                );
+                let c16 = cand as u16;
+                _mm512_mask_compressstoreu_epi32(out as *mut i32, c16, e);
+                let mut k2 = k + c16.count_ones() as usize;
+                for m in 16..18 {
+                    let e = (x.ck[m] as u32) << 24 | (ji as u32) << 5 | m as u32;
+                    out.add(k2 - k).write(e);
+                    k2 += (cand >> m & 1) as usize;
+                }
+                k = k2;
+            }
+        }
+        // SAFETY: the first k entries were written.
+        unsafe { self.rs.runs_tmp.set_len(k) };
+        for &e in &self.rs.runs_tmp {
+            cnt[(e >> 24) as usize] += 1;
+        }
+        sort_runs(&mut self.rs.runs, &self.rs.runs_tmp, &mut cnt);
+        self.rs.run_pos = 0;
+    }
+
+    /// The rest of a split iteration: one entry per job with children not
+    /// run yet, by decreasing best `ck` of those.
+    fn build_rest(&mut self) {
+        let n = self.rs.jx.len();
+        let tag = (self.target as u32) << 24;
+        self.rs.runs_tmp.clear();
+        self.rs.runs_tmp.reserve(n);
+        let mut cnt = [0u32; 256];
+        for ji in 0..n {
+            let x = &self.rs.jx[ji];
+            let rest = self.side_moves(ji) & !done_now(x, tag);
+            if rest != 0 {
+                let key = max18(&x.ck, rest);
+                cnt[key as usize] += 1;
+                self.rs
+                    .runs_tmp
+                    .push((key as u32) << 24 | (ji as u32) << 5 | RUN_ALL);
+            }
+        }
+        sort_runs(&mut self.rs.runs, &self.rs.runs_tmp, &mut cnt);
+        self.rs.run_pos = 0;
+    }
+
+    /// The root children of job `ji` in a split iteration (on its side).
+    #[inline(always)]
+    fn side_moves(&self, ji: usize) -> u32 {
+        if self.rs.jx[ji].done & INV != 0 {
+            ALL_MOVES
+        } else {
+            moves_after(&self.jobs[ji])
+        }
+    }
+
+    /// Whether every child of every feasible job root was run in this
+    /// iteration (split iterations; for debug assertions at their end).
+    fn iteration_covered(&self) -> bool {
+        if self.rs.part == 0 {
+            return true;
+        }
+        let tag = (self.target as u32) << 24;
+        (0..self.rs.jx.len()).all(|ji| self.side_moves(ji) & !done_now(&self.rs.jx[ji], tag) == 0)
+    }
+
+    /// Record the run just finished: the job's statistic, and in split
+    /// iterations' setting its children's statistics, `tw` and `done`.
+    #[inline(always)]
+    fn finish_run(&mut self) {
+        let ji = self.rs.cur_job;
+        let stat: f32 = sum18(&self.cheur);
+        self.jobs[ji].stat = stat;
+        if !self.rs.pstat.is_empty() {
+            self.rs.pstat[ji] = self.cheur;
+        }
+        if self.rs.full {
+            let root = &self.stack[0];
+            let mask = self.rs.cur_mask;
+            let tag = (self.target as u32) << 24;
+            let q = quant18(&self.cheur);
+            let inv = if root.normal { 0 } else { INV };
+            let x = if self.rs.part == 0 {
+                // A whole run: the entry is all new (not read: it is not
+                // in the cache).
+                JobX {
+                    ck: q,
+                    tw: root.tw,
+                    done: tag | inv | ALL_MOVES,
+                    _pad: 0,
+                }
+            } else {
+                let x = &self.rs.jx[ji];
+                let mut ck = x.ck;
+                for m in 0..18 {
+                    if mask >> m & 1 != 0 {
+                        ck[m] = q[m];
+                    }
+                }
+                JobX {
+                    ck,
+                    tw: root.tw,
+                    done: tag | inv | done_now(x, tag) | mask,
+                    _pad: 0,
+                }
+            };
+            self.rs.jx[ji] = x;
+        }
+    }
+
+    /// The run finished: record it and set up the next one (of the next
+    /// iteration if this one is exhausted). Out of line and cold, to keep
+    /// the search loop's registers.
+    #[cold]
+    #[inline(never)]
+    fn next_run(&mut self, s: &Solver) {
+        self.finish_run();
+        if self.rs.part == 0 {
+            self.job_pos += 1;
+        }
+        if !self.setup_run() {
+            debug_assert!(self.iteration_covered());
             self.target += 1;
+            self.start_iteration(s);
+        }
+    }
+
+    /// Set up the root node of the next run of this iteration. Returns
+    /// false if the iteration is exhausted.
+    fn setup_run(&mut self) -> bool {
+        if self.rs.part == 0 {
+            return self.setup_job();
+        }
+        let tag = (self.target as u32) << 24;
+        loop {
+            if self.rs.run_pos == self.rs.runs.len() {
+                if self.rs.part == 2 {
+                    return false;
+                }
+                self.build_rest();
+                self.rs.part = 2;
+                continue;
+            }
+            let e = self.rs.runs[self.rs.run_pos];
+            self.rs.run_pos += 1;
+            // The job vectors are too big for L2 and 20 lanes interleave
+            // their streams: fetch the next entry's job early.
+            unsafe {
+                use std::arch::x86_64::*;
+                let next = *self
+                    .rs
+                    .runs
+                    .get_unchecked(self.rs.run_pos.min(self.rs.runs.len() - 1));
+                let nj = (next >> 5 & 0x7_FFFF) as usize;
+                _mm_prefetch(self.rs.jx.as_ptr().add(nj) as *const i8, _MM_HINT_T0);
+                _mm_prefetch(self.jobs.as_ptr().add(nj) as *const i8, _MM_HINT_T0);
+            }
+            let ji = (e >> 5 & 0x7_FFFF) as usize;
+            let m = e & 31;
+            let x = self.rs.jx[ji];
+            let done = done_now(&x, tag);
+            let side = self.side_moves(ji);
+            let mut mask;
+            if m == RUN_ALL {
+                mask = side & !done;
+            } else {
+                if done >> m & 1 != 0 {
+                    continue; // run with an earlier entry's group
+                }
+                // the child, with the job's candidates at most `MERGE` below
+                // it (those above were run before)
+                let lo = x.ck[m as usize]
+                    .saturating_sub(MERGE + 1)
+                    .max(self.rs.theta);
+                mask = (ck_above(&x.ck, lo) & !done) | 1 << m;
+            }
+            // An X' child needs its X sibling's other-side value (`tw`):
+            // from this run or recorded by an earlier one (always the case:
+            // every full-length job ran whole before its first split
+            // iteration). Else the X sibling runs too.
+            let unknown = tw_unknown(&x.tw);
+            mask |= (mask & unknown) >> 2 & !done;
+            let j = self.jobs[ji];
+            let r = self.target - j.len;
+            debug_assert!(j.ln < r && j.li < r);
+            let (c, ci) = self.job_cubes(&j);
+            let inv = x.done & INV != 0;
+            let mm_n = moves_after(&j);
+            self.cheur = [0.0; 18];
+            self.rs.cur_job = ji;
+            self.rs.cur_mask = mask;
+            self.sp = 0;
+            self.stack[0] = Node {
+                a: Cube::select(inv, ci, c),
+                b: Cube::select(inv, c, ci),
+                lo: if inv { j.ln } else { j.li },
+                mm_o: if inv { mm_n } else { ALL_MOVES },
+                todo: side & mask,
+                skip: 0,
+                r,
+                normal: !inv,
+                fmax: [0; 6],
+                lb: 0,
+                m: 0,
+                m_normal: true,
+                tw: x.tw,
+            };
+            return true;
         }
     }

@@ -386,15 +863,24 @@
             unsafe {
                 use std::arch::x86_64::*;
                 _mm_prefetch(self.jobs.as_ptr().add(ahead) as *const i8, _MM_HINT_T0);
+                if self.rs.full {
+                    // (written at the end of the run)
+                    _mm_prefetch(self.rs.jx.as_ptr().add(ahead) as *const i8, _MM_HINT_ET0);
+                }
             }
             let r = self.target - j.len;
-            self.heur = 0.0;
+            self.cheur = [0.0; 18];
             let (c, ci) = self.job_cubes(&j);
             if j.ln > r || j.li > r || (j.len > 0 && c.is_solved()) {
                 self.jobs[self.job_pos].stat = 0.0;
+                if self.rs.full {
+                    self.rs.jx[self.job_pos] = JOBX0;
+                }
                 self.job_pos += 1;
                 continue;
             }
+            self.rs.cur_job = self.job_pos;
+            self.rs.cur_mask = ALL_MOVES;
             let mut mm_n = if j.len == 0 {
                 ALL_MOVES
             } else {
@@ -450,15 +936,12 @@
                 if sp == 0 {
                     // job finished
                     if self.shared {
+                        self.heur = sum18(&self.cheur);
+                        self.cheur = [0.0; 18];
                         self.stats.nodes += nodes;
                         return Step::JobDone;
                     }
-                    self.jobs[self.job_pos].stat = self.heur;
-                    self.job_pos += 1;
-                    if !self.setup_job() {
-                        self.target += 1;
-                        self.start_iteration(s);
-                    }
+                    self.next_run(s);
                     sp = self.sp;
                     let n = &self.stack[sp];
                     (todo, lb, r) = (n.todo, n.lb, n.r);
@@ -626,7 +1109,13 @@
         self.stats.lookups += 1;
         let po = self.k_po;
         debug_assert!(po < 16 && pm < 16);
-        self.heur += HEUR_PAIR[(po as usize & 15) << 4 | (pm as usize & 15)];
+        // SAFETY: moves are below 18.
+        unsafe {
+            let c =
+                std::hint::select_unpredictable(self.sp == 0, self.k_m, self.stack[1].m) as usize;
+            std::hint::assert_unchecked(c < 18);
+            self.cheur[c] += HEUR_PAIR[(po as usize & 15) << 4 | (pm as usize & 15)];
+        }
         let r1 = self.stack[self.sp].r - 1;
         let face = self.k_face as usize;
         self.note_value(face, pm.max(po));
@@ -722,7 +1211,7 @@
     }

     fn reconstruct(&self) -> Vec<u8> {
-        let j = &self.jobs[self.job_pos];
+        let j = &self.jobs[self.rs.cur_job];
         let mut normal: Vec<u8> = j.moves[..j.len as usize].to_vec();
         let mut pre: Vec<u8> = Vec::new();
         for n in &self.stack[1..=self.sp] {
@@ -778,6 +1267,53 @@
     }
 }

+/// The moves allowed at a job's root by its last move.
+#[inline(always)]
+fn moves_after(j: &Job) -> u32 {
+    if j.len == 0 {
+        ALL_MOVES
+    } else {
+        ALLOWED_AFTER[(j.moves[j.len as usize - 1] / 3) as usize]
+    }
+}
+
+/// `JobX::done`'s children run in the iteration of `tag` (`target << 24`).
+#[inline(always)]
+fn done_now(x: &JobX, tag: u32) -> u32 {
+    if x.done & 0xFF00_0000 == tag {
+        x.done & ALL_MOVES
+    } else {
+        0
+    }
+}
+
+/// The X' moves of the faces whose `tw` is unknown.
+#[inline(always)]
+fn tw_unknown(tw: &[u8; 6]) -> u32 {
+    let mut m = 0;
+    for f in 0..6 {
+        m |= ((tw[f] == 0xFF) as u32) << (3 * f + 2);
+    }
+    m
+}
+
+/// Counting sort of run entries by decreasing key (stable); `cnt` holds
+/// the number of entries per key.
+fn sort_runs(runs: &mut Vec<u32>, tmp: &[u32], cnt: &mut [u32; 256]) {
+    let mut sum = 0;
+    for c in cnt.iter_mut().rev() {
+        (*c, sum) = (sum, sum + *c);
+    }
+    runs.clear();
+    runs.resize(tmp.len(), 0);
+    for &e in tmp {
+        let k = (e >> 24) as usize;
+        // SAFETY: cnt[k] < runs.len() by construction.
+        unsafe { *runs.get_unchecked_mut(cnt[k] as usize) = e };
+        cnt[k] += 1;
+    }
+}
+
 /// Solve cubes `cubes(i)` for i taken from the shared counter `next`, with
 /// `nlanes` interleaved lanes on this thread. Calls `done(index, solution,
 /// stats, seconds)` for every solved cube. Stops when `cubes` returns None
@@ -871,6 +1407,105 @@
 mod tests {
     use super::*;

+    /// Drive the stream mode's runs (`start_iteration`'s parts, `setup_run`,
+    /// `finish_run`) over random full-length jobs with simulated outcomes
+    /// for a whole iteration and then split ones, and check the coverage:
+    /// every child of every job root runs exactly once per iteration, split
+    /// runs branch on the side the job's statistics refer to, and an X'
+    /// child runs with its X sibling or with X's other-side value known.
+    #[test]
+    fn split_runs_cover_each_child_once() {
+        let mut rng = crate::cubie::Rng(11);
+        for trial in 0..40 {
+            let mut l = Lane::new();
+            let n = [1usize, 7, 300, 2000][trial % 4];
+            l.jobs = (0..n)
+                .map(|_| {
+                    let mut moves = [0u8; 4];
+                    let mut mm = ALL_MOVES;
+                    for mv in moves.iter_mut() {
+                        let ms: Vec<u32> = (0..18).filter(|&m| mm >> m & 1 != 0).collect();
+                        let m = ms[rng.next_u64() as usize % ms.len()];
+                        *mv = m as u8;
+                        mm = ALLOWED_AFTER[(m / 3) as usize];
+                    }
+                    // bounds below the first target's remaining depth (13),
+                    // some tight (half turns masked, inverse roots)
+                    let b = |r: u64| if r % 3 == 0 { 13 } else { (r % 13) as u8 };
+                    Job {
+                        moves,
+                        len: 4,
+                        ln: b(rng.next_u64()),
+                        li: b(rng.next_u64()),
+                        stat: 0.0,
+                    }
+                })
+                .collect();
+            l.target = 17;
+            l.rs.full = true;
+            for iter in 0..3 {
+                if iter == 0 {
+                    l.rs.jx.clear();
+                    l.rs.jx.resize(n, JOBX0);
+                    l.rs.part = 0;
+                    l.job_pos = 0;
+                } else {
+                    l.build_first();
+                    l.rs.part = 1;
+                }
+                let r = l.target - 4;
+                let mut ran = vec![0u32; n];
+                let mut runs = vec![0usize; n];
+                while l.setup_run() {
+                    let ji = l.rs.cur_job;
+                    let root = l.stack[0];
+                    assert_eq!(root.r, r);
+                    assert_eq!(root.todo & ran[ji], 0, "child run twice");
+                    if iter > 0 {
+                        assert_eq!(root.normal, l.rs.jx[ji].done & INV == 0, "side");
+                        for f in 0..6 {
+                            let (x, xp) = (1 << (3 * f), 4 << (3 * f));
+                            if root.todo & xp != 0 && root.todo & x == 0 {
+                                assert_ne!(root.tw[f], 0xFF, "X' without X or its value");
+                            }
+                        }
+                    }
+                    ran[ji] |= root.todo;
+                    runs[ji] += 1;
+                    // simulated outcome: statistics and X values
+                    for m in 0..18 {
+                        if root.todo >> m & 1 != 0 {
+                            let y = rng.next_u64();
+                            l.cheur[m] = if y % 4 == 0 {
+                                0.0
+                            } else {
+                                HEUR_TERMS[(y % 40) as usize] * (y >> 8) as u8 as f32
+                            };
+                            if m % 3 == 0 {
+                                l.stack[0].tw[m / 3] = (y >> 16) as u8 % 14;
+                            }
+                        }
+                    }
+                    l.finish_run();
+                    if l.rs.part == 0 {
+                        l.job_pos += 1;
+                    }
+                }
+                assert!(l.iteration_covered());
+                for ji in 0..n {
+                    let j = l.jobs[ji];
+                    if iter == 0 {
+                        assert_eq!(runs[ji], 1);
+                    } else {
+                        assert_eq!(ran[ji], l.side_moves(ji), "children not run");
+                        let _ = j;
+                    }
+                }
+                l.target += 1;
+            }
+        }
+    }
+
     #[test]
     fn sort_jobs_matches_stable_sort() {
         let mut rng = crate::cubie::Rng(7);
