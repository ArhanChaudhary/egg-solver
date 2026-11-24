//! The binaries' command lines on a small table (h0, made here by `gen`):
//! every form that scripts and the README use (the forms from before clap)
//! still works and prints the same format, and the newer options (`--json`,
//! subcommands, `--help`, completions, environment variables) work. Exit
//! statuses: 0, 1 for a failed check or an error, 2 for unusable input.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Short scrambles (fast on h0), with a blank line.
const SCRAMBLES: &str = "R U R' U'\n\nF2 D L2 B' U\nR1 U2 F3 D1 L2 B1 R3\nU R2 F B R B2 R U2\n";

/// A scratch folder for this test run.
fn scratch() -> &'static Path {
    DIR.get_or_init(|| {
        remove_stale_scratch();
        let dir = std::env::temp_dir().join(format!("{SCRATCH_PREFIX}{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // (The test harness has no hook at its end: the C library's.)
        // SAFETY: `remove_scratch` is an `extern "C" fn()` that does not
        // unwind (it ignores the removal's errors) and reads a `static`.
        #[cfg(unix)]
        unsafe {
            libc::atexit(remove_scratch);
        }
        dir
    })
}

static DIR: OnceLock<PathBuf> = OnceLock::new();
const SCRATCH_PREFIX: &str = "egg-solver-cli-";

/// Removes the scratch folder (its tables take over 200 MB) when the test
/// process exits.
#[cfg(unix)]
extern "C" fn remove_scratch() {
    if let Some(dir) = DIR.get() {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Removes the scratch folders of test processes that are gone (killed, or
/// from before the folders were removed at exit: 25 of them once filled a
/// disk). On Linux only, where `/proc` tells whether a process is alive.
fn remove_stale_scratch() {
    if !cfg!(target_os = "linux") {
        return;
    }
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let pid = name
            .to_str()
            .and_then(|name| name.strip_prefix(SCRATCH_PREFIX))
            .and_then(|pid| pid.parse::<u32>().ok());
        if let Some(pid) = pid {
            if !Path::new(&format!("/proc/{pid}")).exists() {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
}

/// A file in the scratch folder.
fn scratch_file(name: &str) -> String {
    scratch().join(name).to_string_lossy().into_owned()
}

/// h0 base 8 made by `gen 0 8 <path> -t 4` (the legacy form), once.
fn h0() -> &'static str {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| {
        let path = scratch_file("h0");
        let out = run(env!("CARGO_BIN_EXE_gen"), &["0", "8", &path, "-t", "4"], "");
        let log = text(&out.stderr);
        assert_eq!(out.status.code(), Some(0), "{log}");
        assert!(log.contains("[gen h0] depth 0:"), "{log}");
        assert!(log.contains("h0 base 8 distribution ["), "{log}");
        path
    })
}

/// A file of `SCRAMBLES`.
fn scrambles_file() -> &'static str {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| {
        let path = scratch_file("scrambles.txt");
        std::fs::write(&path, SCRAMBLES).unwrap();
        path
    })
}

/// Run `binary` with `args` and `stdin`.
fn run(binary: &str, args: &[&str], stdin: &str) -> Output {
    run_env(binary, args, stdin, &[])
}

/// Shared memory keys of this test process: every run gets `shm_key(0)`, a
/// key without a segment (so that no segment a user loaded is used);
/// `solve_shared_memory` makes its segments at `shm_key(1)` and
/// `solve_shared_memory_fallbacks` at `shm_key(2)`.
fn shm_key(n: u32) -> String {
    format!("{:#010x}", 0x5800_0000 | std::process::id() << 2 | n)
}

/// Run `binary` with `args`, `stdin` and the environment variables `env`
/// (the binaries' own variables are unset otherwise, and `EGG_SOLVER_SHM_KEY`
/// is `shm_key(0)` but with `--completions`).
fn run_env(binary: &str, args: &[&str], stdin: &str, env: &[(&str, &str)]) -> Output {
    start(binary, args, stdin, env).wait_with_output().unwrap()
}

/// Start `binary` as `run_env` runs it, with `stdin` written and closed.
fn start(binary: &str, args: &[&str], stdin: &str, env: &[(&str, &str)]) -> std::process::Child {
    let mut command = Command::new(binary);
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for variable in [
        "EGG_SOLVER_LOAD",
        "EGG_SOLVER_PREFAULT",
        "EGG_SOLVER_SHM_DENY_HUGE",
        "EGG_SOLVER_SHM_KEY",
        "EGG_SOLVER_SHM_PAGES",
        "EGG_SOLVER_TABLE",
        "EGG_SOLVER_THREADS",
        "EGG_SOLVER_PREFLIGHT_LIMIT",
        "RUST_LOG",
    ] {
        command.env_remove(variable);
    }
    // (`--completions` is exclusive, and clap counts a variable's value as
    // given.)
    if !args.contains(&"--completions") {
        command.env("EGG_SOLVER_SHM_KEY", shm_key(0));
    }
    for &(modern, previous) in egg_solver::env::ALIASES {
        if modern != "EGG_SOLVER_SHM_KEY" || args.contains(&"--completions") {
            command.env_remove(modern);
        }
        command.env_remove(previous);
    }
    command.envs(env.iter().copied());
    let mut child = command.spawn().unwrap();
    // (Closed at the end of the block. A binary may exit before reading
    // all of it.)
    if let Some(mut input) = child.stdin.take() {
        let _ = std::io::Write::write_all(&mut input, stdin.as_bytes());
    }
    child
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// stdout of a run that must succeed.
fn ok(out: &Output) -> String {
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout:\n{}\nstderr:\n{}",
        text(&out.stdout),
        text(&out.stderr)
    );
    text(&out.stdout)
}

/// Whether `line` is the words of `pattern`, where `#` is any number and
/// `#s` a number of seconds (`1.5s`), and a last `*` any rest.
fn matches(line: &str, pattern: &str) -> bool {
    let mut words = line.split(' ');
    for want in pattern.split(' ') {
        if want == "*" {
            return true;
        }
        let Some(word) = words.next() else {
            return false;
        };
        let number = |w: &str| !w.is_empty() && w.parse::<f64>().is_ok();
        let fits = match want {
            "#" => number(word),
            "#s" => word.strip_suffix('s').is_some_and(number),
            _ => word == want,
        };
        if !fits {
            return false;
        }
    }
    words.next().is_none()
}

/// Check each line of `output` against its pattern (see `matches`).
fn check_lines(output: &str, patterns: &[&str]) {
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), patterns.len(), "{output}");
    for (line, pattern) in lines.iter().zip(patterns) {
        assert!(
            matches(line, pattern),
            "{line:?} is not {pattern:?}\n{output}"
        );
    }
}

/// `solve`'s output: `n` cubes solved (each solution as long as it says),
/// then the total.
fn check_solve_output(output: &str, n: usize, threads: &str, lanes: &str, concurrency: &str) {
    let rows: Vec<&str> = output.lines().collect();
    assert_eq!(rows.len(), n + 1, "{output}");
    let mut indices = Vec::new();
    for line in &rows[..n] {
        assert!(matches(line, "solved # # # # *"), "{line}");
        let words: Vec<&str> = line.split(' ').collect();
        let length: usize = words[2].parse().unwrap();
        assert_eq!(words.len() - 5, length, "{line}");
        indices.push(words[1].parse::<usize>().unwrap());
    }
    indices.sort_unstable();
    assert_eq!(indices, (0..n).collect::<Vec<_>>());
    let total = format!(
        "total {n} cubes wall # wall/cube # nodes/cube # threads {threads} lanes {lanes} concurrency {concurrency}"
    );
    assert!(matches(rows[n], &total), "{}", rows[n]);
}

#[test]
fn solve_legacy_forms() {
    let table = h0();
    let solve = env!("CARGO_BIN_EXE_solve");
    // README: `solve <table> < file` (all cores).
    let out = run(solve, &[table], SCRAMBLES);
    let threads = std::thread::available_parallelism().unwrap().to_string();
    check_solve_output(&ok(&out), 4, &threads, "24", "0");
    assert_eq!(text(&out.stderr), "", "stderr as before: nothing");
    // Scripts: `solve <table> -t T -c C [-n N] [-l L]`.
    let out = run(solve, &[table, "-t", "2", "-c", "1", "-n", "3"], SCRAMBLES);
    check_solve_output(&ok(&out), 3, "2", "24", "1");
    let out = run(solve, &[table, "-t", "2", "-l", "4", "-c", "0"], SCRAMBLES);
    check_solve_output(&ok(&out), 4, "2", "4", "0");
    // runtest.sh sets EGG_SOLVER_LOAD.
    for load in ["copy", "map", "auto", ""] {
        let env = [("EGG_SOLVER_LOAD", load)];
        let out = run_env(solve, &[table, "-t", "2", "-c", "1"], SCRAMBLES, &env);
        check_solve_output(&ok(&out), 4, "2", "24", "1");
    }
}

