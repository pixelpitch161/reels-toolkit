//! Template T, a theme's classic bed: `studio/reference/themes/<name>/` (`theme.json`, `bed.json`, `voicings.json`)
//! against `bf_content::themes::<name>`. The bed alone is the theme's soundtrack under two of the kit's base
//! `Sound`s (silent): its 16-bit master, buses, RMS and notes on each reference config's own recording; then at a
//! second tempo, in a second key and with each theme option (cached recordings); and each optional hook called
//! directly.

use std::sync::Arc;

use bf_content::SoundBuild;
use bf_core::fightfile::FightFile;
use bf_core::grid::Grid;
use bf_core::harmony::{Harmony, HarmonyError, chords_for, scale};
use bf_core::recording::Recording;
use bf_core::show::Show;
use bf_sound::mix::Mix;
use bf_sound::voice::{PpNoise, Voice};
use bf_testdata::Npz;
use serde_json::Value;

use crate::refs::{cached_recording, hexf, json, list, named_recording, num, report, studio, text};
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
/// milestone 1 example's with the config's fighters, tempo, key, scale and chords (all the bed alone reads), and the
/// theme options the config sets (theme.json's `theme_options`, under the fight file's names; absent: none, the
/// theme's defaults).
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
    if let Some(o) = c.get("theme_options") {
        apply(&mut ff, &serde_json::json!({ "theme_options": o }));
    }
    let names = ff.fighters.clone();
    ff.music.roles.retain(|k, _| names.contains(k));
    ff.music
        .lines
        .retain(|k, _| names.iter().any(|f| k == f || bf_core::fightfile::is_extra_line(k, f)));
    ff.music.solos.retain(|k, _| names.contains(k));
    // the config's own LINES (theme.json's `lines`, a rest as null) and ROLES (`roles`, when it sets them): a bed may
    // play a fighter's line or read who plays what (dark_tango's `Bed.theme()` plays the melody fighter's line)
    if let Some(lines) = c.get("lines").and_then(Value::as_object) {
        for (k, v) in lines {
            ff.music.lines.insert(
                k.clone(),
                list(v).iter().map(|n| n.as_i64().map(|n| n as i32)).collect(),
            );
        }
    }
    if let Some(roles) = c.get("roles").and_then(Value::as_object) {
        ff.music.roles = roles.iter().map(|(k, v)| (k.clone(), text(v).to_string())).collect();
    }
    // and the rest a config sets that the soundtrack reads (m3refs.MUSIC_EXTRAS): its MIX (a bus's gain and reverb
    // send: second_line's bed buses), SOLO and TITLE_HITS, each only when the config sets it
    if let Some(mix) = c.get("mix").and_then(Value::as_object) {
        ff.music.mix = mix
            .iter()
            .map(|(k, v)| (k.clone(), (num(&list(v)[0]), num(&list(v)[1]))))
            .collect();
    }
    if let Some(solos) = c.get("solos").and_then(Value::as_object) {
        for (k, v) in solos {
            // single notes, and a chord as a list of notes (ACOUSTIC's strums)
            ff.music.solos.insert(k.clone(), crate::refs::solo(v));
        }
    }
    if let Some(hits) = c.get("title_hits").and_then(Value::as_object) {
        for (k, v) in hits {
            let notes = list(v)
                .iter()
                .map(|n| n.as_i64().expect("a MIDI note") as i32)
                .collect();
            ff.music.title_hits.insert(k.clone(), notes);
        }
    }
    ff.music.title_hits.retain(|k, _| names.contains(k));
    ff
}

/// Theme options as a reference file writes them (`{"sequel": true}`, `{"plugged": "ELECTRIC"}`), as a fight file
/// holds them.
pub fn option_values(v: &Value) -> std::collections::BTreeMap<String, bf_core::fightfile::OptionValue> {
    use bf_core::fightfile::OptionValue as O;
    v.as_object()
        .unwrap_or_else(|| panic!("theme options: {v} is not a table"))
        .iter()
        .map(|(o, x)| {
            let val = match x {
                Value::Bool(b) => O::Bool(*b),
                Value::Number(n) if n.is_i64() => O::Int(n.as_i64().unwrap()),
                Value::Number(n) => O::Float(n.as_f64().unwrap()),
                Value::String(s) => O::Str(s.clone()),
                other => panic!("a theme option's value {other}"),
            };
            (o.clone(), val)
        })
        .collect()
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
            "theme_options" => ff.music.theme_options.extend(option_values(v)),
            other => panic!("bed.json: a variant changes {other}, which the harness does not know"),
        }
    }
}

