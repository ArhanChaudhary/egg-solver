//! cubeopt as a WebAssembly module: a plain C ABI of a few functions, which
//! `cubeopt.mjs` wraps for JavaScript (Node and browsers).
//!
//! Strings go in as (pointer, length) of UTF-8 in buffers from
//! `cubeopt_alloc`. Results come out as text in an output buffer
//! (`cubeopt_output`, `cubeopt_output_len`): JSON after a call that returns 0,
//! an error message after one that returns -1. A table file is copied by the
//! caller straight into a buffer from `cubeopt_table_buffer`, then checked
//! and installed by `cubeopt_table_load`.
//!
//! Built for `wasm32-wasip1` (one thread) or `wasm32-wasip1-threads` (std
//! threads through wasi-threads: the share and stream modes run on several
//! threads as natively). See README.md, "WebAssembly".

use cubeopt::cubie::{format_moves, parse_moves, CubieCube, Rng};
use cubeopt::lanes::share::Pool;
use cubeopt::solver::{Solver, Stats};
use cubeopt::tables::{default_base, h48_bytes, Cocsep, H48Table, Memory};
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Mutex, MutexGuard, PoisonError};

#[cfg(wasm_threads)]
mod threads;

/// Whether this build has threads (`wasm32-wasip1-threads`, see build.rs).
const THREADS: bool = cfg!(wasm_threads);

/// The WASI reactor initializer, which a Rust cdylib does not get from
/// wasi-libc (its `crt1-reactor`): the host calls it once, on the main
/// thread, before any other export (`cubeopt.mjs` does). In the threads
/// build it first sets up the main thread's pthread record. Without that,
/// wasi-libc's thread list grew from a zeroed record and came apart: an
/// exiting thread could find itself alone in the list and call `exit(0)`
/// instead of finishing its exit, so a `join` on it waited forever (the
/// intermittent hang of `wasm/test.mjs`, at ~1 in 3 runs under load). It
/// then installs the thread pool (`threads.rs`).
#[unsafe(no_mangle)]
pub extern "C" fn _initialize() {
    static DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    unsafe extern "C" {
        #[cfg(wasm_threads)]
        fn __wasi_init_tp();
        fn __wasm_call_ctors();
    }
    if DONE.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    // SAFETY: wasi-libc's and the linker's initializers, run once, first.
    unsafe {
        #[cfg(wasm_threads)]
        __wasi_init_tp();
        __wasm_call_ctors();
    }
    #[cfg(wasm_threads)]
    let _ = cubeopt::parallel::set_runner(threads::run);
}

/// The module's state between calls.
struct State {
    corners: Cocsep,
    table: Option<H48Table>,
    /// A table file being copied in by the caller (`cubeopt_table_buffer`).
    incoming: Option<Memory>,
    /// The last call's result (JSON) or error message.
    output: String,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// The state, created on first use (the cocsep table takes ~0.2 s).
fn state() -> MutexGuard<'static, Option<State>> {
    let mut guard = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    if guard.is_none() {
        *guard = Some(State {
            corners: Cocsep::generate(),
            table: None,
            incoming: None,
            output: String::new(),
        });
    }
    guard
}

/// Run `f` on the state; its `Ok` text or `Err` message becomes the output.
/// Returns 0 or -1.
fn with_state(f: impl FnOnce(&mut State) -> Result<String, String>) -> i32 {
    let mut guard = state();
    let state = guard.as_mut().expect("created by `state`");
    let (output, status) = match f(state) {
        Ok(text) => (text, 0),
        Err(message) => (message, -1),
    };
    state.output = output;
    status
}

/// A string argument.
///
/// # Safety
/// `ptr` points to `len` readable bytes (or `len` is 0).
unsafe fn text_argument<'a>(ptr: *const u8, len: usize) -> Result<&'a str, String> {
    if len == 0 {
        return Ok("");
    }
    // SAFETY: as the caller promises.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    std::str::from_utf8(bytes).map_err(|_| "the text is not UTF-8".to_string())
}

/// A cube from a scramble ("R U2 F' ...") or a facelet string (54 stickers
/// of six symbols; see `CubieCube::from_facelets`). A scramble of 54 moves
/// without suffixes of all six faces would read as facelets.
fn parse_cube(text: &str) -> Result<CubieCube, String> {
    let stickers: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut symbols = stickers.clone();
    symbols.sort_unstable();
    symbols.dedup();
    let is_facelets = stickers.len() == 54
        && symbols.len() == 6
        && !symbols.iter().any(|c| matches!(c, '1' | '2' | '3' | '\''));
    if is_facelets {
        return CubieCube::from_facelets(text);
    }
    parse_moves(text)
        .map(|moves| CubieCube::SOLVED.apply_moves(&moves))
        .ok_or_else(|| format!("neither a scramble nor 54 facelets: {text:?}"))
}

