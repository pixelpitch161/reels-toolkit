//! The kit's checks (`kit/checks.py`) against the Studio's (`bf_export::checks`), for a fighter's `left` fight and
//! for a pair gate's kit episode, with `checks_parity.rs`'s bars: every verdict equal; focus: the worst circle of
//! confusion, the frames and the points equal; safe zone: the same boxes, frame by frame; levels within 0.01 dB;
//! band within 0.1 dB; loudness within 0.01 dB; flash: the counts equal, every frame's area within 0.1 point; loop:
//! the numbers within 1 %. The fight's 16-bit soundtrack, as a wav file, must also be the kit's `audio.wav`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use bf_content::{build_picture, build_soundtrack};
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::show::Show;
use bf_export::checks::flash::{analyse, rel_lum};
use bf_export::checks::levels::profile;
use bf_export::checks::safezone::frames as safezone_frames;
use bf_export::checks::{
    BandPlan, CheckResult, as_wav16, band, flash, flash_span, focus, levels, loop_audio, loop_picture, loudness,
    safezone,
};
use bf_picture::Rgb8;
use bf_picture::ui::DrawLog;
use bf_sound::wav::sample_i16;
use bf_story::{BeatSheet, story};
use serde_json::Value;

use crate::fight;
use crate::look::on_threads;
use crate::refs::{cache_json, fighter_folder, index, json, list, num, recording_of, report, sha, text};

fn got(r: &CheckResult, key: &str) -> f64 {
    *r.numbers
        .get(key)
        .unwrap_or_else(|| panic!("{}: no number {key:?} among {:?}", r.name, r.numbers.keys()))
}

