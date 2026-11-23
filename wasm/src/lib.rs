//! egg-solver as a WebAssembly module: a plain C ABI of a few functions, which
//! `egg-solver.mjs` wraps for JavaScript (Bun and browsers).
//!
//! Strings go in as (pointer, length) of UTF-8 in buffers from
//! `egg_solver_alloc`. Results come out as text in an output buffer
//! (`egg_solver_output`, `egg_solver_output_len`): JSON after a call that returns 0,
//! an error message after one that returns -1. A table file is copied by the
//! caller straight into a buffer from `egg_solver_table_buffer`, then checked
//! and installed by `egg_solver_table_load`.
//!
//! Built for `wasm32-wasip1` (one thread) or `wasm32-wasip1-threads` (std
//! threads through wasi-threads: the share and stream modes run on several
//! threads as natively). See README.md, "WebAssembly".
//!
//! The solving and generating exports take a `metric` argument: 0 the
//! table's metric (HTM for HTM tables, the default and the behaviour before
//! QTM; for `egg_solver_generate`: HTM), 1 HTM, 2 QTM (the quarter-turn
//! metric, on an HTM or a QTM table). Callers that pass fewer arguments get
//! 0 (wasm fills missing arguments with zeros).

// No intentional panics in shipped code (user request; asserts, which no
// lint covers, are checked by tests/no_runtime_panics.rs).
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]

use egg_solver::cubie::{format_moves, parse_moves, CubieCube, Rng};
use egg_solver::lanes::share::Pool;
use egg_solver::metric::Metric;
use egg_solver::solutions::{Options, Solutions, Stopped, DEFAULT_MAX_SOLUTIONS};
use egg_solver::solver::{Solver, Stats};
use egg_solver::tables::{default_base_metric, h48_bytes, Cocsep, H48Table, Memory};
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Mutex, MutexGuard, PoisonError};

#[cfg(wasm_threads)]
mod threads;

/// Whether this build has threads (`wasm32-wasip1-threads`, see build.rs).
const THREADS: bool = cfg!(wasm_threads);

/// The WASI reactor initializer, which a Rust cdylib does not get from
/// wasi-libc (its `crt1-reactor`): the host calls it once, on the main
/// thread, before any other export (`egg-solver.mjs` does). In the threads
/// build it first sets up the main thread's pthread record. Without that,
/// wasi-libc's thread list grew from a zeroed record and came apart: an
/// exiting thread could find itself alone in the list and call `exit(0)`
/// instead of finishing its exit, so a `join` on it waited forever (the
/// intermittent hang of `wasm/test.mjs`, at ~1 in 3 runs under load). It
/// then installs the thread pool (`threads.rs`).
///
/// Rust 1.100 and later link wasi-libc's `crt1-reactor` into the module
/// themselves (`cfg(libc_reactor)`, see build.rs). Its `_initialize` runs
/// the constructors, and their wasi-libc (wasi-sdk 34) sets up a thread's
/// pthread record on its first `pthread_self` instead of through
/// `__wasi_init_tp`, which it no longer has. With those toolchains this
/// function is not defined: the host calls wasi-libc's `_initialize`, then
/// `egg_solver_after_initialize`.
#[cfg(not(libc_reactor))]
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
    let _ = egg_solver::parallel::set_runner(threads::run);
}

/// With wasi-libc's own `_initialize` (`cfg(libc_reactor)`): what comes
/// after it, the thread pool (`threads.rs`). The host calls it once, right
/// after `_initialize`.
#[cfg(libc_reactor)]
#[unsafe(no_mangle)]
pub extern "C" fn egg_solver_after_initialize() {
    #[cfg(wasm_threads)]
    let _ = egg_solver::parallel::set_runner(threads::run);
}

