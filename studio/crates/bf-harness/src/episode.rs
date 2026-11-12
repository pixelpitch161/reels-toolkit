//! Template P, the pair gate of a kit config: `studio/fights/<pair>.toml` and `studio/reference/episodes/<pair>/`
//! against the Studio. The kit's own recording of the config (pointed at by `episode.json`: milestone 1's, the
//! theme folder's, or the folder's own `run.bfrec`) is drawn with both real looks and played with both real parts.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use bf_content::{SoundBuild, build_picture};
use bf_core::event::Value as Ev;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::show::Show;
use bf_picture::hull;
use bf_sim::{Setup, setup_from};
use bf_story::{BeatSheet, SearchOpts, search, stats as stats_of};
use serde_json::Value;

use crate::Sim;
use crate::fight::{self, fighter, roster, row_occurs, setup_like, setup_of, traced};
use crate::look::{hull_differs, noise, on_threads};
use crate::refs::{
    cached_recording, committed_recording, index, json, list, num, report, sha, sha_lines, studio, text, value,
};
use crate::sound::{compare, summarise, windows_against};

fn folder(pair: &str) -> String {
    format!("episodes/{pair}")
}

/// The pair's fight file, `studio/fights/<pair>.toml`.
pub fn fight_file(pair: &str) -> FightFile {
    let path = studio(&format!("fights/{pair}.toml"));
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("studio/fights/{pair}.toml is missing: write the kit config as a fight file"));
    FightFile::parse(&src).unwrap_or_else(|e| panic!("studio/fights/{pair}.toml: {e}"))
}

/// The kit's recording of the config and the pair's fight file.
pub fn episode(pair: &str) -> (Recording, FightFile) {
    let doc = json(&folder(pair), "episode.json");
    let rec = committed_recording(text(&doc["recording"]), Some(text(&doc["rec_hash"])));
    (rec, fight_file(pair))
}

fn strings(v: &Value) -> Vec<String> {
    list(v).iter().map(|s| text(s).to_string()).collect()
}

