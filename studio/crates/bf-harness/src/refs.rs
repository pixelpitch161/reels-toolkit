//! Reading a port's reference folder, and the checks that it is there and whole.
//!
//! Every read fails, never skips: a file that is missing names the export that writes it; a cache file goes through
//! [`bf_testdata::cache_file`], which names the `cache` command.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bf_core::event::Value as Ev;
use bf_core::recording::Recording;
use bf_testdata::{parse_hex_f64, reference_dir, repo_root};

/// `studio/reference/<rel>` (without `bf_testdata::reference`'s own panic: the messages here name the export).
pub fn reference(rel: &str) -> PathBuf {
    reference_dir().join(rel)
}
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The kit's folder name of a fighter: `name.lower().replace(' ', '_')` (`BASS CLARINET` -> `bass_clarinet`).
pub fn slug(name: &str) -> String {
    name.to_lowercase().replace(' ', "_")
}

/// `fighters/<slug>`.
pub fn fighter_folder(name: &str) -> String {
    format!("fighters/{}", slug(name))
}

/// The export that writes a folder (`fighters/tuba` -> `fighter_tuba`).
pub fn export_of(folder: &str) -> String {
    let (area, name) = folder.split_once('/').expect("<area>/<name>");
    let prefix = match area {
        "fighters" => "fighter_",
        "themes" => "theme_",
        _ => "episode_",
    };
    format!("python -m tools.studio_export {prefix}{name}")
}

/// `studio/reference/<folder>/<file>` as JSON, or a failure naming the export that writes it.
pub fn json(folder: &str, file: &str) -> Value {
    let path = reference(&format!("{folder}/{file}"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "studio/reference/{folder}/{file} is missing: run the export `{}` (then `python -m tools.studio_export \
             cache {folder}`), see studio/docs/porting-a-fighter.md",
            export_of(folder)
        )
    });
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("studio/reference/{folder}/{file}: not JSON: {e}"))
}

/// A cache file of the folder as JSON (checked against its pin).
pub fn cache_json(rel: &str) -> Value {
    let bytes = bf_testdata::cache_bytes(rel);
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("studio/reference/.cache/m3/{rel}: not JSON: {e}"))
}

/// A recording named by a reference file: `fighters/flute/left.bfrec` or a pointer such as
/// `fights/flute_vs_trombone/seed3.bfrec` (committed), checked to be the one the file describes when `hash` is given.
pub fn committed_recording(rel: &str, hash: Option<&str>) -> Recording {
    let rec = Recording::read(&reference(rel)).unwrap_or_else(|e| panic!("studio/reference/{rel}: {e}"));
    if let Some(h) = hash {
        assert_eq!(
            rec.hash(),
            h,
            "studio/reference/{rel} is not the recording the reference file describes"
        );
    }
    rec
}

/// A cached recording (checked against its pin).
pub fn cached_recording(rel: &str) -> Recording {
    Recording::read(&bf_testdata::cache_file(rel)).unwrap_or_else(|e| panic!("cache {rel}: {e}"))
}

pub fn text(v: &Value) -> &str {
    v.as_str().unwrap_or_else(|| panic!("{v} is not text"))
}

pub fn list(v: &Value) -> &Vec<Value> {
    v.as_array().unwrap_or_else(|| panic!("{v} is not a list"))
}

pub fn index(v: &Value) -> usize {
    v.as_u64().unwrap_or_else(|| panic!("{v} is not a whole number")) as usize
}

pub fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("{v} is not a number"))
}

/// A float Python wrote as `float.hex()`.
pub fn hexf(v: &Value) -> f64 {
    parse_hex_f64(text(v))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// The SHA-256 of lines joined by `\n` (the exports' `sha_lines`).
pub fn sha_lines(lines: &[String]) -> String {
    sha(lines.join("\n").as_bytes())
}

/// A value of a reference JSON file as a recording holds it: a number with a fraction or an exponent is a float,
/// any other number a whole number (`recording-format.md` section 2). A dict with whole-number keys (a rule such as
/// `roll_rows`) becomes the list of `[key, value]` pairs in key order, as the recording format writes it.
pub fn value(j: &Value) -> Ev {
    match j {
        Value::Null => Ev::Null,
        Value::Bool(b) => Ev::Bool(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) if !n.to_string().contains(['.', 'e', 'E']) => Ev::Int(i),
            _ => Ev::Float(n.as_f64().expect("a number")),
        },
        Value::String(s) => Ev::Str(s.clone()),
        Value::Array(items) => Ev::List(items.iter().map(value).collect()),
        Value::Object(map) => {
            let mut pairs: Vec<(i64, Ev)> = map
                .iter()
                .map(|(k, v)| {
                    (
                        k.parse::<i64>().unwrap_or_else(|_| panic!("a table key {k:?}")),
                        value(v),
                    )
                })
                .collect();
            pairs.sort_by_key(|p| p.0);
            Ev::List(pairs.into_iter().map(|(k, v)| Ev::List(vec![Ev::Int(k), v])).collect())
        }
    }
}