/// A fighter's voice in a theme gate when its own is not in this app yet: the roster's voice id, and a panic in
/// plain words if anything ever plays it (the bed alone plays no fighter's voice; a bed that does fails its gate).
/// It lives only here, never in `bf-content`'s registry.
pub struct StandIn {
    pub fighter: String,
    pub pp: String,
}

impl Voice for StandIn {
    fn note(&self, _: &mut PpNoise, midi: f64, _: f64, _: f64) -> Vec<f64> {
        panic!(
            "the bed played {}'s voice (MIDI {midi}), which is a stand-in of the theme gate: the bed alone must play no \
             fighter's voice, or the theme waits for that fighter's port",
            self.fighter
        )
    }
    fn name(&self) -> &str {
        &self.pp
    }
    fn register(&self) -> (i32, i32) {
        panic!(
            "the bed asked for {}'s register, a stand-in of the theme gate",
            self.fighter
        )
    }
}

/// The voices of the two fighters of `show` for a bed-alone build: each fighter's own when it is in this app, a
/// [`StandIn`] when not (or for both when `stand_ins`).
pub fn bed_voices(show: &Show, stand_ins: bool) -> [Arc<dyn Voice>; 2] {
    [0, 1].map(|k| {
        let name = &show.names[k];
        let own = if stand_ins {
            None
        } else {
            bf_content::fighters::voice(name)
        };
        own.unwrap_or_else(|| {
            let pp = crate::fight::fighter(name).pp.clone();
            Arc::new(StandIn {
                fighter: name.clone(),
                pp,
            }) as Arc<dyn Voice>
        })
    })
}

/// The bed alone on `rec` under `ff`: both fighters the kit's base `Sound`, their voices as [`bed_voices`] gives.
fn bed_alone_of(rec: &Recording, ff: &FightFile, stand_ins: bool) -> Summary {
    let show = Show::new(rec, ff).unwrap_or_else(|e| panic!("the show: {e}"));
    let build = SoundBuild::bed_alone(&show, rec, ff, bed_voices(&show, stand_ins))
        .unwrap_or_else(|e| panic!("the bed-alone build: {e}"));
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
            let rec = named_recording(&c["recording"], None);
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
        let s = bed_alone_of(&rec, &ff, false);
        assert_eq!(text(&b["master_sha256"]), text(&c["bed_alone"]["master_sha256"]));
        n += compare(text(&c["config"]), &s, &b, &mut failures);
    }
    report(&f, "sound", "bed", "bed-alone facts", failures, n);
}

/// The bed alone with stand-ins for BOTH fighters' voices: the same 16-bit master as theme.json's. It proves that
/// the bed plays no fighter voice, so a theme gate never waits for a fighter's port.
pub fn bed_alone_stand_ins(name: &str) {
    for (c, b, rec) in configs(name) {
        let s = bed_alone_of(&rec, &config_fight_file(&c, &rec), true);
        assert_eq!(
            crate::sound::pcm_sha(&s.st.master),
            text(&b["master_sha256"]),
            "{name} {}: the bed alone with stand-in voices",
            c["config"]
        );
        println!(
            "{name} {}: the bed alone with two stand-in voices is the kit's master",
            c["config"]
        );
    }
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
        let s = bed_alone_of(&rec, &ff, false);
        n += compare(&format!("variant {}", text(&v["name"])), &s, v, &mut failures);
    }
    assert!(
        variants.len() >= 2,
        "{f}: {} variants; at least a second tempo and a second key",
        variants.len()
    );
    report(&f, "sound", "variants", "variant facts", failures, n);
}

