//! Ability kind `twin` (BASS CLARINET's MULTIPHONICS): `fight.py` `trigger` 2897 to 2899.
//!
//! Two notes at once: for the ability's duration a ghost twin doubles the fighter's signature. An anchor whirl gets
//! a second arm on the far side of the peg (`kinds::anchor::step`, which reads [`State::until`]; `ANC` records it), a
//! mortar salvo doubles. Event `twin`: the owner, the time it ends.
//!
//! The twin is ball `k`'s, whichever fighter it is and whoever's ability started it (IDENTITY CRISIS copies it), so
//! nothing here reads ball `k`'s own roster entry. No draw from the fight's generator.

use bf_core::event::Value;

use crate::abilities::ab_dur;
use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::twin`): each ball's `B.twin_until`.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// By ball: the time its twin ends (`getattr(B, 'twin_until', -1.0)`: -1.0 for a ball that never had one).
    pub until: [f64; 2],
}

impl Default for State {
    fn default() -> State {
        State { until: [-1.0; 2] }
    }
}

/// `trigger`, kind `twin`: `trigger` 2897 to 2899: ball `k` has a twin until the ability's duration has passed.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + ab_dur(f, ab);
    f.twin.until[k] = until;
    f.ev("twin", k as i64, vec![Value::Float(until)]);
}

#[cfg(test)]
mod tests {
    //! Numbers from `tools/studio_export/unit_checks/anchor_unit_vectors.py` (the kit's `fight.py` on the same
    //! hand-built state), pasted as printed.
    use bf_core::event::Event;

    use super::*;
    use crate::abilities::trigger;
    use crate::ball::Slot;
    use crate::fight::Setup;
    use crate::rules::Rules;
    use crate::testing;

    /// A ball has no twin until one starts; MULTIPHONICS lasts its 8 beats on either side's ball, and a copied one
    /// (started with another fighter's ability on a ball whose fighter has none) is the same twin.
    #[test]
    fn a_twin_lasts_its_beats_on_whichever_ball() {
        let roster = testing::roster();
        let ep = testing::episode_rules();
        for k in 0..2 {
            let names = if k == 0 {
                ["BASS CLARINET", "FLUTE"]
            } else {
                ["FLUTE", "BASS CLARINET"]
            };
            let setup = Setup {
                fighters: names.map(|n| testing::fighter(&roster, n)),
                rules: Rules::new(ep.bpm, ep.t_fight),
                seed: 392,
            };
            let mut f = testing::fight(&setup, false);
            assert_eq!(f.twin.until, [-1.0, -1.0]);
            f.t = 12.5;
            trigger(&mut f, k, Slot::Unlock2);
            let until = 16.088235294117645;
            assert_eq!(f.twin.until[k], until);
            assert_eq!(f.twin.until[1 - k], -1.0);
            assert_eq!(
                f.events[1],
                Event {
                    t: 12.5,
                    kind: "twin".into(),
                    fields: vec![Value::Int(k as i64), Value::Float(until)],
                }
            );
            // the copy: FLUTE's ball takes the twin, with no `ability` event
            let ab = &testing::fighter(&roster, "BASS CLARINET").unlock2;
            let n = f.events.len();
            f.t = 13.0;
            start(&mut f, 1 - k, ab);
            assert_eq!(f.twin.until[1 - k], 13.0 + 8.0 * f.rules.beat);
            assert_eq!((f.events.len(), f.events[n].kind.as_str()), (n + 1, "twin"));
        }
    }
}
