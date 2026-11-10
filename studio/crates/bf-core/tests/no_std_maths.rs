//! Source scan: the engine crates must not call the platform's maths library directly.
//!
//! Every transcendental goes through `bf_core::dmath` (software floats, the same bits on every PC), and fused
//! multiply-add is forbidden, so frames and sound stay bit-identical everywhere. This test walks the engine crates'
//! sources, strips comments and string/char literals, and fails listing `file:line` for each forbidden method call.

use std::fs;
use std::path::{Path, PathBuf};

/// The engine crates whose `src/` trees are scanned.
const ENGINE_CRATES: [&str; 7] = [
    "bf-core",
    "bf-sim",
    "bf-story",
    "bf-picture",
    "bf-sound",
    "bf-content",
    "bf-export",
];

/// The one file allowed to implement the maths (relative to the crates folder, `/`-separated).
const ALLOWED: &str = "bf-core/src/dmath.rs";

/// Method names that are forbidden as `.name(` calls.
const FORBIDDEN: [&str; 24] = [
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "exp", "exp2", "exp_m1", "ln",
    "ln_1p", "log", "log2", "log10", "powf", "powi", "hypot", "cbrt", "mul_add", "sin_cos",
];

/// One forbidden call: 1-based line number and the method name.
#[derive(Debug, PartialEq, Eq)]
struct Hit {
    line: usize,
    name: String,
}