/// `bench tune` measures the search speed of `solve`'s two modes at each
/// thread count and lane count (also on h0, where no random cube is solved
/// within a measurement) and saves the best lane counts; `solve` then uses
/// the entry of its mode, and not a record of the earlier tune command. On
/// a copy of the table: the other tests rely on the untuned default.
#[test]
fn bench_tune_and_its_record() {
    let table = scratch_file("h0-tuned");
    std::fs::copy(h0(), &table).unwrap();
    let record = format!("{table}.tune");
    let (bench, solve) = (env!("CARGO_BIN_EXE_bench"), env!("CARGO_BIN_EXE_solve"));
    let out = run(bench, &["tune", &table, "--window", "0.1", "--json"], "");
    let lines: Vec<serde_json::Value> = ok(&out)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let of_type = |kind: &str| -> Vec<&serde_json::Value> {
        lines.iter().filter(|line| line["type"] == kind).collect()
    };
    // Thread counts: 1, half of all (if at least 4) and all.
    let all = std::thread::available_parallelism().unwrap().get();
    let mut thread_counts = vec![1];
    if all >= 4 {
        thread_counts.push(all / 2);
    }
    if all > 1 {
        thread_counts.push(all);
    }
    let (measured, best) = (of_type("tune"), of_type("best"));
    assert_eq!(measured.len(), 2 * thread_counts.len() * 8, "{lines:?}");
    assert_eq!(best.len(), 2 * thread_counts.len(), "{lines:?}");
    for line in &measured {
        assert!(line["nodes_per_second"].as_f64().unwrap() > 0.0, "{line}");
        assert!(line["mode"] == "batch" || line["mode"] == "one", "{line}");
        let threads = usize::try_from(line["threads"].as_u64().unwrap()).unwrap();
        assert!(thread_counts.contains(&threads), "{line}");
    }
    assert_eq!(of_type("saved")[0]["path"], record.as_str());
    // The record: the header, then each best lane count.
    let saved = std::fs::read_to_string(&record).unwrap();
    let mut rows = vec!["# egg-solver tune 2".to_string()];
    for line in &best {
        let mode = line["mode"].as_str().unwrap();
        rows.push(format!("{mode} {} {}", line["threads"], line["lanes"]));
    }
    assert_eq!(saved.lines().collect::<Vec<_>>(), rows);
    // `solve` uses the entry of its mode.
    let lanes_of = |mode: &str| {
        let entry = best.iter().find(|b| b["mode"] == mode && b["threads"] == 1);
        entry.unwrap()["lanes"].to_string()
    };
    for (concurrency, mode) in [("1", "one"), ("0", "batch")] {
        let args = ["-v", "--table", &table, "-t", "1", "-c", concurrency];
        let out = run(solve, &args, SCRAMBLES);
        check_solve_output(&ok(&out), 4, "1", &lanes_of(mode), concurrency);
        let log = text(&out.stderr);
        assert!(log.contains("from the table's .tune file"), "{log}");
    }
    // A record of the earlier tune command (`threads lanes` lines) is not
    // used, and the log says so.
    std::fs::write(&record, "1 48\n8 48\n").unwrap();
    let out = run(solve, &["-v", "--table", &table, "-t", "1"], SCRAMBLES);
    check_solve_output(&ok(&out), 4, "1", "24", "0");
    let log = text(&out.stderr);
    assert!(log.contains("of an earlier `bench tune`"), "{log}");
    // The text output, in the form before subcommands.
    let out = ok(&run(bench, &[&table, "tune", "0.05"], ""));
    assert!(out.contains("batch threads 1 lanes 8: "), "{out}");
    assert!(out.contains(" M nodes/s"), "{out}");
    assert!(out.contains("best for one at 1 threads: "), "{out}");
    assert!(out.ends_with(&format!("saved to {record}\n")), "{out}");
    assert!(std::fs::read_to_string(&record)
        .unwrap()
        .starts_with("# egg-solver tune 2\n"));
}

/// One explicit scramble, with an optional short warm-up.
#[test]
fn solve_fast() {
    let table = h0();
    let solve = env!("CARGO_BIN_EXE_solve");
    let fast = ["--fast", "-t", "2", "--warmup", "F2 D"];
    for args in [
        [&fast[..], &["--table", table, "R", "U", "R'", "U'"]].concat(),
        [&fast[..], &["--table", table, "R U R' U'"]].concat(),
    ] {
        let out = ok(&run(solve, &args, ""));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 6, "{out}");
        assert_eq!(lines[0], format!("table    {table}"));
        assert_eq!(lines[1], "threads  2 (all on this one cube)");
        assert_eq!(lines[2], "scramble R U R' U'");
        assert_eq!(lines[3].split(' ').count(), 5, "{out}");
        assert!(lines[3].starts_with("solution "), "{out}");
        assert_eq!(lines[4], "length   4 moves (optimal, verified)");
        // `time     <ms> ms   (<n> M nodes, <rate> M nodes/s)`
        assert!(
            lines[5].starts_with("time     ") && lines[5].contains(" ms   ("),
            "{out}"
        );
        let time: Vec<&str> = lines[5].split_whitespace().collect();
        assert_eq!(time.len(), 9, "{out}");
        let words = [time[0], time[2], time[4], time[5], time[7], time[8]];
        assert_eq!(
            words,
            ["time", "ms", "M", "nodes,", "M", "nodes/s)"],
            "{out}"
        );
        let numbers = [time[1], time[3].trim_start_matches('('), time[6]];
        assert!(numbers.iter().all(|x| x.parse::<f64>().is_ok()), "{out}");
    }
    let args = [&fast[..], &["--table", table, "--json", "R U R' U'"]].concat();
    let report: serde_json::Value =
        serde_json::from_str(ok(&run(solve, &args, "")).trim()).unwrap();
    assert_eq!(report["length"], 4);
    assert_eq!(report["threads"], 2);
    assert_eq!(report["scramble"], "R U R' U'");
    let missing = scratch_file("missing");
    assert_eq!(
        run(solve, &["--fast", "--table", &missing, "R"], "")
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        run(
            solve,
            &[&fast[..], &["--table", table, "R", "X"]].concat(),
            ""
        )
        .status
        .code(),
        Some(2)
    );
}

/// Empty input and missing table flags must fail without opening a table or
/// starting a hidden solve. The nonexistent path makes that ordering visible.
/// `solve` and `bench` read a line of 54 stickers as that cube (what the web
/// page shows for a random cube), not as 54 moves: the stickers of the cube
/// after R are solved by R', in the face letters or any six symbols. Such a
/// line that is not a cube is unusable input, and so is junk, with messages
/// that say which.
#[test]
fn sticker_strings_are_cubes() {
    let table = h0();
    let solve = env!("CARGO_BIN_EXE_solve");
    let after_r = "UUFUUFUUFRRRRRRRRRFFDFFDFFDDDBDDBDDBLLLLLLLLLUBBUBBUBB";
    let colours = "wwgwwgwwg rrrrrrrrr ggyggyggy yybyybyyb ooooooooo wbbwbbwbb";
    let input = format!("{after_r}\nR\n{colours}\n");
    let out = ok(&run(solve, &[table, "-t", "2"], &input));
    let solved: Vec<&str> = out.lines().filter(|l| l.starts_with("solved ")).collect();
    assert_eq!(solved.len(), 3, "{out}");
    for line in &solved {
        // `solved <index> <length> <nodes> <seconds> <moves>`
        let fields: Vec<&str> = line.split(' ').collect();
        assert_eq!((fields[2], &fields[5..]), ("1", &["R'"][..]), "{out}");
    }
    let out = ok(&run(
        solve,
        &["--fast", "--table", table, "-t", "2", after_r],
        "",
    ));
    assert!(out.contains("\nsolution R'\n"), "{out}");
    assert!(
        out.contains("length   1 moves (optimal, verified)"),
        "{out}"
    );
    let out = ok(&run(solve, &[table, "-t", "2", "--all-optimal"], after_r));
    assert!(out.contains("\nsolution 0 1 R'\n"), "{out}");
    let file = scratch_file("stickers.txt");
    std::fs::write(&file, format!("{after_r}\n")).unwrap();
    let bench = env!("CARGO_BIN_EXE_bench");
    let out = ok(&run(bench, &["batch", "-t", "2", table, &file], ""));
    assert!(
        out.contains("cubes 1 ") && out.contains("lengths 1:1"),
        "{out}"
    );
    // One corner twisted: 54 stickers, but not a cube.
    let twisted = "UUUUUUUURFRRRRRRRRFFUFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB";
    for (args, input) in [
        (vec![table], format!("R\n{twisted}\n")),
        (vec!["--fast", "--table", table, twisted], String::new()),
    ] {
        let out = run(solve, &args, &input);
        let log = text(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{log}");
        assert!(log.contains("54 stickers, which are not a cube"), "{log}");
        assert!(!text(&out.stdout).contains("solved 1"), "{log}");
    }
    let out = run(solve, &[table], "R\nhello\n");
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("line 2: cannot parse \"hello\""));
}

