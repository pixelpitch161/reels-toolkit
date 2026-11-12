//! Template T, a theme's classic bed: `studio/reference/themes/<name>/` (`theme.json`, `bed.json`, `voicings.json`)
//! against `bf_content::themes::<name>`. The bed alone is the theme's soundtrack under two of the kit's base
//! `Sound`s (silent): its 16-bit master, buses, RMS and notes on each reference config's own recording; then at a
//! second tempo, in a second key and with each theme option (cached recordings); and each optional hook called
//! directly.

use bf_content::SoundBuild;
use bf_core::fightfile::FightFile;
use bf_core::grid::Grid;
use bf_core::harmony::{Harmony, HarmonyError, chords_for, scale};
use bf_core::recording::Recording;
use bf_core::show::Show;
use bf_sound::mix::Mix;
use bf_sound::traits::BaseSound;
use bf_testdata::Npz;
use serde_json::Value;

use crate::refs::{cached_recording, committed_recording, hexf, json, list, num, report, studio, text};
use crate::sound::{Summary, compare, summarise, windows_against};

fn folder(name: &str) -> String {
    format!("themes/{name}")
}

/// The pair folder name of a kit config (`examples/flute_vs_trombone` -> `flute_vs_trombone`).
pub fn config_slug(config: &str) -> String {
    let n = config.rsplit('/').next().unwrap_or(config);
    n.strip_prefix("ballfight_").unwrap_or(n).to_string()
}

fn strings(v: &Value) -> Vec<String> {
    list(v).iter().map(|s| text(s).to_string()).collect()
}

/// The fight file of a reference config: its pair gate's `studio/fights/<pair>.toml` when there is one, else the
/// milestone 1 example's with the config's fighters, tempo, key, scale and chords (all the bed alone reads).
pub fn config_fight_file(c: &Value, rec: &Recording) -> FightFile {
    let pair = studio(&format!("fights/{}.toml", config_slug(text(&c["config"]))));
    let path = if pair.is_file() {
        pair
    } else {
        studio("fights/flute_vs_trombone.toml")
    };
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut ff = FightFile::parse(&src).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    ff.fighters = strings(&c["fighters"]);
    ff.tempo_bpm = num(&c["bpm"]);
    ff.music.theme = text(&c["mood"]).to_string();
    ff.music.key = text(&c["key"]).to_string();
    ff.music.scale = text(&c["scale"]).to_string();
    ff.music.chords_intro = strings(&c["intro"]);
    ff.music.chord_loop = strings(&c["loop"]);
    ff.music.chords_tail = strings(&c["tail"]);
    ff.seed = Some(rec.seed);
    let names = ff.fighters.clone();
    ff.music.roles.retain(|k, _| names.contains(k));
    ff.music
        .lines
        .retain(|k, _| names.iter().any(|f| k == f || bf_core::fightfile::is_extra_line(k, f)));
    ff.music.solos.retain(|k, _| names.contains(k));
    ff.music.title_hits.retain(|k, _| names.contains(k));
    ff
}

/// A variant's changes (bed.json's `ff`) on a fight file.
fn apply(ff: &mut FightFile, changes: &Value) {
    for (k, v) in changes.as_object().expect("ff") {
        match k.as_str() {
            "tempo_bpm" => ff.tempo_bpm = num(v),
            "key" => ff.music.key = text(v).to_string(),
            "scale" => ff.music.scale = text(v).to_string(),
            "chords_intro" => ff.music.chords_intro = strings(v),
            "chord_loop" => ff.music.chord_loop = strings(v),
            "chords_tail" => ff.music.chords_tail = strings(v),
            "theme_options" => {
                use bf_core::fightfile::OptionValue as O;
                for (o, x) in v.as_object().expect("options") {
                    let val = match x {
                        Value::Bool(b) => O::Bool(*b),
                        Value::Number(n) if n.is_i64() => O::Int(n.as_i64().unwrap()),
                        Value::Number(n) => O::Float(n.as_f64().unwrap()),
                        Value::String(s) => O::Str(s.clone()),
                        other => panic!("a theme option's value {other}"),
                    };
                    ff.music.theme_options.insert(o.clone(), val);
                }
            }
            other => panic!("bed.json: a variant changes {other}, which the harness does not know"),
        }
    }
}

/// The bed alone on `rec` under `ff`: both fighters the kit's base `Sound`.
fn bed_alone_of(rec: &Recording, ff: &FightFile) -> Summary {
    let show = Show::new(rec, ff).unwrap_or_else(|e| panic!("the show: {e}"));
    let mut build = SoundBuild::new(&show, rec, ff).unwrap_or_else(|e| {
        panic!(
            "the sound build: {e} (the bed alone needs the config's fighters' voices and parts in this app: a theme \
             gate runs once they are ported)"
        )
    });
    build.sounds = [Box::new(BaseSound { k: 0 }), Box::new(BaseSound { k: 1 })];
    summarise(&build)
}

