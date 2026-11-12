//! Source scans of the ports' files.

use crate::refs::studio;

/// The one form in which a fighter's picture code may read the picture's per-side roster entries: its look's
/// `setup` handing its own side's entry to `setup_as` (FLUTE's and TROMBONE's looks). Every roster number a look,
/// its card or its state reads comes from the `cfg` that `setup_as` is given, so EUPHONIUM's copy of a look (made for
/// its own side with the copied fighter's numbers) draws with the copied fighter's numbers.
pub const ALLOWED: &str = "self.setup_as(p, k, &p.cfg[k]);";

/// What reads the picture's per-side config or view, or looks a fighter up by name: `p.cfg`, any `.cfg.` /
/// `.cfg[` (a stored or borrowed roster entry, `.cfg.get(` among them), `p.view` / `.view[` (the per-side view,
/// which is `p.cfg[k]`'s), the roster (`roster()`) and the registry's per-name lookups (`fighters::entry(`,
/// `look_for(`, `mesh_json(`). `p.fighter(k, t)` (a ball's state) is not a roster lookup and is allowed.
pub const FORBIDDEN: [&str; 9] = [
    "p.cfg",
    ".cfg.",
    ".cfg[",
    "p.view",
    ".view[",
    "roster()",
    "fighters::entry(",
    "look_for(",
    "mesh_json(",
];

/// The files of a fighter folder the scan reads: every `.rs` but the sound half (`sound.rs`, `voice.rs`), which
/// reads the music context, not the picture.
pub fn scanned(file: &str) -> bool {
    file.ends_with(".rs") && file != "sound.rs" && file != "voice.rs"
}

/// The code of a line, without a `//` comment (doc comments included); string literals are not looked into, as no
/// fighter file writes these patterns in one.
fn code(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

/// IDENTITY CRISIS's copy (`euphonium.py` 192 to 201): EUPHONIUM's look makes the enemy's own look for the
/// euphonium's side with the enemy's roster entry, the one place a kit look reads the other side's entry and looks a
/// fighter up by name. These two lines are allowed in that file only (`COPY_FILE`, its folder and name), and there only
/// inside `fn setup_as`.
pub const COPY_FILE: (&str, &str) = ("euphonium", "look.rs");

/// The two lines of [`COPY_FILE`]'s `setup_as` that make the copy.
pub const COPY_LINES: [&str; 2] = [
    "let enemy = &p.cfg[e];",
    "let mut dup = crate::fighters::look_for(&enemy.name);",
];

/// The places in `text` (a fighter file's source) that read the picture's per-side config or view other than
/// [`ALLOWED`]: `(line number, line)`.
pub fn cfg_indexing(text: &str) -> Vec<(usize, String)> {
    cfg_indexing_in("", "", text)
}

/// The 0-based line range of `fn setup_as`'s body in `text`: from its signature to the closing brace at the
/// signature's own indent.
fn setup_as_lines(text: &str) -> Option<(usize, usize)> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| code(l).contains("fn setup_as("))?;
    let indent = &lines[start][..lines[start].len() - lines[start].trim_start().len()];
    let close = format!("{indent}}}");
    let end = lines[start + 1..].iter().position(|l| code(l).trim_end() == close)? + start + 1;
    Some((start, end))
}

/// [`cfg_indexing`] for the file `file` of the fighter folder `folder`: in [`COPY_FILE`] the [`COPY_LINES`] are
/// allowed inside `fn setup_as` (and flagged anywhere else, in it or in any other file).
pub fn cfg_indexing_in(folder: &str, file: &str, text: &str) -> Vec<(usize, String)> {
    let copy = if (folder, file) == COPY_FILE {
        setup_as_lines(text)
    } else {
        None
    };
    text.lines()
        .enumerate()
        .filter(|(_, l)| code(l).trim() != ALLOWED)
        .filter(|&(i, l)| !(copy.is_some_and(|(a, b)| a < i && i < b) && COPY_LINES.contains(&code(l).trim())))
        .filter(|(_, l)| FORBIDDEN.iter().any(|f| code(l).contains(f)))
        .map(|(i, l)| (i + 1, l.trim().to_string()))
        .collect()
}