#[test]
fn solve_requires_explicit_input() {
    let solve = env!("CARGO_BIN_EXE_solve");
    let missing = scratch_file("not-a-table");
    for args in [vec!["--fast", "--table", &missing], vec![&missing]] {
        for input in ["", " \n\t\n"] {
            let out = run(solve, &args, input);
            assert_eq!(out.status.code(), Some(2), "{}", text(&out.stderr));
            assert!(text(&out.stderr).contains("no scramble"));
            assert_eq!(text(&out.stdout), "");
        }
    }
    for args in [vec!["--fast", "R"], vec!["--fast", &missing, "R"]] {
        let out = run(solve, &args, "");
        assert_eq!(out.status.code(), Some(2));
        assert!(text(&out.stderr).contains("--table"));
    }
    let out = run_env(
        solve,
        &["--fast", "R"],
        "",
        &[("EGG_SOLVER_TABLE", &missing)],
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("explicit --table"));
    let out = run(solve, &["--fast", "--table", &missing], "R\nU\n");
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("exactly one scramble"));
    assert_eq!(text(&out.stdout), "");
}

#[test]
fn solve_fast_stdin_without_warmup() {
    let solve = env!("CARGO_BIN_EXE_solve");
    let args = ["--fast", "--table", h0(), "-t", "2", "--json"];
    let out = run(solve, &args, "\nR U R' U'\n\n");
    let report: serde_json::Value = serde_json::from_str(&ok(&out)).unwrap();
    assert_eq!(report["scramble"], "R U R' U'");
    assert_eq!(report["length"], 4);
    // With no default warm-up even h0 can solve an explicit argument quickly.
    let out = run(solve, &[&args[..], &["R"]].concat(), "");
    let report: serde_json::Value = serde_json::from_str(&ok(&out)).unwrap();
    assert_eq!(report["length"], 1);
}

#[test]
fn solve_json_env_and_logging() {
    let env = [("EGG_SOLVER_TABLE", h0()), ("EGG_SOLVER_PREFAULT", "0")];
    let args = ["--json", "-t", "2", "-v"];
    let out = run_env(env!("CARGO_BIN_EXE_solve"), &args, SCRAMBLES, &env);
    let stdout = ok(&out);
    let lines: Vec<serde_json::Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 5);
    for line in &lines[..4] {
        assert_eq!(line["type"], "solved");
        assert_eq!(line["ok"], true);
        let moves = line["moves"].as_str().unwrap();
        let length = line["length"].as_u64().unwrap();
        assert_eq!(moves.split_whitespace().count() as u64, length, "{line}");
        assert!(line["nodes"].is_u64() && line["seconds"].is_f64(), "{line}");
    }
    assert_eq!(lines[4]["type"], "total");
    assert_eq!(lines[4]["cubes"], 4);
    assert_eq!(lines[4]["threads"], 2);
    let log = text(&out.stderr);
    assert!(log.contains("h0 base 8"), "{log}");
    assert!(log.contains("2 threads, 24 lanes per thread"), "{log}");
}

/// Removes a shared memory test's segment on drop.
#[cfg(target_os = "linux")]
struct Segment<'a>(&'a [(&'a str, &'a str)]);
#[cfg(target_os = "linux")]
impl Drop for Segment<'_> {
    fn drop(&mut self) {
        let _ = run_env(
            env!("CARGO_BIN_EXE_solve"),
            &["--shm-remove", h0()],
            "",
            self.0,
        );
    }
}

/// `solve --shm-load` makes a table's shared memory segment, later runs
/// attach it (`--shm`, and `auto` loads), explicit copy loads do not, and
/// `--shm-remove` removes it. (With a key of its own and never 1 GiB pages;
/// the segment is removed at the end, also when the test fails.)
#[test]
#[cfg(target_os = "linux")]
fn solve_shared_memory() {
    let table = h0();
    let solve = env!("CARGO_BIN_EXE_solve");
    let key = shm_key(1);
    let env = [
        ("EGG_SOLVER_SHM_KEY", key.as_str()),
        ("EGG_SOLVER_SHM_PAGES", "2m,4k"),
    ];
    let segment = Segment(&env);
    // (One left by a crashed earlier run of this process id.)
    drop(run_env(solve, &["--shm-remove", table], "", &env));
    let made = run_env(solve, &["--shm-load", table], "", &env);
    let stdout = ok(&made);
    assert!(
        stdout.starts_with(&format!("segment {key} shmid "))
            && stdout.contains(": h0 base 8, 0.03 GiB on ")
            && stdout.contains(&format!(" pages, made from {table} in ")),
        "{stdout}"
    );
    if stdout.contains("on 4 KiB pages") {
        // (2 MiB pages were allowed: why they were not had, and a warning.)
        let log = text(&made.stderr);
        assert!(
            log.contains("is on 4 KiB pages") && log.contains("2 MiB pages: "),
            "{log}"
        );
    }
    let again = ok(&run_env(solve, &["--shm-load", table], "", &env));
    assert!(
        again.contains(&format!("already loaded (from {table} by process ")),
        "{again}"
    );
    for load in ["--shm", "--load=auto"] {
        let args = [table, "-t", "2", "-c", "1", "-v", load];
        let out = run_env(solve, &args, SCRAMBLES, &env);
        check_solve_output(&ok(&out), 4, "2", "24", "1");
        let log = text(&out.stderr);
        assert!(
            log.contains(&format!("shared memory segment {key} (shmid "))
                && log.contains("attached (made by process "),
            "{log}"
        );
    }
    // With the segment there, no memory is needed (no copy).
    let low = [
        ("EGG_SOLVER_SHM_KEY", key.as_str()),
        ("EGG_SOLVER_PREFLIGHT_LIMIT", "1000000"),
    ];
    let out = run_env(solve, &[table, "-t", "2", "-c", "1"], SCRAMBLES, &low);
    check_solve_output(&ok(&out), 4, "2", "24", "1");
    let args = [table, "-t", "2", "-c", "1", "-v", "--load", "copy"];
    let out = run_env(solve, &args, SCRAMBLES, &env);
    check_solve_output(&ok(&out), 4, "2", "24", "1");
    assert!(!text(&out.stderr).contains("shared memory segment"));
    let removed = ok(&run_env(solve, &["--shm-remove", table], "", &env));
    assert!(
        removed.starts_with(&format!("removed segment {key} shmid "))
            && removed.contains(": h0 base 8, checksum "),
        "{removed}"
    );
    let none = ok(&run_env(solve, &["--shm-remove", table], "", &env));
    assert_eq!(none.trim(), format!("no segment {key} for {table}"));
    drop(segment);
}

