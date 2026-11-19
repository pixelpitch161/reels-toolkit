//! Template P, the pair gate of a kit config: `studio/fights/<pair>.toml` and `studio/reference/episodes/<pair>/`
//! against the Studio. The kit's own recording of the config (pointed at by `episode.json`: milestone 1's, the
//! theme folder's, or the folder's own `run.bfrec`) is drawn with both real looks and played with both real parts.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use bf_content::{SoundBuild, build_picture};
use bf_core::event::Value as Ev;
use bf_core::fightfile::{FightFile, SoloNote};
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
    cached_recording, index, json, list, named_recording, num, report, sha, sha_lines, studio, text, value,
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
    let rec = named_recording(&doc["recording"], Some(text(&doc["rec_hash"])));
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
    // the theme options the config sets (absent: none, so a config without any reads as before)
    let options = d
        .get("theme_options")
        .map(crate::theme::option_values)
        .unwrap_or_default();
    eq(
        "theme_options",
        ff.music.theme_options == options,
        format!("{:?}", ff.music.theme_options),
    );
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
    // single notes, and a chord as a list of notes (ACOUSTIC's strums)
    let solos: std::collections::BTreeMap<String, Vec<(f64, SoloNote, f64)>> = d["solos"]
        .as_object()
        .expect("solos")
        .iter()
        .map(|(k, v)| (k.clone(), crate::refs::solo(v)))
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
        crate::fight::rules_text(&setup.rules),
        crate::fight::rules_text(&like.rules),
        "{pair}: the fight file's rules"
    );
    let rs = traced(sim, &setup, true, pair);
    window(sim, pair, "the kit's recording", &rs, &py, &setup);
}

