//! The sound half of Template F (`fighters/<slug>/sound.json`), and the soundtrack comparison the theme and pair
//! gates share: milestone 1's `stems.rs` bars (the 16-bit master sample for sample by its SHA-256, the RMS of every
//! 100 ms block within 0.01 dB above -90 dBFS, the first-time notes in the kit's order, the noise stream where the
//! kit's ended, stem windows within -120 dBFS of the kit's float32).

use bf_content::SoundBuild;
use bf_core::rng::Generator;
use bf_core::show::Show;
use bf_sound::SR;
use bf_sound::master::Soundtrack;
use bf_sound::mix::Mix;
use bf_sound::traits::BaseSound;
use bf_sound::voice::PpNoise;
use bf_sound::wav::sample_i16;
use bf_testdata::Npz;
use serde_json::Value;

use crate::fight;
use crate::refs::{
    cached_recording, fighter_folder, index, json, list, num, py_hex, recording_of, report, sha, sha_lines, text,
};

/// A 100 ms block's RMS against the kit's: within this many dB.
pub const RMS_DB: f64 = 0.01;
/// Blocks the kit has below this many dBFS are only checked to be below it.
pub const RMS_FLOOR: f64 = -90.0;
/// A stem window's samples: within this many dBFS of the kit's (float32).
pub const DB: f64 = -120.0;

/// What a soundtrack gives the comparison.
pub struct Summary {
    pub st: Soundtrack,
    pub buses: Vec<String>,
    pub notes: Vec<String>,
    pub noise: PpNoise,
}

/// The RMS of each 100 ms block in dB over both channels (`stems.py`'s formula), to 3 decimals as the export
/// rounds it.
pub fn block_db3(x: &[[f64; 2]]) -> Vec<f64> {
    x.chunks(SR / 10)
        .map(|b| {
            let flat = b.as_flattened();
            let mean = flat.iter().map(|v| v * v).sum::<f64>() / flat.len() as f64;
            let db = 20.0 * bf_core::dmath::log10(mean.sqrt() + 1e-12);
            (db * 1000.0).round() / 1000.0
        })
        .collect()
}

/// The SHA-256 of the 16-bit master's samples (little-endian, interleaved: what `A.write` writes after its
/// header).
pub fn pcm_sha(master: &[[f64; 2]]) -> String {
    let mut pcm = Vec::with_capacity(master.len() * 4);
    for f in master {
        for &x in f {
            pcm.extend(sample_i16(x).to_le_bytes());
        }
    }
    sha(&pcm)
}

/// Score and master a build; the notes as the export writes them (`name midi dur cap vel`, floats as
/// `float.hex()`).
pub fn summarise(build: &SoundBuild) -> Summary {
    let mix: Mix = build.score().unwrap_or_else(|e| panic!("the score: {e}"));
    let buses = mix.bus_names().iter().map(|b| b.to_string()).collect();
    let notes = build
        .ctx
        .first_notes()
        .expect("the note log")
        .iter()
        .map(|c| {
            format!(
                "{} {} {} {} {}",
                c.name,
                c.midi,
                py_hex(c.dur),
                py_hex(c.cap),
                py_hex(c.vel)
            )
        })
        .collect();
    let noise = build.ctx.noise().expect("the noise stream");
    Summary {
        st: build.master(&mix),
        buses,
        notes,
        noise,
    }
}

/// The RMS profile `got` against the kit's 3-decimal `want`.
fn profile(what: &str, got: &[f64], want: &[Value], failures: &mut Vec<(String, String)>) -> usize {
    if got.len() != want.len() {
        failures.push((
            what.to_string(),
            format!("{} blocks, the kit has {}", got.len(), want.len()),
        ));
        return 0;
    }
    let mut compared = 0;
    for (b, (g, w)) in got.iter().zip(want).enumerate() {
        let w = num(w);
        if w < RMS_FLOOR {
            if g.is_nan() || *g >= RMS_FLOOR + RMS_DB {
                failures.push((
                    format!("{what} block {b}"),
                    format!("{g:.3} dB, the kit {w:.3} dB (below {RMS_FLOOR})"),
                ));
            }
            continue;
        }
        compared += 1;
        if (g - w).is_nan() || (g - w).abs() >= RMS_DB {
            failures.push((
                format!("{what} block {b}"),
                format!("{g:.3} dB at {:.1} s, the kit {w:.3} dB", b as f64 / 10.0),
            ));
        }
    }
    compared
}