/// The module's state between calls.
struct State {
    /// The cocsep tables of HTM and QTM, each made on first use (`corners`).
    corners: Option<Cocsep>,
    corners_qtm: Option<Cocsep>,
    table: Option<H48Table>,
    /// A table file being copied in by the caller (`egg_solver_table_buffer`).
    incoming: Option<Memory>,
    /// The last call's result (JSON) or error message.
    output: String,
    /// The trailer of the table's file image (`egg_solver_table_export`).
    exported_trailer: Vec<u8>,
}

static STATE: Mutex<State> = Mutex::new(State {
    corners: None,
    corners_qtm: None,
    table: None,
    incoming: None,
    output: String::new(),
    exported_trailer: Vec::new(),
});

/// The state.
fn state() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The cocsep table of the state (`State::corners`), made now if it is not
/// yet (it takes ~0.2 s).
fn corners(corners: &mut Option<Cocsep>) -> Result<&Cocsep, String> {
    match corners {
        Some(corners) => Ok(corners),
        None => Ok(corners.insert(Cocsep::generate().map_err(|e| e.to_string())?)),
    }
}

/// The metric of argument `metric` (see the module docs) for a search on
/// `table` (none: generating).
fn metric_of(metric: u32, table: Option<&H48Table>) -> Result<Metric, String> {
    match metric {
        0 => Ok(table.map_or(Metric::Htm, |table| table.meta().metric)),
        1 => Ok(Metric::Htm),
        2 => Ok(Metric::Qtm),
        _ => Err(format!(
            "metric {metric}: 0 (the table's), 1 (htm) or 2 (qtm)"
        )),
    }
}

/// The solver of `metric` on the state's table, with the corner table of
/// that metric (made on first use).
fn solver_of(state: &mut State, metric: Metric) -> Result<Solver<'_>, String> {
    let State {
        corners,
        corners_qtm,
        table,
        ..
    } = state;
    let table = table.as_ref().ok_or("no table loaded")?;
    let corners = if metric.is_qtm() {
        match corners_qtm {
            Some(corners) => &*corners,
            None => {
                corners_qtm.insert(Cocsep::generate_metric(Metric::Qtm).map_err(|e| e.to_string())?)
            }
        }
    } else {
        self::corners(corners)?
    };
    Solver::with_metric(corners, table, metric).map_err(|e| e.to_string())
}

/// Run `f` on the state; its `Ok` text or `Err` message becomes the output.
/// Returns 0 or -1.
fn with_state(f: impl FnOnce(&mut State) -> Result<String, String>) -> i32 {
    let mut state = state();
    let (output, status) = match f(&mut state) {
        Ok(text) => (text, 0),
        Err(message) => (message, -1),
    };
    state.output = output;
    status
}

/// A string argument.
///
/// # Safety
/// If `len` is nonzero, `ptr` addresses one live allocation containing at
/// least `len <= isize::MAX` bytes, immutable for the returned borrow. Zero
/// length permits a null pointer.
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
        return CubieCube::from_facelets(text).map_err(|e| e.to_string());
    }
    parse_moves(text)
        .and_then(|moves| CubieCube::SOLVED.apply_moves(&moves))
        .ok_or_else(|| format!("neither a scramble nor 54 facelets: {text:?}"))
}

