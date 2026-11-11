//! Programs are started one way (plan M6, Global Rules): through `bf_pack::run`, never with a `Command` of their
//! own. This scan fails on any `Command::new` in `bf-pack/src` outside `run.rs`, and in the app's packaging modules
//! (`bf-app/src/{version,report,startup,icon,guide}.rs` and `bf-app/src/update/`) once they exist.

use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries {
        let path = entry.expect("a folder entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn programs_start_one_way() {
    let me = Path::new(env!("CARGO_MANIFEST_DIR"));
    let run_rs = me.join("src").join("run.rs");
    // The scan looks for the right words: the one place that may start a program does.
    let one_way = std::fs::read_to_string(&run_rs).expect("src/run.rs");
    assert!(
        one_way.contains("Command::new"),
        "the scan's pattern no longer matches run.rs"
    );

    let mut files = Vec::new();
    rust_files(&me.join("src"), &mut files);
    files.retain(|f| f != &run_rs);
    let app = me.join("..").join("bf-app").join("src");
    for name in ["version.rs", "report.rs", "startup.rs", "icon.rs", "guide.rs"] {
        let f = app.join(name);
        if f.is_file() {
            files.push(f);
        }
    }
    rust_files(&app.join("update"), &mut files);
    assert!(files.len() >= 8, "too few files scanned: {files:?}");

    let mut found = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        for (i, line) in text.lines().enumerate() {
            if line.contains("Command::new") {
                found.push(format!("{}:{}: {}", f.display(), i + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "start programs through bf_pack::run only:\n{}",
        found.join("\n")
    );
}
