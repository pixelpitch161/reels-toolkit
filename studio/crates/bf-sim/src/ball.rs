//! One fighter in the ring: `fight.py`'s `class Ball` (lines 216 to 260).
//!
//! Python hangs every weapon kind's state on the ball as attributes. Here a ball holds what `Ball.__init__` sets for
//! the kinds the engine has ported; a kind's own state lives on [`crate::Fight`] as an `Option` field, indexed by
//! fighter.

use crate::abilities::swan::Swan;
use crate::geom::V2;
use crate::kinds::anchor::Anc;
use crate::kinds::crowd::{Dive, Strut};
use crate::kinds::drill::March;
use crate::roster::{AbilityCfg, BladeMode, FighterCfg, WeaponCfg};
use crate::rules::Rules;

/// An ability slot: the keys of a fighter's `abilities` in `fighters.py`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// `unlock1`: unlocked at level 2.
    Unlock1,
    /// `unlock2`: unlocked at level 3.
    Unlock2,
    /// `low`: fires once when HP drops below `LOW_HP`.
    Low,
}

impl Slot {
    /// The slot's index into [`Ball::ready`] (the two unlocks) and [`Ball::used`] (all three).
    pub fn index(self) -> usize {
        match self {
            Slot::Unlock1 => 0,
            Slot::Unlock2 => 1,
            Slot::Low => 2,
        }
    }

    /// `cfg['abilities'][slot]`.
    pub fn of(self, cfg: &FighterCfg) -> &AbilityCfg {
        match self {
            Slot::Unlock1 => &cfg.unlock1,
            Slot::Unlock2 => &cfg.unlock2,
            Slot::Low => &cfg.low,
        }
    }
}