/// A solved cube as JSON (`{"index":..,"error":..}` if it has no
/// solution); its length in `metric`.
fn result_json(
    index: usize,
    solution: Option<&[u8]>,
    stats: Stats,
    seconds: f64,
    metric: Metric,
) -> String {
    let Some(solution) = solution else {
        return format!(r#"{{"index":{index},"error":"{}"}}"#, no_solution(metric));
    };
    format!(
        r#"{{"index":{index},"moves":"{}","length":{},"metric":"{metric}","nodes":{},"lookups":{},"seconds":{seconds:.6}}}"#,
        format_moves(solution),
        metric.length(solution),
        stats.nodes,
        stats.lookups
    )
}

/// Why a cube has no solution (see `solver::MAX_TARGET`).
fn no_solution(metric: Metric) -> String {
    format!(
        "no solution within {} moves ({metric}): is the table damaged?",
        metric.max_target()
    )
}

/// Threads to use: as asked (0: all the host offers), 1 without threads.
fn threads_for(asked: u32) -> usize {
    if !THREADS {
        return 1;
    }
    match asked {
        0 => egg_solver::thread_count(),
        n => n as usize,
    }
}

/// Allocate `len` bytes for the caller (strings passed in); 0 if out of
/// memory. Free with `egg_solver_free`.
#[unsafe(no_mangle)]
pub extern "C" fn egg_solver_alloc(len: usize) -> *mut u8 {
    let Ok(layout) = std::alloc::Layout::from_size_align(len.max(1), 1) else {
        return std::ptr::null_mut();
    };
    // SAFETY: a non-zero size.
    unsafe { std::alloc::alloc(layout) }
}

/// Free a buffer from `egg_solver_alloc`.
///
/// # Safety
/// `ptr` and `len` are those of a `egg_solver_alloc` buffer not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn egg_solver_free(ptr: *mut u8, len: usize) {
    if let Ok(layout) = std::alloc::Layout::from_size_align(len.max(1), 1) {
        // SAFETY: as the caller promises.
        unsafe { std::alloc::dealloc(ptr, layout) };
    }
}

/// The output of the last call: JSON or an error message (UTF-8).
#[unsafe(no_mangle)]
pub extern "C" fn egg_solver_output() -> *const u8 {
    state().output.as_ptr()
}

/// The length in bytes of `egg_solver_output`.
#[unsafe(no_mangle)]
pub extern "C" fn egg_solver_output_len() -> usize {
    state().output.len()
}

/// 1 in the threads build, else 0.
#[unsafe(no_mangle)]
pub extern "C" fn egg_solver_has_threads() -> u32 {
    u32::from(THREADS)
}

/// A buffer of `len` bytes for a whole table file (data and trailer), which
/// the caller fills before `egg_solver_table_load`. Drops the current table
/// first (wasm32 has room for one). Returns 0 if out of memory.
#[unsafe(no_mangle)]
pub extern "C" fn egg_solver_table_buffer(len: usize) -> *mut u8 {
    let mut state = state();
    state.table = None;
    state.incoming = None;
    state.incoming = Memory::try_anonymous(len);
    state
        .incoming
        .as_ref()
        .map_or(std::ptr::null_mut(), Memory::ptr)
}

/// Install the table file copied into the `egg_solver_table_buffer`: checks
/// its trailer and size, and if `verify` is not 0 its checksum. Output: the
/// table's `{"k":..,"base":..,"bytes":..,"metric":..}`.
#[unsafe(no_mangle)]
pub extern "C" fn egg_solver_table_load(verify: u32) -> i32 {
    with_state(|state| {
        let memory = state
            .incoming
            .take()
            .ok_or("no table buffer (egg_solver_table_buffer)")?;
        let table = H48Table::from_file_memory(memory, verify != 0, threads_for(0))
            .map_err(|e| e.to_string())?;
        let info = table_json(&table);
        state.table = Some(table);
        Ok(info)
    })
}

/// The current table as a table file, for the caller to keep (a browser page
/// stores it in `IndexedDB`) and load later through `egg_solver_table_buffer`
/// and `egg_solver_table_load`: the table's data where it lies, then a
/// trailer with its checksum (computed on `threads` threads, 0: all). Output:
/// `{"data":..,"data_bytes":..,"trailer":..,"trailer_bytes":..}`, the
/// addresses and sizes of the two parts of the file, valid until the next
/// call that changes the table or exports it again.
#[unsafe(no_mangle)]
pub extern "C" fn egg_solver_table_export(threads: u32) -> i32 {
    with_state(|state| {
        let table = state.table.as_ref().ok_or("no table loaded")?;
        let trailer = table
            .trailer(threads_for(threads))
            .map_err(|e| e.to_string())?
            .to_bytes();
        let data = table.data().map_err(|e| e.to_string())?;
        let (data_ptr, data_bytes) = (data.as_ptr() as usize, data.len());
        state.exported_trailer = trailer;
        Ok(format!(
            r#"{{"data":{data_ptr},"data_bytes":{data_bytes},"trailer":{},"trailer_bytes":{}}}"#,
            state.exported_trailer.as_ptr() as usize,
            state.exported_trailer.len()
        ))
    })
}

