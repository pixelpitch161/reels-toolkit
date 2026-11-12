//! Weapon kind `rake`, ACOUSTIC's SIX-STRING RAKE: its roster settings, and the two closed-form motions the fight
//! and the look share. `fight.py` 497 to 700: `fire_rake` (528 to 543), `aim_blend` (545 to 554), `rake_lateral`
//! (556 to 559), `sweep_front` (561 to 564), the rake branch of `surf_rake_step` (610 to 629); `dash_cone_step`
//! (457 to 495) has no rake branch. The `dash`, `cone`, `surf` and `spout` branches belong to older kinds.
//!
//! Every `every` beats on the dash / cone 16th grid (haste divides it) six parallel strings (`gap` apart, `blade_len`
//! long, pointing at the enemy blended with the ball's heading by `aim_k`) sweep `sweep` to either side across the
//! ring in front of the ball in `stroke` seconds, alternating down and up; one hit per stroke (`dmg`, or `dmg` x
//! `sub_dmg` off the weapon's own grid) flicks the enemy along it (`knock`, x 0.6 off the grid); a landed stroke on
//! the grid adds `grow` to the length, up to `max_len`.
//!
//! [`RakeCfg`] holds every field `fight.py` reads: `w[name]`, and `w.get(name, default)` with its default.
//! [`rake_lateral`] and [`sweep_front`] are `Fight.rake_lateral` and `Fight.sweep_front`, which the kit's look calls
//! too (`acoustic.py` 270, 279, 319): one home for the engine (`bf-sim`) and the look (`bf-content`).

use serde_json::Value as Json;

use crate::dmath;
use crate::py::{py_max, py_min};
use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `rake`.
pub const PORTED: bool = true;

/// Weapon kind `rake`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct RakeCfg {
    /// The picture's style (default: the kind name; the fight never reads it).
    pub style: String,
    /// Beats between two strokes on the dash / cone 16th grid (`int(round(4 * every))` 16ths; haste divides it).
    pub every: f64,
    /// The strings' starting length (`blade_len`; `Ball` reads `w.get('base', 0.0)`).
    pub base: f64,
    /// Length added per landed stroke on the weapon's own grid.
    pub grow: f64,
    /// The length never grows past this.
    pub max_len: f64,
    /// How far to either side of the aim the bundle sweeps (`S`).
    pub sweep: f64,
    /// The space between two strings; the stroke's hit band is 2.5 gaps to either side of the bundle's middle.
    pub gap: f64,
    /// Seconds a stroke takes.
    pub stroke: f64,
    /// How much of the aim is the enemy's direction, the rest the ball's heading (`w.get('aim_k', 1.0)`).
    pub aim_k: f64,
    /// Damage of a stroke on the weapon's own grid.
    pub dmg: f64,
    /// The damage factor of a stroke off it (a haste's extra strokes; `w.get('sub_dmg', 0.5)`).
    pub sub_dmg: f64,
    /// Knockback of a landed stroke (x 0.6 off the grid).
    pub knock: f64,
}

/// The keys a rake entry may hold: `kind`, `style` and [`RakeCfg`]'s numbers.
const RAKE_KEYS: [&str; 13] = [
    "kind", "style", "every", "base", "grow", "max_len", "sweep", "gap", "stroke", "aim_k", "dmg", "sub_dmg", "knock",
];

/// Reads a `rake` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<RakeCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a rake weapon", &RAKE_KEYS)?;
    let cfg = RakeCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "rake".to_string(),
        },
        every: e.grid(w, p, "every")?,
        base: e.f64_or(w, p, "base", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        max_len: e.f64(w, p, "max_len")?,
        sweep: e.f64(w, p, "sweep")?,
        gap: e.f64(w, p, "gap")?,
        stroke: e.f64(w, p, "stroke")?,
        aim_k: e.f64_or(w, p, "aim_k", 1.0)?,
        dmg: e.f64(w, p, "dmg")?,
        sub_dmg: e.f64_or(w, p, "sub_dmg", 0.5)?,
        knock: e.f64(w, p, "knock")?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): what the Python fight