fn configs(name: &str) -> Vec<(Value, Value, Recording)> {
    let f = folder(name);
    let theme = json(&f, "theme.json");
    let bed = json(&f, "bed.json");
    let cs = list(&theme["configs"]);
    let bs = list(&bed["configs"]);
    assert_eq!(cs.len(), bs.len(), "{f}: theme.json and bed.json name other configs");
    assert!(!cs.is_empty(), "{f}: no reference config");
    cs.iter()
        .zip(bs)
        .map(|(c, b)| {
            assert_eq!(c["config"], b["config"]);
            let rec = committed_recording(text(&c["recording"]), None);
            (c.clone(), b.clone(), rec)
        })
        .collect()
}

/// The bed alone on every reference config's own recording: theme.json's 16-bit master and bed.json's buses, RMS
/// and notes. Always runs.
pub fn bed_alone(name: &str) {
    let f = folder(name);
    let mut failures = Vec::new();
    let mut n = 0;
    for (c, b, rec) in configs(name) {
        let ff = config_fight_file(&c, &rec);
        let s = bed_alone_of(&rec, &ff);
        assert_eq!(text(&b["master_sha256"]), text(&c["bed_alone"]["master_sha256"]));
        n += compare(text(&c["config"]), &s, &b, &mut failures);
    }
    report(&f, "sound", "bed-alone facts", failures, n);
}

/// The variants (a second tempo, a second key, each option) on their cached recordings. Ignored.
pub fn bed_variants(name: &str) {
    let f = folder(name);
    let bed = json(&f, "bed.json");
    let cs = configs(name);
    let mut failures = Vec::new();
    let mut n = 0;
    let variants = list(&bed["variants"]);
    for v in variants {
        let (c, _, rec0) = cs
            .iter()
            .find(|(c, _, _)| c["config"] == v["config"])
            .expect("the variant's config");
        let rec = cached_recording(text(&v["recording"]));
        assert_eq!(rec.hash(), text(&v["rec_hash"]));
        let mut ff = config_fight_file(c, rec0);
        ff.seed = Some(rec.seed);
        apply(&mut ff, &v["ff"]);
        let s = bed_alone_of(&rec, &ff);
        n += compare(&format!("variant {}", text(&v["name"])), &s, v, &mut failures);
    }
    assert!(
        variants.len() >= 2 || !bed["variants"].is_array(),
        "{f}: a second tempo and a second key"
    );
    report(&f, "sound", "variant facts", failures, n);
}

/// Every stem window of the bed alone (configs and variants) against the cache at -120 dBFS. Ignored.
pub fn bed_stems(name: &str) {
    let f = folder(name);
    let bed = json(&f, "bed.json");
    let mut failures = Vec::new();
    let mut n = 0;
    let cs = configs(name);
    for (c, b, rec) in &cs {
        let s = bed_alone_of(rec, &config_fight_file(c, rec));
        n += windows_against(
            text(&c["config"]),
            &s.st,
            &b["windows"],
            text(&b["stems_cache"]),
            &mut failures,
        );
    }
    for v in list(&bed["variants"]) {
        let (c, _, rec0) = cs
            .iter()
            .find(|(c, _, _)| c["config"] == v["config"])
            .expect("its config");
        let rec = cached_recording(text(&v["recording"]));
        let mut ff = config_fight_file(c, rec0);
        ff.seed = Some(rec.seed);
        apply(&mut ff, &v["ff"]);
        let s = bed_alone_of(&rec, &ff);
        n += windows_against(
            &format!("variant {}", text(&v["name"])),
            &s.st,
            &v["windows"],
            text(&v["stems_cache"]),
            &mut failures,
        );
    }
    report(&f, "stems", "stem windows", failures, n);
}

/// The optional hooks the theme's bed has (`Bed::hooks`) are the methods its kit class defines.
pub fn hooks(name: &str) {
    let bed = json(&folder(name), "bed.json");
    let t = bf_content::themes::theme(name).unwrap_or_else(|| panic!("{name} is not a theme"));
    assert!(t.ported, "{name}: its THEME says ported: false");
    let b = t
        .bed(bf_content::themes::CLASSIC, &bf_content::themes::ThemeOptions::new())
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    let mut got: Vec<&str> = b.hooks().iter().map(|h| h.name()).collect();
    got.sort_unstable();
    let want: Vec<&str> = list(&bed["hooks"]).iter().map(text).collect();
    assert_eq!(got, want, "{name}: the bed's optional hooks");
}

/// The theme's card lists every reference config as a preset with the config's own values.
pub fn card(name: &str) {
    let theme = json(&folder(name), "theme.json");
    let t = bf_content::themes::theme(name).expect("a theme");
    assert!(!t.card.display.is_empty(), "{name}: its card has no name");
    for c in list(&theme["configs"]) {
        let p = t
            .card
            .presets
            .iter()
            .find(|p| p.config == text(&c["config"]))
            .unwrap_or_else(|| panic!("{name}: the card has no preset {}", c["config"]));
        assert_eq!(p.bpm, num(&c["bpm"]), "{name} {}", p.config);
        assert_eq!(
            (p.key, p.scale),
            (text(&c["key"]), text(&c["scale"])),
            "{name} {}",
            p.config
        );
        assert_eq!(p.chord_loop.to_vec(), strings(&c["loop"]), "{name} {}", p.config);
    }
}