/// A soundtrack against the kit's summary (`sound.json`'s `left`, a theme's or a pair's): the 16-bit master, the
/// buses in the score's order, the stems, the RMS of each listed bus and the master, the first-time notes and the
/// noise stream's end. Returns how many things were compared.
pub fn compare(what: &str, s: &Summary, kit: &Value, failures: &mut Vec<(String, String)>) -> usize {
    let mut n = 0;
    let mut check = |key: &str, ok: bool, why: String| {
        n += 1;
        if !ok {
            failures.push((format!("{what} {key}"), why));
        }
    };
    let master = pcm_sha(&s.st.master);
    check(
        "master",
        master == text(&kit["master_sha256"]),
        "the 16-bit master is not the kit's".into(),
    );
    let buses: Vec<&str> = list(&kit["buses"]).iter().map(text).collect();
    check("buses", s.buses == buses, format!("{:?}, the kit {buses:?}", s.buses));
    let stems: Vec<&str> = list(&kit["stems"]).iter().map(text).collect();
    let ours: Vec<&str> = s.st.stems.keys().map(String::as_str).collect();
    check("stems", ours == stems, format!("{ours:?}, the kit {stems:?}"));
    let notes = &kit["notes"];
    check(
        "notes",
        (s.notes.len(), sha_lines(&s.notes)) == (index(&notes["count"]), text(&notes["sha256"]).to_string()),
        format!(
            "{} first-time notes, the kit {}; the first: {:?}, the kit {:?}",
            s.notes.len(),
            index(&notes["count"]),
            &s.notes[..s.notes.len().min(8)],
            list(&notes["head"])
        ),
    );
    let end: u128 = text(&kit["noise"]["end"]).parse().expect("a state");
    let (mut a, mut b) = (s.noise.0.clone(), Generator::from_state(end, PpNoise::START_INC));
    check(
        "noise",
        (0..4).all(|_| a.random().to_bits() == b.random().to_bits()),
        "the noise stream does not end where the kit's did".into(),
    );
    let mut blocks = 0;
    for (bus, want) in kit["rms"].as_object().expect("rms") {
        let got = if bus == "master" {
            block_db3(&s.st.master)
        } else if let Some(x) = s.st.stems.get(bus) {
            block_db3(x)
        } else {
            failures.push((format!("{what} rms {bus}"), "no such stem".into()));
            n += 1;
            continue;
        };
        blocks += profile(&format!("{what} rms {bus}"), &got, list(want), failures);
        n += 1;
    }
    assert!(blocks > 0, "{what}: no block above {RMS_FLOOR} dBFS to compare");
    n
}

/// The largest sample difference in dBFS of `got` rounded to float32 against the float32 `want`.
pub fn window_db(got: &[[f64; 2]], want: &[f32]) -> f64 {
    let mut worst = 0.0f64;
    for (g, w) in got.as_flattened().iter().zip(want) {
        let d = (f64::from(*g as f32) - f64::from(*w)).abs();
        if d > worst || d.is_nan() {
            worst = d;
        }
    }
    if worst == 0.0 {
        f64::NEG_INFINITY
    } else {
        20.0 * bf_core::dmath::log10(worst)
    }
}

/// The stem windows of a soundtrack against the cache file `rel` (members `w<j>__<bus>`, float32 `(n, 2)`).
/// The samples of a recorded stem window (`i0`, `n`) in a stem of `x_len` samples. A whole second (`n` = SR) lies
/// inside; a shorter one is cut by the soundtrack's end, so it must end exactly there; an empty one (`n` = 0) starts
/// at or past the end and is not compared (`Ok(None)`). Anything else is the recording and the Studio disagreeing on
/// the soundtrack's length.
pub fn window_range(x_len: usize, i0: usize, n: usize) -> Result<Option<std::ops::Range<usize>>, String> {
    let sr = bf_sound::SR;
    if n == 0 {
        return if i0 >= x_len {
            Ok(None)
        } else {
            Err(format!(
                "an empty window at sample {i0}, but the soundtrack runs to {x_len}"
            ))
        };
    }
    if n < sr && x_len != i0 + n {
        return Err(format!(
            "a window cut to {n} samples at {i0} must end where the soundtrack does ({x_len}), not at {}",
            i0 + n
        ));
    }
    if n > sr || i0 + n > x_len {
        return Err(format!("a window of {n} samples at {i0} in a soundtrack of {x_len}"));
    }
    Ok(Some(i0..i0 + n))
}