/// Every field of the fight file is the kit config's (`episode.json`): the fighters, tempo, key, scale, chords
/// (intro, loop, tail, rebuilt by `chords_for` at the kit's bar count), handicap, damage scale, pair rules, seed,
/// series tag, lines, roles, solos, title hits, mix.
pub fn fight_file_is_the_config(pair: &str) {
    let d = json(&folder(pair), "episode.json");
    let ff = fight_file(pair);
    let mut wrong = Vec::new();
    let mut eq = |what: &str, ok: bool, got: String| {
        if !ok {
            wrong.push(format!("  {what}: the fight file has {got}"));
        }
    };
    eq(
        "fighters",
        ff.fighters == strings(&d["fighters"]),
        format!("{:?}", ff.fighters),
    );
    eq("tempo", ff.tempo_bpm == num(&d["bpm"]), format!("{}", ff.tempo_bpm));
    eq("theme", ff.music.theme == text(&d["mood"]), ff.music.theme.clone());
    eq("key", ff.music.key == text(&d["key"]), ff.music.key.clone());
    eq("scale", ff.music.scale == text(&d["scale"]), ff.music.scale.clone());
    eq(
        "chords_intro",
        ff.music.chords_intro == strings(&d["intro"]),
        format!("{:?}", ff.music.chords_intro),
    );
    eq(
        "chord_loop",
        ff.music.chord_loop == strings(&d["loop"]),
        format!("{:?}", ff.music.chord_loop),
    );
    eq(
        "chords_tail",
        ff.music.chords_tail == strings(&d["tail"]),
        format!("{:?}", ff.music.chords_tail),
    );
    let n_bars = index(&d["n_bars"]);
    eq("bars", ff.grid().n_bars == n_bars, format!("{}", ff.grid().n_bars));
    eq(
        "chords",
        ff.chords(n_bars) == strings(&d["chords"]),
        format!("{:?}", ff.chords(n_bars)),
    );
    let handicap: Vec<f64> = list(&d["handicap"]).iter().map(num).collect();
    eq("handicap", ff.handicap == handicap, format!("{:?}", ff.handicap));
    eq(
        "dmg_scale",
        ff.dmg_scale == num(&d["dmg_scale"]),
        format!("{}", ff.dmg_scale),
    );
    let rules: std::collections::BTreeMap<String, Ev> = d["rules"]
        .as_object()
        .expect("rules")
        .iter()
        .map(|(k, v)| (k.clone(), value(v)))
        .collect();
    eq("pair_rules", ff.pair_rules == rules, format!("{:?}", ff.pair_rules));
    eq("seed", ff.seed == d["seed"].as_u64(), format!("{:?}", ff.seed));
    eq(
        "series_tag",
        ff.series_tag.as_deref() == d["series"].as_str(),
        format!("{:?}", ff.series_tag),
    );
    let lines: std::collections::BTreeMap<String, Vec<Option<i32>>> = d["lines"]
        .as_object()
        .expect("lines")
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                list(v).iter().map(|n| n.as_i64().map(|m| m as i32)).collect(),
            )
        })
        .collect();
    eq("lines", ff.music.lines == lines, format!("{:?}", ff.music.lines.keys()));
    let roles: std::collections::BTreeMap<String, String> = d["roles"]
        .as_object()
        .expect("roles")
        .iter()
        .map(|(k, v)| (k.clone(), text(v).to_string()))
        .collect();
    eq("roles", ff.music.roles == roles, format!("{:?}", ff.music.roles));
    let hits: std::collections::BTreeMap<String, Vec<i32>> = d["title_hits"]
        .as_object()
        .expect("title_hits")
        .iter()
        .map(|(k, v)| (k.clone(), list(v).iter().map(|n| n.as_i64().unwrap() as i32).collect()))
        .collect();
    eq(
        "title_hits",
        ff.music.title_hits == hits,
        format!("{:?}", ff.music.title_hits),
    );
    let solos: std::collections::BTreeMap<String, Vec<(f64, i32, f64)>> = d["solos"]
        .as_object()
        .expect("solos")
        .iter()
        .map(|(k, v)| {
            let notes = list(v)
                .iter()
                .map(|n| (num(&n[0]), n[1].as_i64().unwrap() as i32, num(&n[2])))
                .collect();
            (k.clone(), notes)
        })
        .collect();
    // the config's solos; a fighter it gives none plays its kit default, which the fight file writes out
    let same_solos = solos.iter().all(|(k, v)| ff.music.solos.get(k) == Some(v));
    eq("solos", same_solos, format!("{:?}", ff.music.solos.keys()));
    let mix: std::collections::BTreeMap<String, (f64, f64)> = d["mix"]
        .as_object()
        .expect("mix")
        .iter()
        .map(|(k, v)| (k.clone(), (num(&v[0]), num(&v[1]))))
        .collect();
    eq("mix", ff.music.mix == mix, format!("{:?}", ff.music.mix));
    eq(
        "shard_seed",
        ff.picture.shard_seed == d["shard_seed"].as_u64().unwrap_or(271),
        format!("{}", ff.picture.shard_seed),
    );
    if let Some(fx) = d["fx_seed"].as_u64() {
        eq(
            "fx_seed",
            ff.picture.fx_seed == Some(fx),
            format!("{:?}", ff.picture.fx_seed),
        );
    }
    if let Some(ms) = d["music_seed"].as_u64() {
        eq(
            "music_seed",
            ff.music.music_seed == ms,
            format!("{}", ff.music.music_seed),
        );
    }
    bf_content::validate(&ff).unwrap_or_else(|e| panic!("studio/fights/{pair}.toml: {e}"));
    assert!(
        wrong.is_empty(),
        "studio/fights/{pair}.toml is not the kit config {}:\n{}",
        text(&d["config"]),
        wrong.join("\n")
    );
}

/// The kit's recording is the fight file's fight (its rules) and the Rust fight agrees with it for the floor.
pub fn recording(sim: &Sim, pair: &str) {
    let (py, ff) = episode(pair);
    let setup = setup_from(&ff, roster(), py.seed).unwrap_or_else(|e| panic!("{pair}: {e}"));
    let like = setup_like(&py);
    assert_eq!(
        format!("{:?}", setup.rules),
        format!("{:?}", like.rules),
        "{pair}: the fight file's rules"
    );
    let rs = traced(sim, &setup, true, pair);
    window(sim, pair, "the kit's recording", &rs, &py, &setup);
}

fn window(sim: &Sim, pair: &str, what: &str, rs: &Recording, py: &Recording, setup: &Setup) {
    let r = bf_sim::parity::prefix_window(rs, py);
    let t_fight = match py.rules.get("t_fight") {
        Some(Ev::Float(x)) => *x,
        _ => 0.0,
    };
    let lasts = r.agree_until_s - t_fight;
    let mut words = r.first_break.clone().unwrap_or_default();
    if r.kind == Some(bf_sim::parity::BreakKind::KnifeEdge)
        && let Some(m) = (sim.clash_margin)(setup, r.agree_until_s)
    {
        words.push_str(&format!("\n the Rust balls are {m:e} from R + R"));
    }
    println!("{pair} {what}: agrees for {lasts:.2} s of fighting; {words}");
    assert!(
        r.first_break.is_none() || lasts >= bf_sim::parity::PREFIX_FLOOR_S,
        "{pair} {what}: the window ends after {lasts:.3} s, before the floor (ruling M3-R5):\n {words}"
    );
}

