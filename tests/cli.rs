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
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("egg-solver-cli-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    })
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
    child.wait_with_output().unwrap()
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
    check_solve_output(&ok(&out), 4, &threads, "40", "0");
    assert_eq!(text(&out.stderr), "", "stderr as before: nothing");
    // Scripts: `solve <table> -t T -c C [-n N] [-l L]`.
    let out = run(solve, &[table, "-t", "2", "-c", "1", "-n", "3"], SCRAMBLES);
    check_solve_output(&ok(&out), 3, "2", "40", "1");
    let out = run(solve, &[table, "-t", "2", "-l", "4", "-c", "0"], SCRAMBLES);
    check_solve_output(&ok(&out), 4, "2", "4", "0");
    // runtest.sh sets EGG_SOLVER_LOAD.
    for load in ["copy", "map", "auto", ""] {
        let env = [("EGG_SOLVER_LOAD", load)];
        let out = run_env(solve, &[table, "-t", "2", "-c", "1"], SCRAMBLES, &env);
        check_solve_output(&ok(&out), 4, "2", "40", "1");
    }
}

/// `solve --fast` (fast.sh's lines), with a given table and scramble and a
/// short warm-up cube (fast.sh's own takes minutes on h0).
#[test]
fn solve_fast() {
    let table = h0();
    let solve = env!("CARGO_BIN_EXE_solve");
    let fast = ["--fast", "-t", "2", "--warmup", "F2 D"];
    for args in [
        [&fast[..], &["--table", table, "R", "U", "R'", "U'"]].concat(),
        [&[table], &fast[..], &["R U R' U'"]].concat(),
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
    assert!(log.contains("2 threads, 40 lanes per thread"), "{log}");
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
        check_solve_output(&ok(&out), 4, "2", "40", "1");
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
    check_solve_output(&ok(&out), 4, "2", "40", "1");
    let args = [table, "-t", "2", "-c", "1", "-v", "--load", "copy"];
    let out = run_env(solve, &args, SCRAMBLES, &env);
    check_solve_output(&ok(&out), 4, "2", "40", "1");
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