/// Every stem window of the bed alone (configs and variants) against the cache at -120 dBFS. Ignored.
pub fn bed_stems(name: &str) {
    let f = folder(name);
    let bed = json(&f, "bed.json");
    let mut failures = Vec::new();
    let mut n = 0;
    let cs = configs(name);
    for (c, b, rec) in &cs {
        let s = bed_alone_of(rec, &config_fight_file(c, rec), false);
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
        let s = bed_alone_of(&rec, &ff, false);
        n += windows_against(
            &format!("variant {}", text(&v["name"])),
            &s.st,
            &v["windows"],
            text(&v["stems_cache"]),
            &mut failures,
        );
    }
    report(&f, "stems", "stems", "stem windows", failures, n);
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
    let hooks: Vec<&str> = list(&bed["hooks"]).iter().map(text).collect();
    let calls = bed["hook_calls"].as_object().cloned().unwrap_or_default();
    let mut called: Vec<&str> = calls.keys().map(String::as_str).collect();
    called.sort_unstable();
    assert_eq!(called, hooks, "{f}: every optional hook has its calls");
    if hooks.is_empty() {
        println!("{name}: its bed has no optional hook");
        return;
    }
    for (hook, c) in &calls {
        assert_eq!(list(c).len(), 3, "{f}: {hook}: three calls");
    }
    let npz = Npz::open(&bf_testdata::cache_file(text(&bed["hook_cache"])));
    let (c, _, rec) = configs(name).into_iter().next().expect("a config");
    let ff = config_fight_file(&c, &rec);
    let show = Show::new(&rec, &ff).expect("the show");
    let build = SoundBuild::bed_alone(&show, &rec, &ff, bed_voices(&show, false)).expect("the build");
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
                    got.iter()
                        .zip(want)
                        .map(|(g, w)| (g - w).abs())
                        .fold(0.0, crate::refs::gap_max)
                } else {
                    f64::INFINITY
                };
                if worst.is_nan() || worst > 1e-6 {
                    failures.push((format!("{hook} call {j} bus {bus}"), format!("{worst:e}")));
                }
            }
        }
    }
    report(&f, "sound", "hooks", "hook buses", failures, n);
}

/// The voicing functions of a theme (its own: a theme port passes them) for the 12 roots x 13 qualities of
/// `voicings.json`, each chord in a harmony that holds it, as the kit's episode does.
pub type Voicing = fn(&Harmony, &str) -> Result<Vec<i32>, HarmonyError>;

/// The theme's `voicings.json`, or nothing (`Value::Null`) when it has no voicing method: such a theme needs no file
/// (theme.json's `voicing_methods` must then be empty too, which [`voicings`] checks).
fn voicings_doc(name: &str, methods: &[(&str, Voicing)]) -> Value {
    if methods.is_empty() {
        Value::Null
    } else {
        json(&folder(name), "voicings.json")
    }
}

