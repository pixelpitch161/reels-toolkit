//! The roll's unit tests. The rows of `roll_rows.json` are what `kit/story.py` gave on the reference PC for six
//! configs (`tools/studio_export/unit_checks/roll_unit_vectors.py` writes the file); the fights themselves are
//! proven against the kit by PIANO's suite (`bf-harness/tests/fighter_piano.rs`).

use bf_core::event::Value;
use bf_core::fightfile::FightFile;
use serde_json::Value as Json;

use super::*;
use crate::fight::{Setup, setup_from};
use crate::testing;

/// The kit's rows for six configs (and its refusal of a seventh), as `roll_unit_vectors.py` wrote them.
const VECTORS: &str = include_str!("roll_rows.json");

fn strings(v: &Json) -> Vec<String> {
    let list = v.as_array().expect("a list");
    list.iter().map(|s| s.as_str().expect("a text").to_string()).collect()
}

/// The example fight file with PIANO in the left corner and a vector case's music (the kit config's key, scale,
/// chords and PIANO line; the other fighter is FLUTE, whose line the rows do not read).
fn fight_file(case: &Json) -> FightFile {
    let mut ff = testing::example_fight_file();
    ff.fighters = vec!["PIANO".to_string(), "FLUTE".to_string()];
    ff.pair_rules.clear();
    ff.tempo_bpm = case["tempo_bpm"].as_f64().expect("a tempo");
    ff.music.key = case["key"].as_str().expect("a key").to_string();
    ff.music.scale = case["scale"].as_str().expect("a scale").to_string();
    ff.music.chords_intro = strings(&case["chords_intro"]);
    ff.music.chord_loop = strings(&case["chord_loop"]);
    ff.music.chords_tail.clear();
    ff.music.lines.clear();
    if let Some(line) = case["line"].as_array() {
        let notes = line.iter().map(|m| m.as_i64().map(|m| m as i32)).collect();
        ff.music.lines.insert("PIANO".to_string(), notes);
    }
    ff
}

/// `kit/story.py` `roll_rows` for six configs: 8 a bar with rests, 4 a bar, the sketch line, a 4-chord loop, notes
/// above and below the range, a line that is nearly all rests. And a line of 6 a bar is refused, naming the line.
#[test]
fn roll_rows_are_the_kits_for_six_configs() {
    let roster = testing::roster();
    let cases: Vec<Json> = serde_json::from_str(VECTORS).expect("the vectors parse");
    let mut names = Vec::new();
    for case in &cases {
        let name = case["name"].as_str().expect("a name");
        names.push(name);
        let ff = fight_file(case);
        let got = setup_from(&ff, &roster, 1);
        if let Some(why) = case.get("error") {
            // Python: AssertionError("LINES['PIANO']: 8 (or 4) notes a bar over the 8-bar LOOP")
            assert_eq!(
                why.as_str(),
                Some("LINES['PIANO']: 8 (or 4) notes a bar over the 8-bar LOOP")
            );
            let e = got.expect_err("a line of 6 notes a bar is refused");
            assert_eq!(e.field, "music.lines.PIANO", "{name}");
            assert_eq!(
                e.message,
                "48 notes over the 8-chord loop: a roll's line has 8 notes a bar (or 4), so 64 or 32"
            );
            continue;
        }
        let rules = got.unwrap_or_else(|e| panic!("{name}: {e}")).rules;
        let want: Vec<Vec<i64>> = case["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| {
                r.as_array()
                    .expect("a row")
                    .iter()
                    .map(|m| m.as_i64().expect("a note"))
                    .collect()
            })
            .collect();
        assert!(want.len() >= 32, "{name}");
        let rows = rules
            .roll_rows
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: no roll_rows"));
        assert_eq!(
            rows.keys().copied().collect::<Vec<_>>(),
            [0],
            "{name}: PIANO is fighter 0"
        );
        assert_eq!(rows[&0], want, "{name}");
        assert_eq!(rules.roll_start8, case["roll_start8"].as_i64(), "{name}");
        // the recorded rules list them as [fighter index, rows] pairs (ruling M3-R19)
        let listed = &rules.to_values()["roll_rows"];
        let Value::List(pairs) = listed else {
            panic!("{name}: {listed:?}");
        };
        assert_eq!(pairs.len(), 1, "{name}");
    }
    assert_eq!(
        names,
        [
            "eight_a_bar",
            "four_a_bar",
            "sketch",
            "four_chords",
            "outside",
            "mostly_rests",
            "six_a_bar"
        ]
    );
}

