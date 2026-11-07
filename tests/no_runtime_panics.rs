//! The shipped code has no intentional panics (user request): no `assert!`,
//! `assert_eq!`, `assert_ne!`, `panic!`, `unreachable!`, `todo!`,
//! `unimplemented!`, `.unwrap()` or `.expect(` outside test code and
//! compile-time contexts. (Clippy's restriction lints, denied at each crate
//! root, cover all but the asserts, which no lint flags; this covers them
//! all, on every target's code.) `debug_assert!`s are allowed: they are not
//! in release builds.
//!
//! How it works: each Rust file of the library, the binaries, build.rs, the
//! wasm module and the benchmarks is read with comments, strings and
//! character literals blanked out. Then these regions are left out:
//! * test code: an item, statement or field after a `#[cfg(test)]` or
//!   `#[cfg(all(test, ...))]` attribute, up to the end of its `{ ... }` block,
//!   or to its `;` or `,` if that comes first;
//! * compile-time code: the initializer of a `const` or `static` item (from
//!   its `=` to its `;`, e.g. `const _: () = assert!(...);`) and inline
//!   `const { ... }` blocks (not `const fn` bodies, which run at run time
//!   too).
//!
//! Any of the macros or calls above left in the rest fails the test, with
//! its file and line. tests/ and examples/ are test and development code,
//! not checked.

use std::path::{Path, PathBuf};

/// What a panicking call looks like (the text before the match must not be
/// part of a longer name).
const FORBIDDEN: [&str; 9] = [
    "assert!",
    "assert_eq!",
    "assert_ne!",
    "panic!",
    "unreachable!",
    "todo!",
    "unimplemented!",
    ".unwrap()",
    ".expect(",
];

/// The Rust files of the shipped code.
fn shipped_files(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    for dir in ["src", "wasm/src", "benches"] {
        walk(&root.join(dir), &mut files);
    }
    for file in ["build.rs", "wasm/build.rs"] {
        files.push(root.join(file));
    }
    files.sort();
    files
}