/// The golden frames with both real looks: bytes, draw logs, hull calls (a fallback stops the task, M3-R4).
pub fn golden_frames(pair: &str) {
    let f = folder(pair);
    let doc = json(&f, "frames.json");
    let (rec, mut ff) = episode(pair);
    ff.seed = Some(rec.seed);
    let pic = build_picture(Arc::new(rec), &ff).unwrap_or_else(|e| panic!("{pair}: the picture: {e}"));
    let rows = list(&doc["frames"]);
    assert!(rows.len() >= 40, "{f}: {} golden frames, at least 40", rows.len());
    let fallbacks = AtomicUsize::new(0);
    let failures = on_threads(rows, |row| {
        let n = index(&row["frame"]);
        let ((img, log, rep), calls) = hull::logged(|| pic.frame_reported(n));
        if rep.hull_fallbacks > 0 {
            fallbacks.fetch_add(1, Ordering::Relaxed);
        }
        let boxes: Vec<String> = log
            .boxes
            .iter()
            .map(|b| format!("{},{},{},{},{},{}", b.x0, b.y0, b.x1, b.y1, b.kind.as_str(), b.info))
            .collect();
        let mut wrong = Vec::new();
        if sha(&img.data) != text(&row["sha256"]) {
            wrong.push("its bytes".to_string());
        }
        if (boxes.len(), sha_lines(&boxes)) != (index(&row["boxes"][0]), text(&row["boxes"][1]).to_string()) {
            wrong.push("the draw log".into());
        }
        wrong.extend(hull_differs(&calls, &row["hull"]));
        (!wrong.is_empty()).then(|| {
            (
                format!("frame {n}"),
                format!("{} ({:?})", wrong.join("; "), row["rules"]),
            )
        })
    });
    assert_eq!(
        fallbacks.load(Ordering::Relaxed),
        0,
        "{pair}: a hull fallback: ruling M3-R4 stops the task; tell the controller"
    );
    report(&f, "frames", "golden frames", failures, rows.len());
}

/// The picture's VS card on the noise backdrop at the 16 times of `frames.json`'s `cards`.
pub fn title_card(pair: &str) {
    let f = folder(pair);
    let doc = json(&f, "frames.json");
    let (rec, mut ff) = episode(pair);
    ff.seed = Some(rec.seed);
    let pic = build_picture(Arc::new(rec), &ff).unwrap_or_else(|e| panic!("{pair}: the picture: {e}"));
    let backdrop = noise();
    let rows = list(&doc["cards"]);
    assert!(rows.len() >= 16, "{f}: 16 card frames");
    let mut failures = Vec::new();
    for r in rows {
        let n = index(&r[0]);
        let mut img = backdrop.clone();
        pic.card().draw(&mut img, n as f64 / 30.0);
        if sha(&img.data) != text(&r[1]) {
            failures.push((format!("card frame {n}"), "its bytes".to_string()));
        }
    }
    report(&f, "title", "card frames", failures, rows.len());
}

fn summary(pair: &str) -> crate::sound::Summary {
    let (rec, mut ff) = episode(pair);
    ff.seed = Some(rec.seed);
    let show = Show::new(&rec, &ff).expect("the show");
    let build = SoundBuild::new(&show, &rec, &ff).unwrap_or_else(|e| panic!("{pair}: {e}"));
    summarise(&build)
}

/// The soundtrack with both real parts: the kit's 16-bit master (its `audio.wav`), buses, RMS, notes.
pub fn soundtrack(pair: &str) {
    let f = folder(pair);
    let kit = json(&f, "sound.json");
    let mut failures = Vec::new();
    let n = compare("soundtrack", &summary(pair), &kit, &mut failures);
    report(&f, "sound", "soundtrack facts", failures, n);
}

/// Every stem window against the cache at -120 dBFS. Ignored.
pub fn stems(pair: &str) {
    let f = folder(pair);
    let kit = json(&f, "sound.json");
    let mut failures = Vec::new();
    let n = windows_against(
        "soundtrack",
        &summary(pair).st,
        &kit["windows"],
        text(&kit["cache"]),
        &mut failures,
    );
    report(&f, "stems", "stem windows", failures, n);
}