/// PIANO on the right is fighter 1; a fight without a roller has no `roll_rows` and no `roll_start8`, so its rules
/// (and its content hash) are what they were.
#[test]
fn the_rows_are_keyed_by_the_fighters_index_and_only_a_roller_has_them() {
    let roster = testing::roster();
    let mut ff = testing::example_fight_file();
    let plain = setup_from(&ff, &roster, 1).expect("the example sets up").rules;
    assert_eq!((&plain.roll_rows, plain.roll_start8), (&None, None));
    ff.pair_rules.clear();
    ff.fighters = vec!["FLUTE".to_string(), "PIANO".to_string()];
    let right = setup_from(&ff, &roster, 1).expect("PIANO on the right").rules;
    let rows = right.roll_rows.expect("a roller's rules have its rows");
    assert_eq!(rows.keys().copied().collect::<Vec<_>>(), [1]);
    assert_eq!(right.roll_start8, Some(16));
    ff.fighters = vec!["PIANO".to_string(), "FLUTE".to_string()];
    let left = setup_from(&ff, &roster, 1).expect("PIANO on the left").rules;
    // the same line on either side: the rows are the fighter's, not the corner's
    assert_eq!(left.roll_rows.expect("rows")[&0], rows[&1]);
}

fn piano(roster: &[crate::roster::FighterCfg]) -> Setup<'_> {
    let mut ff = testing::example_fight_file();
    ff.pair_rules.clear();
    ff.handicap = vec![1.0, 1.0];
    ff.fighters = vec!["PIANO".to_string(), "FLUTE".to_string()];
    setup_from(&ff, roster, 7).expect("PIANO against FLUTE sets up")
}

/// `roll_setup`: the roll lies on the side through its own corner; with rows in the rules they are the ball's, from
/// the rules' first 8th; without, `ROLL_DEFAULT` round the middle of the range, from 8th 0.
#[test]
fn setup_takes_the_rules_rows_or_the_default_roll() {
    let roster = testing::roster();
    let mut s = piano(&roster);
    let mut fight = testing::fight(&s, false);
    setup(&mut fight, 0);
    let r = fight.roll.ball[0].clone().expect("a roller");
    // the left corner is at x < 0: axis 0
    assert_eq!((r.ax, r.start8, r.rows.len()), (0, 16, 64));
    assert_eq!(r.rows, s.rules.roll_rows.as_ref().expect("rows")[&0]);
    assert_eq!(
        (fight.roll.sustain_until, fight.roll.reload_until),
        ([-1.0; 2], [-1.0; 2])
    );
    assert_eq!(fight.balls[0].n_vol, 1.0);
    // no rows in the rules: mid = (48 + 84) // 2 = 66
    s.rules.roll_rows = None;
    s.rules.roll_start8 = None;
    let mut fight = testing::fight(&s, false);
    setup(&mut fight, 0);
    let r = fight.roll.ball[0].clone().expect("a roller");
    assert_eq!(r.start8, 0);
    assert_eq!(r.rows.len(), 16);
    assert_eq!(r.rows[0], [66, 54]);
    assert_eq!(r.rows[1], Vec::<i64>::new());
    assert_eq!(r.rows[12], [81, 69]);
}