/// The text with comments, string and character literals replaced by
/// spaces (newlines kept, so that byte offsets and lines stay).
fn code_only(text: &str) -> Vec<u8> {
    let s = text.as_bytes();
    let mut out = s.to_vec();
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for byte in &mut out[from..to] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    };
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        if rest.starts_with(b"//") {
            let end = rest
                .iter()
                .position(|&b| b == b'\n')
                .map_or(s.len(), |n| i + n);
            blank(&mut out, i, end);
            i = end;
        } else if rest.starts_with(b"/*") {
            // (Nested block comments.)
            let (mut depth, mut j) = (0, i);
            while j < s.len() {
                if s[j..].starts_with(b"/*") {
                    depth += 1;
                    j += 2;
                } else if s[j..].starts_with(b"*/") {
                    depth -= 1;
                    j += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    j += 1;
                }
            }
            blank(&mut out, i, j);
            i = j;
        } else if (rest.starts_with(b"r\"") || rest.starts_with(b"r#") || rest.starts_with(b"br"))
            && (i == 0 || !is_ident(s[i - 1]))
            && raw_string_end(s, i).is_some()
        {
            let end = raw_string_end(s, i).unwrap();
            blank(&mut out, i, end);
            i = end;
        } else if rest[0] == b'"' {
            let mut j = i + 1;
            while j < s.len() && s[j] != b'"' {
                j += if s[j] == b'\\' { 2 } else { 1 };
            }
            blank(&mut out, i, (j + 1).min(s.len()));
            i = j + 1;
        } else if rest[0] == b'\'' {
            // A character literal ('x', '\n', '\u{..}'), not a lifetime.
            let end = if rest.get(1) == Some(&b'\\') {
                rest.iter()
                    .skip(2)
                    .position(|&b| b == b'\'')
                    .map(|n| i + n + 3)
            } else {
                (rest.get(2) == Some(&b'\'')).then_some(i + 3)
            };
            if let Some(end) = end {
                blank(&mut out, i, end);
                i = end;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// The end of a raw string literal starting at `i` (`r"..."`, `r#"..."#`,
/// `br"..."`), if there is one.
fn raw_string_end(s: &[u8], i: usize) -> Option<usize> {
    let mut j = i + if s[i] == b'b' { 2 } else { 1 };
    let hashes = s[j..].iter().take_while(|&&b| b == b'#').count();
    j += hashes;
    if s.get(j) != Some(&b'"') {
        return None;
    }
    let close: Vec<u8> = std::iter::once(b'"')
        .chain(std::iter::repeat_n(b'#', hashes))
        .collect();
    let body = j + 1;
    s[body..]
        .windows(close.len())
        .position(|w| w == close.as_slice())
        .map(|n| body + n + close.len())
}

/// The end of the item, statement or field starting at `from`: after its
/// first `{ ... }` block at its own level, or at its `;` or `,` (or at the
/// end of the enclosing block).
fn item_end(code: &[u8], from: usize) -> usize {
    let mut depth = 0i32;
    let mut i = from;
    while i < code.len() {
        match code[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' => depth -= 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
                if depth < 0 {
                    return i;
                }
            }
            b';' | b',' if depth == 0 => return i + 1,
            _ => {}
        }
        i += 1;
    }
    code.len()
}

/// The end of a `const`/`static` initializer: the `;` at the level of the
/// `=` at `from`.
fn initializer_end(code: &[u8], from: usize) -> usize {
    let mut depth = 0i32;
    for (i, &b) in code.iter().enumerate().skip(from) {
        match b {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b';' if depth == 0 => return i + 1,
            _ => {}
        }
    }
    code.len()
}

/// Whether `word` is at `i` of `code` as a whole word.
fn word_at(code: &[u8], i: usize, word: &str) -> bool {
    code[i..].starts_with(word.as_bytes())
        && (i == 0 || !is_ident(code[i - 1]))
        && code.get(i + word.len()).is_none_or(|&b| !is_ident(b))
}

/// The index after the bracket that closes the one at `open`.
fn after_close(code: &[u8], open: usize) -> usize {
    let mut depth = 0i32;
    for (i, &b) in code.iter().enumerate().skip(open) {
        match b {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
    }
    code.len()
}

/// For a `const` or `static` item or statement whose name starts at `from`:
/// the `=` of its initializer, if it has one (a `:` and a type, then `=`,
/// at one level, before any `;` or `{` there: not a `const fn`).
fn initializer_start(code: &[u8], from: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut seen_colon = false;
    for (i, &b) in code.iter().enumerate().skip(from) {
        match b {
            b'[' | b'(' => depth += 1,
            b']' | b')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            b':' if depth == 0 => seen_colon = true,
            b'=' if depth == 0 => {
                let next = code.get(i + 1).copied().unwrap_or(b' ');
                let previous = code[i - 1];
                if next == b'=' || next == b'>' || matches!(previous, b'<' | b'>' | b'!' | b'=') {
                    return None;
                }
                return seen_colon.then_some(i);
            }
            b';' | b'{' | b'}' if depth == 0 => return None,
            _ => {}
        }
    }
    None
}

/// The last character before `i` that is not whitespace.
fn previous_char(code: &[u8], i: usize) -> Option<u8> {
    code[..i]
        .iter()
        .rev()
        .find(|b| !b.is_ascii_whitespace())
        .copied()
}

/// The byte ranges of `code` that are test code or compile-time code.
fn excluded_ranges(code: &[u8]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut i = 0;
    while i < code.len() {
        // `#[cfg(test)]`, `#[cfg(all(test, ...))]`.
        if code[i..].starts_with(b"#[") {
            let close = after_close(code, i + 1);
            let attribute: String = String::from_utf8_lossy(&code[i..close])
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            if attribute == "#[cfg(test)]" || attribute.starts_with("#[cfg(all(test,") {
                let end = item_end(code, close);
                ranges.push((i, end));
                i = end;
                continue;
            }
        }
        // `const X: T = ...;`, `static X: T = ...;`, `const { ... }`.
        // (Not a `'static` lifetime, nor a generic `<const N: usize>`.)
        let keyword = ["const", "static"].into_iter().find(|&k| {
            word_at(code, i, k) && !matches!(previous_char(code, i), Some(b'\'' | b'<' | b','))
        });
        if let Some(keyword) = keyword {
            let after = i + keyword.len();
            let next = code[after..].iter().position(|b| !b.is_ascii_whitespace());
            let next = next.map_or(code.len(), |n| after + n);
            if keyword == "const" && code.get(next) == Some(&b'{') {
                let end = after_close(code, next);
                ranges.push((i, end));
                i = end;
                continue;
            }
            if let Some(equals) = initializer_start(code, next) {
                let end = initializer_end(code, equals);
                ranges.push((i, end));
                i = end;
                continue;
            }
        }
        i += 1;
    }
    ranges
}

#[test]
fn shipped_code_has_no_intentional_panics() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = shipped_files(root);
    assert!(files.len() > 20, "found only {files:?}");
    let mut found = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        let code = code_only(&text);
        let excluded = excluded_ranges(&code);
        for pattern in FORBIDDEN {
            let bytes = pattern.as_bytes();
            for (at, _) in code
                .windows(bytes.len())
                .enumerate()
                .filter(|(_, w)| *w == bytes)
            {
                let starts_word = bytes[0] == b'.' || at == 0 || !is_ident(code[at - 1]);
                let in_excluded = excluded.iter().any(|&(a, b)| (a..b).contains(&at));
                if starts_word && !in_excluded {
                    let line = code[..at].split(|&b| b == b'\n').count();
                    let relative = path.strip_prefix(root).unwrap_or(path);
                    found.push(format!("{}:{line}: {pattern}", relative.display()));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "panicking calls outside test and compile-time code (user request: none):\n{}",
        found.join("\n")
    );
}

/// The scanner itself: test code, constants, comments and strings are left
/// out; run-time asserts and unwraps are found.
#[test]
fn scanner_finds_runtime_panics_only() {
    let text = r##"
        // assert!(comment);
        /* panic!("block") */
        const _: () = assert!(1 < 2);
        pub const X: usize = { assert!(true); 3 };
        static Y: [u8; 2] = { assert!(true); [0; 2] };
        const fn f(x: usize) -> usize { x }
        fn g() -> &'static str { let c = '"'; "assert!(no)" }
        fn h() { let s = r#"panic!("raw")"#; }
        #[cfg(test)]
        mod tests { fn t() { assert!(false); x.unwrap(); } }
        #[cfg(all(test, backend = "x86"))]
        fn cross() { panic!(); }
        struct S { #[cfg(test)] field: bool, other: u8 }
        fn shipped() { assert_eq!(1, 1); }
        fn shipped2(y: Option<u8>) -> u8 { debug_assert!(true); y.unwrap() }
    "##;
    let code = code_only(text);
    let excluded = excluded_ranges(&code);
    let mut hits = Vec::new();
    for pattern in FORBIDDEN {
        let bytes = pattern.as_bytes();
        for (at, _) in code
            .windows(bytes.len())
            .enumerate()
            .filter(|(_, w)| *w == bytes)
        {
            let starts_word = bytes[0] == b'.' || !is_ident(code[at - 1]);
            if starts_word && !excluded.iter().any(|&(a, b)| (a..b).contains(&at)) {
                hits.push(pattern);
            }
        }
    }
    hits.sort_unstable();
    assert_eq!(hits, [".unwrap()", "assert_eq!"]);
}