/// A rules table of a reference file (`vars(story.rules(...))`) as the engine's rules map.
pub fn rules_map(j: &Value) -> BTreeMap<String, Ev> {
    j.as_object()
        .unwrap_or_else(|| panic!("rules: {j} is not a table"))
        .iter()
        .map(|(k, v)| (k.clone(), value(v)))
        .collect()
}

/// `studio/<rel>`.
pub fn studio(rel: &str) -> PathBuf {
    repo_root().join("studio").join(rel)
}

// ---------------------------------------------------------------- the folder is there and whole
fn folder_has(folder: &str, files: &[&str]) {
    let dir = reference(folder);
    assert!(
        dir.is_dir(),
        "studio/reference/{folder}/ does not exist: run the export `{}` and `python -m tools.studio_export cache \
         {folder}` (studio/docs/porting-a-fighter.md)",
        export_of(folder)
    );
    for f in files {
        assert!(
            dir.join(f).is_file(),
            "studio/reference/{folder}/{f} is missing: run the export `{}` (a folder made before the proving harness \
             gains it with `python -m tools.studio_export remake {folder}`)",
            export_of(folder)
        );
    }
    // every committed file of the folder is in its manifest shard (bf-testdata's budget test checks the bytes)
    let shard = json(folder, "manifest.json");
    let listed: Vec<String> = list(&shard["entries"])
        .iter()
        .map(|e| text(&e["path"]).to_string())
        .collect();
    for f in files {
        let want = format!("studio/reference/{folder}/{f}");
        assert!(
            listed.contains(&want),
            "{want} is not in its manifest shard: run `python -m tools.studio_export verify`"
        );
    }
}

/// The pins of a folder name only files that exist and match (each is hashed): the cache is the pinned one.
pub fn cache_is_pinned(folder: &str) {
    let pins = bf_testdata::pins(folder);
    assert!(
        !pins.is_empty(),
        "studio/reference/{folder}/pins.json pins nothing: run `{}`",
        export_of(folder)
    );
    for p in &pins {
        bf_testdata::cache_file(&p.path);
    }
}

/// `fighters/<slug>/` holds every file the suites read: `fight.json`, `look.json`, `sound.json`, `voice.json`,
/// `checks.json`, `exceptions.toml`, `pins.json`, `manifest.json`; `left.bfrec` or the committed recording
/// `fight.json` points at.
pub fn fighter_folder_is_whole(name: &str) {
    let folder = fighter_folder(name);
    let files = [
        "fight.json",
        "look.json",
        "sound.json",
        "voice.json",
        "checks.json",
        "exceptions.toml",
        "pins.json",
    ];
    folder_has(&folder, &files);
    let doc = json(&folder, "fight.json");
    assert_eq!(text(&doc["fighter"]), name, "{folder}/fight.json is another fighter's");
    let left = &doc["left"];
    committed_recording(text(&left["file"]), Some(text(&left["hash"])));
}

/// `themes/<name>/` holds `theme.json`, `bed.json`, `exceptions.toml`, `pins.json`.
pub fn theme_folder_is_whole(name: &str) {
    folder_has(
        &format!("themes/{name}"),
        &["theme.json", "bed.json", "exceptions.toml", "pins.json"],
    );
}

/// `episodes/<pair>/` holds every file Template P names.
pub fn episode_folder_is_whole(pair: &str) {
    folder_has(
        &format!("episodes/{pair}"),
        &[
            "episode.json",
            "frames.json",
            "sound.json",
            "checks.json",
            "stats.json",
            "pairs.json",
            "exceptions.toml",
            "pins.json",
        ],
    );
}

// ---------------------------------------------------------------- the exceptions list
/// One entry of a folder's `exceptions.toml`: what a gate may miss, under a controller ruling.
#[derive(Clone, Debug)]
pub struct Exception {
    pub gate: String,
    pub what: String,
    pub ruling: String,
    pub reason: String,
}

/// The gates an exception can name.
pub const GATES: [&str; 8] = [
    "look", "title", "sprites", "sound", "stems", "voice", "checks", "frames",
];

