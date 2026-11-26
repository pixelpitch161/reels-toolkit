//! Voice Engine 2's recordings tool: fetches the free recording sets, studies every set of an instrument side by side,
//! measures the shipped set into the instrument's recipe (`bf-content/data/voices/<NAME>.ve2`), checks the engine's
//! notes against the recordings, and writes the yardstick the `engine2_voices` test reads.
//!
//! ```text
//! cargo run --release -p bf-harness --example recipes -- fetch <set>
//! cargo run --release -p bf-harness --example recipes -- study <PIANO|ORGAN>
//! cargo run --release -p bf-harness --example recipes -- measure <PIANO|ORGAN>
//! cargo run --release -p bf-harness --example recipes -- check <PIANO|ORGAN>
//! cargo run --release -p bf-harness --example recipes -- reference <PIANO|ORGAN>
//! ```
//!
//! The recordings live in `studio/reference/.cache/recordings/<set>/` (git-ignored; `BF_RECORDINGS` names another
//! folder), never in the repository. ffmpeg decodes them (`BF_FFMPEG`, else `ffmpeg` on the PATH). What ships is
//! measured only from sets that are CC0 or free for any use; the others are studied, never shipped
//! (`docs/studio/voice-engine-2-sources.md`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use bf_harness::e2measure::{self, Opts, Yard};
use bf_sound::voice::engine2::player::{self, Carry, Shaping, zero};
use bf_sound::voice::engine2::recipe::{Layer, Note, RecipeSet, grid};
use serde_json::{Value, json};

/// A recording set.
struct Set {
    name: &'static str,
    /// What it is and who made it.
    about: &'static str,
    licence: &'static str,
    /// Whether a recipe may be measured from it.
    ships: bool,
    /// Where it comes from: a git repository on GitHub, or a web page only the PC reaches.
    from: From,
}

enum From {
    /// Cloned without its files, then only `paths` checked out.
    Git {
        url: &'static str,
        paths: &'static [&'static str],
    },
    /// Downloaded on the PC into the set's folder, as `how` says.
    Pc { how: &'static str },
}

const SETS: &[Set] = &[
    Set {
        name: "vcsl",
        about: "Versilian Community Sample Library: Steinway B grand (no pedal, releases), pipe organ, Renaissance organ",
        licence: "CC0",
        ships: true,
        from: From::Git {
            url: "https://github.com/sgossner/VCSL",
            paths: &[
                "/Chordophones/Zithers/Grand Piano, Steinway B/NoSus/",
                "/Chordophones/Zithers/Grand Piano, Steinway B/Rel/",
                "/Aerophones/Edge-blown Aerophones/Renaissance Organ/",
            ],
        },
    },
    Set {
        name: "vsco",
        about: "VSCO 2 CE: pipe organ by Simon Dalzell of Ivy Audio, main chorus, quiet and loud",
        licence: "CC0",
        ships: true,
        from: From::Git {
            url: "https://github.com/sgossner/VSCO-2-CE",
            paths: &["/Keys/Organ/"],
        },
    },
    Set {
        name: "salamander",
        about: "Salamander Grand Piano V3 by Alexander Holm: Yamaha C5, 16 strengths (4 studied)",
        licence: "CC BY 3.0 (study only)",
        ships: false,
        from: From::Git {
            url: "https://github.com/sfzinstruments/SalamanderGrandPiano",
            paths: &[
                "/Samples/*v4.flac",
                "/Samples/*v8.flac",
                "/Samples/*v12.flac",
                "/Samples/*v16.flac",
            ],
        },
    },
    Set {
        name: "headroom",
        about: "Headroom Piano by Bengt Nilsson: Yamaha C3, 5 strengths, close microphones",
        licence: "CC BY 4.0 (study only)",
        ships: false,
        from: From::Git {
            url: "https://github.com/sfzinstruments/BengtNilsson.HeadroomPiano",
            paths: &["/Samples/HEADROOM PIANO LEVEL* CLOSE *.flac"],
        },
    },
    Set {
        name: "iowa-piano",
        about: "University of Iowa Musical Instrument Samples: Steinway B, pp mf ff, every key",
        licence: "free for any use, without restrictions (study only)",
        ships: false,
        from: From::Pc {
            how: "download Piano.<pp|mf|ff>.<note>.aiff (Bb0, C1, D1 ... C8: every second key, as VCSL) from theremin.music.uiowa.edu/MISpiano.html into the folder",
        },
    },
    Set {
        name: "palo-burea",
        about: "Bureå Church organ by Lars Palo (GrandOrgue set): principals studied",
        licence: "CC BY-SA 2.5 (study only)",
        ships: false,
        from: From::Pc {
            how: "download the set from familjenpalo.se/vpo/download (listed at sourceforge.net/p/ourorgan/samplesets) and unpack its Principal ranks into the folder",
        },
    },
    Set {
        name: "palo-pitea",
        about: "Piteå MHS organ by Lars Palo (GrandOrgue set): principals studied",
        licence: "CC BY-SA 2.5 (study only)",
        ships: false,
        from: From::Pc {
            how: "download the set from familjenpalo.se/vpo/download (listed at sourceforge.net/p/ourorgan/samplesets) and unpack its Principal ranks into the folder",
        },
    },
];

fn studio() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the studio folder")
}

fn cache() -> PathBuf {
    std::env::var_os("BF_RECORDINGS").map_or_else(|| studio().join("reference/.cache/recordings"), PathBuf::from)
}

fn ffmpeg() -> PathBuf {
    std::env::var_os("BF_FFMPEG").map_or_else(|| PathBuf::from("ffmpeg"), PathBuf::from)
}

fn git(dir: &Path, args: &[&str]) -> Result<(), String> {
    let ok = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .map_err(|e| e.to_string())?
        .success();
    if ok {
        Ok(())
    } else {
        Err(format!("git {}", args.join(" ")))
    }
}