pub fn windows_against(
    what: &str,
    st: &Soundtrack,
    windows: &Value,
    rel: &str,
    failures: &mut Vec<(String, String)>,
) -> usize {
    let npz = Npz::open(&bf_testdata::cache_file(rel));
    let mut n = 0;
    assert!(
        !list(windows).is_empty() && !st.stems.is_empty(),
        "{what}: no stem window to compare ({rel})"
    );
    for (j, w) in list(windows).iter().enumerate() {
        let (i0, len) = (index(&w["i0"]), index(&w["n"]));
        for (bus, x) in &st.stems {
            let Some(range) = window_range(x.len(), i0, len)
                .unwrap_or_else(|e| panic!("{what} stem {bus} window {}: {e}", text(&w["window"])))
            else {
                continue;
            };
            let member = format!("w{j}__{bus}");
            assert!(npz.has(&member), "{rel}: no window {member}");
            let a = npz.get(&member);
            assert_eq!(a.shape, [len, 2], "{rel} {member}");
            let db = window_db(&x[range], a.f32());
            n += 1;
            if db.is_nan() || db >= DB {
                failures.push((
                    format!("{what} stem {bus} window {}", text(&w["window"])),
                    format!("{db:.1} dBFS"),
                ));
            }
        }
    }
    assert!(n > 0, "{what}: no stem window compared ({rel})");
    n
}

/// A kit summary with the RMS its folder keeps in the cache (`rms_cache`) put back beside the committed RMS.
pub fn with_cached_rms(kit: &Value) -> Value {
    let mut out = kit.clone();
    if let Some(rel) = kit.get("rms_cache").and_then(Value::as_str) {
        let cached = crate::refs::cache_json(rel);
        let rms = out["rms"].as_object_mut().map(std::mem::take).unwrap_or_default();
        let mut all = cached["rms"].as_object().expect("the cached rms").clone();
        all.extend(rms);
        out["rms"] = Value::Object(all);
    }
    out
}

/// The soundtrack of a fighter's fight (left or swapped) with its real part (in `role` when given) and the kit's
/// base `Sound` for the other side.
pub fn fighter_soundtrack(
    name: &str,
    rec: &bf_core::recording::Recording,
    swapped: bool,
    role: Option<&str>,
) -> Summary {
    let fd = fight::doc(name);
    let mut ff = fight::fight_file(&fd, swapped);
    ff.seed = Some(rec.seed);
    if let Some(r) = role {
        ff.music.roles.insert(name.to_string(), r.to_string());
    }
    let show = Show::new(rec, &ff).unwrap_or_else(|e| panic!("{name}: the show: {e}"));
    let mut build = SoundBuild::new(&show, rec, &ff).unwrap_or_else(|e| panic!("{name}: the sound build: {e}"));
    let k = usize::from(swapped);
    assert_eq!(show.names[k], name);
    build.sounds[1 - k] = Box::new(BaseSound { k: 1 - k });
    summarise(&build)
}

/// Blocks of 100 ms above this many dBFS count as the part's bus sounding.
pub const LOUD_DB: f64 = -60.0;
/// A part with no first-time note must sound in at least this many blocks of its own bus (10 s).
pub const LOUD_BLOCKS: usize = 100;

/// The `sound` gate compares something of the fighter's part (a suite that compared nothing must not pass): the
/// kit's soundtrack has first-time notes of it (`kit.music.note`), or, when the kit's own count is 0 (a part that
/// makes every note through its own synth, CLARINET's reed cache, never calling `M.note`), its own bus is one of the
/// kit's stems, has an RMS profile, and sounds above [`LOUD_DB`] in at least [`LOUD_BLOCKS`] blocks, so the bus
/// profile and the 16-bit master the gate compares carry the part.
pub fn part_is_compared(name: &str, kit: &Value) -> Result<(), String> {
    if index(&kit["notes"]["count"]) > 0 {
        return Ok(());
    }
    let bus = name.to_lowercase();
    let in_stems = list(&kit["stems"]).iter().any(|s| text(s) == bus);
    let loud = kit["rms"][bus.as_str()]
        .as_array()
        .map_or(0, |blocks| blocks.iter().filter(|b| num(b) > LOUD_DB).count());
    if in_stems && loud >= LOUD_BLOCKS {
        Ok(())
    } else {
        Err(format!(
            "the kit's left soundtrack has no first-time note of {name}'s part and its bus `{bus}` {}: nothing of the \
             part to compare",
            if in_stems {
                format!("sounds above {LOUD_DB} dBFS in {loud} blocks, fewer than {LOUD_BLOCKS}")
            } else {
                "is not one of its stems".to_string()
            }
        ))
    }
}

fn sound_doc(name: &str) -> Value {
    let d = json(&fighter_folder(name), "sound.json");
    assert_eq!(text(&d["fighter"]), name);
    d
}