/// One fighter's ball. Field names are `fight.py`'s.
///
/// `Ball.__init__` also sets `cool`, `pull`, `beam_w`, `charge`, `dash_t`, `dash_hit`, `dash_main` and `rake_dir`:
/// state of the spinning blade, pulse, beam and dash kinds, none of them ported. They come with their kinds.
///
/// The per-ball states that the shared engine reads for the kit's other kinds (`move`, `damage`, `knock`, `_stunned`,
/// the timers, `abilities_step`) are here too, from `stun` on, at what Python's `getattr(B, name, default)` gives a
/// ball that never had the attribute: a fight of FLUTE and TROMBONE never changes them. Each kind's own state is on
/// [`crate::Fight`] (`Fight::<kind>`).
#[derive(Clone, Debug)]
pub struct Ball<'a> {
    /// The fighter's name.
    pub name: &'a str,
    /// Its roster entry (`cfg`; the weapon `w` is `cfg.weapon`).
    pub cfg: &'a FighterCfg,
    /// 0 = red corner (left), 1 = blue corner (right).
    pub side: usize,
    /// Hit points left (may go below 0 on the last hit).
    pub hp: f64,
    /// Damage dealt so far: it sets the level.
    pub dealt: f64,
    /// Level, 1 to 3.
    pub level: i64,
    /// The knockback speed boost: the ball moves at `V * (1 + boost)`, and the boost decays.
    pub boost: f64,
    /// Position, in ring units.
    pub p: V2,
    /// Velocity, ring units per second.
    pub v: V2,
    /// The weapon's angle, 0 to 2 pi.
    pub th: f64,
    /// Spin direction: 1.0 on the left, -1.0 on the right.
    pub spin_dir: f64,
    /// A spinning blade's rate in rad/s (`w.get('spin', 0.0)`; no ported kind has one).
    pub spin: f64,
    /// The weapon's reach beyond the ball's surface (`w.get('base', 0.0)`); it grows with landed hits.
    pub blade_len: f64,
    /// Growth steps so far (`w.get('frets0', 0)`, an int: it goes into `hit` events).
    pub frets: i64,
    /// Projectiles per volley (`float(w.get('n0', 0))`; no ported kind fires any).
    pub n_vol: f64,
    /// The grid multiplier while `haste` runs.
    pub haste_mult: f64,
    /// Seconds of shield left.
    pub shield: f64,
    /// Seconds of haste left.
    pub haste: f64,
    /// When each unlock may fire next, by [`Slot::index`] (`None` = still locked): Python's `ready` dict.
    pub ready: [Option<f64>; 2],
    /// Which abilities have fired at least once, by [`Slot::index`]: Python's `used` set.
    pub used: [bool; 3],
    /// Whether the `low` ability has had its turn.
    pub low_done: bool,
    /// Thrust: the lance's current length beyond the surface (`w.get('rest', 0.0)` at the start).
    pub ext: f64,
    /// Thrust: the time the current thrust started (`None` = idle).
    pub thr: Option<f64>,
    /// Thrust: whether the current thrust has hit.
    pub thr_hit: bool,
    /// Thrust: the time scale of the current thrust (haste).
    pub thr_k: f64,
    /// A curving volley's current turn rate in deg/s (`w.get('curve', 0.0)`; no ported kind curves).
    pub curve_now: f64,
    /// Seconds left of a slow on this ball.
    pub slow: f64,
    /// The speed multiplier while slowed.
    pub slow_mult: f64,
    /// Seconds left of a stun (#37, SQUEAK; `getattr(B, 'stun', 0.0)`): a stunned ball holds still and starts no
    /// attack (`_stunned`, `fight.py` 2355 to 2357; `move` 3135).
    pub stun: f64,
    /// Seconds left out of tune (#37, ILL WIND; `getattr(B, 'detune_t', 0.0)`): its hits land weaker (`damage` 345
    /// to 346).
    pub detune_t: f64,
    /// How much weaker an out-of-tune ball's hits land (`_detune` 2447 to 2451): `1.0 - detune_w * detune_d`.
    pub detune_w: f64,
    /// The depth of the detune (`_detune`).
    pub detune_d: f64,
    /// Seconds left sunk in shadow (#38, LIGHTS OUT; `getattr(B, 'shadow', 0.0)`): intangible, no clash, steered
    /// before `move` (`damage` 327, `move` 3182, `_stunned`).
    pub shadow: f64,
    /// Swallowed (#32 v2, the TUBA's bell; `getattr(B, 'held', False)`): it rides inside, no clash (`move` 3133,
    /// 3181).
    pub held: bool,
    /// The peg it whirls round (#38, ENDPIN ANCHOR; `B.anc`, `None` when it does not): `anchor_step` drives it
    /// (`move` 3137).
    pub anc: Option<Anc>,
    /// Mid-hop (#39, TIPTOE; `getattr(B, 'air', False)`): `tiptoe_step` drives it, no clash (`move` 3139, 3183).
    pub air: bool,
    /// A hopper (#39): between hops it stands still and only slides when knocked (`move` 3153 to 3154).
    pub hopper: bool,
    /// Seconds left in pieces (#44, the smash; `getattr(B, 'smashed', 0.0)`): still, intangible, no clash, starts
    /// nothing; when it runs out the ball is rebuilt (`rebuild` event, `run()` 6104 to 6107).
    pub smashed: f64,
    /// Mid-step (#47, WALKING BASS; `getattr(B, 'walking', False)`): `walk_step` drives it, it cannot be knocked
    /// (`move` 3143, `knock` 355).
    pub walking: bool,
    /// A walker (#47): planted between steps (only slides when knocked, `move` 3155 to 3156), and it does not give
    /// way in a clash (3190 to 3208).
    pub walker: bool,
    /// Held in THE EMBRACE (#47; `getattr(B, 'grabbed', False)`): `grapple_step` drives it, no clash (`move` 3145,
    /// 3185).
    pub grabbed: bool,
    /// Holding an enemy in THE EMBRACE (#47; `getattr(B, 'pinned', False)`) (`move` 3145).
    pub pinned: bool,
    /// Airborne in SWAN SONG (#47; `B.swan`, `None` on the ground): out of reach, not knocked, no clash (`damage`
    /// 331, `knock` 357, `move` 3147, 3186).
    pub swan: Option<Swan>,
    /// Strutting down WALK THE BAR (#51; `B.strut`): not knocked, does not give way in a clash (`knock` 359, `move`
    /// 3149, 3191 to 3193).
    pub strut: Option<Strut>,
    /// In a STAGE DIVE (#51; `B.dive`): out of reach, not knocked, no clash (`damage` 333, `knock` 359, `move` 3149,
    /// 3187).
    pub dive: Option<Dive>,
    /// A drill set's march (#53; `B.march`, `None` between sets). Whether the ball is a driller at all
    /// (`hasattr(B, 'march')`) is `kinds::drill::is`.
    pub march: Option<March>,
    /// Rolling (#56, STEAMROLLER; `getattr(O, 'rolling', False)`): a swallowing bell cannot gulp it unless the rule
    /// `swallow_rollers` is on (`fight.py` 1100).
    pub rolling: bool,
    /// The last ability it fired (`B.last_ab`, `trigger` 2840): what IDENTITY CRISIS copies (`copy_source`).
    pub last_ab: Option<&'a AbilityCfg>,
}