fn fetch(name: &str) -> Result<(), String> {
    let set = SETS
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| format!("no set {name}"))?;
    let dir = cache().join(set.name);
    match &set.from {
        From::Git { url, paths } => {
            if !dir.join(".git").exists() {
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let st = Command::new("git")
                    .args(["clone", "--filter=blob:none", "--no-checkout", "--depth", "1", url])
                    .arg(&dir)
                    .status()
                    .map_err(|e| e.to_string())?;
                if !st.success() {
                    return Err(format!("not reachable here ({url}): fetch it on the PC"));
                }
            }
            let mut args = vec!["sparse-checkout", "set", "--no-cone"];
            args.extend(paths.iter().copied());
            git(&dir, &args)?;
            git(&dir, &["checkout"])?;
            println!("{}: {} ({})", set.name, dir.display(), set.licence);
            Ok(())
        }
        From::Pc { how } => {
            if dir.exists() {
                println!("{}: {} is there", set.name, dir.display());
                Ok(())
            } else {
                Err(format!("{} is fetched on the PC: {how} ({})", set.name, dir.display()))
            }
        }
    }
}

/// One recording of a set: which file, which note it is meant to be, how hard it was played.
#[derive(Clone, Debug)]
struct Rec {
    path: PathBuf,
    midi: f64,
    layer: String,
    strength: f64,
    /// For a piano's release recording: none; for an organ: a pedal note.
    pedal: bool,
}

/// `C4`, `A#0`, `Db4`, `F#7` as MIDI.
fn note_midi(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let base = match b.first()? {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => return None,
    };
    let (acc, rest) = match b.get(1) {
        Some(b'#') => (1, &s[2..]),
        Some(b'b') => (-1, &s[2..]),
        _ => (0, &s[1..]),
    };
    let oct: i32 = rest.parse().ok()?;
    Some(f64::from(12 * (oct + 1) + base + acc))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut items: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    items.sort();
    for p in items {
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n != ".git") {
                walk(&p, out);
            }
        } else {
            out.push(p);
        }
    }
}

fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The recordings of `set` for `inst`.
fn recordings(set: &str, inst: &str) -> Vec<Rec> {
    let mut all = Vec::new();
    walk(&cache().join(set), &mut all);
    let mut out = Vec::new();
    for path in all {
        let s = stem(&path);
        let ps = path.to_string_lossy().into_owned();
        let rec = |midi: Option<f64>, layer: &str, strength: f64, pedal: bool| {
            midi.map(|midi| Rec {
                path: path.clone(),
                midi,
                layer: layer.into(),
                strength,
                pedal,
            })
        };
        let r = match (set, inst) {
            ("vcsl", "PIANO") if ps.contains("/NoSus/") => s.strip_prefix("JHPiano_NoSus_Close_").and_then(|r| {
                let (note, rest) = r.split_once("_vl")?;
                let v: u32 = rest.split('_').next()?.parse().ok()?;
                let strength = [0.0, 0.2, 0.45, 0.7, 0.95][v.min(4) as usize];
                rec(note_midi(note), &format!("vl{v}"), strength, false)
            }),
            ("vcsl", "PIANO-REL") if ps.contains("/Rel/") => s.strip_prefix("JHPiano_NoSusRel_Close_").and_then(|r| {
                let (note, rest) = r.split_once("_vl")?;
                let v: u32 = rest.split('_').next()?.parse().ok()?;
                rec(
                    note_midi(note),
                    &format!("vl{v}"),
                    [0.0, 0.2, 0.45, 0.7, 0.95][v.min(4) as usize],
                    false,
                )
            }),
            ("vcsl", "ORGAN") if ps.contains("Renaissance Organ/Full/") => s
                .strip_prefix("RenOrgan_Full_Room_")
                .or_else(|| s.split("Room_").nth(1))
                .and_then(|r| rec(note_midi(r.split('_').next()?), "full", 1.0, false)),
            ("vsco", "ORGAN") => {
                let num = |p: &str| s.strip_prefix(p).and_then(|r| r.split('_').next()?.parse::<f64>().ok());
                if let Some(n) = num("Rode_Man3Open_") {
                    rec(Some(35.0 + n), "loud", 1.0, false)
                } else if let Some(n) = num("Rode_Pedal_") {
                    rec(Some(23.0 + n), "loud", 1.0, true)
                } else if let Some(n) = num("NT5_Man3Quiet_") {
                    rec(Some(n - 86.0), "quiet", 0.5, false)
                } else if let Some(n) = num("NT5_PedalQuiet_") {
                    rec(Some(n - 40.0), "quiet", 0.5, true)
                } else {
                    None
                }
            }
            ("salamander", "PIANO") => s.split_once('v').and_then(|(note, v)| {
                let v: f64 = v.parse().ok()?;
                rec(note_midi(note), &format!("v{v}"), v / 16.0, false)
            }),
            ("headroom", "PIANO") => s.strip_prefix("HEADROOM PIANO LEVEL").and_then(|r| {
                let (lv, m) = r.split_once(" CLOSE ")?;
                let lv: f64 = lv.parse().ok()?;
                rec(m.parse().ok(), &format!("level{lv}"), lv / 5.0, false)
            }),
            ("iowa-piano", "PIANO") => {
                let parts: Vec<&str> = s.split('.').collect();
                match parts.as_slice() {
                    ["Piano", dynamic, note] => {
                        let st = match *dynamic {
                            "pp" => 0.3,
                            "mf" => 0.6,
                            "ff" => 0.9,
                            _ => continue,
                        };
                        rec(note_midi(note), dynamic, st, false)
                    }
                    _ => None,
                }
            }
            (p, "ORGAN") if p.starts_with("palo-") => {
                let lower = ps.to_lowercase();
                if !(lower.contains("princip") || lower.contains("prestant")) || !lower.ends_with(".wav") {
                    continue;
                }
                // a rank's release recordings sit in rel* folders beside its notes, named the same: not notes
                if path.parent().is_some_and(|d| {
                    d.file_name()
                        .is_some_and(|n| n.to_string_lossy().to_lowercase().starts_with("rel"))
                }) {
                    continue;
                }
                // every rank is filed by key; a 4' rank sounds an octave over its key, a 2' two (the sets'
                // HarmonicNumber 16 and 32 against the 8's 8)
                let feet = path
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
                    .find(|c| c.contains("princip") || c.contains("prestant"))
                    .and_then(|c| {
                        let digits: String = c.chars().rev().take_while(char::is_ascii_digit).collect();
                        digits.chars().rev().collect::<String>().parse::<f64>().ok()
                    });
                let Some(shift) = feet
                    .filter(|f| [1.0, 2.0, 4.0, 8.0, 16.0].contains(f))
                    .map(|f| 12.0 * (8.0 / f).log2())
                else {
                    continue;
                };
                let m: Option<f64> = s.get(..3).and_then(|n| n.parse::<f64>().ok()).map(|m| m + shift);
                rec(m, "principal", 1.0, false)
            }
            _ => None,
        };
        out.extend(r);
    }
    out.sort_by(|a, b| a.midi.total_cmp(&b.midi).then(a.strength.total_cmp(&b.strength)));
    out
}