/// A new segment's fallbacks: without the permission for huge pages
/// (simulated), 4 KiB pages with a warning; without the memory for 4 KiB
/// pages, refused before anything is made. (Its own key; removed at the
/// end, also when the test fails.)
#[test]
#[cfg(target_os = "linux")]
fn solve_shared_memory_fallbacks() {
    let table = h0();
    let solve = env!("CARGO_BIN_EXE_solve");
    let key = shm_key(2);
    let env = [("EGG_SOLVER_SHM_KEY", key.as_str())];
    let segment = Segment(&env);
    // A new segment on 4 KiB pages without the memory for it: refused
    // before anything is made (exit 2), as a copy would be.
    let low = [
        ("EGG_SOLVER_SHM_KEY", key.as_str()),
        ("EGG_SOLVER_SHM_PAGES", "4k"),
        ("EGG_SOLVER_PREFLIGHT_LIMIT", "1000000"),
    ];
    let refused = run_env(solve, &["--shm-load", table], "", &low);
    let log = text(&refused.stderr);
    assert_eq!(refused.status.code(), Some(2), "{log}");
    assert!(
        log.contains("a shared memory segment of table") && log.contains("on 4 KiB pages"),
        "{log}"
    );
    let none = ok(&run_env(solve, &["--shm-remove", table], "", &env));
    assert_eq!(none.trim(), format!("no segment {key} for {table}"));
    // Without the permission for huge pages (simulated: the kernel is not
    // asked), 1 GiB and then 2 MiB pages are skipped with how to get it,
    // and the segment gets 4 KiB pages, with a warning.
    let denied = [
        ("EGG_SOLVER_SHM_KEY", key.as_str()),
        ("EGG_SOLVER_SHM_DENY_HUGE", "1"),
        ("EGG_SOLVER_SHM_PAGES", "1g,2m,4k"),
    ];
    let made = run_env(solve, &["--shm-load", table], "", &denied);
    assert!(ok(&made).contains(" 0.03 GiB on 4 KiB pages, made from "));
    let log = text(&made.stderr);
    let gib = log.find("1 GiB pages: no permission: huge-page segments need a group of yours in vm.hugetlb_shm_group");
    let mib = log.find("2 MiB pages: no permission: ");
    assert!(gib.is_some() && mib > gib, "{log}");
    assert!(
        log.contains("or CAP_IPC_LOCK; to allow your group: `sudo sysctl vm.hugetlb_shm_group="),
        "{log}"
    );
    assert!(
        log.contains(&format!("shared memory segment {key} is on 4 KiB pages")),
        "{log}"
    );
    drop(segment);
}

#[test]
fn bench_legacy_stream_forms() {
    let table = h0();
    for (args, mode) in [
        (vec![table, "stream", "0.1", "0.5", "2", "4"], ""),
        (
            vec![table, "stream", "0.1", "0.5", "2", "4", "7", "share"],
            " share 0",
        ),
        (
            vec![table, "stream", "0.1", "0.5", "2", "4", "7", "share:1"],
            " share 1",
        ),
    ] {
        let out = run(env!("CARGO_BIN_EXE_bench"), &args, "");
        let first = format!("threads 2 lanes 4 window 0.5s{mode}");
        let patterns = [
            &first,
            "nodes/cube # lookups/cube #",
            "lengths *",
            "throughput #",
        ];
        check_lines(&ok(&out), &patterns);
        assert!(text(&out.stderr).starts_with("table h0 base 8 loaded in "));
    }
}

/// (Not `random:N:SEED` sets: a random state takes minutes on h0. Their
/// parsing is in bench's unit tests; verify.sh runs them on h11.)
#[test]
fn bench_legacy_batch_forms() {
    let table = h0();
    let file = scrambles_file();
    for (args, cubes, mode) in [
        (vec![table, file, "4", "2", "4"], "4", ""),
        (vec![table, file, "3", "2", "4"], "3", ""),
        (vec![table, file, "4", "2", "4", "share"], "4", " share 0"),
        (vec![table, file, "4", "2", "4", "share:1"], "4", " share 1"),
    ] {
        let out = run(env!("CARGO_BIN_EXE_bench"), &args, "");
        let first = format!("cubes {cubes} threads 2 lanes 4 time #s{mode}");
        let patterns = [
            &first,
            "solves/sec #",
            "nodes/cube # lookups/cube #",
            "lengths *",
            "solve time max #s p50 #s",
        ];
        check_lines(&ok(&out), &patterns);
    }
}

/// The subcommand forms give what the legacy forms give (the counts are
/// deterministic), and `--json` the same numbers.
#[test]
fn bench_subcommands_match_legacy() {
    let table = h0();
    let file = scrambles_file();
    let bench = env!("CARGO_BIN_EXE_bench");
    let counts = |out: &str| -> Vec<String> {
        out.lines()
            .filter(|l| l.starts_with("nodes/cube") || l.starts_with("lengths"))
            .map(String::from)
            .collect()
    };
    let legacy = ok(&run(bench, &[table, file, "4", "2", "4"], ""));
    let args = [
        "batch",
        table,
        file,
        "-n",
        "4",
        "--threads",
        "2",
        "--lanes",
        "4",
    ];
    let new = ok(&run(bench, &args, ""));
    assert_eq!(counts(&legacy).len(), 2);
    assert_eq!(counts(&legacy), counts(&new));
    let args = ["batch", table, file, "-t", "2", "-l", "4", "--json"];
    let json = ok(&run(bench, &args, ""));
    let report: serde_json::Value = serde_json::from_str(json.trim()).unwrap();
    assert_eq!(report["type"], "batch");
    assert_eq!(report["cubes"], 4);
    let nodes = format!("{:.0}", report["nodes_per_cube"].as_f64().unwrap());
    assert!(counts(&legacy)[0].starts_with(&format!("nodes/cube {nodes} ")));
    let mut lengths: Vec<(u64, u64)> = report["lengths"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(length, count)| (length.parse().unwrap(), count.as_u64().unwrap()))
        .collect();
    lengths.sort_unstable();
    let lengths: Vec<String> = lengths.iter().map(|(l, c)| format!("{l}:{c}")).collect();
    assert_eq!(counts(&legacy)[1], format!("lengths {}", lengths.join(" ")));
    let args = [table, "stream", "0", "0.3", "2", "4", "--json"];
    let json = ok(&run(bench, &args, ""));
    let report: serde_json::Value = serde_json::from_str(json.trim()).unwrap();
    assert_eq!(report["type"], "stream");
    assert_eq!(report["window"], 0.3);
    assert!(report["share"].is_null());
}

#[test]
fn egg_solver_legacy_form() {
    let out = run(
        env!("CARGO_BIN_EXE_egg-solver"),
        &[h0()],
        "R U R' U'\nnonsense\n",
    );
    let stdout = ok(&out);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    // `<solution> (<length>) <seconds>s nodes <nodes>`
    let words: Vec<&str> = lines[0].split(' ').collect();
    assert_eq!(words.len(), 8, "{stdout}");
    assert!(matches(&words[4..].join(" "), "(4) #s nodes #"), "{stdout}");
    assert_eq!(lines[1], "parse error");
}

#[test]
fn gen_legacy_forms() {
    let table = h0();
    let gen = env!("CARGO_BIN_EXE_gen");
    // README: `gen <k> auto <out>`.
    let auto = scratch_file("h0-auto");
    let out = run(gen, &["0", "auto", &auto, "-t", "4"], "");
    ok(&out);
    let auto_base = egg_solver::tables::default_base(0);
    let log = text(&out.stderr);
    let head = format!("[gen] h0 base {auto_base}: 0.03 GiB, 4 threads\n");
    assert!(log.starts_with(&head), "{log}");
    let tail = format!("\nh0 base {auto_base} distribution [");
    assert!(log.contains(&tail), "{log}");
    let checked = ok(&run(gen, &["check", table], ""));
    let lines: Vec<&str> = checked.lines().collect();
    assert_eq!(lines.len(), 3, "{checked}");
    assert_eq!(lines[0], format!("{table}: h0 base 8"));
    assert!(lines[1].starts_with("distribution ["), "{checked}");
    assert_eq!(lines[2], "ok");
    let json = ok(&run(gen, &["check", table, "--json"], ""));
    let report: serde_json::Value = serde_json::from_str(json.trim()).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["checksum"], report["trailer_checksum"]);
    let distribution: Vec<u64> = report["distribution"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap())
        .collect();
    assert_eq!(format!("distribution {distribution:?}"), lines[1]);
    // derive (h0 from h0: a copy), compare.
    let derived = scratch_file("h0-derived");
    let out = run(gen, &["derive", table, "0", &derived], "");
    ok(&out);
    assert!(text(&out.stderr).starts_with("h0 base 8 derived in "));
    let compared = ok(&run(gen, &["compare", table, &derived], ""));
    let lines: Vec<&str> = compared.lines().collect();
    let head = format!("{table} (h0 base 8) vs {derived} (h0 base 8): Diff {{");
    assert!(lines[0].starts_with(&head), "{compared}");
    assert_eq!(lines[1..], ["same"]);
    // stamp: the table without its trailer gets it back.
    let mut bytes = std::fs::read(table).unwrap();
    let data_len = bytes.len() - 4096;
    let bare = scratch_file("h0-bare");
    let trailer = bytes.split_off(data_len);
    std::fs::write(&bare, [&bytes[..], &[0; 4096]].concat()).unwrap();
    let stamped = ok(&run(gen, &["stamp", &bare, "0", "8"], ""));
    let head = format!("stamped {bare}: Trailer {{ k: 0, base: 8,");
    assert!(stamped.starts_with(&head), "{stamped}");
    assert_eq!(std::fs::read(&bare).unwrap(), std::fs::read(table).unwrap());
    // Damaged data fails the check (status 1).
    bytes[1000] ^= 0x55;
    let damaged = scratch_file("h0-damaged");
    std::fs::write(&damaged, [bytes, trailer].concat()).unwrap();
    let out = run(gen, &["check", &damaged], "");
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stdout).contains("MISMATCH checksum"));
}