fn is_ident_char(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// Replaces comments and the contents of string and char literals with spaces, keeping every newline so line
/// numbers stay the same. Handles nested block comments, raw strings (`r"…"`, `r#"…"#`, `br`/`cr` prefixes),
/// escapes, char literals (including `'"'`) and lifetimes.
fn strip(src: &str) -> String {
    let s: Vec<char> = src.chars().collect();
    let n = s.len();
    let mut out = String::with_capacity(src.len());
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut i = 0;
    while i < n {
        let c = s[i];
        let next = s.get(i + 1).copied();
        // Line comment (also doc comments).
        if c == '/' && next == Some('/') {
            while i < n && s[i] != '\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }
        // Block comment, possibly nested.
        if c == '/' && next == Some('*') {
            let mut depth = 0usize;
            while i < n {
                if s[i] == '/' && s.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    i += 2;
                } else if s[i] == '*' && s.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(s[i]));
                    i += 1;
                }
            }
            continue;
        }
        // Raw string: `r` (optionally after `b` or `c`) at an identifier boundary, then `#*"`.
        if c == 'r' {
            let start_ok = match i.checked_sub(1).map(|p| s[p]) {
                None => true,
                Some(p) if !is_ident_char(p) => true,
                Some('b' | 'c') => i < 2 || !is_ident_char(s[i - 2]),
                Some(_) => false,
            };
            if start_ok {
                let mut j = i + 1;
                while j < n && s[j] == '#' {
                    j += 1;
                }
                if j < n && s[j] == '"' {
                    let hashes = j - i - 1;
                    out.extend(s[i..=j].iter());
                    i = j + 1;
                    loop {
                        if i >= n {
                            break;
                        }
                        if s[i] == '"' && (1..=hashes).all(|k| s.get(i + k) == Some(&'#')) {
                            out.push('"');
                            out.extend(std::iter::repeat_n('#', hashes));
                            i += 1 + hashes;
                            break;
                        }
                        out.push(blank(s[i]));
                        i += 1;
                    }
                    continue;
                }
            }
        }
        // Ordinary (or byte / C) string with escapes.
        if c == '"' {
            out.push('"');
            i += 1;
            while i < n {
                if s[i] == '\\' {
                    out.push(' ');
                    i += 1;
                    if i < n {
                        out.push(blank(s[i]));
                        i += 1;
                    }
                } else if s[i] == '"' {
                    out.push('"');
                    i += 1;
                    break;
                } else {
                    out.push(blank(s[i]));
                    i += 1;
                }
            }
            continue;
        }
        // Char literal or lifetime / label.
        if c == '\'' {
            if next == Some('\\') {
                // Escaped char literal: skip to the closing quote.
                out.push('\'');
                i += 1;
                out.push(' ');
                i += 1; // the backslash
                if i < n {
                    out.push(' ');
                    i += 1; // the escaped character (may itself be a quote)
                }
                while i < n && s[i] != '\'' && s[i] != '\n' {
                    out.push(' ');
                    i += 1;
                }
                if i < n && s[i] == '\'' {
                    out.push('\'');
                    i += 1;
                }
                continue;
            }
            if next.is_some() && s.get(i + 2) == Some(&'\'') {
                out.push_str("' '");
                i += 3;
                continue;
            }
            // A lifetime or loop label: keep it as code.
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Scans one source text and returns every forbidden `.name(` call (whitespace allowed around the name).
fn scan_source(src: &str) -> Vec<Hit> {
    let stripped = strip(src);
    let mut hits = Vec::new();
    for (idx, line) in stripped.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        for (pos, &c) in chars.iter().enumerate() {
            if c != '.' {
                continue;
            }
            let mut j = pos + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let name_start = j;
            while j < chars.len() && is_ident_char(chars[j]) {
                j += 1;
            }
            let name: String = chars[name_start..j].iter().collect();
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && chars[j] == '(' && FORBIDDEN.contains(&name.as_str()) {
                hits.push(Hit { line: idx + 1, name });
            }
        }
    }
    hits
}

/// All `.rs` files under `dir`, sorted, recursively.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|e| e.expect("directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn engine_crates_use_no_std_maths() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("bf-core sits in the crates folder")
        .to_path_buf();
    let mut problems = Vec::new();
    let mut scanned = 0usize;
    for name in ENGINE_CRATES {
        let src = crates.join(name).join("src");
        assert!(src.is_dir(), "engine crate source folder missing: {}", src.display());
        let mut files = Vec::new();
        rust_files(&src, &mut files);
        for file in files {
            let rel = file
                .strip_prefix(&crates)
                .expect("file is under the crates folder")
                .to_string_lossy()
                .replace('\\', "/");
            if rel == ALLOWED {
                continue;
            }
            let text = fs::read_to_string(&file).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"));
            scanned += 1;
            for hit in scan_source(&text) {
                problems.push(format!(
                    "crates/{rel}:{}: .{}( is not allowed here (use bf_core::dmath)",
                    hit.line, hit.name
                ));
            }
        }
    }
    assert!(scanned > 0, "no engine sources found under {}", crates.display());
    assert!(
        problems.is_empty(),
        "{} direct std maths call(s) found:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[test]
fn self_check_flags_a_direct_call() {
    assert_eq!(
        scan_source("let y = x.sin();"),
        vec![Hit {
            line: 1,
            name: "sin".into()
        }]
    );
}

#[test]
fn self_check_flags_every_forbidden_name() {
    for name in FORBIDDEN {
        let src = format!("let y = x.{name}(a);\nlet z = w\n    . {name} (b);\n");
        let lines: Vec<usize> = scan_source(&src).iter().map(|h| h.line).collect();
        assert_eq!(lines, vec![1, 3], "{name}");
    }
}

#[test]
fn self_check_ignores_comments_and_strings() {
    let src = r####"
// let y = x.sin();
/// Doc: x.cos()
//! Inner doc: x.tan()
/* x.exp() */
/* outer /* inner */ still a comment x.ln() */
let a = "x.sin()";
let b = "escaped \" x.cos() \\";
let c = r"x.tan()";
let d = r#"quote " inside x.exp() "#;
let e = br##"x.powf(2.0) "# still raw"##;
let f = b"x.hypot(y)";
let g = c"x.atan2(y)";
let h = '"'; let i = '\''; let j = '\\'; let k = '\u{1F600}'; let l = b'"';
"####;
    assert_eq!(scan_source(src), vec![]);
}

#[test]
fn self_check_keeps_code_after_literals() {
    let src = r####"let h = '"'; let y = x.sin();
fn f<'a>(x: &'a f64) -> f64 { x.cos() }
let s = "a"; let t = r#"b"#; /* c
spans lines */ let z = q.exp2(1.0);
'outer: loop { let w = v.mul_add(a, b); break 'outer; }
let ok = x.sinx() + x.expect("e") + dmath::sin(x) + x.sqrt() + x.logger(1);
let rs = r#"s"# ; let raw = r#type.ln();
let bar = bar.powi(2);
"####;
    let got: Vec<(usize, String)> = scan_source(src).into_iter().map(|h| (h.line, h.name)).collect();
    assert_eq!(
        got,
        vec![
            (1, "sin".to_string()),
            (2, "cos".to_string()),
            (4, "exp2".to_string()),
            (5, "mul_add".to_string()),
            (7, "ln".to_string()),
            (8, "powi".to_string()),
        ]
    );
}