/// The sets an instrument is studied from, shipped one first.
fn sets_of(inst: &str) -> &'static [&'static str] {
    match inst {
        "PIANO" => &["vcsl", "iowa-piano", "salamander", "headroom"],
        "ORGAN" => &["vsco", "vcsl", "palo-burea", "palo-pitea"],
        _ => &[],
    }
}

/// How an instrument's notes are measured.
fn opts(inst: &str) -> Opts {
    match inst {
        "PIANO" => Opts {
            stretched: true,
            sub: false,
            max_partials: 96,
            end: 6.0,
            floor_db: -70.0,
            dehiss: true,
            top_hz: 18_000.0,
        },
        _ => Opts {
            stretched: false,
            sub: true,
            max_partials: 320,
            end: 4.0,
            floor_db: -70.0,
            dehiss: false,
            top_hz: 10_000.0,
        },
    }
}

/// Where an unedited recording's note begins, 10 ms before it: the first 5 ms frame over a tenth of the loudest
/// frame's level, or over twice the room's noise (the first 50 ms) if that is higher. The Iowa recordings keep the
/// room before each note, often only 15 to 45 dB under it, so the measuring's own onset (-40 dB of the peak) would
/// land on the room.
fn lead_in(x: &[f64]) -> usize {
    const FRAME: usize = 240;
    let rms: Vec<f64> = x
        .chunks(FRAME)
        .map(|c| (c.iter().map(|v| v * v).sum::<f64>() / c.len() as f64).sqrt())
        .collect();
    let loudest = rms.iter().copied().fold(0.0, f64::max);
    let mut room: Vec<f64> = rms.iter().take(10).copied().collect();
    room.sort_by(f64::total_cmp);
    let room = room.get(room.len() / 2).copied().unwrap_or(0.0);
    let over = (0.1 * loudest).max(2.0 * room);
    rms.iter()
        .position(|&v| v > over)
        .map_or(0, |i| (i * FRAME).saturating_sub(2 * FRAME))
}

/// A recording, decoded (an unedited one from its note's start: [`lead_in`]).
fn load(r: &Rec) -> Option<Vec<f64>> {
    let unedited = r.path.starts_with(cache().join("iowa-piano"));
    match e2measure::decode(&r.path, &ffmpeg()).map(|x| if unedited { x[lead_in(&x)..].to_vec() } else { x }) {
        Ok(x) if x.len() > 4800 => Some(x),
        Ok(_) => None,
        Err(e) => {
            eprintln!("{e}");
            None
        }
    }
}

/// A note's loudness: its mean square over 0.3 s from the onset, dB.
fn loudness(x: &[f64]) -> f64 {
    let x = &x[e2measure::onset(x)..];
    let n = x.len().min(14_400);
    let ms = x[..n].iter().map(|v| v * v).sum::<f64>() / n as f64;
    10.0 * ms.max(1e-20).log10()
}

/// One studied note.
struct Studied {
    set: &'static str,
    midi: f64,
    layer: String,
    strength: f64,
    f0: f64,
    loud: f64,
    yard: Yard,
}

fn study_note(set: &'static str, r: &Rec, inst: &str) -> Option<Studied> {
    let x = load(r)?;
    let x = &x[e2measure::onset(&x)..];
    let o = opts(inst);
    let nominal = 440.0 * 2f64.powf((r.midi - 69.0) / 12.0);
    let (f0, b) = e2measure::pitch(x, nominal, o.stretched);
    let off = 1200.0 * (f0 / nominal).log2();
    if off.abs() > 60.0 {
        eprintln!(
            "{}: measured {f0:.2} Hz is {off:.0} cents off its name, skipped",
            r.path.display()
        );
        return None;
    }
    let grid_f0 = if o.sub { f0 / 2.0 } else { f0 };
    Some(Studied {
        set,
        midi: 69.0 + 12.0 * (f0 / 440.0).log2(),
        layer: r.layer.clone(),
        strength: r.strength,
        f0,
        loud: loudness(x),
        yard: e2measure::yard(x, grid_f0, b, None),
    })
}

/// Runs `f` over `items` on every core, keeping the order.
fn par_map<T: Sync, U: Send>(items: &[T], f: impl Fn(&T) -> U + Sync) -> Vec<U> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = items.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let hs: Vec<_> = items
            .chunks(chunk)
            .map(|c| s.spawn(|| c.iter().map(&f).collect::<Vec<U>>()))
            .collect();
        hs.into_iter().flat_map(|h| h.join().expect("a worker")).collect()
    })
}

/// The register a MIDI note is in, for the study's tables.
fn region(midi: f64) -> &'static str {
    if midi < 48.0 {
        "low"
    } else if midi < 72.0 {
        "middle"
    } else {
        "high"
    }
}