/// The checks before a run (disk, memory), with every measured amount cut
/// to 1 MB (`EGG_SOLVER_PREFLIGHT_LIMIT`, for tests): exit 2 with the numbers
/// before any work, nothing written; `--force` skips them.
#[test]
fn preflight_failures() {
    let table = h0();
    let gen = env!("CARGO_BIN_EXE_gen");
    let limit = [("EGG_SOLVER_PREFLIGHT_LIMIT", "1000000")];
    let out = scratch_file("h0-preflight");
    let failed = run_env(gen, &["0", "8", &out, "-t", "2"], "", &limit);
    assert_eq!(failed.status.code(), Some(2));
    let message = text(&failed.stderr);
    assert!(message.starts_with("gen: "), "{message}");
    assert!(
        message.contains(" GiB free, h0 needs 0.0 GiB (+ 0.1 GiB margin)"),
        "{message}"
    );
    assert!(message.contains("--force"), "{message}");
    assert!(!std::path::Path::new(&out).exists());
    let failed = run_env(gen, &["derive", table, "0", &out], "", &limit);
    assert_eq!(failed.status.code(), Some(2), "{}", text(&failed.stderr));
    let forced = run_env(gen, &["0", "8", &out, "-t", "2", "--force"], "", &limit);
    ok(&forced);
    // A missing folder fails at once too.
    let nowhere = scratch_file("no-such-folder/h0");
    let failed = run(gen, &["0", "8", &nowhere], "");
    assert_eq!(failed.status.code(), Some(2));
    assert!(
        text(&failed.stderr).contains("its folder"),
        "{}",
        text(&failed.stderr)
    );
    // solve: copying the table into memory needs its size in memory.
    let solve = env!("CARGO_BIN_EXE_solve");
    let args = [table, "--load", "copy", "-t", "2"];
    let failed = run_env(solve, &args, "R U\n", &limit);
    assert_eq!(failed.status.code(), Some(2));
    let message = text(&failed.stderr);
    assert!(
        message.contains("of memory available (MemAvailable"),
        "{message}"
    );
    assert!(message.contains("hugetlbfs"), "{message}");
    // (Mapped: no copy, no check.)
    let args = [table, "--load", "map", "-t", "2"];
    ok(&run_env(solve, &args, "R U\n", &limit));
}

#[test]
fn help_version_and_completions() {
    for binary in [
        env!("CARGO_BIN_EXE_solve"),
        env!("CARGO_BIN_EXE_bench"),
        env!("CARGO_BIN_EXE_gen"),
        env!("CARGO_BIN_EXE_egg-solver"),
    ] {
        let help = ok(&run(binary, &["--help"], ""));
        assert!(help.contains("Usage:"), "{help}");
        let version = ok(&run(binary, &["--version"], ""));
        assert!(version.ends_with(concat!(" ", env!("CARGO_PKG_VERSION"), "\n")));
        for shell in ["bash", "zsh", "fish"] {
            let script = ok(&run(binary, &["--completions", shell], ""));
            assert!(script.len() > 100, "{script}");
        }
    }
    let help = ok(&run(env!("CARGO_BIN_EXE_gen"), &["--help"], ""));
    for command in ["generate", "stamp", "check", "derive", "compare"] {
        assert!(help.contains(command), "{help}");
    }
    let help = ok(&run(env!("CARGO_BIN_EXE_bench"), &["--help"], ""));
    for command in ["stream", "batch", "tune", "bench <table> stream [warmup]"] {
        assert!(help.contains(command), "{help}");
    }
}

#[test]
fn exit_statuses() {
    let table = h0();
    let status = |binary: &str, args: &[&str], stdin: &str| run(binary, args, stdin).status.code();
    let solve = env!("CARGO_BIN_EXE_solve");
    let bench = env!("CARGO_BIN_EXE_bench");
    let gen = env!("CARGO_BIN_EXE_gen");
    let missing = scratch_file("missing");
    // Usage errors and unusable input: 2.
    assert_eq!(status(solve, &[], ""), Some(2));
    assert_eq!(status(solve, &[table, "-t", "x"], ""), Some(2));
    assert_eq!(status(solve, &[table, "-t", "0"], ""), Some(2));
    assert_eq!(status(solve, &[table], "R U X\n"), Some(2));
    assert_eq!(status(solve, &[&missing], "R U\n"), Some(2));
    let bad_load = run_env(solve, &[table], "R U\n", &[("EGG_SOLVER_LOAD", "fast")]);
    assert_eq!(bad_load.status.code(), Some(2));
    assert_eq!(status(bench, &[table, "random:x"], ""), Some(2));
    assert_eq!(status(bench, &[table, "stream", "x"], ""), Some(2));
    assert_eq!(status(gen, &["12", "auto", &missing], ""), Some(2));
    assert_eq!(status(gen, &["1", "auto", &missing, "--dfs"], ""), Some(2));
    assert_eq!(status(gen, &["5", "auto", &missing, "0:8:x"], ""), Some(2));
    assert_eq!(status(gen, &["stamp", table, "0", "13"], ""), Some(2));
    assert_eq!(status(bench, &[&missing, "stream", "0", "1"], ""), Some(2));
    let out = run(solve, &[&missing], "R U\n");
    let message = format!("solve: cannot open table: {missing}: No such file");
    assert!(
        text(&out.stderr).starts_with(&message),
        "{}",
        text(&out.stderr)
    );
    // Other errors and failed checks: 1.
    assert_eq!(status(gen, &["check", &missing], ""), Some(1));
    assert_eq!(status(bench, &[table, &missing], ""), Some(1));
}

/// Existing settings work, and modern settings take precedence.
#[test]
fn previous_environment_names_remain_fallbacks() {
    let legacy = run_env(
        env!("CARGO_BIN_EXE_solve"),
        &["-c", "1"],
        "R U\n",
        &[("CUBEOPT_TABLE", h0()), ("CUBEOPT_THREADS", "2")],
    );
    assert!(ok(&legacy).contains("threads 2"));
    let modern = run_env(
        env!("CARGO_BIN_EXE_solve"),
        &["-c", "1"],
        "R U\n",
        &[
            ("CUBEOPT_TABLE", "missing"),
            ("CUBEOPT_THREADS", "invalid"),
            ("EGG_SOLVER_TABLE", h0()),
            ("EGG_SOLVER_THREADS", "1"),
        ],
    );
    assert!(ok(&modern).contains("threads 1"));
}