/// `roll_tick` and `fire_row`: only on an even 16th, not while a new roll is threaded, not for a rest; a row fires
/// its first `int(n_vol)` holes (at least one), one shot per hole from its lane, in one volley, each carrying its
/// pitch; while sustained each gets its stop and release times.
#[test]
fn a_row_fires_one_shot_per_hole() {
    let roster = testing::roster();
    let s = piano(&roster);
    let mut fight = testing::fight(&s, false);
    setup(&mut fight, 0);
    let rows = fight.roll.ball[0].clone().expect("a roller").rows;
    let beat = fight.rules.beat;
    fight.t = 8.0 * beat;
    // an odd 16th: nothing
    tick(&mut fight, 0, 33);
    assert!(fight.keys.is_empty() && fight.events.is_empty());
    // 16th 32 is 8th 16, the roll's first row
    tick(&mut fight, 0, 32);
    assert_eq!(fight.keys.len(), 1);
    let key = fight.keys[0].clone();
    assert_eq!((key.own, key.id, key.vol, key.idx, key.dmg), (0, 0, 0, rows[0][0], 0.5));
    // from the roll's side (x = -A + 0.1), at the weapon's speed
    assert_eq!(key.p[0], -fight.rules.a + 0.1);
    assert!((geom::norm(key.v) - 5.2).abs() < 1e-12);
    assert!(key.v[0] > 0.0, "it flies into the ring");
    let e = fight.events.last().expect("a roll event").clone();
    assert_eq!(e.kind, "roll");
    assert_eq!(
        e.fields,
        vec![
            Value::Int(0),
            Value::Int(0),
            Value::Int(16),
            Value::Int(0),
            Value::List(vec![Value::Int(rows[0][0])])
        ]
    );
    // three holes once the roll has grown; sustained: each hangs, staggered by a 16th
    fight.balls[0].n_vol = 3.0;
    fight.roll.sustain_until[0] = fight.t + 1.0;
    tick(&mut fight, 0, 34);
    let n = rows[1].len().min(3);
    assert!(n >= 2, "the sketch line's second row has harmony");
    assert_eq!(fight.keys.len(), 1 + n);
    assert_eq!(fight.roll.khold.len(), n);
    for (j, (id, h)) in fight.roll.khold.iter().enumerate() {
        assert_eq!(*id, 1 + j as i64);
        // out16 = 2, step16 = 1, hang_beats = 1
        assert_eq!(h.t_stop, fight.t + (2.0 + j as f64) * beat / 4.0);
        assert_eq!(h.t_rel, h.t_stop + beat);
        assert_eq!((h.sp, h.stopped), (5.2, false));
    }
    assert_eq!(fight.events.last().expect("a roll event").fields[3], Value::Int(1));
    // while a new roll is threaded no row fires
    let before = fight.keys.len();
    fight.roll.reload_until[0] = fight.t + 1.0;
    tick(&mut fight, 0, 36);
    assert_eq!(fight.keys.len(), before);
}

/// `hold_step`: a sustained shot stops at its time, hangs, then flies at the enemy at its speed; a shot that is
/// gone is forgotten.
#[test]
fn a_sustained_shot_stops_hangs_and_flies_at_the_enemy() {
    let roster = testing::roster();
    let s = piano(&roster);
    let mut fight = testing::fight(&s, false);
    setup(&mut fight, 0);
    fight.t = 10.0;
    fight.roll.sustain_until[0] = 11.0;
    fire_row(&mut fight, 0, &[60, 64], 40);
    let (stop0, rel0) = (fight.roll.khold[&0].t_stop, fight.roll.khold[&0].t_rel);
    hold_step(&mut fight);
    assert!(fight.keys.iter().all(|k| k.v != [0.0, 0.0]), "not yet");
    fight.t = stop0;
    hold_step(&mut fight);
    assert_eq!(fight.keys[0].v, [0.0, 0.0]);
    assert_ne!(fight.keys[1].v, [0.0, 0.0]);
    assert!(fight.roll.khold[&0].stopped && !fight.roll.khold[&1].stopped);
    // the second shot leaves the fight: its entry goes at the next step
    fight.keys.truncate(1);
    fight.t = rel0;
    fight.balls[1].p = [fight.keys[0].p[0] + 3.0, fight.keys[0].p[1] + 4.0];
    hold_step(&mut fight);
    assert!(fight.roll.khold.is_empty());
    // d = (3, 4) / 5, times the speed 5.2
    assert_eq!(fight.keys[0].v, [3.0 / 5.0 * 5.2, 4.0 / 5.0 * 5.2]);
}

/// `start_runoff`: `n` shots at once from lanes spread over the whole side, straight across, in one volley, each
/// with its rounded pitch; then no row until the reload is over.
#[test]
fn the_runoff_fires_every_lane_at_once() {
    let roster = testing::roster();
    let s = piano(&roster);
    let mut fight = testing::fight(&s, false);
    setup(&mut fight, 0);
    fight.t = 20.0;
    let ab = Runoff {
        n: 40.0,
        speed: 5.0,
        dmg: 0.45,
        margin: 0.25,
        reload_beats: 4.0,
    };
    start_runoff(&mut fight, 0, ab);
    assert_eq!(fight.keys.len(), 40);
    assert!(
        fight
            .keys
            .iter()
            .all(|k| k.v == [5.0, 0.0] && k.vol == 0 && k.dmg == 0.45)
    );
    // m = lo + (hi - lo) * j / (n - 1): 48 to 84, low notes at the own corner's end (+)
    assert_eq!((fight.keys[0].idx, fight.keys[39].idx), (48, 84));
    let a = fight.rules.a;
    assert_eq!(fight.keys[0].p, [-a + 0.1, a - 0.25]);
    assert_eq!(fight.keys[39].p, [-a + 0.1, (a - 0.25) - 2.0 * (a - 0.25)]);
    assert_eq!(fight.roll.reload_until[0], 20.0 + 4.0 * fight.rules.beat);
    let e = fight.events.last().expect("a runoff event");
    assert_eq!(
        (e.kind.as_str(), &e.fields[..3]),
        ("runoff", &[Value::Int(0), Value::Int(0), Value::Int(40)][..])
    );
    let Value::List(ms) = &e.fields[3] else {
        panic!("{:?}", e.fields[3]);
    };
    assert_eq!(ms.len(), 40);
    // int(round(m)): 48 + 36 * 13 / 39 = 60.0, 48 + 36 * 1 / 39 = 48.92 -> 49
    assert_eq!((&ms[13], &ms[1]), (&Value::Int(60), &Value::Int(49)));
}