fn study(inst: &str) -> Result<(), String> {
    let mut all: Vec<Studied> = Vec::new();
    let mut missing = Vec::new();
    for &set in sets_of(inst) {
        let mut recs = recordings(set, inst);
        if inst == "PIANO" && set != "vcsl" {
            // the other pianos at the shipped set's pitches (a whole-tone grid), so the tables line up
            recs.retain(|r| (r.midi as i64 - 22).rem_euclid(2) == 0 || set == "salamander" || set == "headroom");
        }
        if recs.is_empty() {
            missing.push(set);
            continue;
        }
        eprintln!("{set}: {} recordings", recs.len());
        all.extend(par_map(&recs, |r| study_note(set, r, inst)).into_iter().flatten());
    }
    let dir = studio().join("reference/.cache/study");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let rows: Vec<Value> = all
        .iter()
        .map(|s| {
            json!({"set": s.set, "midi": s.midi, "layer": s.layer, "strength": s.strength, "f0": s.f0,
                   "loud": s.loud, "yard": s.yard.to_json()})
        })
        .collect();
    let path = dir.join(format!("{inst}.json"));
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&json!({"notes": rows, "missing": missing})).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    // the tables: per register and set, per strength, each number's median over the notes
    println!("# {inst}: every source side by side (median over the register's notes)\n");
    println!(
        "missing (not fetched here): {}\n",
        if missing.is_empty() {
            "none".into()
        } else {
            missing.join(", ")
        }
    );
    println!(
        "| register | source | layer | loudness vs loudest dB | brightness early x f0 | late x f0 | fall dB/s | start ms | noise early % | noise steady % |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|");
    let mut groups: BTreeMap<(usize, &str, &str, String), Vec<&Studied>> = BTreeMap::new();
    for s in &all {
        let reg = region(s.midi);
        let order = ["low", "middle", "high"].iter().position(|r| *r == reg).unwrap_or(0);
        groups
            .entry((order, reg, s.set, format!("{:.2} {}", s.strength, s.layer)))
            .or_default()
            .push(s);
    }
    // the loudest layer of each set and pitch, for the loudness steps
    let mut loudest: BTreeMap<(&str, i64), f64> = BTreeMap::new();
    for s in &all {
        let e = loudest.entry((s.set, s.midi.round() as i64)).or_insert(f64::MIN);
        *e = e.max(s.loud);
    }
    let med = |mut v: Vec<f64>| {
        v.retain(|x| x.is_finite());
        v.sort_by(f64::total_cmp);
        if v.is_empty() { f64::NAN } else { v[v.len() / 2] }
    };
    for ((_, reg, set, layer), v) in &groups {
        let steps = med(v
            .iter()
            .map(|s| s.loud - loudest[&(s.set, s.midi.round() as i64)])
            .collect());
        let be = med(v.iter().map(|s| s.yard.bright_early / s.f0).collect());
        let bl = med(v.iter().map(|s| s.yard.bright_late / s.f0).collect());
        let fall = med(v.iter().map(|s| s.yard.fall).collect());
        let start = med(v.iter().map(|s| s.yard.start * 1000.0).collect());
        let ne = med(v.iter().map(|s| s.yard.noise_early * 100.0).collect());
        let ns = med(v.iter().map(|s| s.yard.noise_steady * 100.0).collect());
        println!(
            "| {reg} | {set} | {layer} | {steps:.1} | {be:.2} | {bl:.2} | {fall:.1} | {start:.0} | {ne:.1} | {ns:.1} |"
        );
    }
    println!("\nwritten {}", path.display());
    Ok(())
}

fn median(mut v: Vec<f64>) -> f64 {
    v.retain(|x| x.is_finite());
    v.sort_by(f64::total_cmp);
    match v.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => v[n / 2],
        n => 0.5 * (v[n / 2 - 1] + v[n / 2]),
    }
}

/// The study's notes of `inst` (written by `study`).
fn studied(inst: &str) -> Result<Vec<Value>, String> {
    let path = studio().join(format!("reference/.cache/study/{inst}.json"));
    let v: Value = serde_json::from_str(
        &std::fs::read_to_string(&path).map_err(|e| format!("{}: {e} (run study)", path.display()))?,
    )
    .map_err(|e| e.to_string())?;
    Ok(v["notes"].as_array().cloned().unwrap_or_default())
}

/// How loud each of the shipped piano's strengths should be, dB under its loudest, from what the other pianos agree
/// on. The shipped set was evened out to one loudness, but its tone still brightens as it was played harder, and so
/// does every other piano's. For each register, each shipped strength's brightness against its loudest strength is
/// matched to the strength of each other piano that brightens as far, and that piano's loudness there is read off;
/// a shipped strength darker than any other piano's softest takes that softest loudness. The median over the other
/// pianos and the registers is the answer.
/// A study table's cell: its notes' brightness and loudness, and its strength.
type Cell = (Vec<f64>, Vec<f64>, f64);

/// A check row: the note, its worst overtone difference, and the start, noise, brightness and fall differences.
type CheckRow = (String, f64, f64, f64, f64, f64);

fn piano_gains(names: &[&str]) -> Result<Vec<f64>, String> {
    let notes = studied("PIANO")?;
    // per (set, register, layer): median brightness and loudness against the set's loudest
    let mut cells: BTreeMap<(String, &str, String), Cell> = BTreeMap::new();
    let mut loudest: BTreeMap<(String, i64), f64> = BTreeMap::new();
    for n in &notes {
        let key = (
            n["set"].as_str().unwrap_or("").to_string(),
            n["midi"].as_f64().unwrap_or(0.0).round() as i64,
        );
        let e = loudest.entry(key).or_insert(f64::MIN);
        *e = e.max(n["loud"].as_f64().unwrap_or(f64::MIN));
    }
    for n in &notes {
        let set = n["set"].as_str().unwrap_or("").to_string();
        let midi = n["midi"].as_f64().unwrap_or(0.0);
        let bright = n["yard"]["bright_early"].as_f64().unwrap_or(f64::NAN) / n["f0"].as_f64().unwrap_or(1.0);
        let loud = n["loud"].as_f64().unwrap_or(f64::NAN) - loudest[&(set.clone(), midi.round() as i64)];
        let c = cells
            .entry((set, region(midi), n["layer"].as_str().unwrap_or("").to_string()))
            .or_default();
        c.0.push(bright);
        c.1.push(loud);
        c.2 = n["strength"].as_f64().unwrap_or(0.0);
    }
    let mut out = Vec::new();
    for name in names {
        let mut reads = Vec::new();
        for reg in ["low", "middle"] {
            let cell = |set: &str, layer: &str| cells.get(&(set.to_string(), reg, layer.to_string()));
            let (Some(mine), Some(top)) = (cell("vcsl", name), cell("vcsl", names[names.len() - 1])) else {
                continue;
            };
            let ratio = median(mine.0.clone()) / median(top.0.clone());
            for set in ["iowa-piano", "salamander", "headroom"] {
                // the set's layers in this register, softest first: (brightness against its loudest, loudness)
                let mut ls: Vec<(f64, f64, f64)> = cells
                    .iter()
                    .filter(|(k, _)| k.0 == set && k.1 == reg)
                    .map(|(_, v)| (v.2, median(v.0.clone()), median(v.1.clone())))
                    .collect();
                if ls.len() < 2 {
                    continue;
                }
                ls.sort_by(|a, b| a.0.total_cmp(&b.0));
                let top_b = ls[ls.len() - 1].1;
                let pts: Vec<(f64, f64)> = ls.iter().map(|l| (l.1 / top_b, l.2)).collect();
                let read = if ratio <= pts[0].0 {
                    pts[0].1
                } else if ratio >= pts[pts.len() - 1].0 {
                    0.0
                } else {
                    pts.windows(2)
                        .find(|w| ratio >= w[0].0.min(w[1].0) && ratio <= w[0].0.max(w[1].0))
                        .map_or(0.0, |w| {
                            let t = (ratio - w[0].0) / (w[1].0 - w[0].0);
                            w[0].1 + t * (w[1].1 - w[0].1)
                        })
                };
                reads.push(read);
            }
        }
        let g = median(reads);
        out.push(if g.is_finite() { g.min(0.0) } else { 0.0 });
    }
    Ok(out)
}