/// `--metric qtm`: `gen` makes a QTM table (its trailer's layout is
/// `h48-qtm`; check, stamp), `solve` solves in QTM on HTM and QTM tables
/// with the same lengths (`(Nq)` after each solution, `metric qtm` in the
/// summary and in JSON), and an HTM solve refuses a QTM table (status 2).
#[test]
fn qtm_metric() {
    let gen = env!("CARGO_BIN_EXE_gen");
    let solve = env!("CARGO_BIN_EXE_solve");
    let qtm_table = scratch_file("h0-qtm");
    let out = run(
        gen,
        &["0", "auto", &qtm_table, "-t", "4", "--metric", "qtm"],
        "",
    );
    ok(&out);
    let base = egg_solver::tables::default_base_metric(0, egg_solver::metric::Metric::Qtm);
    let log = text(&out.stderr);
    let head = format!("[gen] h0 base {base} qtm: 0.03 GiB, 4 threads\n");
    assert!(log.starts_with(&head), "{log}");
    let bytes = std::fs::read(&qtm_table).unwrap();
    let trailer = text(&bytes[bytes.len() - 4096..]);
    assert!(trailer.contains("\nlayout h48-qtm\n"), "{trailer}");
    let checked = ok(&run(gen, &["check", &qtm_table], ""));
    assert!(
        checked.starts_with(&format!("{qtm_table}: h0 base {base} qtm\n")),
        "{checked}"
    );
    // stamp: the QTM table without its trailer gets it back.
    let stripped = scratch_file("h0-qtm-stripped");
    let data_len = bytes.len() - 4096;
    std::fs::write(&stripped, [&bytes[..data_len], &[0; 4096]].concat()).unwrap();
    let base_text = base.to_string();
    ok(&run(
        gen,
        &["stamp", &stripped, "0", &base_text, "--metric", "qtm"],
        "",
    ));
    assert_eq!(std::fs::read(&stripped).unwrap(), bytes);
    // QTM solves on both tables.
    let qtm_length = |moves: &[&str]| -> usize {
        moves
            .iter()
            .map(|m| if m.ends_with('2') { 2 } else { 1 })
            .sum()
    };
    let mut lengths = Vec::new();
    for table in [h0(), qtm_table.as_str()] {
        let args = ["--table", table, "--metric", "qtm", "-t", "2"];
        let output = ok(&run(solve, &args, SCRAMBLES));
        let rows: Vec<&str> = output.lines().collect();
        assert_eq!(rows.len(), 5, "{output}");
        let mut by_index = vec![0; 4];
        for line in &rows[..4] {
            assert!(matches(line, "solved # # # # *"), "{line}");
            let words: Vec<&str> = line.split(' ').collect();
            let length: usize = words[2].parse().unwrap();
            let (moves, mark) = words[5..].split_at(words.len() - 6);
            assert_eq!(mark, [format!("({length}q)").as_str()], "{line}");
            assert_eq!(qtm_length(moves), length, "{line}");
            by_index[words[1].parse::<usize>().unwrap()] = length;
        }
        assert!(
            rows[4].ends_with(" concurrency 0 metric qtm"),
            "{}",
            rows[4]
        );
        lengths.push(by_index);
    }
    assert_eq!(lengths[0], lengths[1]);
    assert_eq!(lengths[0][0], 4, "R U R' U' is 4q");
    // JSON, --fast.
    let args = [
        "--fast", "--table", &qtm_table, "--metric", "qtm", "--json", "-t", "2", "U2",
    ];
    let report: serde_json::Value =
        serde_json::from_str(ok(&run(solve, &args, "")).trim()).unwrap();
    assert_eq!(
        (report["metric"].as_str(), report["length"].as_u64()),
        (Some("qtm"), Some(2))
    );
    let args = ["--fast", "--table", h0(), "-t", "2", "U2"];
    assert!(ok(&run(solve, &args, "")).contains("length   1 moves"));
    // An HTM solve on the QTM table: refused, with a hint.
    let out = run(solve, &["--table", &qtm_table, "-t", "2"], SCRAMBLES);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).contains("use --metric qtm"),
        "{}",
        text(&out.stderr)
    );
}

/// The lists of `solve`'s text output with `--all-optimal`, `--plus` or
/// `--max-length`: per cube (in input order) its `solutions` line's words,
/// its `truncated` or `stopped` line's (if any) and its solutions as
/// (length, moves); then the total line. Each cube's lines are together,
/// and its `solutions` line counts its `solution` lines.
type Lists = Vec<(Vec<String>, Option<Vec<String>>, Vec<(usize, String)>)>;
fn parse_lists(output: &str, cubes: usize) -> (Lists, String) {
    let mut lists: Lists = vec![Default::default(); cubes];
    let mut lines = output.lines().peekable();
    let mut total = String::new();
    while let Some(line) = lines.next() {
        if line.starts_with("total ") {
            total = line.to_string();
            assert!(lines.next().is_none(), "{output}");
            break;
        }
        assert!(matches(line, "solutions # # * "), "{line}");
        let head: Vec<String> = line.split(' ').map(str::to_string).collect();
        assert_eq!(head.len(), 7, "{line}");
        let index: usize = head[1].parse().unwrap();
        assert!(lists[index].0.is_empty(), "cube {index} listed twice");
        let truncated = lines
            .next_if(|line| line.starts_with("truncated ") || line.starts_with("stopped "))
            .map(|line| line.split(' ').map(str::to_string).collect::<Vec<_>>());
        if let Some(truncated) = &truncated {
            assert_eq!(truncated[1], head[1], "{output}");
        }
        let mut solutions = Vec::new();
        while let Some(line) = lines.next_if(|line| line.starts_with("solution ")) {
            let words: Vec<&str> = line.split(' ').collect();
            assert_eq!(words[1], head[1], "{output}");
            let length: usize = words[2].parse().unwrap();
            let moves: Vec<&str> = (words[3..].iter().copied())
                .filter(|word| !word.starts_with('('))
                .collect();
            solutions.push((length, moves.join(" ")));
        }
        assert_eq!(solutions.len(), head[2].parse::<usize>().unwrap(), "{line}");
        // (By length; of one length, by the moves' numbers: U, D, R, ...)
        assert!(
            solutions.is_sorted_by_key(|solution| solution.0),
            "{output}"
        );
        lists[index] = (head, truncated, solutions);
    }
    assert!(lists.iter().all(|list| !list.0.is_empty()), "{output}");
    (lists, total)
}

/// The checkerboard of 6 half turns (6 optimal solutions, 12 more of 8
/// moves), a 4-move scramble with one solution and one with two.
const LISTED: &str = "U2 D2 R2 L2 F2 B2\nR U R' U'\n\nU2 D2 R2 L2\n";

/// `solve <h0> -t 2 <args>` on `LISTED`: its lists and total line.
fn lists(args: &[&str]) -> (Lists, String) {
    let args = [&[h0(), "-t", "2"], args].concat();
    parse_lists(&ok(&run(env!("CARGO_BIN_EXE_solve"), &args, LISTED)), 3)
}

/// The solutions of each cube of some lists.
fn listed(lists: &Lists) -> Vec<Vec<(usize, String)>> {
    lists.iter().map(|list| list.2.clone()).collect()
}

/// `--max-length N`: every distinct solution of each cube of at most N
/// moves, as text and as JSON, for a batch and with `--fast`; a list cut
/// by `--max-solutions` says so; and what cannot be searched is refused.
#[test]
fn solve_max_length() {
    let table = h0();
    let solve = env!("CARGO_BIN_EXE_solve");
    let (short, total) = lists(&["--max-length", "4", "-c", "1"]);
    assert!(
        matches(
            &total,
            "total 3 cubes wall # wall/cube # nodes/cube # threads 2 lanes 24 concurrency 1"
        ),
        "{total}"
    );
    // (No solution of at most 4 moves: no optimal length.)
    assert_eq!(short[0].0[2..5], ["0", "-", "4"]);
    assert_eq!(short[1].2, [(4, "U R U' R'".to_string())]);
    assert_eq!(
        short[2].2,
        [
            (4, "U2 D2 R2 L2".to_string()),
            (4, "R2 L2 U2 D2".to_string())
        ]
    );
    assert!(short.iter().all(|list| list.1.is_none()));
    // Up to 8 moves: by length.
    let (long, _) = lists(&["--max-length", "8"]);
    let lengths: Vec<usize> = long[0].2.iter().map(|solution| solution.0).collect();
    assert_eq!(lengths, [vec![6; 6], vec![8; 12]].concat());
    assert_eq!(long[0].0[3..5], ["6", "8"], "optimal and limit");
    assert_eq!(long[0].2[0], (6, "U2 D2 R2 L2 F2 B2".to_string()));
    // The one solution of the default solve is among the shortest (in the
    // order of the list: U before D).
    let one = ok(&run(solve, &[table, "-t", "2", "-n", "1"], LISTED));
    let one: Vec<&str> = one.lines().next().unwrap().split(' ').skip(5).collect();
    assert!(long[0].2[..6].contains(&(6, one.join(" "))), "{one:?}");
    // More solutions than --max-solutions: said, with the last complete
    // length (5: the 6 optimal ones do not fit; 7: those of 8 do not).
    let truncated = |cap: &str, complete: &str| {
        Some(
            ["truncated", "0", cap, complete]
                .map(str::to_string)
                .to_vec(),
        )
    };
    let (cut, _) = lists(&["--max-length", "8", "--max-solutions", "5"]);
    assert_eq!(cut[0].1, truncated("5", "5"));
    assert_eq!(cut[0].2.len(), 5);
    let (cut, _) = lists(&["--max-length", "8", "--max-solutions", "7"]);
    assert_eq!(cut[0].1, truncated("7", "7"));
    assert_eq!(cut[0].2[..6], long[0].2[..6]);
    assert_eq!(cut[0].2.len(), 7);
    // Refused: a length that no search reaches, a cap without a list, no
    // room for any solution.
    for (args, message) in [
        (&["--max-length", "21"][..], "--max-length is at most 20"),
        (
            &["--metric", "qtm", "--max-length", "27"][..],
            "--max-length is at most 26",
        ),
        (&["--max-solutions", "5"][..], "--max-solutions"),
        (
            &["--max-length", "5", "--max-solutions", "0"][..],
            "--max-solutions",
        ),
    ] {
        let out = run(solve, &[&[table, "-t", "2"], args].concat(), LISTED);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let error = text(&out.stderr);
        assert!(error.contains(message), "{args:?}: {error}");
        assert_eq!(text(&out.stdout), "", "{args:?}");
    }
}