/// Every link of `pairs.json` occurs in the Rust run of its scenario at its time with its fields.
pub fn pair_links(sim: &Sim, pair: &str) {
    let doc = json(&folder(pair), "pairs.json");
    let mut missing = Vec::new();
    let links = list(&doc["links"]);
    for row in links {
        let scen = list(&doc["scenarios"])
            .iter()
            .find(|s| s["name"] == row["scenario"])
            .expect("the link's scenario");
        let setup = setup_of(&scen["fighters"], &scen["rules"], row["seed"].as_u64().unwrap());
        let rs = traced(sim, &setup, false, pair);
        if let Some(why) = row_occurs(&rs, row) {
            missing.push(why);
        }
    }
    assert!(
        missing.is_empty(),
        "{pair}: links that do not occur as in the kit:\n{}",
        missing.join("\n\n")
    );
    println!("{pair}: {} links occur in the Rust fights", links.len());
}

/// The kit's recording and every cached scenario fight agree with the Rust fights for the floor. Ignored.
pub fn prefix(sim: &Sim, pair: &str) {
    let (py, _) = episode(pair);
    let setup = setup_like(&py);
    let rs = traced(sim, &setup, true, pair);
    window(sim, pair, "the kit's recording", &rs, &py, &setup);
    let doc = json(&folder(pair), "pairs.json");
    for s in list(&doc["scenarios"]) {
        for (file, hash) in list(&s["cache"]).iter().zip(list(&s["hashes"])) {
            let py = cached_recording(text(file));
            assert_eq!(py.hash(), text(hash), "{}", text(file));
            let setup = setup_like(&py);
            let rs = traced(sim, &setup, true, pair);
            window(sim, pair, text(file), &rs, &py, &setup);
        }
    }
}

/// 4,000 Rust fights at the config's own handicap against the kit's pooled figures, milestone 1's bars. Release.
pub fn statistics(pair: &str) {
    let doc = json(&folder(pair), "stats.json");
    let n = list(&doc["seeds"])[1].as_u64().unwrap();
    let ff = fight_file(pair);
    let base = setup_from(&ff, roster(), 0).unwrap_or_else(|e| panic!("{pair}: {e}"));
    let sheet = BeatSheet::series(base.fighters);
    let workers = std::thread::available_parallelism().map_or(3, |w| w.get()).min(3);
    let rs = stats_of(&|seed| Setup { seed, ..base.clone() }, &sheet, 1..n + 1, workers).expect("the fights");
    let py = fight::pooled(&doc["pooled"]);
    println!("{pair} kit: {py:?}\n{pair} Rust: {rs:?}");
    fight::assert_the_kits(pair, &rs, &py);
}

/// A Studio-native fight of the pair from the command line's path: the search (`ballfight search --stop 10`) on
/// the fight file with its seed removed, then the render of the first fit, which must pass every check. Prints the
/// seed, the K.O. and the render time. Ignored (long; needs ffmpeg).
pub fn native_render(pair: &str) {
    let mut ff = fight_file(pair);
    ff.seed = None;
    let base = setup_from(&ff, roster(), 0).unwrap_or_else(|e| panic!("{pair}: {e}"));
    let sheet = BeatSheet::series([fighter(&ff.fighters[0]), fighter(&ff.fighters[1])]);
    let opts = SearchOpts {
        from: 1,
        stop_fits: 10,
        max_seeds: Some(5_000),
        workers: 3,
    };
    let found = search(
        &|seed| Setup { seed, ..base.clone() },
        &sheet,
        &opts,
        &|_| {},
        &AtomicBool::new(false),
    );
    let fit = found
        .candidates
        .iter()
        .find(|c| c.verdict.fits)
        .unwrap_or_else(|| panic!("{pair}: no fit in {} seeds", found.checked));
    ff.seed = Some(fit.seed);
    let setup = setup_from(&ff, roster(), fit.seed).expect("the setup");
    let rec = bf_sim::run(&setup, true).expect("the fight");
    let out = std::env::temp_dir().join(format!("bf-harness-{pair}"));
    let started = Instant::now();
    let failed = std::sync::Mutex::new(Vec::new());
    let opts = bf_export::JobOpts {
        out_root: out,
        workers: 3,
        frames: None,
        speed: Default::default(),
    };
    let done = bf_export::run_job(
        &ff,
        Arc::new(rec),
        &opts,
        &|e| {
            if let bf_export::JobEvent::Check(c) = e
                && !c.pass
            {
                failed.lock().unwrap().push(format!("{}: {}", c.name, c.summary));
            }
        },
        &AtomicBool::new(false),
    )
    .unwrap_or_else(|e| panic!("{pair}: the render of seed {}: {e}", fit.seed));
    let failed = failed.into_inner().unwrap();
    println!(
        "{pair}: seed {} (K.O. {:?}) rendered in {:.1} s to {}",
        fit.seed,
        fit.story.t_ko,
        started.elapsed().as_secs_f64(),
        done.display()
    );
    assert!(failed.is_empty(), "{pair}: checks that failed: {failed:?}");
}