fn window(sim: &Sim, pair: &str, what: &str, rs: &Recording, py: &Recording, setup: &Setup) {
    let r = bf_sim::parity::prefix_window(rs, py);
    let t_fight = match py.rules.get("t_fight") {
        Some(Ev::Float(x)) => *x,
        other => {
            panic!("{pair} {what}: the Python recording's rules.t_fight is {other:?}: the floor is measured from it")
        }
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
        if rep.hull_fallbacks > 0 || calls.iter().any(|c| c.1.fallback.is_some()) {
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
    report(&f, "frames", "pair", "golden frames", failures, rows.len());
}

/// The picture's VS card on the noise backdrop at the 16 times of `frames.json`'s `cards`.
pub fn title_card(pair: &str) {
    let f = folder(pair);
    let doc = json(&f, "frames.json");
    let (rec, mut ff) = episode(pair);
    ff.seed = Some(rec.seed);
    let pic = build_picture(Arc::new(rec), &ff).unwrap_or_else(|e| panic!("{pair}: the picture: {e}"));
    let backdrop = noise();
    let fps = f64::from(pic.ctx().show.grid.fps);
    let rows = list(&doc["cards"]);
    assert!(rows.len() >= 16, "{f}: 16 card frames");
    let mut failures = Vec::new();
    for r in rows {
        let n = index(&r[0]);
        let mut img = backdrop.clone();
        pic.card().draw(&mut img, n as f64 / fps);
        if sha(&img.data) != text(&r[1]) {
            failures.push((format!("card frame {n}"), "its bytes".to_string()));
        }
    }
    report(&f, "title", "pair", "card frames", failures, rows.len());
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
    report(&f, "sound", "pair", "soundtrack facts", failures, n);
}

/// Every stem window against the cache at -120 dBFS. Ignored.
pub fn stems(pair: &str) {
    let f = folder(pair);
    let kit = json(&f, "sound.json");
    let mut failures = Vec::new();
    let s = summary(pair);
    let mut n = compare("soundtrack", &s, &crate::sound::with_cached_rms(&kit), &mut failures);
    n += windows_against("soundtrack", &s.st, &kit["windows"], text(&kit["cache"]), &mut failures);
    report(
        &f,
        "stems",
        "pair",
        "soundtrack facts with the bed buses, and stem windows",
        failures,
        n,
    );
}

/// Every link of `pairs.json` occurs in the Rust run of its scenario at its time with its fields.
pub fn pair_links(sim: &Sim, pair: &str) {
    let doc = json(&folder(pair), "pairs.json");
    let mut missing = Vec::new();
    let links = list(&doc["links"]);
    let rules = json(&folder(pair), "episode.json")["rules"]
        .as_object()
        .map_or(0, |r| r.len());
    assert!(
        rules == 0 || !links.is_empty(),
        "{pair}: the pair has {rules} pair rule(s) but pairs.json covers no link: name its events in the spec's `links`"
    );
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
    assert!(
        !list(&doc["scenarios"]).is_empty(),
        "{pair}: pairs.json has no scenario fights"
    );
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

/// How many of the search's fits `native_render` renders before it turns to the kit's verdicts (ruling M3-R47).
pub const NATIVE_FITS: usize = 10;

/// A Studio-native fight of the pair from the command line's path: the search (`ballfight search --stop 10`) on
/// the fight file with its seed removed, then the render of its fits in order (at most [`NATIVE_FITS`]) until one
/// passes every check; it prints that fit's seed, K.O. and render time. When none of them passes (ruling M3-R47),
/// the kit's own checks of the first of those seeds (`native_checks.json`, exported with the spec's `native_seeds`)
/// must give the Studio's verdicts and numbers (`checks_parity`'s bars, through [`crate::checks::check_fight`]),
/// with at least one check failing in the kit too. Ignored (long; needs ffmpeg).
pub fn native_render(pair: &str) {
    let mut ff = fight_file(pair);
    ff.seed = None;
    let base = setup_from(&ff, roster(), 0).unwrap_or_else(|e| panic!("{pair}: {e}"));
    let sheet = BeatSheet::series([fighter(&ff.fighters[0]), fighter(&ff.fighters[1])]);
    let opts = SearchOpts {
        from: 1,
        stop_fits: NATIVE_FITS,
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
    let fits: Vec<_> = found
        .candidates
        .iter()
        .filter(|c| c.verdict.fits)
        .take(NATIVE_FITS)
        .collect();
    assert!(!fits.is_empty(), "{pair}: no fit in {} seeds", found.checked);
    let mut refused = Vec::new();
    for fit in fits {
        match render_fit(pair, &ff, fit.seed) {
            Ok((secs, out)) => {
                let after = if refused.is_empty() {
                    String::new()
                } else {
                    format!(" (after {} fits that did not: {refused:?})", refused.len())
                };
                println!(
                    "{pair}: seed {} (K.O. {:?}) rendered in {secs:.1} s to {out}, every check passed{after}",
                    fit.seed, fit.story.t_ko
                );
                return;
            }
            Err(why) => {
                println!(
                    "{pair}: seed {} (K.O. {:?}) does not pass every check: {why}",
                    fit.seed, fit.story.t_ko
                );
                refused.push(fit.seed);
            }
        }
    }
    native_parity(pair, &ff, &refused);
}

/// The render of one fit of the search: `Ok((seconds, the video's path))` when every check passes, else why not
/// (the job's refusal, or the checks that failed).
fn render_fit(pair: &str, ff: &FightFile, seed: u64) -> Result<(f64, String), String> {
    let mut ff = ff.clone();
    ff.seed = Some(seed);
    let setup = setup_from(&ff, roster(), seed).expect("the setup");
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
    );
    let secs = started.elapsed().as_secs_f64();
    if opts.out_root.exists() {
        std::fs::remove_dir_all(&opts.out_root).unwrap_or_else(|e| panic!("{}: {e}", opts.out_root.display()));
    }
    let done = done.map_err(|e| e.to_string())?;
    let failed = failed.into_inner().unwrap();
    if failed.is_empty() {
        Ok((secs, done.display().to_string()))
    } else {
        Err(format!("checks that failed: {failed:?}"))
    }
}

/// Why the kit's seeds of `native_checks.json` cannot stand for the rendered fits: they must be the first of them,
/// in the search's order. `None` when they are.
pub fn native_seeds_differ(rendered: &[u64], kit: &[u64]) -> Option<String> {
    if kit.is_empty() {
        return Some("native_checks.json names no seed".into());
    }
    (kit.len() > rendered.len() || rendered[..kit.len()] != *kit).then(|| {
        format!(
            "the kit's seeds {kit:?} are not the first fits the Studio rendered, {rendered:?}: export again with the \
             spec's native_seeds set to the first of these"
        )
    })
}

/// The checks of a kit checks document (a `checks.json`'s `checks`) that fail, by name.
pub fn failing_checks(checks: &Value) -> Vec<String> {
    checks
        .as_object()
        .map(|m| {
            m.iter()
                .filter(|(_, v)| v["pass"].as_bool() == Some(false))
                .map(|(k, _)| k.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Ruling M3-R47: no rendered fit passed every check; the kit's own checks of the first of them give the Studio's
/// verdicts and numbers, and the kit fails at least one check on each of them too.
fn native_parity(pair: &str, ff: &FightFile, rendered: &[u64]) {
    let f = folder(pair);
    assert!(
        crate::refs::reference(&format!("{f}/native_checks.json")).exists(),
        "{pair}: none of the {} fits {rendered:?} passes every check, and {f}/native_checks.json is missing: set \
         native_seeds in tools/studio_export/episode_{pair}.py to the first of them and export again (M3-R47)",
        rendered.len()
    );
    let doc = json(&f, "native_checks.json");
    let kit: Vec<u64> = list(&doc["seeds"])
        .iter()
        .map(|s| s.as_u64().expect("a seed"))
        .collect();
    if let Some(why) = native_seeds_differ(rendered, &kit) {
        panic!("{pair}: {why}");
    }
    for row in list(&doc["fights"]) {
        let seed = row["seed"].as_u64().expect("a seed");
        let kit_doc = crate::refs::cache_json(text(&row["checks"]));
        let fails = failing_checks(&kit_doc["checks"]);
        assert!(
            !fails.is_empty(),
            "{pair}: seed {seed} fails a check in the Studio but passes every check in the kit"
        );
        let mut one = ff.clone();
        one.seed = Some(seed);
        let setup = setup_from(&one, roster(), seed).expect("the setup");
        let rec = bf_sim::run(&setup, true).expect("the fight");
        crate::checks::check_fight(&f, &format!("native s{seed}"), rec, &one, &kit_doc);
        println!("{pair}: seed {seed}: the kit's checks are the Studio's; both fail {fails:?}");
    }
    println!(
        "{pair}: no fit passed every check, in the kit too: the {} rendered fits {rendered:?}; the kit's checks of \
         {kit:?} give the Studio's verdicts and numbers",
        rendered.len()
    );
}

#[cfg(test)]
mod tests {
    use super::{failing_checks, native_seeds_differ};

    #[test]
    fn the_kit_seeds_are_the_first_rendered_fits() {
        assert_eq!(native_seeds_differ(&[820, 75, 983, 203], &[820, 75, 983]), None);
        assert_eq!(native_seeds_differ(&[820, 75], &[820, 75]), None);
        assert!(native_seeds_differ(&[820, 75, 983], &[75, 820]).is_some());
        assert!(native_seeds_differ(&[820, 75, 983], &[820, 983]).is_some());
        assert!(native_seeds_differ(&[820], &[820, 75]).is_some());
        assert!(native_seeds_differ(&[820], &[]).is_some());
    }

    #[test]
    fn failing_checks_reads_the_verdicts() {
        let doc = serde_json::json!({"levels": {"pass": false}, "band": {"pass": true}, "flash": {"pass": false}});
        assert_eq!(failing_checks(&doc), ["flash", "levels"]);
        assert!(failing_checks(&serde_json::json!({"band": {"pass": true}})).is_empty());
    }
}