/// `--max-length` as JSON lines.
#[test]
fn solve_max_length_json() {
    let table = h0();
    let solve = env!("CARGO_BIN_EXE_solve");
    let args = [
        table,
        "-t",
        "2",
        "--max-length",
        "8",
        "--max-solutions",
        "7",
        "--json",
    ];
    let json = ok(&run(solve, &args, LISTED));
    let values: Vec<serde_json::Value> = json
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(values.len(), 4);
    assert_eq!(values[3]["type"], "total");
    let first = values.iter().find(|value| value["index"] == 0).unwrap();
    assert_eq!(first["type"], "solutions");
    assert_eq!(
        (first["count"].as_u64(), first["optimal"].as_u64()),
        (Some(7), Some(6))
    );
    assert_eq!(
        (
            first["max_length"].as_u64(),
            first["complete_length"].as_u64()
        ),
        (Some(8), Some(7))
    );
    assert_eq!(
        (first["truncated"].as_bool(), first["ok"].as_bool()),
        (Some(true), Some(true))
    );
    assert_eq!(first["metric"], "htm");
    assert_eq!(
        first["by_length"],
        serde_json::json!([{"length": 6, "count": 6}, {"length": 8, "count": 1}])
    );
    assert_eq!(first["solutions"].as_array().unwrap().len(), 7);
    assert_eq!(
        first["solutions"][0],
        serde_json::json!({"moves": "U2 D2 R2 L2 F2 B2", "length": 6})
    );
    // (One solution of 4 moves and two of 8.)
    let second = values.iter().find(|value| value["index"] == 1).unwrap();
    assert_eq!(
        (second["truncated"].as_bool(), second["count"].as_u64()),
        (Some(false), Some(3))
    );
    assert_eq!(second["complete_length"], 8);
}

/// `--max-length` with `--fast`, and in the quarter-turn metric.
#[test]
fn solve_max_length_fast() {
    let table = h0();
    let solve = env!("CARGO_BIN_EXE_solve");
    // --fast: the solutions under a line per length.
    let fast = ["--fast", "--table", table, "-t", "2"];
    let args = [&fast[..], &["--max-length", "8", "U2 D2 R2 L2 F2 B2"]].concat();
    let out = ok(&run(solve, &args, ""));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 26, "{out}");
    assert_eq!(lines[2], "scramble U2 D2 R2 L2 F2 B2");
    assert_eq!(lines[3], "optimal  6 moves");
    assert_eq!(
        lines[4],
        "listed   18 solutions of at most 8 moves (verified)"
    );
    assert_eq!(lines[5], "6 moves: 6 solutions");
    assert_eq!(lines[6], "  U2 D2 R2 L2 F2 B2");
    assert_eq!(lines[12], "8 moves: 12 solutions");
    assert!(lines[25].starts_with("time     "), "{out}");
    let args = [
        &fast[..],
        &[
            "--max-length",
            "6",
            "--max-solutions",
            "4",
            "U2 D2 R2 L2 F2 B2",
        ],
    ]
    .concat();
    let out = ok(&run(solve, &args, ""));
    assert!(
        out.contains("\nlisted   4 solutions of at most 6 moves (verified)\n"),
        "{out}"
    );
    let cut = "\ntruncated: there are more than 4 (--max-solutions); complete up to 5 moves\n";
    assert!(out.contains(cut), "{out}");
    let args = [&fast[..], &["--max-length", "3", "R U R' U'"]].concat();
    let out = ok(&run(solve, &args, ""));
    let none = "\noptimal  more than 3 moves\nlisted   0 solutions of at most 3 moves";
    assert!(out.contains(none), "{out}");
    let args = [&fast[..], &["--max-length", "4", "--json", "U2 D2 R2 L2"]].concat();
    let report: serde_json::Value =
        serde_json::from_str(ok(&run(solve, &args, "")).trim()).unwrap();
    assert_eq!(
        (report["count"].as_u64(), report["optimal"].as_u64()),
        (Some(2), Some(4))
    );
    assert_eq!(report["scramble"], "U2 D2 R2 L2");
    assert_eq!(report["solutions"][1]["moves"], "R2 L2 U2 D2");
    assert_eq!(report["truncated"], false);
    // The quarter-turn metric: lengths in quarter turns.
    let args = [table, "-t", "2", "--metric", "qtm", "--max-length", "8"];
    let out = ok(&run(solve, &args, "U2 D2 R2 L2\nR U2\n"));
    let (qtm, total) = parse_lists(&out, 2);
    assert_eq!(qtm[0].0[3..5], ["8", "8"]);
    assert_eq!(qtm[0].2.len(), 2);
    assert_eq!(qtm[1].2[0], (3, "U2 R'".to_string()));
    assert!(out.contains("\nsolution 1 3 U2 R' (3q)\n"), "{out}");
    assert!(total.ends_with(" metric qtm"), "{total}");
    let args = [&fast[..], &["--metric", "qtm", "--max-length", "4", "U2"]].concat();
    let out = ok(&run(solve, &args, ""));
    let quarter =
        "\noptimal  2q\nlisted   1 solution of at most 4q (verified)\n2q: 1 solution\n  U2\n";
    assert!(out.contains(quarter), "{out}");
}

/// `--all-optimal` and `--plus K`: every distinct optimal solution of each
/// cube, and those of at most K moves more; with `--max-length` the
/// smaller limit.
#[test]
fn solve_all_optimal_and_plus() {
    let table = h0();
    let solve = env!("CARGO_BIN_EXE_solve");
    let (optimal, total) = lists(&["--all-optimal"]);
    assert!(
        matches(
            &total,
            "total 3 cubes wall # wall/cube # nodes/cube # threads 2 lanes 24 concurrency 0"
        ),
        "{total}"
    );
    let counts: Vec<usize> = optimal.iter().map(|list| list.2.len()).collect();
    assert_eq!(counts, [6, 1, 2]);
    assert_eq!(optimal[0].0[3..5], ["6", "6"], "optimal and limit");
    assert_eq!(optimal[1].0[3..5], ["4", "4"]);
    assert!(optimal.iter().all(|list| list.1.is_none()));
    // The one solution of the default solve is among them.
    let one = ok(&run(solve, &[table, "-t", "2", "-n", "1"], LISTED));
    let one: Vec<&str> = one.lines().next().unwrap().split(' ').skip(5).collect();
    assert!(optimal[0].2.contains(&(6, one.join(" "))), "{one:?}");
    // --plus 0 is the same, and --max-length <optimal> for each cube.
    assert_eq!(listed(&lists(&["--plus", "0"]).0), listed(&optimal));
    assert_eq!(
        listed(&lists(&["--max-length", "4"]).0)[1..],
        listed(&optimal)[1..]
    );
    assert_eq!(
        listed(&lists(&["--max-length", "6"]).0)[0],
        listed(&optimal)[0]
    );
    // --plus 2: what --max-length <optimal + 2> lists.
    let (plus, _) = lists(&["--plus", "2"]);
    assert_eq!(plus[0].0[3..5], ["6", "8"]);
    assert_eq!(plus[1].0[3..5], ["4", "6"]);
    assert_eq!(
        listed(&plus)[0],
        listed(&lists(&["--max-length", "8"]).0)[0]
    );
    assert_eq!(
        listed(&plus)[1..],
        listed(&lists(&["--max-length", "6"]).0)[1..]
    );
    assert_eq!(plus[0].2[..6], optimal[0].2[..]);
    // Both limits: the smaller.
    let both = lists(&["--plus", "2", "--max-length", "7"]).0;
    assert_eq!(listed(&both)[0], optimal[0].2);
    assert_eq!(both[0].0[3..5], ["6", "7"]);
    assert_eq!(
        listed(&lists(&["--plus", "0", "--max-length", "9"]).0),
        listed(&optimal)
    );
    // Truncated within the optimal solutions: complete below them.
    let (cut, _) = lists(&["--all-optimal", "--max-solutions", "4"]);
    let truncated = ["truncated", "0", "4", "5"].map(str::to_string).to_vec();
    assert_eq!(cut[0].1, Some(truncated));
    assert_eq!(cut[0].2.len(), 4);
    // JSON and --fast.
    let fast = ["--fast", "--table", table, "-t", "2"];
    let args = [&fast[..], &["--all-optimal", "--json", "U2 D2 R2 L2"]].concat();
    let report: serde_json::Value =
        serde_json::from_str(ok(&run(solve, &args, "")).trim()).unwrap();
    assert_eq!(
        (report["count"].as_u64(), report["max_length"].as_u64()),
        (Some(2), Some(4))
    );
    let args = [&fast[..], &["--plus", "2", "U2 D2 R2 L2 F2 B2"]].concat();
    let out = ok(&run(solve, &args, ""));
    let listing = "\noptimal  6 moves\nlisted   18 solutions of at most 8 moves (verified)\n";
    assert!(out.contains(listing), "{out}");
    // The quarter-turn metric.
    let args = [table, "-t", "2", "--metric", "qtm", "--all-optimal"];
    let (qtm, _) = parse_lists(&ok(&run(solve, &args, "U2 D2 R2 L2\nR U2\n")), 2);
    assert_eq!(qtm[0].0[2..5], ["2", "8", "8"]);
    assert_eq!(qtm[1].2, [(3, "U2 R'".to_string())]);
    // (God's number as --max-length is accepted; with --plus the search
    // ends early.)
    let args = [
        table,
        "-t",
        "2",
        "--metric",
        "qtm",
        "--max-length",
        "26",
        "--plus",
        "0",
    ];
    assert!(ok(&run(solve, &args, "U2\n")).starts_with("solutions 0 1 2 2 "));
    // Refused: two limits above the optimum at once.
    let args = [table, "-t", "2", "--all-optimal", "--plus", "1"];
    let out = run(solve, &args, LISTED);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).contains("--plus"),
        "{}",
        text(&out.stderr)
    );
}