/// A solved cube as JSON.
fn result_json(index: usize, solution: &[u8], stats: Stats, seconds: f64) -> String {
    format!(
        r#"{{"index":{index},"moves":"{}","length":{},"nodes":{},"lookups":{},"seconds":{seconds:.6}}}"#,
        format_moves(solution),
        solution.len(),
        stats.nodes,
        stats.lookups
    )
}

/// Threads to use: as asked (0: all the host offers), 1 without threads.
fn threads_for(asked: u32) -> usize {
    if !THREADS {
        return 1;
    }
    match asked {
        0 => cubeopt::thread_count(),
        n => n as usize,
    }
}

/// Allocate `len` bytes for the caller (strings passed in); 0 if out of
/// memory. Free with `cubeopt_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cubeopt_alloc(len: usize) -> *mut u8 {
    let Ok(layout) = std::alloc::Layout::from_size_align(len.max(1), 1) else {
        return std::ptr::null_mut();
    };
    // SAFETY: a non-zero size.
    unsafe { std::alloc::alloc(layout) }
}

/// Free a buffer from `cubeopt_alloc`.
///
/// # Safety
/// `ptr` and `len` are those of a `cubeopt_alloc` buffer not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cubeopt_free(ptr: *mut u8, len: usize) {
    if let Ok(layout) = std::alloc::Layout::from_size_align(len.max(1), 1) {
        // SAFETY: as the caller promises.
        unsafe { std::alloc::dealloc(ptr, layout) };
    }
}

/// The output of the last call: JSON or an error message (UTF-8).
#[unsafe(no_mangle)]
pub extern "C" fn cubeopt_output() -> *const u8 {
    state()
        .as_ref()
        .map_or(std::ptr::null(), |s| s.output.as_ptr())
}

/// The length in bytes of `cubeopt_output`.
#[unsafe(no_mangle)]
pub extern "C" fn cubeopt_output_len() -> usize {
    state().as_ref().map_or(0, |s| s.output.len())
}

/// 1 in the threads build, else 0.
#[unsafe(no_mangle)]
pub extern "C" fn cubeopt_has_threads() -> u32 {
    u32::from(THREADS)
}

/// A buffer of `len` bytes for a whole table file (data and trailer), which
/// the caller fills before `cubeopt_table_load`. Drops the current table
/// first (wasm32 has room for one). Returns 0 if out of memory.
///
/// # Panics
/// Never (the state exists once `state` returns).
#[unsafe(no_mangle)]
pub extern "C" fn cubeopt_table_buffer(len: usize) -> *mut u8 {
    let mut guard = state();
    let state = guard.as_mut().expect("created by `state`");
    state.table = None;
    state.incoming = None;
    state.incoming = Memory::try_anonymous(len);
    state
        .incoming
        .as_ref()
        .map_or(std::ptr::null_mut(), Memory::ptr)
}

/// Install the table file copied into the `cubeopt_table_buffer`: checks
/// its trailer and size, and if `verify` is not 0 its checksum. Output: the
/// table's `{"k":..,"base":..,"bytes":..}`.
#[unsafe(no_mangle)]
pub extern "C" fn cubeopt_table_load(verify: u32) -> i32 {
    with_state(|state| {
        let memory = state
            .incoming
            .take()
            .ok_or("no table buffer (cubeopt_table_buffer)")?;
        let table = H48Table::from_file_memory(memory, verify != 0, threads_for(0))
            .map_err(|e| e.to_string())?;
        let info = table_json(&table);
        state.table = Some(table);
        Ok(info)
    })
}

/// `{"k":..,"base":..,"bytes":..}` of a table.
fn table_json(table: &H48Table) -> String {
    format!(
        r#"{{"k":{},"base":{},"bytes":{}}}"#,
        table.meta.k,
        table.meta.base,
        h48_bytes(table.meta.k)
    )
}