/// The entries of `studio/reference/<folder>/exceptions.toml`. Fails when the file is missing (the export writes it
/// once) and on an entry without a ruling of the controller (`ruling = "M3-R<n>"`), a reason or a known gate: an
/// exception is never a port's own decision.
pub fn exceptions(folder: &str) -> Vec<Exception> {
    let path = reference(&format!("{folder}/exceptions.toml"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "studio/reference/{folder}/exceptions.toml is missing: run the export `{}`",
            export_of(folder)
        )
    });
    let doc: toml::Table = text
        .parse()
        .unwrap_or_else(|e| panic!("studio/reference/{folder}/exceptions.toml: {e}"));
    let rows = doc
        .get("exceptions")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("studio/reference/{folder}/exceptions.toml: no list `exceptions`"));
    rows.iter()
        .map(|e| {
            let field = |k: &str| e.get(k).and_then(toml::Value::as_str).unwrap_or("").to_string();
            let x = Exception {
                gate: field("gate"),
                what: field("what"),
                ruling: field("ruling"),
                reason: field("reason"),
            };
            let ruled = x
                .ruling
                .strip_prefix("M3-R")
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
            assert!(
                ruled,
                "studio/reference/{folder}/exceptions.toml: an entry without a controller ruling ({e}): an exception \
                 needs the controller's ruling (`ruling = \"M3-R<n>\"`); report the difference with its frame or \
                 sample, stage and numbers and ask the controller"
            );
            assert!(
                GATES.contains(&x.gate.as_str()),
                "{folder}/exceptions.toml: unknown gate {:?} ({GATES:?})",
                x.gate
            );
            assert!(
                !x.what.is_empty() && x.reason.trim().len() >= 20,
                "{folder}/exceptions.toml: say what and why: {e}"
            );
            x
        })
        .collect()
}

/// The always-run check of a folder's `exceptions.toml` (the macros' `exceptions` test); prints the entries.
pub fn exceptions_are_ruled(folder: &str) {
    for e in exceptions(folder) {
        println!(
            "{folder}: exception ({}) {} under {}: {}",
            e.gate, e.what, e.ruling, e.reason
        );
    }
}

/// The `what` of the folder's exceptions for one gate.
pub fn excepted(folder: &str, gate: &str) -> Vec<String> {
    exceptions(folder)
        .into_iter()
        .filter(|e| e.gate == gate)
        .map(|e| e.what)
        .collect()
}

/// Fails with the list when any item failed and is not excepted; prints the excepted ones.
pub fn report(folder: &str, gate: &str, what: &str, failures: Vec<(String, String)>, total: usize) {
    let allowed = excepted(folder, gate);
    let (ok, bad): (Vec<_>, Vec<_>) = failures.into_iter().partition(|(key, _)| allowed.contains(key));
    for (key, why) in &ok {
        println!("{folder} {gate}: {key} differs and is excepted: {why}");
    }
    println!(
        "{folder} {gate}: {} of {total} {what} equal the kit's",
        total - ok.len() - bad.len()
    );
    assert!(
        bad.is_empty(),
        "{folder} {gate}: {} of {total} {what} are not the kit's:\n{}",
        bad.len(),
        bad.iter()
            .map(|(k, w)| format!("  {k}: {w}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Python's `float.hex()` of `x`: `0x1.<13 hex digits>p<exp>` (every digit, as Python writes it).
pub fn py_hex(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let bits = x.to_bits();
    let sign = if bits >> 63 == 1 { "-" } else { "" };
    let exp = ((bits >> 52) & 0x7ff) as i64;
    let mant = bits & ((1u64 << 52) - 1);
    if exp == 0 && mant == 0 {
        return format!("{sign}0x0.0p+0");
    }
    let (lead, e) = if exp == 0 { (0, -1022) } else { (1, exp - 1023) };
    let es = if e >= 0 { format!("+{e}") } else { format!("{e}") };
    format!("{sign}0x{lead}.{mant:013x}p{es}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn py_hex_is_pythons() {
        // (python: x.hex())
        assert_eq!(py_hex(1.0), "0x1.0000000000000p+0");
        assert_eq!(py_hex(0.18), "0x1.70a3d70a3d70ap-3");
        assert_eq!(py_hex(-2.5), "-0x1.4000000000000p+1");
        assert_eq!(py_hex(0.0), "0x0.0p+0");
        assert_eq!(py_hex(5e-324), "0x0.0000000000001p-1022");
        for x in [0.18, 1.0, 0.85, 0.6, 1e-300, 123.456] {
            assert_eq!(parse_hex_f64(&py_hex(x)).to_bits(), x.to_bits());
        }
    }
}