/// A scramble whose list up to 16 moves takes long on h0.
const LONG_LIST: &str = "R U F2 D L' B\n";

/// `--time-limit`: a list that the clock stops says so as text, as JSON
/// and with `--fast`, holds what was found (all of it up to the complete
/// length) and is no error; a limit that is not reached changes nothing.
#[test]
fn solve_time_limit() {
    let table = h0();
    let solve = env!("CARGO_BIN_EXE_solve");
    let limited = [
        table,
        "-t",
        "2",
        "--max-length",
        "16",
        "--time-limit",
        "0.05",
    ];
    let out = ok(&run(solve, &limited, LONG_LIST));
    let (stopped, total) = parse_lists(&out, 1);
    assert!(total.starts_with("total 1 cubes "), "{total}");
    let line = stopped[0].1.clone().expect("no stopped line");
    assert_eq!(line[..3], ["stopped", "0", "time"], "{out}");
    let complete: usize = line[3].parse().unwrap();
    assert!((5..16).contains(&complete), "{out}");
    assert_eq!(stopped[0].0[3..5], ["6", "16"], "optimal and limit");
    // All of the solutions up to the complete length, some longer ones.
    let args = [table, "-t", "2", "--max-length", "11"];
    let (full, _) = parse_lists(&ok(&run(solve, &args, LONG_LIST)), 1);
    let up_to = |list: &[(usize, String)], length: usize| -> Vec<(usize, String)> {
        let short = |solution: &&(usize, String)| solution.0 <= length;
        list.iter().filter(short).cloned().collect()
    };
    let common = complete.min(11);
    assert_eq!(up_to(&stopped[0].2, common), up_to(&full[0].2, common));
    assert!(stopped[0].2.iter().all(|solution| solution.0 <= 16));
    // JSON.
    let json = ok(&run(
        solve,
        &[&limited[..], &["--json"]].concat(),
        LONG_LIST,
    ));
    let value: serde_json::Value = serde_json::from_str(json.lines().next().unwrap()).unwrap();
    assert_eq!(value["stopped"], "time");
    assert_eq!(
        (value["truncated"].as_bool(), value["ok"].as_bool()),
        (Some(false), Some(true))
    );
    assert!(value["complete_length"].as_u64().unwrap() < 16);
    assert_eq!(value["max_length"], 16);
    // --fast.
    let fast = ["--fast", "--table", table, "-t", "2"];
    let list = [
        "--max-length",
        "16",
        "--time-limit",
        "0.05",
        "R U F2 D L' B",
    ];
    let out = ok(&run(solve, &[&fast[..], &list[..]].concat(), ""));
    assert!(out.contains("\noptimal  6 moves\n"), "{out}");
    let flag = "\nstopped: the time limit was reached; complete up to ";
    assert!(out.contains(flag), "{out}");
    let json = ok(&run(
        solve,
        &[&fast[..], &list[..], &["--json"]].concat(),
        "",
    ));
    let report: serde_json::Value = serde_json::from_str(json.trim()).unwrap();
    assert_eq!(report["stopped"], "time");
    // A limit that is not reached: the list without it, not marked.
    let args = [
        table,
        "-t",
        "2",
        "--max-length",
        "11",
        "--time-limit",
        "3600",
    ];
    let (roomy, _) = parse_lists(&ok(&run(solve, &args, LONG_LIST)), 1);
    assert!(roomy[0].1.is_none());
    assert_eq!(roomy[0].2, full[0].2);
    let args = [table, "-t", "2", "--plus", "0", "--json"];
    let json = ok(&run(solve, &args, LONG_LIST));
    let value: serde_json::Value = serde_json::from_str(json.lines().next().unwrap()).unwrap();
    assert!(value["stopped"].is_null(), "{json}");
    // Refused: no time, and a limit without a list.
    for args in [
        &["--plus", "1", "--time-limit", "0"][..],
        &["--plus", "1", "--time-limit", "soon"][..],
        &["--time-limit", "5"][..],
    ] {
        let out = run(solve, &[&[table, "-t", "2"], args].concat(), LONG_LIST);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(text(&out.stderr).contains("--time-limit"), "{args:?}");
    }
}

/// Ctrl-C during a list: the search stops, what was found is printed as
/// cancelled, the cubes not started are left out and the exit status is
/// 130.
#[cfg(unix)]
#[test]
fn solve_list_interrupted() {
    let table = h0();
    let solve = env!("CARGO_BIN_EXE_solve");
    // Start a list with `-v`, wait until its log says that the search
    // starts (`marker`), let it search for a while and send SIGINT.
    let interrupt = |args: &[&str], stdin: &str, marker: &str| {
        let mut child = start(solve, &[args, &["-v"]].concat(), stdin, &[]);
        let mut log = std::io::BufReader::new(child.stderr.take().unwrap());
        let mut line = String::new();
        while !line.contains(marker) {
            line.clear();
            let read = std::io::BufRead::read_line(&mut log, &mut line).unwrap();
            assert!(read > 0, "the search did not start");
        }
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let pid = libc::pid_t::try_from(child.id()).unwrap();
        // SAFETY: a signal to our own child process.
        assert_eq!(unsafe { libc::kill(pid, libc::SIGINT) }, 0);
        child.wait_with_output().unwrap()
    };
    let list = ["--max-length", "17", "--max-solutions", "100000000"];
    let args = [&[table, "-t", "2", "-c", "1"], &list[..]].concat();
    let out = interrupt(
        &args,
        &format!("{LONG_LIST}U2 D2 R2 L2\n"),
        " cubes, at most ",
    );
    assert_eq!(out.status.code(), Some(130));
    let output = text(&out.stdout);
    let (lists, total) = parse_lists(&output, 1);
    let line = lists[0].1.clone().expect("no stopped line");
    assert_eq!(line[..3], ["stopped", "0", "cancelled"]);
    assert!(lists[0].2.len() > 10, "what was found is listed");
    // (The second cube was not started: the total is of the first.)
    assert!(total.starts_with("total 1 cubes "), "{total}");
    // --fast and JSON.
    let fast = ["--fast", "--table", table, "-t", "2", "--json"];
    let args = [&fast[..], &list[..], &["R U F2 D L' B"]].concat();
    let out = interrupt(&args, "", "listing, ");
    assert_eq!(out.status.code(), Some(130));
    let report: serde_json::Value = serde_json::from_str(text(&out.stdout).trim()).unwrap();
    assert_eq!(report["stopped"], "cancelled");
    assert!(report["count"].as_u64().unwrap() > 10);
    assert_eq!(report["optimal"], 6);
}
