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
    if left.get("cache").is_some() {
        // ruling M3-R36: the recording is in the pinned cache; the folder must pin it with the committed SHA-256
        let rel = text(&left["cache"]);
        let pin = bf_testdata::pins(&folder)
            .into_iter()
            .find(|p| p.path == rel)
            .unwrap_or_else(|| panic!("{folder}: fight.json's left recording {rel} is not pinned in pins.json"));
        assert_eq!(
            pin.sha256,
            text(&left["sha256"]),
            "{folder}: the left recording's pin is not fight.json's"
        );
    } else {
        recording_of(left, Some(text(&left["hash"])));
    }
}

/// The recording a reference entry names: `file` (a committed file below `studio/reference/`) or `cache` (ruling
/// M3-R36: a fighter's own recording too big for its folder's budget, in the pinned cache, its SHA-256 committed
/// in pins.json; a missing or changed cache file fails with the command that makes it, so a gate that reads it
/// never passes by comparing nothing). Checked to be the recording described when `hash` is given.
pub fn recording_of(entry: &Value, hash: Option<&str>) -> Recording {
    let rec = match (entry.get("file"), entry.get("cache")) {
        (Some(f), None) => return committed_recording(text(f), hash),
        (None, Some(c)) => cached_recording(text(c)),
        _ => panic!("a recording entry names exactly one of `file` and `cache`: {entry}"),
    };
    if let Some(h) = hash {
        assert_eq!(
            rec.hash(),
            h,
            "the cached recording {} is not the one the reference file describes",
            text(&entry["cache"])
        );
    }
    rec
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
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// `(ruling, folder, gate, what)` rows of a rulings list (`studio/reference/rulings.toml`, the controller's alone).
pub fn rulings_of(text: &str) -> Result<Vec<[String; 4]>, String> {
    let doc: toml::Table = text.parse().map_err(|e| format!("not TOML: {e}"))?;
    let rows = doc
        .get("rulings")
        .and_then(toml::Value::as_array)
        .ok_or("no list `rulings`")?;
    rows.iter()
        .map(|r| {
            let f = |k: &str| {
                r.get(k)
                    .and_then(toml::Value::as_str)
                    .map(str::to_string)
                    .ok_or_else(|| format!("a ruling row without {k}: {r}"))
            };
            Ok([f("ruling")?, f("folder")?, f("gate")?, f("what")?])
        })
        .collect()
}

/// The entries of a folder's `exceptions.toml` text, each checked: a ruling the controller's list holds for exactly
/// this folder, gate and item (a port can never excuse itself), a known gate, what and why.
pub fn check_exceptions(folder: &str, text: &str, rulings: &[[String; 4]]) -> Result<Vec<Exception>, String> {
    let doc: toml::Table = text.parse().map_err(|e| format!("{folder}/exceptions.toml: {e}"))?;
    let rows = doc
        .get("exceptions")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| format!("{folder}/exceptions.toml: no list `exceptions`"))?;
    let mut out = Vec::new();
    for e in rows {
        let field = |k: &str| e.get(k).and_then(toml::Value::as_str).unwrap_or("").to_string();
        let x = Exception {
            gate: field("gate"),
            what: field("what"),
            ruling: field("ruling"),
            reason: field("reason"),
        };
        if !GATES.contains(&x.gate.as_str()) {
            return Err(format!(
                "{folder}/exceptions.toml: unknown gate {:?} ({GATES:?})",
                x.gate
            ));
        }
        if x.what.is_empty() || x.reason.trim().len() < 20 {
            return Err(format!("{folder}/exceptions.toml: say what and why: {e}"));
        }
        let row = [x.ruling.clone(), folder.to_string(), x.gate.clone(), x.what.clone()];
        if !rulings.contains(&row) {
            return Err(format!(
                "{folder}/exceptions.toml: {e} is not a ruling of studio/reference/rulings.toml (ruling {:?}, folder \
                 {folder:?}, gate {:?}, what {:?}): an exception is the controller's decision alone; report the \
                 difference with its frame or sample, stage and numbers and ask the controller",
                x.ruling, x.gate, x.what
            ));
        }
        out.push(x);
    }
    Ok(out)
}

