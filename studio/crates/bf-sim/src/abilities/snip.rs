//! Ability kind `snip` (OBOE's DOUBLE REED, BASS CLARINET's SLAP TONGUE): `fight.py` `trigger` 2882 to 2883; 2526 to
//! 2567 (`snip_ready`, `start_snip`, `snip_step`); the gate in `abilities_step` 3037 to 3039.
//!
//! The reed's two blades scissor out of the ball at the enemy (turning to it at `track` radians a second while they
//! open over `open16` 16ths) and snap shut: a hit if the enemy is within `reach` beyond the surface and `half`
//! degrees of the aim (`dmg`, `knock`). It waits for its moment: it only starts when the enemy is within `ready`
//! beyond the surface ([`ready`]). Events `snip_open` (the owner, the time of the snap, the aim) and `snip` (the
//! owner, hit 0 / 1, the aim, the reach); recorded as `SNIP` (state 1 open / 2 shut, aim, time of the snap) when a
//! fighter has the kind.
//!
//! Everything here is about "ball `k`" and its enemy `1 - k`, with the ability passed in: [`start`] takes any snip
//! ability (the fighter's own, or one IDENTITY CRISIS copied from the enemy), keeps a copy of it in ball `k`'s
//! [`Snip`], and [`snip_step`] reads only that copy. So the kind runs the same for either ball and for a copier
//! whose own roster entry has no snip. None of this code draws from the fight's generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::{has, param};
use crate::fight::Fight;
use crate::geom;
use crate::record::Frames;
use crate::roster::{AbilityCfg, AbilityKind};

/// `B.snip`: `dict(ab=ab, aim=aim, t_snap=..., state=1, t_end=None)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Snip {
    /// The ability that started it (its `track`, `reach`, `half`, `dmg`, `knock`).
    pub ab: AbilityCfg,
    /// Where the blades point (radians; it follows the enemy while they open).
    pub aim: f64,
    /// When they snap shut.
    pub t_snap: f64,
    /// 1 while they open, 2 once shut.
    pub state: i64,
    /// When the shut blades are gone (`None` while they open).
    pub t_end: Option<f64>,
}

/// This ability's per-fight state (`Fight::snip`): each ball's `B.snip` (`None`: no blades out).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// By ball.
    pub balls: [Option<Snip>; 2],
}

/// `trigger`, kind `snip`: `trigger` 2882 to 2883, `start_snip(k, ab)` (2537 to 2542): ball `k`'s blades open at the
/// enemy.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let aim = dmath::atan2(d[1], d[0]);
    // t_snap=self.t + ab['open16'] * self.R.beat / 4.0
    let t_snap = f.t + param(ab, "open16") * f.rules.beat / 4.0;
    f.snip.balls[k] = Some(Snip {
        ab: ab.clone(),
        aim,
        t_snap,
        state: 1,
        t_end: None,
    });
    f.ev("snip_open", k as i64, vec![Value::Float(t_snap), Value::Float(aim)]);
}

/// `snip_ready(k, ab)` (2532 to 2535): an unlock of this kind waits in `abilities_step` (3037 to 3039, and 3046 to 3049
/// for a copied snip) until this is true: the enemy of ball `k` is within `ready` beyond the surfaces.
pub fn ready(f: &Fight, k: usize, ab: &AbilityCfg) -> bool {
    let rel = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    geom::hypot(rel) - 2.0 * f.rules.r <= param(ab, "ready")
}