pub fn voicings(name: &str, methods: &[(&str, Voicing)]) {
    let doc = voicings_doc(name, methods);
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
    let mut given: Vec<&str> = methods.iter().map(|m| m.0).collect();
    let theme_doc = json(&folder(name), "theme.json");
    let mut kit: Vec<&str> = list(&theme_doc["voicing_methods"]).iter().map(text).collect();
    given.sort_unstable();
    kit.sort_unstable();
    assert_eq!(
        given, kit,
        "{name}: the voicing functions given against the kit bed's voicing methods"
    );
    assert!(n > 0 || kit.is_empty(), "{name}: no voicing compared");
    println!("{name}: {n} voicings equal the kit's");
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_core::fightfile::OptionValue;

    /// A reference config as theme.json writes it (brass_trailer's rematch, dark_drumline's
    /// `ballfight_bassdrum_vs_tuba`), on milestone 1's seed 1.
    fn config(options: Option<Value>) -> (Value, Recording) {
        let mut c = serde_json::json!({
            "config": "ballfight_nobody_vs_nobody", "fighters": ["FLUTE", "TROMBONE"], "bpm": 132.0,
            "mood": "brass_trailer", "key": "E", "scale": "minor", "intro": ["Em", "Em"], "loop": ["Em", "C"],
            "tail": [],
        });
        if let Some(o) = options {
            c["theme_options"] = o;
        }
        let rec = crate::refs::committed_recording("fights/flute_vs_trombone/seed1.bfrec", None);
        (c, rec)
    }

    #[test]
    fn a_configs_theme_options_reach_its_fight_file() {
        let (c, rec) = config(Some(serde_json::json!({"sequel": true})));
        let ff = config_fight_file(&c, &rec);
        assert_eq!(ff.music.theme_options["sequel"], OptionValue::Bool(true));
        let (c, rec) = config(Some(serde_json::json!({"bass_drums": false, "after_ko": true})));
        let ff = config_fight_file(&c, &rec);
        assert_eq!(
            (
                ff.music.theme_options["bass_drums"].clone(),
                ff.music.theme_options["after_ko"].clone()
            ),
            (OptionValue::Bool(false), OptionValue::Bool(true))
        );
        let (c, rec) = config(Some(serde_json::json!({"plugged": "TROMBONE"})));
        assert_eq!(
            config_fight_file(&c, &rec).music.theme_options["plugged"],
            OptionValue::Str("TROMBONE".into())
        );
    }

    #[test]
    fn a_theme_without_voicing_methods_needs_no_voicings_file() {
        assert_eq!(voicings_doc("no_such_theme", &[]), Value::Null);
    }

    #[test]
    #[should_panic(expected = "studio/reference/themes/no_such_theme/voicings.json is missing")]
    fn a_theme_with_voicing_methods_reads_its_file() {
        fn none(_: &Harmony, _: &str) -> Result<Vec<i32>, HarmonyError> {
            Ok(vec![])
        }
        voicings_doc("no_such_theme", &[("root", none)]);
    }

    #[test]
    fn a_configs_lines_and_roles_reach_its_fight_file() {
        let (mut c, rec) = config(None);
        c["lines"] = serde_json::json!({"FLUTE": [76, null, 79]});
        c["roles"] = serde_json::json!({"FLUTE": "bass", "TROMBONE": "melody"});
        let ff = config_fight_file(&c, &rec);
        assert_eq!(ff.music.lines["FLUTE"], vec![Some(76), None, Some(79)]);
        assert_eq!(
            (ff.music.roles["FLUTE"].as_str(), ff.music.roles["TROMBONE"].as_str()),
            ("bass", "melody")
        );
        // the MIX, SOLO and TITLE_HITS the config sets
        c["mix"] = serde_json::json!({"sousa": [1.45, 0.10], "drums": [1.25, 0.14]});
        c["solos"] = serde_json::json!({"TROMBONE": [[0.0, 40, 0.5], [0.5, 47, 1.0]]});
        c["title_hits"] = serde_json::json!({"TROMBONE": [40, 52, 59]});
        let ff = config_fight_file(&c, &rec);
        assert_eq!(ff.music.mix.len(), 2);
        assert_eq!(ff.music.mix["sousa"], (1.45, 0.10));
        assert_eq!(
            ff.music.solos["TROMBONE"],
            bf_core::fightfile::solo_of_notes(&[(0.0, 40, 0.5), (0.5, 47, 1.0)])
        );
        assert_eq!(ff.music.title_hits["TROMBONE"], vec![40, 52, 59]);
        // a chord in a solo (the export writes the kit's tuple as a list of notes) reaches the fight file as one
        c["solos"] = serde_json::json!({"TROMBONE": [[0.0, 40, 0.5], [3.0, [40, 47, 52], 1.0]]});
        let ff = config_fight_file(&c, &rec);
        assert_eq!(
            ff.music.solos["TROMBONE"][1],
            (3.0, bf_core::fightfile::SoloNote::Chord(vec![40, 47, 52]), 1.0)
        );
        // without them: the base fight file's, for the config's fighters
        let (c, rec) = config(None);
        let base = config_fight_file(&c, &rec);
        assert_eq!(base.music.roles["FLUTE"], "melody");
    }

    #[test]
    fn a_config_without_theme_options_has_none() {
        let (c, rec) = config(None);
        assert!(config_fight_file(&c, &rec).music.theme_options.is_empty());
    }
}