/// The controller's rulings list, `studio/reference/rulings.toml`.
pub fn rulings() -> Vec<[String; 4]> {
    let path = reference("rulings.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("studio/reference/rulings.toml: {e}"));
    rulings_of(&text).unwrap_or_else(|e| panic!("studio/reference/rulings.toml: {e}"))
}

/// The entries of `studio/reference/<folder>/exceptions.toml`, checked by [`check_exceptions`] against the
/// controller's `rulings.toml`. Fails when the file is missing (the export writes it once).
pub fn exceptions(folder: &str) -> Vec<Exception> {
    let path = reference(&format!("{folder}/exceptions.toml"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "studio/reference/{folder}/exceptions.toml is missing: run the export `{}`",
            export_of(folder)
        )
    });
    check_exceptions(folder, &text, &rulings()).unwrap_or_else(|e| panic!("{e}"))
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

/// Fails with the list when any item of one run of a gate failed and is not excepted, and when an exception of
/// that run excuses nothing (stale). `scope` names the run (`left`, `right`, `pair`, ...): every failure's key is
/// `<scope> <item>`, which is what an exception's `what` names.
pub fn report(folder: &str, gate: &str, scope: &str, what: &str, failures: Vec<(String, String)>, total: usize) {
    let failures: Vec<(String, String)> = failures.into_iter().map(|(k, w)| (format!("{scope} {k}"), w)).collect();
    let mine: Vec<String> = exceptions(folder)
        .into_iter()
        .filter(|e| e.gate == gate && e.what.starts_with(&format!("{scope} ")))
        .map(|e| e.what)
        .collect();
    let stale: Vec<&String> = mine.iter().filter(|w| !failures.iter().any(|(k, _)| k == *w)).collect();
    assert!(
        stale.is_empty(),
        "{folder} {gate}: exceptions that excuse nothing any more: {stale:?}; the controller removes their rulings"
    );
    let (ok, bad): (Vec<_>, Vec<_>) = failures.into_iter().partition(|(key, _)| mine.contains(key));
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

/// The larger of two differences, a NaN counting as infinitely far (`f64::max` drops a NaN, which would let a
/// NaN pass a bar).
pub fn gap_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::INFINITY
    } else {
        a.max(b)
    }
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

    const RULINGS: &str = "rulings = [\n  { ruling = \"M3-R30\", folder = \"fighters/tuba\", gate = \"look\", what = \"left frame 12\" },\n]\n";

    fn entry(ruling: &str, what: &str) -> String {
        format!(
            "[[exceptions]]\ngate = \"look\"\nwhat = \"{what}\"\nruling = \"{ruling}\"\nreason = \"two levels on a few pixels of the glow\"\n"
        )
    }

    #[test]
    fn an_exception_needs_the_controllers_ruling_for_exactly_that_item() {
        let rulings = rulings_of(RULINGS).unwrap();
        assert!(
            check_exceptions("fighters/tuba", "exceptions = []\n", &rulings)
                .unwrap()
                .is_empty()
        );
        let ok = entry("M3-R30", "left frame 12");
        assert_eq!(check_exceptions("fighters/tuba", &ok, &rulings).unwrap().len(), 1);
        // a well-formed ruling that is not in the list, another item, another folder: refused
        for (folder, text) in [
            ("fighters/tuba", entry("M3-R31", "left frame 12")),
            ("fighters/tuba", entry("M3-R30", "left frame 13")),
            ("fighters/oboe", entry("M3-R30", "left frame 12")),
            ("fighters/tuba", entry("", "left frame 12")),
        ] {
            let e = check_exceptions(folder, &text, &rulings).unwrap_err();
            assert!(e.contains("is not a ruling of studio/reference/rulings.toml"), "{e}");
        }
    }

    #[test]
    fn the_committed_rulings_list_reads() {
        let _ = rulings();
    }

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