/// Generate table hk (k = 0..6) in memory (`base` 0: the default for `k`)
/// on `threads` threads (0: all), replacing the current table. Output: as
/// `cubeopt_table_load`.
#[unsafe(no_mangle)]
pub extern "C" fn cubeopt_generate(k: u32, base: u32, threads: u32) -> i32 {
    with_state(|state| {
        // (h7 fits in wasm32's 4 GiB, but not with the generator's own
        // memory: make it with the native `gen` and load the file.)
        if k > 6 {
            return Err(format!(
                "h{k} is too big to generate here (h0..h6): use the native gen and load its file"
            ));
        }
        let base = match base {
            0 => default_base(k),
            b => u8::try_from(b)
                .ok()
                .filter(|&b| (1..=12).contains(&b))
                .ok_or(format!("base {b}: 1..12"))?,
        };
        state.table = None;
        state.incoming = None;
        // (A table that does not fit would abort in the allocator: check
        // that it can be allocated first.)
        let bytes = usize::try_from(h48_bytes(k)).map_err(|_| format!("h{k} is too big"))?;
        drop(Memory::try_anonymous(bytes).ok_or(format!("no room for h{k} ({bytes} bytes)"))?);
        let table = cubeopt::generate_table(&state.corners, k, base, threads_for(threads), false);
        let info = table_json(&table);
        state.table = Some(table);
        Ok(info)
    })
}

/// Solve one cube (a scramble or facelets, see `parse_cube`) with all lanes
/// of all threads on it (the share mode with one cube in progress: the
/// lowest latency). `lanes` per thread (0: 16, see `lanes_for`), `threads`
/// (0: all). Output: `{"index":0,"moves":..,"length":..,"nodes":..,
/// "lookups":..,"seconds":..}`.
///
/// # Safety
/// `text` points to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cubeopt_solve(
    text: *const u8,
    len: usize,
    lanes: u32,
    threads: u32,
) -> i32 {
    // SAFETY: as the caller promises.
    let text = unsafe { text_argument(text, len) };
    with_state(|state| {
        let cube = parse_cube(text?)?;
        let table = state.table.as_ref().ok_or("no table loaded")?;
        let solver = Solver::new(&state.corners, table);
        let lanes = lanes_for(lanes);
        let threads = threads_for(threads);
        let get = |i: usize| (i == 0).then_some(cube);
        let result = Mutex::new(None);
        let done = |i: usize, solution: Vec<u8>, stats: Stats, seconds: f64| {
            *result.lock().unwrap_or_else(PoisonError::into_inner) =
                Some(result_json(i, &solution, stats, seconds));
        };
        // (`share::solve_all`, on `on_threads`: the pool's threads in the
        // threads build, this thread alone in the other.)
        let pool = Pool::new(&get, 1, threads);
        let stop = AtomicBool::new(false);
        cubeopt::parallel::on_threads(threads, |_| {
            pool.run(&solver, lanes, &stop, |i, solution, stats, seconds| {
                done(i, solution, stats, seconds);
            });
        });
        let result = result.into_inner().unwrap_or_else(PoisonError::into_inner);
        result.ok_or_else(|| "not solved".to_string())
    })
}

/// Solve many cubes, one per line, for throughput: the stream mode (each
/// lane solves its own cube) on `threads` threads of `lanes` lanes. Output:
/// a JSON array of the results (as `cubeopt_solve`) in input order.
///
/// # Safety
/// `text` points to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cubeopt_solve_batch(
    text: *const u8,
    len: usize,
    lanes: u32,
    threads: u32,
) -> i32 {
    // SAFETY: as the caller promises.
    let text = unsafe { text_argument(text, len) };
    with_state(|state| {
        let cubes: Vec<CubieCube> = text?
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(parse_cube)
            .collect::<Result<_, _>>()?;
        let table = state.table.as_ref().ok_or("no table loaded")?;
        let solver = Solver::new(&state.corners, table);
        let lanes = lanes_for(lanes);
        let get = |i: usize| cubes.get(i).copied();
        let results = Mutex::new(vec![String::new(); cubes.len()]);
        let done = |i: usize, solution: Vec<u8>, stats: Stats, seconds: f64| {
            results.lock().unwrap_or_else(PoisonError::into_inner)[i] =
                result_json(i, &solution, stats, seconds);
        };
        stream(
            &solver,
            &get,
            lanes,
            threads_for(threads),
            &AtomicBool::new(false),
            &done,
        );
        let results = results.into_inner().unwrap_or_else(PoisonError::into_inner);
        Ok(format!("[{}]", results.join(",")))
    })
}