/// The shipped recipes of `inst`, measured.
fn measure(inst: &str) -> Result<(), String> {
    let o = opts(inst);
    let (recs, layer_names, set_name): (Vec<Rec>, Vec<&str>, &str) = match inst {
        "PIANO" => (recordings("vcsl", "PIANO"), vec!["vl2", "vl3", "vl4"], "vcsl"),
        "ORGAN" => {
            let mut r = recordings("vsco", "ORGAN");
            // a pedal note where a manual note of the same pitch is recorded is left out
            let manual: Vec<(String, i64)> = r
                .iter()
                .filter(|x| !x.pedal)
                .map(|x| (x.layer.clone(), x.midi as i64))
                .collect();
            r.retain(|x| !x.pedal || !manual.contains(&(x.layer.clone(), x.midi as i64)));
            (r, vec!["quiet", "loud"], "vsco")
        }
        _ => return Err(format!("no recipe for {inst}")),
    };
    if recs.is_empty() {
        return Err(format!("no {set_name} recordings: run fetch {set_name}"));
    }
    let strengths: Vec<f64> = layer_names
        .iter()
        .map(|n| recs.iter().find(|r| r.layer == *n).map_or(1.0, |r| r.strength))
        .collect();
    eprintln!("{inst}: measuring {} recordings", recs.len());
    let mut notes: Vec<Note> = par_map(&recs, |r| {
        let x = load(r)?;
        let layer = layer_names.iter().position(|n| *n == r.layer)?;
        let n = e2measure::measure(&x, r.midi, layer, &o);
        let off = (n.midi - r.midi) * 100.0;
        if off.abs() > 60.0 {
            eprintln!(
                "{}: measured {:.2} Hz is {off:.0} cents off its name",
                r.path.display(),
                n.f0
            );
            return None;
        }
        Some(n)
    })
    .into_iter()
    .flatten()
    .collect();
    let gains: Vec<f64> = match inst {
        "PIANO" => piano_gains(&layer_names)?,
        _ => {
            // the quiet chorus against the loud, from the recordings' own levels (clamped: the two were taken with
            // different microphones)
            let lv = |layer: usize| -> BTreeMap<i64, f64> {
                notes
                    .iter()
                    .filter(|n| n.layer == layer)
                    .map(|n| {
                        let top = n
                            .partials
                            .iter()
                            .flat_map(|p| p.level.iter())
                            .fold(-200.0_f32, |m, &v| m.max(v));
                        (n.midi.round() as i64, f64::from(top))
                    })
                    .collect()
            };
            let (q, l) = (lv(0), lv(1));
            let d = median(q.iter().filter_map(|(m, v)| l.get(m).map(|w| v - w)).collect());
            vec![if d.is_finite() { d.clamp(-12.0, -6.0) } else { -6.0 }, 0.0]
        }
    };
    // how fast each note stops
    match inst {
        "PIANO" => {
            let rel = recordings("vcsl", "PIANO-REL");
            let taus: Vec<(f64, f64)> = par_map(&rel, |r| {
                let x = load(r)?;
                let f0 = notes
                    .iter()
                    .min_by(|a, b| (a.midi - r.midi).abs().total_cmp(&(b.midi - r.midi).abs()))?
                    .f0;
                Some((r.midi, e2measure::release(&x, f0, Some(0.0))?))
            })
            .into_iter()
            .flatten()
            .filter(|t| t.1 > 0.01 && t.1 < 1.0)
            .collect();
            for n in &mut notes {
                let near: Vec<f64> = taus
                    .iter()
                    .filter(|t| (t.0 - n.midi.round()).abs() < 0.5)
                    .map(|t| t.1)
                    .collect();
                n.release = if near.is_empty() {
                    taus.iter()
                        .min_by(|a, b| (a.0 - n.midi).abs().total_cmp(&(b.0 - n.midi).abs()))
                        .map_or(0.07, |t| t.1)
                } else {
                    median(near)
                };
            }
            eprintln!("release time constants from {} recordings: {:?}", taus.len(), {
                let mut v: Vec<String> = taus
                    .iter()
                    .filter(|t| t.0 as i64 % 12 == 0)
                    .map(|t| format!("{}:{:.3}", t.0, t.1))
                    .collect();
                v.dedup();
                v
            });
        }
        _ => {
            let r: Vec<f64> = par_map(&recs, |r| {
                let x = load(r)?;
                let n = notes.iter().find(|n| (n.midi - r.midi).abs() < 0.6)?;
                e2measure::release(&x, n.f0, None)
            })
            .into_iter()
            .flatten()
            .collect();
            let tau = median(r);
            let tau = if tau.is_finite() { tau.clamp(0.03, 0.12) } else { 0.06 };
            for n in &mut notes {
                n.release = tau;
            }
            eprintln!("release time constant: {tau:.3} s");
        }
    }
    notes.sort_by(|a, b| a.layer.cmp(&b.layer).then(a.midi.total_cmp(&b.midi)));
    let set = RecipeSet {
        name: inst.into(),
        grid: grid(o.end),
        layers: layer_names
            .iter()
            .zip(&strengths)
            .zip(&gains)
            .map(|((n, &s), &g)| Layer {
                name: (*n).into(),
                strength: s,
                gain_db: g,
            })
            .collect(),
        notes,
    };
    let bytes = set.to_bytes();
    let path = studio().join(format!("crates/bf-content/data/voices/{inst}.ve2"));
    std::fs::create_dir_all(path.parent().expect("a folder")).map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    for l in &set.layers {
        println!(
            "layer {:<6} strength {:.2} gain {:+.1} dB",
            l.name, l.strength, l.gain_db
        );
    }
    let partials: usize = set.notes.iter().map(|n| n.partials.len()).sum();
    println!(
        "{inst}: {} notes, {:.0} overtones a note, {} bytes -> {}",
        set.notes.len(),
        partials as f64 / set.notes.len().max(1) as f64,
        bytes.len(),
        path.display()
    );
    if bytes.len() > 1_500_000 {
        return Err(format!(
            "{inst}'s recipe is {} bytes, over the 1.5 MB budget: lower max_partials or end",
            bytes.len()
        ));
    }
    Ok(())
}