/// `{"k":..,"base":..,"bytes":..,"metric":..}` of a table.
fn table_json(table: &H48Table) -> String {
    format!(
        r#"{{"k":{},"base":{},"bytes":{},"metric":"{}"}}"#,
        table.meta().k,
        table.meta().base,
        h48_bytes(table.meta().k),
        table.meta().metric
    )
}

/// Generate table hk (k = 0..6) of `metric` (module docs: 0 or 1 HTM, 2
/// QTM) in memory (`base` 0: the default for `k` and the metric) on
/// `threads` threads (0: all), replacing the current table. Output: as
/// `egg_solver_table_load`.
#[unsafe(no_mangle)]
pub extern "C" fn egg_solver_generate(k: u32, base: u32, threads: u32, metric: u32) -> i32 {
    with_state(|state| {
        let metric = metric_of(metric, None)?;
        // h7 exceeds the isize::MAX limit for one table allocation on wasm32.
        if k > 6 {
            return Err(format!(
                "h{k} exceeds the wasm32 table limit (h0..h6): use the native solver"
            ));
        }
        let base = match base {
            0 => default_base_metric(k, metric),
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
        let corners = corners(&mut state.corners)?;
        let table = egg_solver::generate_table_metric(
            corners,
            k,
            base,
            metric,
            threads_for(threads),
            false,
        )
        .map_err(|e| e.to_string())?;
        let info = table_json(&table);
        state.table = Some(table);
        Ok(info)
    })
}

/// Solve one cube (a scramble or facelets, see `parse_cube`) with all lanes
/// of all threads on it (the share mode with one cube in progress: the
/// lowest latency). `lanes` per thread (0: 16, see `lanes_for`), `threads`
/// (0: all), `metric` (module docs). Output: `{"index":0,"moves":..,
/// "length":..,"metric":..,"nodes":..,"lookups":..,"seconds":..}`, the
/// length in that metric.
///
/// # Safety
/// If `len` is nonzero, `text` points into one live allocation containing
/// `len <= isize::MAX` bytes, immutable until this call returns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn egg_solver_solve(
    text: *const u8,
    len: usize,
    lanes: u32,
    threads: u32,
    metric: u32,
) -> i32 {
    // SAFETY: as the caller promises.
    let text = unsafe { text_argument(text, len) };
    with_state(|state| {
        let cube = parse_cube(text?)?;
        let metric = metric_of(metric, state.table.as_ref())?;
        let solver = solver_of(state, metric)?;
        let lanes = lanes_for(lanes);
        let threads = threads_for(threads);
        let get = |i: usize| (i == 0).then_some(cube);
        let result = Mutex::new(None);
        let done = |i: usize, solution: Option<Vec<u8>>, stats: Stats, seconds: f64| {
            *result.lock().unwrap_or_else(PoisonError::into_inner) = Some(
                solution.map(|solution| result_json(i, Some(&solution), stats, seconds, metric)),
            );
        };
        // (`share::solve_all`, on `on_threads`: the pool's threads in the
        // threads build, this thread alone in the other.)
        let pool = Pool::new(&get, 1, threads);
        let stop = AtomicBool::new(false);
        egg_solver::parallel::on_threads(threads, |_| {
            pool.run(&solver, lanes, &stop, |i, solution, stats, seconds| {
                done(i, solution, stats, seconds);
            });
        })
        .map_err(|e| e.to_string())?;
        let result = result.into_inner().unwrap_or_else(PoisonError::into_inner);
        result
            .ok_or("not solved")?
            .ok_or_else(|| no_solution(metric))
    })
}