/// cannot run. `int(round(4 * every))` raises when 4 x `every` is not finite, and a grid that rounds to 0 raises at
/// the first 16th (`dc_k % dc_main`: `ZeroDivisionError`; the other kinds' grids take `max(1, ...)`, this one does
/// not); `rake_lateral` divides by `stroke` in the step that fires the stroke (`tau / dur` with `tau` 0:
/// `ZeroDivisionError` for a stroke of 0 s). Every other number runs as it is.
pub(crate) fn check(e: &Entry, w: &RakeCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    if crate::py::round_half_even(4.0 * w.every) == 0.0 {
        return Err(e.err(
            "weapon.every",
            format!(
                "{} beats rounds to no 16th note (the stroke's grid divides by it)",
                w.every
            ),
        ));
    }
    if w.stroke == 0.0 {
        return Err(e.err(
            "weapon.stroke",
            "a stroke of no time cannot sweep (its lateral position divides by the stroke's length)",
        ));
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): only `base`, the strings' starting length.
pub fn ball_start(w: &RakeCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`dc_main = int(round(4 * every))`, as a float): the `dc` block of
/// `run()` (5971 to 5972) divides it by the haste's factor.
pub(crate) fn main16(w: &RakeCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}

/// `Fight.rake_lateral(tau, dur, S, direction)` (556 to 559): where across the stroke the bundle's middle is `tau`
/// seconds after the strum, smoothstepped from `-direction * S` to `direction * S` over `dur` seconds and held there.
pub fn rake_lateral(tau: f64, dur: f64, s: f64, direction: f64) -> f64 {
    // x = min(1.0, max(0.0, tau / dur))
    let x = py_min(1.0, py_max(0.0, tau / dur));
    // -direction * S + 2 * direction * S * x * x * (3 - 2 * x), left to right
    -direction * s + 2.0 * direction * s * x * x * (3.0 - 2.0 * x)
}

/// `Fight.sweep_front(tau, dur, reach, R)` (561 to 564): how far from where BIG STRUM started its front is `tau`
/// seconds in: `R + reach * x ** 1.5` with `x` the clamped share of `dur` (`**` on floats is the C `pow`).
pub fn sweep_front(tau: f64, dur: f64, reach: f64, r: f64) -> f64 {
    let x = py_min(1.0, py_max(0.0, tau / dur));
    r + reach * dmath::pow(x, 1.5)
}

#[cfg(test)]
mod tests {
    //! The expected numbers are what `fight.py` gave on the reference PC
    //! (`tools/studio_export/unit_checks/rake_unit_vectors.py`).
    use super::*;
    use crate::roster::{FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn acoustic() -> Json {
        let all: Json = serde_json::from_str(ROSTER).expect("the roster is JSON");
        let list = all["fighters"].as_array().expect("a list of fighters");
        list.iter()
            .find(|f| f["name"] == "ACOUSTIC")
            .expect("in the roster")
            .clone()
    }

    #[test]
    fn the_acoustic_loads_with_the_rosters_numbers() {
        let all = parse_roster(ROSTER).expect("the roster loads");
        let f = all.iter().find(|f| f.name == "ACOUSTIC").expect("in the roster");
        let WeaponCfg::Rake(w) = &f.weapon else {
            panic!("the ACOUSTIC's weapon is a rake: {:?}", f.weapon)
        };
        assert_eq!((w.style.as_str(), w.every, w.base, w.grow), ("rake", 1.0, 1.2, 0.03));
        assert_eq!((w.max_len, w.sweep, w.gap, w.stroke), (3.0, 0.6, 0.085, 0.16));
        assert_eq!((w.aim_k, w.dmg, w.sub_dmg, w.knock), (0.47, 2.1, 0.5, 0.7));
        assert_eq!((f.weapon.kind(), main16(w)), ("rake", Some(4.0)));
        assert_eq!(f.weapon.ball_start().base, 1.2);
        assert!(f.unlock1.kind.is_ported() && f.unlock2.kind.is_ported() && f.low.kind.is_ported());
        assert_eq!(f.unlock2.get("t_ext"), Some(0.07));
        assert_eq!(f.low.get("reach"), Some(9.0));
    }

    #[test]
    fn the_defaults_are_pythons() {
        let mut v = acoustic();
        let w = v["weapon"].as_object_mut().expect("a table");
        for key in ["base", "aim_k", "sub_dmg", "style"] {
            w.remove(key);
        }
        let f = FighterCfg::from_json(&v).expect("every option has a default");
        let WeaponCfg::Rake(w) = &f.weapon else {
            panic!("a rake")
        };
        assert_eq!((w.style.as_str(), w.base, w.aim_k, w.sub_dmg), ("rake", 0.0, 1.0, 0.5));
    }

    #[test]
    fn what_the_fight_cannot_run_is_refused_naming_the_field() {
        let refused = |key: &str, value: Json| {
            let mut v = acoustic();
            v["weapon"][key] = value;
            let err = FighterCfg::from_json(&v).expect_err("refused");
            assert_eq!(
                (err.fighter.as_str(), err.field.as_str()),
                ("ACOUSTIC", &*format!("weapon.{key}"))
            );
            err.message
        };
        assert!(refused("every", 1e308.into()).contains("finite"));
        assert!(refused("every", 0.12.into()).contains("no 16th"));
        assert!(refused("stroke", 0.0.into()).contains("no time"));
        assert!(refused("gap", "wide".into()).contains("number"));
        assert!(refused("pull", 1.into()).contains("not a setting of a rake weapon"));
        for key in ["every", "grow", "max_len", "sweep", "gap", "stroke", "dmg", "knock"] {
            let mut v = acoustic();
            v["weapon"].as_object_mut().expect("a table").remove(key);
            let err = FighterCfg::from_json(&v).expect_err("a missing number");
            assert_eq!(
                (err.field.as_str(), err.message.as_str()),
                (&*format!("weapon.{key}"), "missing")
            );
        }
        let mut f = FighterCfg::from_json(&acoustic()).expect("loads");
        if let WeaponCfg::Rake(w) = &mut f.weapon {
            w.stroke = 0.0;
        }
        assert_eq!(f.check().expect_err("refused").field, "weapon.stroke");
    }

    /// `Fight.rake_lateral` and `Fight.sweep_front` on the reference PC, bit for bit.
    #[test]
    fn the_closed_form_motions_are_pythons() {
        let mut n = 0;
        for d in [1.0, -1.0] {
            for (s, want) in [(0.6, LATERAL_06), (0.37, LATERAL_037)] {
                for (&tau, bits) in TAUS.iter().zip(want) {
                    // the two directions are mirror images: Python's numbers for -1 are those for +1, negated, but
                    // the middle is +0.0 both ways (`0.6 - 0.6`)
                    let want = if d > 0.0 || bits == 0 {
                        bits
                    } else {
                        (-f64::from_bits(bits)).to_bits()
                    };
                    let got = rake_lateral(tau, 0.16, s, d);
                    assert_eq!(got.to_bits(), want, "rake_lateral({tau}, 0.16, {s}, {d}) = {got}");
                    n += 1;
                }
            }
        }
        for (&tau, bits) in SWEEP_TAUS.iter().zip(SWEEP_FRONT) {
            if tau == POW_GAP_TAU {
                continue;
            }
            let got = sweep_front(tau, 0.9090909090909091, 9.0, 0.5);
            assert_eq!(got.to_bits(), bits, "sweep_front({tau}, 1/1.1, 9, 0.5) = {got}");
            n += 1;
        }
        assert_eq!(n, 49);
    }

    /// The vector of [`SWEEP_TAUS`] where `x ** 1.5` meets the last-bit gap between `dmath::pow` (musl) and the
    /// reference PC's CPython: `x = 0x1.c28f5c28f5c2ap-4`, Python `0x1.2ade2122b3ff5p-5`, `dmath::pow`
    /// `0x1.2ade2122b3ff4p-5`, so the front is one step low (`0x1.a81cf2a3853f9p-1` against `...3fap-1`). No fight or
    /// frame of the reference set meets it; it is the libm audit's (ruling M3-R38: `pow` made to give CPython's).
    const POW_GAP_TAU: f64 = 0.1;

    /// [`POW_GAP_TAU`]'s vector, bit for bit: it passes once `dmath::pow` gives CPython's `pow` (M3-R38).
    #[test]
    #[ignore = "dmath::pow is one step off CPython's x ** 1.5 here until the libm audit (M3-R38) merges"]
    fn the_sweep_front_at_the_pow_gap_is_pythons() {
        let got = sweep_front(POW_GAP_TAU, 0.9090909090909091, 9.0, 0.5);
        assert_eq!(got.to_bits(), SWEEP_FRONT[3], "sweep_front(0.1, 1/1.1, 9, 0.5) = {got}");
    }

    /// The times of the lateral vectors (a stroke of 0.16 s).
    const TAUS: [f64; 10] = [
        -0.01,
        0.0,
        0.004166666666666667,
        0.03,
        0.05,
        0.08,
        0.1234,
        0.15,
        0.16,
        0.2,
    ];
    /// `Fight.rake_lateral(tau, 0.16, 0.6, 1.0)` at [`TAUS`].
    const LATERAL_06: [u64; 10] = [
        0xbfe3333333333333,
        0xbfe3333333333333,
        0xbfe31f8c16c16c17,
        0xbfdf500000000000,
        0xbfd4966666666666,
        0x0000000000000000,
        0x3fdc2eb8b842d20e,
        0x3fe2c4cccccccccd,
        0x3fe3333333333333,
        0x3fe3333333333333,
    ];
    /// `Fight.rake_lateral(tau, 0.16, 0.37, 1.0)` at [`TAUS`].
    const LATERAL_037: [u64; 10] = [
        0xbfd7ae147ae147ae,
        0xbfd7ae147ae147ae,
        0xbfd795d77166054f,
        0xbfd34f3333333333,
        0xbfc96428f5c28f5c,
        0x0000000000000000,
        0x3fd161140b3a45ce,
        0x3fd725eb851eb852,
        0x3fd7ae147ae147ae,
        0x3fd7ae147ae147ae,
    ];
    /// The times of the sweep vectors (BIG STRUM of 2 beats at 132 BPM: 1 / 1.1 s).
    const SWEEP_TAUS: [f64; 10] = [
        -0.01,
        0.0,
        0.004166666666666667,
        0.1,
        0.3333,
        0.5,
        0.7,
        0.9,
        0.9090909090909091,
        1.0,
    ];
    /// `Fight.sweep_front(tau, 1 / 1.1, 9.0, 0.5)` at [`SWEEP_TAUS`].
    const SWEEP_FRONT: [u64; 10] = [
        0x3fe0000000000000,
        0x3fe0000000000000,
        0x3fe016e09637da5b,
        0x3fea81cf2a3853fa,
        0x4003fbccf30f7c22,
        0x4010af1f688e2fcc,
        0x401a52fedaeedf04,
        0x4022bb0d973f0db5,
        0x4023000000000000,
        0x4023000000000000,
    ];
}