/// The instrument's shipped recipe, read back.
fn recipe(inst: &str) -> Result<RecipeSet, String> {
    let path = studio().join(format!("crates/bf-content/data/voices/{inst}.ve2"));
    RecipeSet::parse(&std::fs::read(&path).map_err(|e| format!("{}: {e} (run measure)", path.display()))?)
        .map_err(|e| e.to_string())
}

/// A recipe note rebuilt by the engine as recorded: at its own pitch and strength, held `hold` seconds and then
/// stopped with its own release, `secs` long.
fn rebuild(set: &RecipeSet, n: &Note, hold: f64, secs: f64) -> Vec<f64> {
    let strength = set.layers[n.layer].strength;
    let st = move |_: f64| strength;
    let carry = if set.name == "ORGAN" {
        Carry::Steady
    } else {
        Carry::Decay
    };
    let sh = Shaping {
        strength: &st,
        pitch_cents: &zero,
        level_db: &zero,
        tilt_db: &zero,
        noise_db: &zero,
        stop: Some((hold, if n.release > 0.0 { n.release } else { 0.07 })),
        carry,
    };
    player::render(set, n.midi, &sh, (secs * bf_sound::SRF) as usize, 0, 1)
}

fn write_wav(path: &Path, x: &[f64]) -> Result<(), String> {
    let peak = x.iter().fold(1e-9_f64, |m, v| m.max(v.abs()));
    let st: Vec<[f64; 2]> = x.iter().map(|v| [v / peak * 0.5; 2]).collect();
    bf_sound::write_wav16(path, &st).map_err(|e| e.to_string())
}

/// Each recipe note rebuilt against its recording, measured the same way.
fn check(inst: &str, wav_dir: Option<&Path>) -> Result<(), String> {
    let set = recipe(inst)?;
    let (src, sub) = if inst == "PIANO" {
        ("vcsl", false)
    } else {
        ("vsco", true)
    };
    let recs = recordings(src, inst);
    let rows: Vec<Option<CheckRow>> = par_map(&set.notes, |n| {
        let layer = &set.layers[n.layer].name;
        let r = recs
            .iter()
            .find(|r| &r.layer == layer && (r.midi - n.midi).abs() < 0.6)?;
        let x = load(r)?;
        let x = x[e2measure::onset(&x)..].to_vec();
        let y = rebuild(&set, n, 4.0, 6.0);
        let g = if sub { n.f0 / 2.0 } else { n.f0 };
        let (a, b) = (e2measure::yard(&x, g, n.b, None), e2measure::yard(&y, g, n.b, None));
        let mut worst: f64 = 0.0;
        for j in 0..e2measure::YARD_TIMES.len() {
            if (e2measure::YARD_TIMES[j] * bf_sound::SRF) as usize >= x.len() {
                continue;
            }
            for k in 0..16 {
                if a.levels[j][k] > -50.0 {
                    worst = worst.max((a.levels[j][k] - b.levels[j][k]).abs());
                }
            }
        }
        if let Some(dir) = wav_dir
            && (n.midi.round() as i64) % 12 == 0
        {
            let name = format!("{inst}_{}_{layer}", n.midi.round());
            write_wav(&dir.join(format!("{name}_real.wav")), &x[..x.len().min(288_000)]).ok()?;
            write_wav(&dir.join(format!("{name}_rebuilt.wav")), &y).ok()?;
        }
        Some((
            format!("{} {layer}", n.midi.round()),
            worst,
            (a.start - b.start) * 1000.0,
            (a.noise_early - b.noise_early) * 100.0,
            a.bright_early / n.f0 - b.bright_early / n.f0,
            a.fall - b.fall,
        ))
    });
    println!(
        "| note | worst overtone dB | start ms (real - rebuilt) | early noise % | brightness early x f0 | fall dB/s |"
    );
    println!("|---|---|---|---|---|---|");
    let rows: Vec<_> = rows.into_iter().flatten().collect();
    for r in &rows {
        println!(
            "| {} | {:.1} | {:+.0} | {:+.1} | {:+.2} | {:+.1} |",
            r.0, r.1, r.2, r.3, r.4, r.5
        );
    }
    let mut w: Vec<f64> = rows.iter().map(|r| r.1).collect();
    w.sort_by(f64::total_cmp);
    if !w.is_empty() {
        println!(
            "\n{inst}: {} notes; worst overtone difference median {:.1} dB, 90% {:.1} dB, worst {:.1} dB",
            w.len(),
            w[w.len() / 2],
            w[w.len() * 9 / 10],
            w[w.len() - 1]
        );
    }
    Ok(())
}

/// A yardstick row: the note, its differences, its stop and the stop wanted.
type RefRow = (String, e2measure::YardDiff, f64, f64);