/// `snip_step(k, dt)` (2543 to 2567), from `run()` 6152 to 6156 (the snippers, then each fighter in `extra_snip` that
/// is not one)
pub fn snip_step(f: &mut Fight, k: usize, dt: f64) {
    let Some(sn) = &mut f.snip.balls[k] else {
        return;
    };
    let rel = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    if sn.state == 1 {
        let want = dmath::atan2(rel[1], rel[0]);
        let step = param(&sn.ab, "track") * dt;
        // Sn['aim'] = _wrap(Sn['aim'] + max(-step, min(step, _wrap(want - Sn['aim']))))
        let turn = py::py_max(-step, py::py_min(step, geom::wrap(want - sn.aim)));
        sn.aim = geom::wrap(sn.aim + turn);
        if f.t + 1e-9 >= sn.t_snap {
            let aim = sn.aim;
            let (reach, half) = (param(&sn.ab, "reach"), param(&sn.ab, "half"));
            let (dmg, knock) = (param(&sn.ab, "dmg"), param(&sn.ab, "knock"));
            let r = f.rules.r;
            let dist = geom::hypot(rel);
            let off = geom::wrap(dmath::atan2(rel[1], rel[0]) - aim).abs();
            // math.degrees(off): `off * (180.0 / pi)`, the constant formed first, as CPython does
            let hit = dist - 2.0 * r <= reach && off * (180.0 / dmath::PI) <= half;
            if hit {
                let n = if dist > 1e-6 {
                    geom::div(rel, dist)
                } else {
                    [dmath::cos(aim), dmath::sin(aim)]
                };
                // hx, hy = (O.p - n * self.R.R).tolist()
                let h = geom::sub(f.balls[1 - k].p, geom::scale(n, r));
                f.damage(k, 1 - k, dmg, "snip", h, 0);
                f.knock(1 - k, n, knock);
            }
            let data = vec![Value::Int(i64::from(hit)), Value::Float(aim), Value::Float(reach)];
            f.ev("snip", k as i64, data);
            let t_end = f.t + 0.25;
            if let Some(sn) = &mut f.snip.balls[k] {
                sn.state = 2;
                sn.t_end = Some(t_end);
            }
        }
    } else if sn.t_end.is_some_and(|t_end| f.t >= t_end) {
        f.snip.balls[k] = None;
    }
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5803 to 5805: `SNIP` when a fighter has
/// a snip in its own roster entry (a copied snip alone adds no key).
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| has(f, k, &AbilityKind::Snip)) {
        &["SNIP"]
    } else {
        &[]
    }
}

/// This kind's rows of one frame: `snap`, `SNIP` (5555 to 5561), into `out` (only the keys [`record_keys`] gave): each
/// ball's (state, aim, time of the snap), NaN for a ball with no blades out.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut row = [f64::NAN; 6];
    for (j, sn) in f.snip.balls.iter().enumerate() {
        if let Some(sn) = sn {
            row[3 * j] = sn.state as f64;
            row[3 * j + 1] = sn.aim;
            row[3 * j + 2] = sn.t_snap;
        }
    }
    out.push_f64("SNIP", &[2, 3], &row);
}

#[cfg(test)]
mod tests {
    //! Numbers from `tools/studio_export/unit_checks/snip_unit_vectors.py` (the kit's `fight.py` on the same
    //! hand-built state), pasted as printed: the snipper is ball `k` at (-1, 0.5), the clock at 12.5 s, the engine's
    //! default rules at the example episode's tempo.
    use bf_core::event::Value::{Float as F, Int as I, Str as S};
    use bf_core::event::{Event, Value};

    use super::*;
    use crate::abilities::trigger;
    use crate::ball::Slot;
    use crate::fight::Setup;
    use crate::geom::V2;
    use crate::roster::FighterCfg;
    use crate::rules::Rules;
    use crate::testing;

    const T: f64 = 12.5;
    const DT: f64 = 0.008333333333333333;

    /// The enemy put at a place before a step, if at all: `(step, place)`.
    type Jump = Option<(usize, V2)>;

    fn ev(t: f64, kind: &str, fields: Vec<Value>) -> Event {
        Event {
            t,
            kind: kind.into(),
            fields,
        }
    }