/// Each optional hook called directly on an empty mix: every bus against the cache's float64 samples within
/// 1e-6 (-120 dBFS). Ignored.
pub fn hook_vectors(name: &str) {
    let f = folder(name);
    let bed = json(&f, "bed.json");
    if bed["hook_calls"].as_object().is_none_or(|m| m.is_empty()) {
        assert!(list(&bed["hooks"]).is_empty(), "{f}: hooks without vectors");
        return;
    }
    let npz = Npz::open(&bf_testdata::cache_file(text(&bed["hook_cache"])));
    let (c, _, rec) = configs(name).into_iter().next().expect("a config");
    let ff = config_fight_file(&c, &rec);
    let show = Show::new(&rec, &ff).expect("the show");
    let build = SoundBuild::new(&show, &rec, &ff).expect("the build");
    let dur = Grid::new(ff.tempo_bpm).dur;
    let mut failures = Vec::new();
    let mut n = 0;
    for (hook, calls) in bed["hook_calls"].as_object().unwrap() {
        for (j, call) in list(calls).iter().enumerate() {
            let a = &call["args"];
            let mut mix = Mix::new(dur + 4.0);
            let ctx = &build.ctx;
            let played = match hook.as_str() {
                "boom" => build.bed.boom(ctx, &mut mix, hexf(&a["t"]), hexf(&a["g"])),
                "hit" => build.bed.hit(
                    ctx,
                    &mut mix,
                    hexf(&a["t"]),
                    text(&a["chord"]),
                    hexf(&a["g"]),
                    hexf(&a["ring"]),
                ),
                "press_roll" => build.bed.press_roll(
                    ctx,
                    &mut mix,
                    hexf(&a["t0"]),
                    hexf(&a["t1"]),
                    hexf(&a["g0"]),
                    hexf(&a["g1"]),
                ),
                "erupt" => build.bed.erupt(ctx, &mut mix, hexf(&a["t"])),
                "trailer_hit" => build.bed.trailer_hit(
                    ctx,
                    &mut mix,
                    hexf(&a["t"]),
                    hexf(&a["taiko_g"]),
                    a["seed"].as_i64().unwrap(),
                    a["braam"].as_str().map(bf_testdata::parse_hex_f64),
                    hexf(&a["braam_g"]),
                ),
                other => panic!("bed.json: an unknown hook {other}"),
            };
            played.unwrap_or_else(|e| panic!("{name} {hook}: {e}"));
            for bus in call["buses"].as_object().unwrap().keys() {
                n += 1;
                let want = npz.get(&format!("{hook}{j}__{bus}")).f64();
                let got = mix.bus(bus).map(|x| x.as_flattened().to_vec()).unwrap_or_default();
                let worst = if got.len() == want.len() {
                    got.iter().zip(want).map(|(g, w)| (g - w).abs()).fold(0.0, f64::max)
                } else {
                    f64::INFINITY
                };
                if worst.is_nan() || worst > 1e-6 {
                    failures.push((format!("{hook} call {j} bus {bus}"), format!("{worst:e}")));
                }
            }
        }
    }
    report(&f, "sound", "hook buses", failures, n);
}

/// The voicing functions of a theme (its own: a theme port passes them) for the 12 roots x 13 qualities of
/// `voicings.json`, each chord in a harmony that holds it, as the kit's episode does.
pub type Voicing = fn(&Harmony, &str) -> Result<Vec<i32>, HarmonyError>;

pub fn voicings(name: &str, methods: &[(&str, Voicing)]) {
    let doc = json(&folder(name), "voicings.json");
    let v = &doc["voicings"];
    let g = Grid::new(132.0);
    let mut n = 0;
    for (method, f) in methods {
        let table = v[*method]
            .as_object()
            .unwrap_or_else(|| panic!("voicings.json has no {method}"));
        assert_eq!(table.len(), 12 * 13, "{name}: {method}");
        for (ch, want) in table {
            let one = vec![ch.clone()];
            let h = Harmony::new(
                &g,
                "E",
                scale("harmonic minor").unwrap(),
                chords_for(&one, &one, &[], g.n_bars),
                Some(one.clone()),
                2,
            )
            .unwrap_or_else(|e| panic!("{ch}: {e}"));
            let want: Vec<i32> = list(want).iter().map(|x| x.as_i64().unwrap() as i32).collect();
            assert_eq!(
                f(&h, ch).unwrap_or_else(|e| panic!("{method}({ch}): {e}")),
                want,
                "{name}: {method}({ch})"
            );
            n += 1;
        }
    }
    assert_eq!(
        methods.len(),
        list(&json(&folder(name), "theme.json")["voicing_methods"]).len(),
        "{name}: every voicing method"
    );
    println!("{name}: {n} voicings equal the kit's");
}