/// The `left` fight's soundtrack with the fighter's real part and the base `Sound` for the other side, and each
/// role variant: the kit's 16-bit master, buses, RMS, notes. Always runs.
pub fn sound(name: &str) {
    let folder = fighter_folder(name);
    let doc = sound_doc(name);
    let fd = fight::doc(name);
    let rec = recording_of(&fd["left"], Some(text(&fd["left"]["hash"])));
    let mut failures = Vec::new();
    if let Err(why) = part_is_compared(name, &doc["left"]) {
        panic!("{folder}: {why}");
    }
    let s = fighter_soundtrack(name, &rec, false, None);
    let mut n = compare("left", &s, &doc["left"], &mut failures);
    for (role, kit) in doc["roles"].as_object().expect("roles") {
        let s = fighter_soundtrack(name, &rec, false, Some(role));
        n += compare(&format!("role {role}"), &s, kit, &mut failures);
    }
    report(&folder, "sound", "sound", "soundtrack facts", failures, n);
}

/// The `right` fight's soundtrack (cached recording) as `sound` compares the left's, and every stem window of the
/// left, right and role soundtracks against the cache at -120 dBFS. Ignored: reads the cache.
pub fn sound_stems(name: &str) {
    let folder = fighter_folder(name);
    let doc = sound_doc(name);
    let fd = fight::doc(name);
    let mut failures = Vec::new();
    let left = recording_of(&fd["left"], None);
    let right = cached_recording(text(&fd["right"]["cache"]));
    let r = fighter_soundtrack(name, &right, true, None);
    let mut n = compare("right", &r, &with_cached_rms(&doc["right"]), &mut failures);
    n += windows_against(
        "right",
        &r.st,
        &doc["right"]["windows"],
        text(&doc["right"]["cache"]),
        &mut failures,
    );
    let l = fighter_soundtrack(name, &left, false, None);
    n += windows_against(
        "left",
        &l.st,
        &doc["left"]["windows"],
        text(&doc["left"]["cache"]),
        &mut failures,
    );
    for (role, kit) in doc["roles"].as_object().expect("roles") {
        let s = fighter_soundtrack(name, &left, false, Some(role));
        n += windows_against(
            &format!("role {role}"),
            &s.st,
            &kit["windows"],
            text(&kit["cache"]),
            &mut failures,
        );
    }
    report(
        &folder,
        "stems",
        "stems",
        "soundtrack facts and stem windows",
        failures,
        n,
    );
}

/// The hooks the fighter's part calls on the bed without checking (`bf_sound::traits::Sound::bed_calls`) are the
/// kit's: every `bed.<hook>(` of its Python `Sound` not guarded by `hasattr`.
pub fn bed_calls(name: &str) {
    let doc = sound_doc(name);
    let ff = fight::fight_file(&fight::doc(name), false);
    let part =
        bf_content::fighters::sound(name, 0, &ff).unwrap_or_else(|| panic!("{name}'s sound() is None: port its part"));
    let mut got: Vec<&str> = part.bed_calls().iter().map(|h| h.name()).collect();
    got.sort_unstable();
    let want: Vec<&str> = list(&doc["bed_calls"]).iter().map(text).collect();
    assert_eq!(got, want, "{name}: the bed hooks its part calls unguarded");
}

#[cfg(test)]
mod tests {
    use super::window_range;

    const SR: usize = bf_sound::SR;

    #[test]
    fn a_whole_window_lies_inside() {
        assert_eq!(window_range(61 * SR, 12 * SR, SR), Ok(Some(12 * SR..13 * SR)));
        assert_eq!(window_range(61 * SR, 60 * SR, SR), Ok(Some(60 * SR..61 * SR)));
        assert!(
            window_range(61 * SR, 60 * SR + 1, SR).is_err(),
            "a whole second past the end"
        );
    }

    #[test]
    fn a_cut_window_ends_where_the_soundtrack_does() {
        let (na, i0) = (61 * SR, 2_913_600); // 60.7 s: 14,400 samples left
        assert_eq!(window_range(na, i0, 14_400), Ok(Some(i0..na)));
        let e = window_range(na, i0, 14_000).unwrap_err();
        assert!(e.contains("must end where the soundtrack does"), "{e}");
        assert!(
            window_range(na + 10, i0, 14_400).is_err(),
            "the Studio's soundtrack is longer"
        );
    }

    #[test]
    fn an_empty_window_is_past_the_end_and_not_compared() {
        assert_eq!(window_range(61 * SR, 61 * SR, 0), Ok(None));
        assert_eq!(window_range(61 * SR, 61 * SR + 500, 0), Ok(None));
        let e = window_range(61 * SR, 60 * SR, 0).unwrap_err();
        assert!(e.contains("an empty window at sample"), "{e}");
    }
}
