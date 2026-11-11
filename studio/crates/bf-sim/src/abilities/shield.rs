//! Ability kind `shield` (TUBA's, HORN's and ORGAN's): `fight.py` 2847 to 2848 (`trigger`).
//!
//! For `dur` seconds (or `dur_beats` beats) every hit on the fighter is blocked: `damage` (322 to 337) emits `block`
//! instead of `hit` and takes nothing off (the striker's bite and the thrust lance test the shield themselves, with
//! their own `block`); the fight loop runs `shield` down by one step per step (6097), and the record shows it (`SH`).

use crate::abilities::ab_dur;
use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// `trigger`, kind `shield`: `B.shield = self.ab_dur(ab)`. A shield that fires while one still holds starts the
/// time again.
pub fn start(fight: &mut Fight, k: usize, ab: &AbilityCfg) {
    let dur = ab_dur(fight, ab);
    fight.balls[k].shield = dur;
}

#[cfg(test)]
mod tests {
    //! Numbers from `tools/studio_export/unit_checks/shield_unit_vectors.py` (the kit's `fight.py` on the same
    //! hand-built state), pasted as printed.

    use bf_core::event::Value::{Float as F, Int as I, Str as S};

    use crate::abilities::{effects_step, trigger};
    use crate::ball::Slot;
    use crate::fight::{Fight, Setup};
    use crate::testing;

    const T: f64 = 12.5;

    fn fight<'a>(roster: &'a [crate::roster::FighterCfg], left: &str, right: &str) -> Fight<'a> {
        let setup = Setup {
            fighters: [testing::fighter(roster, left), testing::fighter(roster, right)],
            rules: testing::episode_rules(),
            seed: 392,
        };
        let mut f = testing::fight(&setup, false);
        f.t = T;
        f
    }

    /// `trigger`, kind `shield` (2847 to 2848): `B.shield = ab_dur(ab)`, 6 beats at the episode's tempo, for each of
    /// the three fighters that have one, on either side; announced like any ability.
    #[test]
    fn each_shield_holds_for_its_beats() {
        let roster = testing::roster();
        for (name, slot, ab) in [
            ("TUBA", Slot::Unlock2, "BRASS WALL"),
            ("HORN", Slot::Unlock1, "STOPPED HORN"),
            ("ORGAN", Slot::Unlock1, "CATHEDRAL"),
        ] {
            for k in 0..2 {
                let mut f = if k == 0 {
                    fight(&roster, name, "FLUTE")
                } else {
                    fight(&roster, "FLUTE", name)
                };
                trigger(&mut f, k, slot);
                assert_eq!(f.balls[k].shield, 2.691176470588235, "{name} {k}");
                assert_eq!(f.balls[1 - k].shield, 0.0);
                assert!(f.balls[k].used[slot.index()]);
                assert_eq!(f.events.len(), 1);
                let e = &f.events[0];
                assert_eq!((e.t, e.kind.as_str()), (12.5, "ability"));
                assert_eq!(e.fields, [I(k as i64), S(ab.into()), S("shield".into()), I(1)]);
            }
        }
        // a shield fired while one still holds starts the time again
        let mut f = fight(&roster, "TUBA", "FLUTE");
        f.balls[0].shield = 5.25;
        trigger(&mut f, 0, Slot::Unlock2);
        assert_eq!(f.balls[0].shield, 2.691176470588235);
    }

    /// `damage` 335 to 337: a hit on a shielded ball is a `block` (the shielded fighter, the attacker, x, y, what hit)
    /// and takes nothing off; once the shield is down the same hit lands, with the episode's handicap.
    #[test]
    fn a_hit_on_a_shielded_ball_is_blocked() {
        let roster = testing::roster();
        let mut f = fight(&roster, "FLUTE", "TUBA");
        f.balls[1].shield = 0.75;
        assert!(!f.damage(0, 1, 3.5, "fang", [0.25, -0.5], 2));
        assert_eq!((f.balls[1].hp, f.balls[0].dealt), (100.0, 0.0));
        assert_eq!(f.events.len(), 1);
        assert_eq!((f.events[0].t, f.events[0].kind.as_str()), (12.5, "block"));
        assert_eq!(f.events[0].fields, [I(1), I(0), F(0.25), F(-0.5), S("fang".into())]);
        f.balls[1].shield = 0.0;
        assert!(f.damage(0, 1, 3.5, "fang", [0.25, -0.5], 2));
        assert_eq!((f.balls[1].hp, f.balls[0].dealt), (93.0, 7.0));
        assert_eq!(f.events[1].kind, "hit");
        assert_eq!(
            f.events[1].fields,
            [I(0), I(1), F(7.0), F(0.25), F(-0.5), S("fang".into()), I(2)]
        );
    }

    /// A ring that passes a shielded enemy is blocked, still knocks it (`knock` is called whether or not the hit
    /// landed) and does not slow it (the slow needs a landed hit).
    #[test]
    fn a_ring_on_a_shielded_ball_is_blocked_and_still_knocks() {
        let roster = testing::roster();
        let mut f = fight(&roster, "TROMBONE", "TUBA");
        let dt = f.rules.dt;
        f.balls[0].p = [0.0, 0.0];
        f.balls[1].p = [1.5, 0.5];
        f.balls[1].v = [0.0, 2.6];
        f.balls[1].shield = 1.0;
        trigger(&mut f, 0, Slot::Unlock1);
        let mut hit_step = None;
        for i in 0..40 {
            f.t = T + (i + 1) as f64 * dt;
            let n0 = f.events.len();
            effects_step(&mut f, dt);
            if f.events.len() > n0 {
                hit_step = Some((i + 1, f.t, f.events[n0..].to_vec()));
                break;
            }
        }
        let (step, t, events) = hit_step.expect("the ring reaches the enemy");
        assert_eq!((step, t), (14, 12.616666666666667));
        assert_eq!(events.len(), 1);
        assert_eq!((events[0].t, events[0].kind.as_str()), (12.61667, "block"));
        assert_eq!(
            events[0].fields,
            [
                I(1),
                I(0),
                F(0.9307900211696918),
                F(0.3102633403898973),
                S("shockwave".into())
            ]
        );
        let b = &f.balls[1];
        assert_eq!(b.v, [2.2325718785852158, 1.3325249742256937]);
        assert_eq!((b.boost, b.slow, b.hp), (1.0, 0.0, 100.0));
    }
}