/// In every scanned file of `bf-content/src/fighters/<slug>/` (not `shared/`), the picture's per-side config and
/// view are read only in [`ALLOWED`]: a look reads its roster numbers from the `cfg` of `setup_as`.
pub fn looks_take_numbers_from_setup_as() {
    let dir = studio("crates/bf-content/src/fighters");
    let mut bad = Vec::new();
    let (mut folders, mut files) = (0, 0);
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let folder = entry.expect("a folder entry").path();
        if !folder.is_dir() || folder.file_name().is_some_and(|n| n == "shared") {
            continue;
        }
        folders += 1;
        for f in std::fs::read_dir(&folder).unwrap_or_else(|e| panic!("{}: {e}", folder.display())) {
            let path = f.expect("a file entry").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            if !scanned(&name) {
                continue;
            }
            files += 1;
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let slug = folder.file_name().and_then(|n| n.to_str()).unwrap_or("");
            for (n, line) in cfg_indexing_in(slug, &name, &text) {
                bad.push(format!("  {}:{n}: {line}", path.display()));
            }
        }
    }
    assert!(
        folders >= 20 && files >= 40,
        "only {folders} fighter folders, {files} files under {}",
        dir.display()
    );
    assert!(
        bad.is_empty(),
        "a fighter's picture code reads the picture's per-side roster entries or view directly; take the numbers \
         from `cfg` in setup_as and make setup exactly `{ALLOWED}`:\n{}",
        bad.join("\n")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scan_finds_a_look_reading_its_own_side() {
        let good = "    fn setup(&mut self, p: &PictureCtx, k: usize) {\n        self.setup_as(p, k, &p.cfg[k]);\n    }\n\
                    /// `cfg['view'][2]`: the roll\n        // self.cfg['view'][2] (p.view[k] is p.cfg[k]'s view)\n\
                    let at = p.fighter(kk, t).p;\n        self.low = cfg.low.get(\"radius\");\n";
        assert!(cfg_indexing(good).is_empty(), "{:?}", cfg_indexing(good));
        let bad = "        let rad = p.cfg[self.k].low.get(\"radius\");\n        let x = &p.cfg[1 - k];\n\
                   let roll = p.view[k].2;\n        let r = self.cfg.low.get(\"radius\");\n\
                   let f = bf_content::roster().iter().find(|f| f.name == \"TUBA\");\n\
                   let m = crate::fighters::entry(\"TUBA\");\n";
        let found = cfg_indexing(bad);
        assert_eq!(
            found.iter().map(|f| f.0).collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6],
            "{found:?}"
        );
        assert!(scanned("look.rs") && scanned("state.rs") && scanned("card.rs") && scanned("mod.rs"));
    }

    /// IDENTITY CRISIS's two lines pass in EUPHONIUM's look inside `setup_as` only: elsewhere in that file, in its
    /// other files and in any other fighter's look they are still flagged.
    #[test]
    fn the_copys_two_lines_pass_only_in_the_euphoniums_setup_as() {
        let body = format!(
            "        {}
        {}
",
            COPY_LINES[0], COPY_LINES[1]
        );
        let inside = format!(
            "impl Look for EuphoniumLook {{
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {{
             {body}        dup.setup_as(p, k, enemy);
    }}

    fn state(&self) {{
{body}    }}
}}
"
        );
        let found = cfg_indexing_in("euphonium", "look.rs", &inside);
        assert_eq!(found.iter().map(|f| f.0).collect::<Vec<_>>(), [9, 10], "{found:?}");
        assert_eq!(cfg_indexing_in("euphonium", "state.rs", &inside).len(), 4);
        assert_eq!(cfg_indexing_in("trumpet", "look.rs", &inside).len(), 4);
        assert_eq!(cfg_indexing(&inside).len(), 4);
        assert!(!scanned("sound.rs") && !scanned("voice.rs"));
    }
}