/// The cancel word of `egg_solver_solve_all`: a byte of the module's
/// memory that stops a running list when another thread sets it to 1.
static CANCEL: AtomicBool = AtomicBool::new(false);

/// The address of the cancel word in the module's memory: a thread that
/// shares the memory (the threads build) stops a running
/// `egg_solver_solve_all` by storing 1 there with an atomic byte store
/// (`cancel` in `egg-solver.mjs`). The list is then returned with what was
/// found, `"stopped":"cancelled"`. Every `egg_solver_solve_all` clears the
/// word when it starts.
#[unsafe(no_mangle)]
pub extern "C" fn egg_solver_cancel_address() -> *const u8 {
    CANCEL.as_ptr().cast_const().cast::<u8>()
}

/// List every distinct solution of one cube (a scramble or facelets, see
/// `parse_cube`) up to a length, instead of one optimal solution
/// (`egg_solver::solutions`), with all lanes of all threads on it. The
/// length limit: `plus` is 1 + K for "at most K moves more than an optimal
/// solution" (0: no such limit), `max_length` is N for "at most N moves"
/// (0: no such limit; at most God's number of the metric); with both the
/// smaller, with neither the optimal solutions. `max_solutions`: at most so
/// many are listed (0: 100000); if the cube has more, the search stops and
/// the list is marked truncated. `time_limit`: seconds after which the
/// search stops (0: never); it also stops when the cancel word is set
/// (`egg_solver_cancel_address`). `lanes`, `threads` and `metric` as for
/// `egg_solver_solve`. Output: `{"index":0,"count":..,"optimal":..,
/// "max_length":..,"truncated":false,"stopped":null,"complete_length":..,
/// "metric":..,"by_length":[{"length":..,"count":..}],"solutions":
/// [{"moves":..,"length":..}],"nodes":..,"lookups":..,"seconds":..}`: the
/// solutions by length (in that metric), each in canonical form; `optimal`
/// is null if there is no solution of at most `max_length` moves (or none
/// was found before the search was stopped); `stopped` is `"time"` or
/// `"cancelled"` if the search was stopped. A truncated or stopped list
/// holds all solutions of at most `complete_length` moves and maybe some
/// longer ones.
///
/// # Safety
/// If `len` is nonzero, `text` points into one live allocation containing
/// `len <= isize::MAX` bytes, immutable until this call returns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn egg_solver_solve_all(
    text: *const u8,
    len: usize,
    lanes: u32,
    threads: u32,
    metric: u32,
    plus: u32,
    max_length: u32,
    max_solutions: u32,
    time_limit: f64,
) -> i32 {
    // SAFETY: as the caller promises.
    let text = unsafe { text_argument(text, len) };
    CANCEL.store(false, std::sync::atomic::Ordering::SeqCst);
    with_state(|state| {
        let cube = parse_cube(text?)?;
        let metric = metric_of(metric, state.table.as_ref())?;
        let longest = metric.max_target();
        if max_length > u32::from(longest) {
            return Err(format!(
                "max_length {max_length}: at most {longest} ({metric})"
            ));
        }
        let limit = |value: u32| u8::try_from(value).ok();
        let plus = plus.checked_sub(1).map(|k| limit(k).unwrap_or(u8::MAX));
        let max_length = limit(max_length).filter(|&n| n > 0);
        // (0, or not a time: no limit.)
        let time_limit = Some(time_limit)
            .filter(|&seconds| seconds > 0.0)
            .and_then(|seconds| std::time::Duration::try_from_secs_f64(seconds).ok());
        let options = Options {
            // (Neither limit: the optimal solutions.)
            plus: plus.or(max_length.is_none().then_some(0)),
            max_length,
            max_solutions: match max_solutions {
                0 => DEFAULT_MAX_SOLUTIONS,
                n => n as usize,
            },
            time_limit,
            cancel: Some(&CANCEL),
        };
        let solver = solver_of(state, metric)?;
        let get = |i: usize| (i == 0).then_some(cube);
        let result = Mutex::new(None);
        let done = |i: usize, list: Solutions, stats: Stats, seconds: f64| {
            *result.lock().unwrap_or_else(PoisonError::into_inner) =
                Some(solutions_json(i, &list, stats, seconds, metric));
        };
        egg_solver::lanes::share::all_solutions(
            &solver,
            &get,
            threads_for(threads),
            lanes_for(lanes),
            1,
            &options,
            &done,
        )
        .map_err(|e| e.to_string())?;
        let result = result.into_inner().unwrap_or_else(PoisonError::into_inner);
        // (Cancelled before the cube's search began: no list was reported.
        // The recursive solver, which looks at the flag first, gives the
        // empty one, marked as cancelled.)
        result.unwrap_or_else(|| {
            let (list, stats) = solver.all_solutions(&cube, &options);
            solutions_json(0, &list, stats, 0.0, metric)
        })
    })
}