    /// `left` against `right`, the snipper (ball `k`) at (-1, 0.5), its enemy at `enemy`.
    fn duel<'a>(roster: &'a [FighterCfg], left: &str, right: &str, k: usize, enemy: V2) -> Fight<'a> {
        let ep = testing::episode_rules();
        let setup = Setup {
            fighters: [testing::fighter(roster, left), testing::fighter(roster, right)],
            rules: Rules::new(ep.bpm, ep.t_fight),
            seed: 392,
        };
        let mut f = testing::fight(&setup, false);
        f.balls[k].p = [-1.0, 0.5];
        f.balls[1 - k].p = enemy;
        f.balls[k].v = [1.0, -2.0];
        f.balls[1 - k].v = [-0.5, 2.0];
        f.t = T;
        f
    }

    /// `snip_step` once per step from `T` until the blades are gone: the enemy drifts by `drift` a step and, with
    /// `jump`, is put at a place before that step. Returns each step's `(state, aim)` (the last entry, `None`, is
    /// the step the blades went).
    fn run(f: &mut Fight, k: usize, drift: V2, jump: Jump) -> Vec<Option<(i64, f64)>> {
        let mut seen = Vec::new();
        for i in 1..=80 {
            f.t = T + i as f64 * DT;
            let o = &mut f.balls[1 - k];
            o.p = [o.p[0] + drift[0], o.p[1] + drift[1]];
            if let Some((_, at)) = jump.filter(|j| j.0 == i) {
                f.balls[1 - k].p = at;
            }
            snip_step(f, k, DT);
            seen.push(f.snip.balls[k].as_ref().map(|sn| (sn.state, sn.aim)));
            if f.snip.balls[k].is_none() {
                break;
            }
        }
        seen
    }

    /// `snip_ready` (2532 to 2535): ready while the enemy is within `ready` (1.1) beyond the two surfaces, for the
    /// ball asked about, whichever fighter it is.
    #[test]
    fn it_waits_until_the_enemy_is_close() {
        let roster = testing::roster();
        let ab = &testing::fighter(&roster, "OBOE").unlock1;
        for (x, want) in [(2.29, true), (2.3, true), (2.31, false)] {
            let f = duel(&roster, "OBOE", "FLUTE", 0, [-1.0 + x, 0.5]);
            assert_eq!(ready(&f, 0, ab), want, "{x}");
        }
        let f = duel(&roster, "FLUTE", "OBOE", 1, [1.29, 0.5]);
        assert!(ready(&f, 1, ab) && ready(&f, 0, ab));
    }

    /// `start_snip` and `snip_step` (2537 to 2567), from either corner: the blades open at the enemy, follow it at
    /// `track` while they open over 2 16ths, snap on the first step past the time, hit (`dmg`, the knock straight
    /// away from the snipper), stay shut for 0.25 s and go.
    #[test]
    fn the_blades_open_follow_snap_and_hit_from_either_corner() {
        let roster = testing::roster();
        for k in 0..2 {
            let (left, right) = if k == 0 { ("OBOE", "FLUTE") } else { ("FLUTE", "OBOE") };
            let mut f = duel(&roster, left, right, k, [0.5, -0.1]);
            let who = k as i64;
            trigger(&mut f, k, Slot::Unlock1);
            let (t_snap, aim0) = (12.724264705882353, -0.38050637711236485);
            assert_eq!(
                f.events,
                [
                    ev(
                        12.5,
                        "ability",
                        vec![I(who), S("DOUBLE REED".into()), S("snip".into()), I(1)]
                    ),
                    ev(12.5, "snip_open", vec![I(who), F(t_snap), F(aim0)]),
                ]
            );
            let sn = f.snip.balls[k].as_ref().expect("the blades are out");
            assert_eq!((sn.state, sn.aim, sn.t_snap, sn.t_end), (1, aim0, t_snap, None));
            assert!(f.snip.balls[1 - k].is_none());
            let seen = run(&mut f, k, [0.0, 0.02], None);
            assert_eq!(seen[0], Some((1, -0.36895954620585636)));
            assert_eq!(seen[1], Some((1, -0.35730868300810004)));
            let aim = -0.03997868712328989;
            assert_eq!((seen[25].map(|s| s.0), seen[26]), (Some(1), Some((2, aim))));
            assert_eq!((seen.len(), seen[56], f.t), (57, None, 12.975));
            let hit = vec![
                I(who),
                I(1 - who),
                F(6.0),
                F(-0.09952057523307367),
                F(0.46398082300932303),
                S("snip".into()),
                I(0),
            ];
            assert_eq!(
                f.events[2..],
                [
                    ev(12.725, "hit", hit),
                    ev(12.725, "snip", vec![I(who), I(1), F(aim), F(1.2)]),
                ]
            );
            let o = &f.balls[1 - k];
            assert_eq!(
                (o.hp, o.v, o.boost, f.balls[k].dealt),
                (94.0, [2.5414812731935124, 0.5485188583714191], 1.2, 6.0)
            );
        }
    }

    /// A snap misses when the enemy is beyond `reach` (it still follows it), and when it is within reach but more
    /// than `half` degrees off the aim (the enemy jumped over the snipper one step before the snap).
    #[test]
    fn a_snap_misses_by_reach_and_by_angle() {
        let roster = testing::roster();
        let cases: [(V2, Jump, f64); 2] = [
            ([1.2, -0.7], None, -0.49934672168012995),
            ([0.5, -0.1], Some((27, [-1.0, 1.9])), -0.3305063771123651),
        ];
        for (enemy, jump, aim) in cases {
            let mut f = duel(&roster, "OBOE", "FLUTE", 0, enemy);
            trigger(&mut f, 0, Slot::Unlock1);
            let seen = run(&mut f, 0, [0.0, 0.0], jump);
            assert_eq!((seen[26], seen.len()), (Some((2, aim)), 57));
            assert_eq!(f.events.len(), 3);
            assert_eq!(f.events[2], ev(12.725, "snip", vec![I(0), I(0), F(aim), F(1.2)]));
            let o = &f.balls[1];
            assert_eq!((o.hp, o.v, o.boost, f.balls[0].dealt), (100.0, [-0.5, 2.0], 0.0, 0.0));
            if jump.is_some() {
                // before the jump the aim sat on the still enemy
                assert_eq!(seen[25], Some((1, -0.3805063771123649)));
            }
        }
    }

    /// A copied snip: BASS CLARINET's SLAP TONGUE started on FLUTE's ball in a fight of FLUTE against TROMBONE
    /// (what IDENTITY CRISIS does: `start_snip(k, src)` for a ball whose own fighter has no snip). The kind runs on
    /// the ability it was given: 1 16th to open, reach 0.9, 6.5 damage, knock 1.8; no `ability` event, nothing
    /// marked used, and no `SNIP` key in the record.
    #[test]
    fn a_copied_snip_runs_on_a_ball_whose_fighter_has_none() {
        let roster = testing::roster();
        let slap = &testing::fighter(&roster, "BASS CLARINET").unlock1;
        assert_eq!((slap.name.as_str(), &slap.kind), ("SLAP TONGUE", &AbilityKind::Snip));
        let mut f = duel(&roster, "FLUTE", "TROMBONE", 0, [0.2, 0.3]);
        assert!(ready(&f, 0, slap));
        start(&mut f, 0, slap);
        let (t_snap, aim0) = (12.612132352941176, -0.16514867741462685);
        assert_eq!(f.events, [ev(12.5, "snip_open", vec![I(0), F(t_snap), F(aim0)])]);
        assert_eq!(f.balls[0].used, [false; 3]);
        assert!(record_keys(&f).is_empty());
        let seen = run(&mut f, 0, [0.0, 0.0], None);
        let aim = -0.1651486774146269;
        assert_eq!(
            (seen[0], seen[12], seen[13]),
            (Some((1, aim)), Some((1, aim)), Some((2, aim)))
        );
        assert_eq!((seen.len(), f.t), (44, 12.866666666666667));
        let hit = vec![
            I(0),
            I(1),
            F(6.5),
            F(-0.39183635429928626),
            F(0.3986393923832144),
            S("snip".into()),
            I(0),
        ];
        assert_eq!(
            f.events[1..],
            [
                ev(12.61667, "hit", hit),
                ev(12.61667, "snip", vec![I(0), I(1), F(aim), F(0.9)]),
            ]
        );
        let o = &f.balls[1];
        assert_eq!(
            (o.hp, o.v, o.boost, f.balls[0].dealt),
            (93.5, [2.59078822932492, 0.21866950124161066], 1.8, 6.5)
        );
    }

    /// The record has `SNIP` exactly when a fighter has a snip of its own.
    #[test]
    fn the_record_key_follows_the_roster() {
        let roster = testing::roster();
        let f = duel(&roster, "FLUTE", "OBOE", 1, [0.5, -0.1]);
        assert_eq!(record_keys(&f), ["SNIP"]);
    }
}