/// The canonical wav file of a 16-bit stereo 48 kHz master (`A.write`: a 44-byte header, then the samples).
pub fn wav_bytes(master: &[[f64; 2]]) -> Vec<u8> {
    let data_len = (master.len() * 4) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend(b"RIFF");
    b.extend((36 + data_len).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(48_000u32.to_le_bytes());
    b.extend(192_000u32.to_le_bytes());
    b.extend(4u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(data_len.to_le_bytes());
    for f in master {
        for &x in f {
            b.extend(sample_i16(x).to_le_bytes());
        }
    }
    b
}

/// The bed buses the kit's band check lists (`kit/checks.py` 117: the bed's own and `fx`, each only when its stem
/// file exists): the declared ones the soundtrack has. A bus a bed declares but does not play, such as brass_trailer's
/// `pad` outside the sequel, has no stem, so the kit leaves it out.
pub fn bed_buses_played(declared: &[String], has: impl Fn(&str) -> bool) -> Vec<&str> {
    declared.iter().map(String::as_str).filter(|b| has(b)).collect()
}

/// Every check of the fight `rec` under `ff` (both real looks and parts, through `build_picture` and
/// `build_soundtrack`) against the kit's `checks` (a `checks.json`).
pub fn check_fight(folder: &str, scope: &str, rec: Recording, ff: &FightFile, kit_doc: &Value) {
    let mut ff = ff.clone();
    ff.seed = Some(rec.seed);
    let facts = &kit_doc["episode_facts"];
    assert_eq!(
        facts["seed"].as_u64(),
        Some(rec.seed),
        "{folder}: the checks are of another fight"
    );
    let rec = Arc::new(rec);
    let show = Show::new(&rec, &ff).expect("the show");
    let pic = build_picture(Arc::clone(&rec), &ff).unwrap_or_else(|e| panic!("{folder}: the picture: {e}"));
    let st = build_soundtrack(&rec, &ff).unwrap_or_else(|e| panic!("{folder}: the soundtrack: {e}"));
    let kit = &kit_doc["checks"];
    // the per-frame rows are in the cache (the safe-zone boxes, the flash areas, the loop steps, the levels windows)
    let rows_doc = cache_json(text(&kit_doc["rows_cache"]));
    let mut failures: Vec<(String, String)> = Vec::new();
    let mut n = 0;
    let mut fail = |key: &str, ok: bool, why: String| {
        n += 1;
        if !ok {
            failures.push((key.to_string(), why));
        }
    };
    // the soundtrack as the kit wrote it
    let wav = wav_bytes(&st.master);
    fail(
        "audio.wav",
        sha(&wav) == text(&facts["audio_wav_sha256"]),
        "the 16-bit soundtrack (as a wav file) is not the kit's audio.wav".into(),
    );
    let w16 = as_wav16(&st.master);
    // the frames the picture checks read, drawn once
    let nf = pic.nf();
    let cfg = &pic.ctx().cfg;
    let low = [cfg[0].low.name.clone(), cfg[1].low.name.clone()];
    let sheet = BeatSheet::series([&cfg[0], &cfg[1]]);
    let stry = story(&rec, &show.grid, &sheet, [&low[0], &low[1]]);
    let (f0, f1) = flash_span(&show, &stry, [&low[0], &low[1]]);
    let safe = safezone_frames(nf, show.grid.fps);
    let (nt, nh) = (5 * show.grid.fps as usize, show.grid.fps as usize);
    let loop_frames: Vec<usize> = (nf - nt..nf).chain(0..nh).collect();
    let want: Vec<usize> = safe
        .iter()
        .map(|&(_, f)| f)
        .chain(f0..f1)
        .chain(loop_frames.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let drawn: Mutex<BTreeMap<usize, Arc<(Rgb8, DrawLog)>>> = Mutex::new(BTreeMap::new());
    on_threads(&want, |&f| {
        let d = Arc::new(pic.frame_logged(f));
        drawn.lock().unwrap().insert(f, d);
        None::<()>
    });
    let drawn = drawn.into_inner().unwrap();
    println!("{folder}: {} frames drawn for the checks", drawn.len());
    let verdict = |r: &CheckResult, k: &Value| Some(r.pass) == k["pass"].as_bool();
    let fps = show.grid.fps as usize;
    // focus
    let r = focus(&pic);
    let k = &kit["focus"];
    fail("focus verdict", verdict(&r, k), r.summary.clone());
    for key in ["worst_coc", "frames", "points"] {
        fail(
            &format!("focus {key}"),
            got(&r, key) == num(&k[key]),
            format!("{} against {}", got(&r, key), num(&k[key])),
        );
    }
    // safe zone: every box, frame by frame
    let logs: Vec<(f64, DrawLog)> = safe.iter().map(|&(t, f)| (t, drawn[&f].1.clone())).collect();
    let r = safezone(&logs);
    let k = &kit["safezone"];
    fail("safezone verdict", verdict(&r, k), r.summary.clone());
    for key in ["frames_checked", "tracked"] {
        fail(
            &format!("safezone {key}"),
            got(&r, key) == num(&k[key]),
            format!("{}", got(&r, key)),
        );
    }
    fail(
        "safezone outside",
        got(&r, "outside") == list(&k["outside"]).len() as f64,
        r.summary.clone(),
    );
    fail(
        "safezone overlaps",
        got(&r, "overlaps") == list(&k["overlaps"]).len() as f64,
        r.summary.clone(),
    );
    let boxes = list(&rows_doc["safezone_boxes"]);
    assert!(
        !boxes.is_empty() && boxes.len() == index(&k["frames_rows"]) && boxes.len() == safe.len(),
        "{folder}: {} safe-zone frames in the cache, {} in checks.json, {} checked here",
        boxes.len(),
        k["frames_rows"],
        safe.len()
    );
    // per frame `[frame, boxes, SHA-256 of the boxes]`, and one SHA-256 over the frames' box hashes
    let mut frame_shas = Vec::new();
    for (w, (_, log)) in boxes.iter().zip(&logs) {
        let f = w[0].as_u64().unwrap();
        let lines: Vec<String> = log
            .boxes
            .iter()
            .map(|b| format!("{},{},{},{},{},{}", b.x0, b.y0, b.x1, b.y1, b.kind.as_str(), b.info))
            .collect();
        let mine = sha(lines.join("\n").as_bytes());
        let same = lines.len() as u64 == w[1].as_u64().unwrap() && text(&w[2]) == mine;
        fail(
            &format!("safezone boxes of frame {f}"),
            same,
            format!("{} boxes", lines.len()),
        );
        frame_shas.push(mine);
    }
    fail(
        "safezone boxes",
        sha(frame_shas.join("\n").as_bytes()) == text(&k["boxes_sha256"]),
        "the boxes of some frame".into(),
    );
    // levels
    let r = levels(&w16);
    let k = &kit["levels"];
    fail("levels verdict", verdict(&r, k), r.summary.clone());
    for key in ["ending", "opening", "opening_minus_ending", "build", "spread"] {
        fail(
            &format!("levels {key}"),
            (got(&r, key) - num(&k[key])).abs() <= 0.01,
            format!("{}", got(&r, key)),
        );
    }
    fail(
        "levels dips",
        got(&r, "dips") == list(&k["dips"]).len() as f64,
        r.summary.clone(),
    );
    let mono: Vec<f64> = w16.iter().map(|f| (f[0] + f[1]) / 2.0).collect();
    let rows = profile(&mono, 62.0);
    let kr = list(&rows_doc["levels_rows"]);
    assert!(
        !kr.is_empty() && kr.len() == index(&k["rows_count"]) && rows.len() == kr.len(),
        "{folder}: {} levels windows here, {} in the cache, {} in checks.json",
        rows.len(),
        kr.len(),
        k["rows_count"]
    );
    for (row, w) in rows.iter().zip(kr) {
        let ok = row.t == num(&w["t"])
            && (row.dbfs - num(&w["dbfs"])).abs() <= 0.01
            && row.hits_per_s == num(&w["hits_per_s"]);
        fail(
            &format!("levels window {}", row.t),
            ok,
            format!("{} dBFS, {} hits/s", row.dbfs, row.hits_per_s),
        );
    }
    // band
    let plan = BandPlan::of(&ff).unwrap_or_else(|e| panic!("{folder}: {e}"));
    let k = &kit["band"];
    let bed: Vec<&str> = list(&k["bed_buses"]).iter().map(text).collect();
    let have = bed_buses_played(&plan.bed_buses, |b| st.stems.contains_key(b));
    fail("band bed buses", bed == have, format!("{have:?}"));
    let sounds = [
        (plan.sounds[0].0.as_str(), plan.sounds[0].1),
        (plan.sounds[1].0.as_str(), plan.sounds[1].1),
    ];
    let r = band(&st, &show, sounds, &bed);
    fail("band verdict", verdict(&r, k), r.summary.clone());
    let fighters = list(&k["fighters"]);
    assert_eq!(fighters.len(), 2, "{folder}: the band check's fighters");
    for f in fighters {
        let bus = text(&f["bus"]);
        for (kind, key) in [("section", "section"), ("while it sounds", "while_it_sounds")] {
            assert_eq!(list(&f[key]).len(), 5, "{folder}: band {bus} {kind}: the five sections");
            for (i, w) in list(&f[key]).iter().enumerate() {
                let g = got(&r, &format!("{bus} {kind} {}", i + 1));
                fail(
                    &format!("band {bus} {kind} {}", i + 1),
                    (g - num(w)).abs() <= 0.1,
                    format!("{g}"),
                );
            }
        }
    }
    // loudness
    let r = loudness(&w16, &show);
    let k = &kit["loudness"];
    for key in ["intro_lufs", "fight_lufs", "intro_minus_fight"] {
        fail(
            &format!("loudness {key}"),
            (got(&r, key) - num(&k[key])).abs() <= 0.01,
            format!("{}", got(&r, key)),
        );
    }
    // flash
    let k = &kit["flash"];
    fail(
        "flash span",
        (f0 as u64, f1 as u64) == (k["f0"].as_u64().unwrap(), k["f1"].as_u64().unwrap()),
        format!("{f0}..{f1}"),
    );
    let lum: Vec<_> = (f0..f1).map(|f| rel_lum(&drawn[&f].0)).collect();
    let r = flash(&lum);
    fail("flash verdict", verdict(&r, k), r.summary.clone());
    for key in ["general_flashes", "worst_in_1s"] {
        fail(
            &format!("flash {key}"),
            got(&r, key) == num(&k[key]),
            format!("{}", got(&r, key)),
        );
    }
    let a = analyse(&lum, fps);
    let area = list(&rows_doc["flash_area"]);
    assert!(
        !area.is_empty() && area.len() == index(&k["area_rows"]) && a.area.len() == area.len(),
        "{folder}: {} flash frames here, {} in the cache, {} in checks.json",
        a.area.len(),
        area.len(),
        k["area_rows"]
    );
    for (j, (g, w)) in a.area.iter().zip(area).enumerate() {
        fail(
            &format!("flash area of frame {}", f0 + j),
            (g - num(w)).abs() <= 0.001,
            format!("{g}"),
        );
    }
    // loop
    let k = &kit["loop"];
    let pics: Vec<Rgb8> = loop_frames.iter().map(|f| drawn[f].0.clone()).collect();
    let (tail, head) = pics.split_at(nt);
    let p = loop_picture(tail, head);
    let au = loop_audio(&w16);
    fail(
        "loop verdict",
        Some(p.pass && au.pass) == k["pass"].as_bool(),
        format!("{} | {}", p.summary, au.summary),
    );
    let pct = |g: f64, w: f64| (g - w).abs() <= 0.01 * w.abs();
    let kp = &k["picture"];
    fail("loop fails", got(&p, "fails") == num(&kp["fails"]), p.summary.clone());
    for key in ["seam", "normal", "ratio"] {
        fail(
            &format!("loop {key}"),
            pct(got(&p, key), num(&kp[key])),
            format!("{}", got(&p, key)),
        );
    }
    let steps = bf_export::checks::loopcheck::steps(tail, head);
    let ksteps = list(&rows_doc["loop_steps"]);
    assert!(
        !ksteps.is_empty() && ksteps.len() == index(&kp["steps_rows"]) && steps.len() == ksteps.len(),
        "{folder}: {} loop steps here, {} in the cache",
        steps.len(),
        ksteps.len()
    );
    for (i, (g, w)) in steps.iter().zip(ksteps).enumerate() {
        fail(&format!("loop step {i}"), pct(*g, num(w)), format!("{g}"));
    }
    let ka = &k["audio"];
    fail(
        "loop audio fails",
        got(&au, "fails") == num(&ka["fails"]),
        au.summary.clone(),
    );
    for key in ["last_20ms_db", "first_20ms_db", "step_db", "typical_step", "jump"] {
        fail(
            &format!("loop audio {key}"),
            pct(got(&au, key), num(&ka[key])),
            format!("{}", got(&au, key)),
        );
    }
    report(folder, "checks", scope, "check verdicts and numbers", failures, n);
}

/// The `left` fight's checks against `fighters/<slug>/checks.json`. Always runs.
pub fn fighter_checks(name: &str) {
    let folder = fighter_folder(name);
    let fd = fight::doc(name);
    let rec = recording_of(&fd["left"], Some(text(&fd["left"]["hash"])));
    let kit = json(&folder, "checks.json");
    check_fight(&folder, "left", rec, &fight::fight_file(&fd, false), &kit);
}

/// A pair gate's kit episode against `episodes/<pair>/checks.json`.
pub fn episode_checks(pair: &str) {
    let folder = format!("episodes/{pair}");
    let (rec, ff) = crate::episode::episode(pair);
    check_fight(&folder, "pair", rec, &ff, &json(&folder, "checks.json"));
}

#[cfg(test)]
mod tests {
    use super::bed_buses_played;

    #[test]
    fn a_declared_bus_without_a_stem_is_not_listed() {
        // brass_trailer's plain bed declares `pad` (the sequel's) and never plays it: the kit lists five buses
        let declared: Vec<String> = ["strings", "taiko", "braam", "tremolo", "pad", "fx"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let has = |b: &str| b != "pad";
        assert_eq!(
            bed_buses_played(&declared, has),
            ["strings", "taiko", "braam", "tremolo", "fx"]
        );
        // a bed that plays every bus it declares keeps them all, in its order
        assert_eq!(bed_buses_played(&declared, |_| true), declared);
        assert!(bed_buses_played(&declared, |_| false).is_empty());
    }
}