/// A cube's solutions up to a length as JSON (see `egg_solver_solve_all`),
/// or the error of a cube without a solution within God's number.
fn solutions_json(
    index: usize,
    list: &Solutions,
    stats: Stats,
    seconds: f64,
    metric: Metric,
) -> Result<String, String> {
    if list.optimal.is_none() && list.max_length >= metric.max_target() && list.stopped.is_none() {
        return Err(no_solution(metric));
    }
    let stopped = list.stopped.map_or_else(
        || "null".to_string(),
        |stopped| format!(r#""{}""#, Stopped::name(stopped)),
    );
    let optimal = list
        .optimal
        .map_or_else(|| "null".to_string(), |optimal| optimal.to_string());
    let mut json = format!(
        r#"{{"index":{index},"count":{},"optimal":{optimal},"max_length":{},"truncated":{},"stopped":{stopped},"complete_length":{},"metric":"{metric}","by_length":["#,
        list.solutions.len(),
        list.max_length,
        list.truncated,
        list.complete_length
    );
    for (i, (length, count)) in list.counts(metric).into_iter().enumerate() {
        let separator = if i == 0 { "" } else { "," };
        let _ = write!(json, r#"{separator}{{"length":{length},"count":{count}}}"#);
    }
    json.push_str(r#"],"solutions":["#);
    for (i, solution) in list.solutions.iter().enumerate() {
        let separator = if i == 0 { "" } else { "," };
        let _ = write!(
            json,
            r#"{separator}{{"moves":"{}","length":{}}}"#,
            format_moves(solution),
            metric.length(solution)
        );
    }
    let _ = write!(
        json,
        r#"],"nodes":{},"lookups":{},"seconds":{seconds:.6}}}"#,
        stats.nodes, stats.lookups
    );
    Ok(json)
}

/// Solve many cubes, one per line, for throughput: the stream mode (each
/// lane solves its own cube) on `threads` threads of `lanes` lanes, in
/// `metric` (module docs). Output: a JSON array of the results (as
/// `egg_solver_solve`) in input order.
///
/// # Safety
/// If `len` is nonzero, `text` points into one live allocation containing
/// `len <= isize::MAX` bytes, immutable until this call returns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn egg_solver_solve_batch(
    text: *const u8,
    len: usize,
    lanes: u32,
    threads: u32,
    metric: u32,
) -> i32 {
    // SAFETY: as the caller promises.
    let text = unsafe { text_argument(text, len) };
    with_state(|state| {
        let cubes: Vec<CubieCube> = text?
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(parse_cube)
            .collect::<Result<_, _>>()?;
        let metric = metric_of(metric, state.table.as_ref())?;
        let solver = solver_of(state, metric)?;
        let lanes = lanes_for(lanes);
        let get = |i: usize| cubes.get(i).copied();
        let results = Mutex::new(vec![String::new(); cubes.len()]);
        let done = |i: usize, solution: Option<Vec<u8>>, stats: Stats, seconds: f64| {
            // (`i` is the index of a cube: `get` has no others.)
            let mut results = results.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(slot) = results.get_mut(i) {
                *slot = result_json(i, solution.as_deref(), stats, seconds, metric);
            }
        };
        stream(
            &solver,
            &get,
            lanes,
            threads_for(threads),
            &AtomicBool::new(false),
            &done,
        )?;
        let results = results.into_inner().unwrap_or_else(PoisonError::into_inner);
        Ok(format!("[{}]", results.join(",")))
    })
}

/// Stream throughput, as the native `bench <table> stream <warmup> <window>
/// <threads> <lanes> <seed>`: an endless stream of random cubes (cube i
/// from its own seed, the same cubes as `bench`), solves counted during
/// `window` seconds after `warmup`, in `metric` (module docs). Output:
/// `{"solves_per_second":..,"nodes_per_cube":..,"lookups_per_cube":..,
/// "cubes":..,"threads":..,"lanes":..,"metric":..}`.
#[unsafe(no_mangle)]
pub extern "C" fn egg_solver_bench(
    seed: u32,
    warmup: f64,
    window: f64,
    lanes: u32,
    threads: u32,
    metric: u32,
) -> i32 {
    with_state(|state| {
        let metric = metric_of(metric, state.table.as_ref())?;
        let solver = solver_of(state, metric)?;
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
        let done = |_: usize, _: Option<Vec<u8>>, stats: Stats, _: f64| {
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
        stream(&solver, &cubes, lanes, threads, &stop, &done)?;
        let (cubes_counted, nodes, lookups) =
            counted.into_inner().unwrap_or_else(PoisonError::into_inner);
        #[expect(clippy::cast_precision_loss, reason = "statistics")]
        let per_cube = |x: u64| x as f64 / cubes_counted.max(1) as f64;
        #[expect(clippy::cast_precision_loss, reason = "statistics")]
        let rate = cubes_counted as f64 / window;
        Ok(format!(
            r#"{{"solves_per_second":{rate:.4},"nodes_per_cube":{:.0},"lookups_per_cube":{:.0},"cubes":{cubes_counted},"threads":{threads},"lanes":{lanes},"metric":"{metric}"}}"#,
            per_cube(nodes),
            per_cube(lookups)
        ))
    })
}

/// The stream mode (`lanes::run_lanes`: each lane solves its own cube) on
/// `threads` threads, until `cubes` has no more or `stop` is set. Every
/// export runs this one compiled copy of the search: V8 (browsers, Node)
/// first runs code with its baseline compiler and gives a function its
/// optimized code from its next call on, so `egg-solver.mjs`'s warm-up, which
/// solves a few cubes, gets the optimized search for all later calls.
#[inline(never)]
fn stream(
    solver: &Solver,
    cubes: &(dyn Fn(usize) -> Option<CubieCube> + Sync),
    lanes: usize,
    threads: usize,
    stop: &AtomicBool,
    done: &(dyn Fn(usize, Option<Vec<u8>>, Stats, f64) + Sync),
) -> Result<(), String> {
    let next = AtomicUsize::new(0);
    egg_solver::parallel::on_threads(threads, |_| {
        egg_solver::lanes::run_lanes(solver, cubes, &next, stop, lanes, done);
    })
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Lanes per thread: as asked, else 16. Lanes pay off in wasm too (the
/// table load of one lane overlaps the work of the next ones; 8 lanes: +62%
/// against 1), but already fully at 8: V8, h6, 100 depth-17 cubes, 1 thread,
/// 1/8/16/24/40/64 lanes: 200.6/123.8/125.3/128.2/130.3/127.9 s. 16 lanes,
/// with some margin, take 23 MB instead of the 56 MB of 40.
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
pub extern "C" fn egg_solver_random(seed: u32, count: u32) -> i32 {
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