impl<'a> Ball<'a> {
    /// `Ball(name, side, rules)`: a fighter at full health, standing still at the origin (`Fight::new` places and
    /// launches it). The weapon options `Ball.__init__` reads with `w.get(name, default)` come from the typed
    /// settings of the ported kinds; a kind not ported yet leaves every one at its default.
    pub fn new(cfg: &'a FighterCfg, side: usize, rules: &Rules) -> Ball<'a> {
        // w.get('base', 0.0), w.get('frets0', 0), w.get('rest', 0.0), float(w.get('n0', 0)): each kind's own module
        // gives them (`roster::kinds::<kind>::ball_start`)
        let start = cfg.weapon.ball_start();
        Ball {
            name: &cfg.name,
            cfg,
            side,
            hp: rules.hp,
            dealt: 0.0,
            level: 1,
            boost: 0.0,
            p: [0.0, 0.0],
            v: [0.0, 0.0],
            th: 0.0,
            spin_dir: if side == 0 { 1.0 } else { -1.0 },
            spin: 0.0,
            blade_len: start.base,
            frets: start.frets0,
            n_vol: start.n0,
            haste_mult: 1.0,
            shield: 0.0,
            haste: 0.0,
            ready: [None, None],
            used: [false; 3],
            low_done: false,
            ext: start.rest,
            thr: None,
            thr_hit: false,
            thr_k: 1.0,
            curve_now: 0.0,
            slow: 0.0,
            slow_mult: 1.0,
            stun: 0.0,
            detune_t: 0.0,
            detune_w: 0.0,
            detune_d: 0.0,
            shadow: 0.0,
            held: false,
            anc: None,
            air: false,
            hopper: false,
            smashed: 0.0,
            walking: false,
            walker: false,
            grabbed: false,
            pinned: false,
            swan: None,
            strut: None,
            dive: None,
            march: None,
            rolling: false,
            last_ab: None,
        }
    }

    /// `w.get('mode') == 'thrust'`: the weapon is a lance that thrusts instead of a blade that spins.
    pub fn thrusts(&self) -> bool {
        matches!(&self.cfg.weapon, WeaponCfg::Blade(w) if w.mode == BladeMode::Thrust)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    #[test]
    fn a_new_ball_has_its_weapons_start_values() {
        let roster = testing::roster();
        let rules = testing::episode_rules();
        // what Ball.__init__ gave on the reference PC: blade_len, frets, ext, spin, n_vol
        let flute = Ball::new(testing::fighter(&roster, "FLUTE"), 0, &rules);
        assert_eq!(
            (flute.blade_len, flute.frets, flute.ext, flute.spin, flute.n_vol),
            (1.4, 0, 0.0, 0.0, 0.0)
        );
        assert_eq!((flute.name, flute.side, flute.spin_dir), ("FLUTE", 0, 1.0));
        assert!(!flute.thrusts());
        let trombone = Ball::new(testing::fighter(&roster, "TROMBONE"), 1, &rules);
        assert_eq!(
            (trombone.blade_len, trombone.frets, trombone.ext, trombone.spin),
            (0.95, 1, 0.3, 0.0)
        );
        assert_eq!(trombone.spin_dir, -1.0);
        assert!(trombone.thrusts());
        for b in [&flute, &trombone] {
            assert_eq!((b.hp, b.dealt, b.level, b.boost), (100.0, 0.0, 1, 0.0));
            assert_eq!((b.p, b.v, b.th), ([0.0, 0.0], [0.0, 0.0], 0.0));
            assert_eq!(
                (b.shield, b.haste, b.haste_mult, b.slow, b.slow_mult),
                (0.0, 0.0, 1.0, 0.0, 1.0)
            );
            assert_eq!((b.ready, b.used, b.low_done), ([None, None], [false; 3], false));
            assert_eq!((b.thr, b.thr_hit, b.thr_k, b.curve_now), (None, false, 1.0, 0.0));
        }
    }

    /// The states the shared engine reads for the kit's other kinds start where Python's `getattr` defaults are,
    /// for every fighter of the roster: no stun, detune, shadow, smash or flight, not swallowed, anchored, hopping,
    /// walking, held or rolling, no last ability.
    #[test]
    fn the_kit_states_start_at_pythons_defaults() {
        let roster = testing::roster();
        let rules = testing::episode_rules();
        for (k, cfg) in roster.iter().enumerate() {
            let b = Ball::new(cfg, k % 2, &rules);
            assert_eq!(
                (b.stun, b.detune_t, b.detune_w, b.detune_d, b.shadow, b.smashed),
                (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
                "{}",
                cfg.name
            );
            assert!(!(b.held || b.air || b.hopper || b.walking || b.walker || b.grabbed || b.pinned || b.rolling));
            assert!(b.anc.is_none() && b.swan.is_none() && b.strut.is_none() && b.dive.is_none());
            assert!(b.march.is_none() && b.last_ab.is_none());
        }
    }

    #[test]
    fn slots_index_the_roster_entry() {
        let roster = testing::roster();
        let flute = testing::fighter(&roster, "FLUTE");
        assert_eq!(Slot::Unlock1.of(flute).name, "FLUTTER TONGUE");
        assert_eq!(Slot::Unlock2.of(flute).name, "SNAKE DANCE");
        assert_eq!(Slot::Low.of(flute).name, "PICCOLO");
        assert_eq!([Slot::Unlock1, Slot::Unlock2, Slot::Low].map(Slot::index), [0, 1, 2]);
    }

    #[test]
    fn a_kind_not_ported_starts_with_the_defaults() {
        let roster = testing::roster();
        let piano = Ball::new(testing::fighter(&roster, "PIANO"), 0, &Rules::new(120.0, 4.0));
        assert_eq!((piano.blade_len, piano.frets, piano.ext), (0.0, 0, 0.0));
        assert!(!piano.thrusts());
    }
}
