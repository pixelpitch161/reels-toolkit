//! Source scans of the ports' files.

use crate::refs::studio;

/// The one form in which a look's code may index the picture's roster entries: its `setup` handing its own
/// numbers to `setup_as` (FLUTE's and TROMBONE's looks). Every other number a look reads from the roster comes
/// from the `cfg` that `setup_as` is given, so EUPHONIUM's copy of a look (made for its own side with the copied
/// fighter's numbers) draws with the copied fighter's numbers.
pub const ALLOWED: &str = "self.setup_as(p, k, &p.cfg[k]);";

/// The code of a line, without a `//` comment (doc comments included); string literals are not looked into, as no
/// look writes `cfg[` in one.
fn code(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

/// The places in `text` (a look's source) that index `cfg[` other than [`ALLOWED`]: `(line number, line)`.
pub fn cfg_indexing(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| code(l).contains("cfg["))
        .filter(|(_, l)| code(l).trim() != ALLOWED)
        .map(|(i, l)| (i + 1, l.trim().to_string()))
        .collect()
}

/// In `bf-content/src/fighters/*/look.rs`, an index into `cfg[` appears only as `self.setup_as(p, k,
/// &p.cfg[k]);`: a look reads its roster numbers from the `cfg` of `setup_as`, never `p.cfg[self.k]`.
pub fn looks_take_numbers_from_setup_as() {
    let dir = studio("crates/bf-content/src/fighters");
    let mut bad = Vec::new();
    let mut looks = 0;
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("a folder entry").path().join("look.rs");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        looks += 1;
        for (n, line) in cfg_indexing(&text) {
            bad.push(format!("  {}:{n}: {line}", path.display()));
        }
    }
    assert!(looks >= 20, "only {looks} look.rs files under {}", dir.display());
    assert!(
        bad.is_empty(),
        "a look reads the picture's roster entries directly; take the numbers from `cfg` in setup_as and make \
         setup call `{ALLOWED}`:\n{}",
        bad.join("\n")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scan_finds_a_look_reading_its_own_side() {
        let good = "    fn setup(&mut self, p: &PictureCtx, k: usize) {\n        self.setup_as(p, k, &p.cfg[k]);\n    }\n\
                    /// `cfg['view'][2]`: the roll\n        // self.cfg['view'][2] (p.view[k] is p.cfg[k]'s view)\n";
        assert!(cfg_indexing(good).is_empty());
        let bad = "        let rad = p.cfg[self.k].low.get(\"radius\");\n        let x = &p.cfg[1 - k];\n";
        let found = cfg_indexing(bad);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].0, 1);
    }
}