/// `move`, the projectiles: a shot past a rope leaves with a `wall` event; a shot that reaches the enemy hits it
/// (`key`, the shot's index as the hit's sub-number), a landed volley grows the roll once, and the ball is nudged
/// along the shot's flight without changing its speed; a shot never hits its owner.
#[test]
fn shots_fly_leave_at_the_ropes_and_hit_the_enemy() {
    let roster = testing::roster();
    let s = piano(&roster);
    let mut fight = testing::fight(&s, false);
    setup(&mut fight, 0);
    fight.t = 12.0;
    let dt = fight.rules.dt;
    let a = fight.rules.a;
    fight.balls[0].p = [-2.0, -2.0];
    fight.balls[1].p = [1.0, 1.0];
    fight.balls[1].v = [0.0, 3.0];
    let shot = |pos, ang, vol, idx| Shot {
        pos,
        ang,
        speed: 6.0,
        dmg: 0.5,
        vol,
        idx,
    };
    // 0: about to cross the right rope; 1 and 2: on the enemy (one volley); 3: on its owner
    spawn_key(&mut fight, 0, shot([a - 0.01, 0.0], 0.0, 0, 60));
    spawn_key(&mut fight, 0, shot([0.5, 1.0], 0.0, 1, 64));
    spawn_key(&mut fight, 0, shot([0.6, 1.0], 0.0, 1, 67));
    spawn_key(&mut fight, 0, shot([-2.0, -2.0], 0.0, 2, 72));
    let hp = fight.balls[1].hp;
    keys_move(&mut fight, dt);
    let kinds: Vec<&str> = fight.events.iter().map(|e| e.kind.as_str()).collect();
    assert_eq!(kinds, ["wall", "hit", "hit"]);
    let wall = &fight.events[0];
    assert_eq!(wall.fields[0], Value::Int(0));
    assert_eq!(wall.fields[1], Value::Int(-1));
    assert_eq!(wall.fields[4], Value::Int(0));
    for (e, idx) in fight.events[1..].iter().zip([64, 67]) {
        assert_eq!(e.fields[0], Value::Int(0));
        assert_eq!(e.fields[1], Value::Int(1));
        assert_eq!(e.fields[2], Value::Float(0.5));
        assert_eq!(e.fields[5], Value::Str("key".into()));
        assert_eq!(e.fields[6], Value::Int(idx));
    }
    assert_eq!(fight.balls[1].hp, hp - 1.0);
    // one landed volley: n_vol 1.0 + 0.25, once
    assert_eq!(fight.balls[0].n_vol, 1.25);
    assert!(fight.roll.vol_landed.contains(&1) && fight.roll.vol_landed.len() == 1);
    // nudged toward +x, the speed kept (to rounding)
    let v = fight.balls[1].v;
    assert!(v[0] > 0.0 && (geom::norm(v) - 3.0).abs() < 1e-8, "{v:?}");
    // the owner's own shot flies on
    assert_eq!(fight.keys.len(), 1);
    assert_eq!((fight.keys[0].id, fight.keys[0].p), (3, [-2.0 + 6.0 * dt, -2.0]));
    // the K row of that shot: x, y, heading, owner, id, index, volley; the other slots NaN
    let mut k = vec![f64::NAN; KMAX * 7];
    k_rows(&fight, &mut k);
    assert_eq!(k[..7], [-2.0 + 6.0 * dt, -2.0, 0.0, 0.0, 3.0, 72.0, 2.0]);
    assert!(k[7..].iter().all(|v| v.is_nan()));
}

/// The roll's record keys are there exactly when a fighter is a roller.
#[test]
fn only_a_rollers_fight_records_roll_and_hold() {
    let roster = testing::roster();
    let s = piano(&roster);
    let fight = testing::fight(&s, true);
    assert_eq!(record_keys(&fight), ["ROLL", "HOLD"]);
    let plain = testing::episode_setup(&roster, 392);
    let fight = testing::fight(&plain, true);
    assert!(record_keys(&fight).is_empty());
}