/// The rivals each instrument's notes must be farther from than from its own real one, in FluidR3's set
/// (`tests/reference/fluidr3.json`, which has no electric piano or reed organ).
fn rivals(inst: &str) -> (&'static str, &'static [&'static str]) {
    match inst {
        "PIANO" => ("piano", &["harp", "marimba", "nylon guitar"]),
        _ => ("church organ", &["accordion", "drawbar organ", "trumpet"]),
    }
}

/// The yardstick the `engine2_voices` test reads (`tests/reference/engine2/<INST>.json`): each shipped recording in
/// the fighter's register measured, and the real instruments' spread by register from the study (every set studied
/// here); then the engine's notes against both, printed, to set the test's bars.
fn reference(inst: &str) -> Result<(), String> {
    let set = recipe(inst)?;
    let voice = bf_content::fighters::voice(inst).ok_or(format!("no fighter {inst}"))?;
    let engine = voice.engine2().ok_or(format!("{inst} has no engine 2 instrument"))?;
    let (lo, hi) = voice.register();
    let (src, sub) = if inst == "PIANO" {
        ("vcsl", false)
    } else {
        ("vsco", true)
    };
    let recs = recordings(src, inst);
    let picked: Vec<&Note> = set
        .notes
        .iter()
        .filter(|n| n.midi >= f64::from(lo) - 2.0 && n.midi <= f64::from(hi) + 2.0)
        .collect();
    let notes: Vec<Option<Value>> = par_map(&picked, |n| {
        let layer = &set.layers[n.layer];
        let r = recs
            .iter()
            .find(|r| r.layer == layer.name && (r.midi - n.midi).abs() < 0.6)?;
        let x = load(r)?;
        let x = x[e2measure::onset(&x)..].to_vec();
        let g = if sub { n.f0 / 2.0 } else { n.f0 };
        let yard = e2measure::yard(&x, g, n.b, None);
        let print = bf_harness::timbre::fingerprint(&x, n.midi.round() as i32, 1.0, true);
        Some(
            json!({"midi": n.midi, "layer": layer.name, "strength": layer.strength, "f0": n.f0, "grid_f0": g,
                    "b": n.b, "release": n.release, "yard": yard.to_json(), "fingerprint": print.to_json()}),
        )
    });
    let notes: Vec<Value> = notes.into_iter().flatten().collect();
    // the spread of the real instruments, by register, over every set and strength studied
    let mut spread: BTreeMap<&str, [(f64, f64); 4]> = BTreeMap::new();
    for v in studied(inst)? {
        let (Some(midi), Some(f0), Some(y)) = (v["midi"].as_f64(), v["f0"].as_f64(), Yard::from_json(&v["yard"]))
        else {
            continue;
        };
        let vals = [y.bright_early / f0, y.bright_late / f0, y.fall, y.noise_early];
        let e = spread.entry(region(midi)).or_insert([(f64::MAX, f64::MIN); 4]);
        for (i, (slot, v)) in e.iter_mut().zip(vals).enumerate() {
            // a brightness of 0 is a note too short to read there
            if v.is_finite() && (i >= 2 || v > 0.0) {
                *slot = (slot.0.min(v), slot.1.max(v));
            }
        }
    }
    let spread_json: serde_json::Map<String, Value> = spread
        .iter()
        .map(|(reg, s)| {
            (
                reg.to_string(),
                json!({"bright_early": [s[0].0, s[0].1], "bright_late": [s[1].0, s[1].1],
                                     "fall": [s[2].0, s[2].1], "noise_early": [s[3].0, s[3].1]}),
            )
        })
        .collect();
    let out = studio().join(format!("crates/bf-harness/tests/reference/engine2/{inst}.json"));
    std::fs::create_dir_all(out.parent().expect("a folder")).map_err(|e| e.to_string())?;
    let doc = json!({
        "what": format!("Voice Engine 2's yardstick for {inst}: the shipped set's recordings ({src}) in the register, \
                         measured by bf_harness::e2measure::yard, and the spread of every set studied by register \
                         (cargo run --release -p bf-harness --example recipes -- reference {inst})"),
        "notes": notes,
        "spread": spread_json,
    });
    std::fs::write(
        &out,
        serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    println!("written {} ({} notes)", out.display(), notes.len());

    // the engine against the recordings, as the test measures it
    println!(
        "\n| note | early overtone dB (time, overtone) | median dB | any dB | start ms | stop s (ours, want) | noise early % | noise steady % |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    let rows: Vec<RefRow> = par_map(&notes, |v| {
        let (midi, strength, g, b, rel) = (
            v["midi"].as_f64()?,
            v["strength"].as_f64()?,
            v["grid_f0"].as_f64()?,
            v["b"].as_f64()?,
            v["release"].as_f64()?,
        );
        let real = Yard::from_json(&v["yard"])?;
        let y = e2measure::engine2_note(engine, midi, strength, 4.0, 6.0, 0);
        let ours = e2measure::yard(&y, g, b, Some(4.0));
        Some((
            format!("{:.0} {}", midi, v["layer"].as_str()?),
            e2measure::yard_diff(&real, &ours),
            ours.stop.unwrap_or(f64::NAN),
            rel * std::f64::consts::LN_10 * 2.0,
        ))
    })
    .into_iter()
    .flatten()
    .collect();
    for (name, d, stop, want) in &rows {
        println!(
            "| {name} | {:.1} ({}, {}) | {:.1} | {:.1} | {:+.0} | {stop:.3} {want:.3} | {:+.1} | {:+.1} |",
            d.early_db,
            d.early_at.0,
            d.early_at.1,
            d.median_db,
            d.any_db,
            d.start_s * 1000.0,
            d.noise_early * 100.0,
            d.noise_steady * 100.0
        );
    }
    let worst = |f: &dyn Fn(&RefRow) -> f64| {
        let mut v: Vec<f64> = rows.iter().map(f).collect();
        v.sort_by(f64::total_cmp);
        (v[v.len() / 2], v[v.len() * 9 / 10], v[v.len() - 1])
    };
    println!(
        "\n{inst} worst (median, 90%, worst): early overtone {:.1?} dB, median {:.1?} dB, start {:.3?} s, stop {:.3?} s, noise early {:.3?}, noise steady {:.3?}",
        worst(&|r| r.1.early_db),
        worst(&|r| r.1.median_db),
        worst(&|r| r.1.start_s.abs()),
        worst(&|r| (r.2 - r.3).abs()),
        worst(&|r| r.1.noise_early.abs()),
        worst(&|r| r.1.noise_steady.abs())
    );

    println!(
        "{inst} held tone (median, 90%, worst): late median {:.1?} dB, steady overtone {:.1?} dB",
        worst(&|r| r.1.late_median_db),
        worst(&|r| r.1.steady_db)
    );

    // the engine's notes across the register in whole tones against the spread
    println!("\n| note | bright early x f0 | late | fall dB/s | noise early | register's spread |");
    println!("|---|---|---|---|---|---|");
    let grid_notes: Vec<i32> = (lo..=hi).step_by(2).collect();
    let vel = if inst == "PIANO" { 0.8 } else { 1.0 };
    let tones: Vec<(i32, Yard)> = par_map(&grid_notes, |&m| {
        let f0 = 440.0 * 2f64.powf((f64::from(m) - 69.0) / 12.0);
        let c = set.nearest(f64::from(m), vel);
        let b: f64 = c.iter().map(|(i, w)| w * set.notes[i].b).sum();
        let y = e2measure::engine2_note(engine, f64::from(m), vel, 4.0, 6.0, 0);
        (m, e2measure::yard(&y, if sub { f0 / 2.0 } else { f0 }, b, None))
    });
    for (m, y) in &tones {
        let f0 = 440.0 * 2f64.powf((f64::from(*m) - 69.0) / 12.0);
        let s = spread.get(region(f64::from(*m)));
        println!(
            "| {m} | {:.2} | {:.2} | {:.1} | {:.3} | {} |",
            y.bright_early / f0,
            y.bright_late / f0,
            y.fall,
            y.noise_early,
            s.map_or("none".into(), |s| format!(
                "{:.2}-{:.2}, {:.2}-{:.2}, {:.1}-{:.1}, {:.3}-{:.3}",
                s[0].0, s[0].1, s[1].0, s[1].1, s[2].0, s[2].1, s[3].0, s[3].1
            ))
        );
    }

    // the rivals, as real_voices.rs measures them
    let (own, rivals) = rivals(inst);
    let path = studio().join("crates/bf-harness/tests/reference/fluidr3.json");
    let fl: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let refs: BTreeMap<String, BTreeMap<i32, bf_harness::timbre::Fingerprint>> = fl["references"]
        .as_object()
        .ok_or("no references")?
        .iter()
        .map(|(name, r)| {
            let notes = r["notes"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(bf_harness::timbre::Fingerprint::from_json)
                        .filter(|f| f.harmonics.iter().any(|&h| h > -200.0))
                        .map(|f| (f.midi, f))
                        .collect()
                })
                .unwrap_or_default();
            (name.clone(), notes)
        })
        .collect();
    println!("\n| note | to the real {own} | {} |", rivals.join(" | "));
    let mut worst_own: f64 = 0.0;
    let mut closest: f64 = f64::MAX;
    for m in lo..=hi {
        let Some(real) = refs[own].get(&m) else { continue };
        let x = e2measure::engine2_note(engine, f64::from(m), 0.8, 1.0, 2.0, 0);
        let b = bf_harness::timbre::fingerprint(&x, m, 1.0, true);
        let d_own = bf_harness::timbre::distance(&b.harmonics, &real.harmonics);
        worst_own = worst_own.max(d_own);
        let ds: Vec<String> = rivals
            .iter()
            .map(|r| match refs[*r].get(&m) {
                Some(rr) => {
                    let d = bf_harness::timbre::distance(&b.harmonics, &rr.harmonics);
                    closest = closest.min(d - d_own);
                    format!("{d:.1}")
                }
                None => "-".into(),
            })
            .collect();
        println!("| {m} | {d_own:.1} | {} |", ds.join(" | "));
    }
    println!("\n{inst}: worst distance to its own {worst_own:.1}; nearest rival margin {closest:.1}");
    // the recordings themselves against FluidR3, and the engine against the recordings, by the same fingerprint
    println!(
        "\n| note | recording to FluidR3 {own} | engine to recording | engine to {} |",
        rivals.join(" | ")
    );
    let (mut worst_rec, mut margin) = (0.0_f64, f64::MAX);
    for v in &notes {
        let (Some(midi), Some(strength), Some(fr)) = (
            v["midi"].as_f64(),
            v["strength"].as_f64(),
            bf_harness::timbre::Fingerprint::from_json(&v["fingerprint"]),
        ) else {
            continue;
        };
        let m = midi.round() as i32;
        let y = e2measure::engine2_note(engine, midi, strength, 1.0, 2.0, 0);
        let fe = bf_harness::timbre::fingerprint(&y, m, 1.0, true);
        let d_own = bf_harness::timbre::distance(&fe.harmonics, &fr.harmonics);
        worst_rec = worst_rec.max(d_own);
        let to = |a: &bf_harness::timbre::Fingerprint, name: &str| {
            refs[name]
                .get(&m)
                .map(|rr| bf_harness::timbre::distance(&a.harmonics, &rr.harmonics))
        };
        let ds: Vec<String> = rivals
            .iter()
            .map(|r| match to(&fe, r) {
                Some(d) => {
                    margin = margin.min(d - d_own);
                    format!("{d:.1}")
                }
                None => "-".into(),
            })
            .collect();
        println!(
            "| {m} {} | {} | {d_own:.1} | {} |",
            v["layer"].as_str().unwrap_or(""),
            to(&fr, own).map_or("-".into(), |d| format!("{d:.1}")),
            ds.join(" | ")
        );
    }
    println!("\n{inst}: engine to its recordings at most {worst_rec:.1}; nearest rival margin {margin:.1}");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let r = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["fetch", set] => fetch(set),
        ["study", inst] => study(inst),
        ["measure", inst] => measure(inst),
        ["check", inst] => check(inst, None),
        ["check", inst, "--wav", dir] => check(inst, Some(Path::new(dir))),
        ["reference", inst] => reference(inst),
        ["sets"] => {
            for s in SETS {
                let how = match &s.from {
                    From::Git { url, .. } => url,
                    From::Pc { .. } => "the PC",
                };
                println!(
                    "{:<12} {:<45} ships: {:<5} {}  ({how})",
                    s.name, s.licence, s.ships, s.about
                );
            }
            Ok(())
        }
        _ => Err("usage: recipes fetch <set> | study <PIANO|ORGAN> | measure | check | reference | sets".into()),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