/// Stream throughput, as the native `bench <table> stream <warmup> <window>
/// <threads> <lanes> <seed>`: an endless stream of random cubes (cube i
/// from its own seed, the same cubes as `bench`), solves counted during
/// `window` seconds after `warmup`. Output: `{"solves_per_second":..,
/// "nodes_per_cube":..,"lookups_per_cube":..,"cubes":..,"threads":..,
/// "lanes":..}`.
#[unsafe(no_mangle)]
pub extern "C" fn cubeopt_bench(
    seed: u32,
    warmup: f64,
    window: f64,
    lanes: u32,
    threads: u32,
) -> i32 {
    with_state(|state| {
        let table = state.table.as_ref().ok_or("no table loaded")?;
        let solver = Solver::new(&state.corners, table);
        let lanes = lanes_for(lanes);
        let threads = threads_for(threads);
        let seed = u64::from(seed);
        let cubes = |i: usize| {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (i as u64));
            rng.next_u64();
            Some(CubieCube::random(&mut rng))
        };
        let stop = AtomicBool::new(false);
        // (solves, nodes, lookups) in the window.
        let counted = Mutex::new((0u64, 0u64, 0u64));
        let start = std::time::Instant::now();
        let done = |_: usize, _: Vec<u8>, stats: Stats, _: f64| {
            let at = start.elapsed().as_secs_f64();
            if at >= warmup + window {
                stop.store(true, std::sync::atomic::Ordering::Relaxed);
            } else if at >= warmup {
                let mut counted = counted.lock().unwrap_or_else(PoisonError::into_inner);
                counted.0 += 1;
                counted.1 += stats.nodes;
                counted.2 += stats.lookups;
            }
        };
        stream(&solver, &cubes, lanes, threads, &stop, &done);
        let (cubes_counted, nodes, lookups) =
            counted.into_inner().unwrap_or_else(PoisonError::into_inner);
        #[expect(clippy::cast_precision_loss, reason = "statistics")]
        let per_cube = |x: u64| x as f64 / cubes_counted.max(1) as f64;
        #[expect(clippy::cast_precision_loss, reason = "statistics")]
        let rate = cubes_counted as f64 / window;
        Ok(format!(
            r#"{{"solves_per_second":{rate:.4},"nodes_per_cube":{:.0},"lookups_per_cube":{:.0},"cubes":{cubes_counted},"threads":{threads},"lanes":{lanes}}}"#,
            per_cube(nodes),
            per_cube(lookups)
        ))
    })
}

/// The stream mode (`lanes::run_lanes`: each lane solves its own cube) on
/// `threads` threads, until `cubes` has no more or `stop` is set. Every
/// export runs this one compiled copy of the search: V8 (browsers, Node)
/// first runs code with its baseline compiler and gives a function its
/// optimized code from its next call on, so `cubeopt.mjs`'s warm-up, which
/// solves a few cubes, gets the optimized search for all later calls.
#[inline(never)]
fn stream(
    solver: &Solver,
    cubes: &(dyn Fn(usize) -> Option<CubieCube> + Sync),
    lanes: usize,
    threads: usize,
    stop: &AtomicBool,
    done: &(dyn Fn(usize, Vec<u8>, Stats, f64) + Sync),
) {
    let next = AtomicUsize::new(0);
    cubeopt::parallel::on_threads(threads, |_| {
        cubeopt::lanes::run_lanes(solver, cubes, &next, stop, lanes, done);
    });
}

/// Lanes per thread: as asked, else 16. Lanes pay off in wasm too (the
/// table load of one lane overlaps the work of the next ones), but already
/// fully at 8: V8, h6, 100 depth-17 cubes, 1 thread, 1/8/16/24/40/64 lanes:
/// 200.6/123.8/125.3/128.2/130.3/127.9 s. 16 lanes, with some margin,
/// take 23 MB instead of the 56 MB of 40.
fn lanes_for(asked: u32) -> usize {
    match asked {
        0 => 16,
        n => n as usize,
    }
}

/// `count` uniformly random cube states from `seed` (the same ones as the
/// native `bench <table> random:<count>:<seed>`), as facelet strings.
/// Output: a JSON array of strings.
#[unsafe(no_mangle)]
pub extern "C" fn cubeopt_random(seed: u32, count: u32) -> i32 {
    with_state(|_| {
        let mut rng = Rng(u64::from(seed));
        let mut json = String::from("[");
        for i in 0..count {
            let separator = if i == 0 { "" } else { "," };
            let _ = write!(
                json,
                r#"{separator}"{}""#,
                CubieCube::random(&mut rng).to_facelets()
            );
        }
        json.push(']');
        Ok(json)
    })
}
