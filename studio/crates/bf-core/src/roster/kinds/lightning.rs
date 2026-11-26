//! Weapon kind `lightning`, ELECTRIC's LIGHTNING RIFF: its roster settings. `fight.py` 3845 to 3922 (`POSTS`,
//! `lightning_setup`, `lightning_tick`, `bolt_source`, `bolt`), and the abilities `fork` and `tapping` (`trigger` 2933
//! to 2938).
//!
//! Every `every` beats on the 16th grid (a haste does not divide it) the fighter plays a riff of `int(n_vol)` notes
//! (`n0`, plus `grow` per riff that lands, up to `n_max`) on the 16th slots `slots` (the first `int(n_vol)` of them,
//! played in time order). Every note is an instant bolt: from the headstock when the enemy is within `reach` of the
//! ball's surface, else from the corner post nearest the enemy when the enemy is within `post_r` of it, else a miss.
//! A bolt hits for `dmg` and knocks the enemy `knock` away from where it came.
//!
//! [`LightningCfg`] holds every field `fight.py` reads: `w[name]`, and `w.get(name, default)` with its default.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `lightning`.
pub const PORTED: bool = true;

/// Weapon kind `lightning`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct LightningCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// The weapon's name on the card (`w.get('name')`; the fight never reads it).
    pub name: Option<String>,
    /// Beats between two riffs on the 16th grid (`int(round(4 * every))`, at least one 16th).
    pub every: f64,
    /// `w.get('n0')`: the riff's notes at the start. `Ball.__init__` reads it with the default 0 and
    /// `lightning_setup` again with the default 2, so absent is `None`.
    pub n0: Option<f64>,
    /// Notes added per riff that lands.
    pub grow: f64,
    /// A riff never has more notes than this.
    pub n_max: f64,
    /// The riff's 16th slots in the bar, in the order they are taken (`w['slots'][:n]`, then sorted).
    pub slots: Vec<f64>,
    /// How far from the ball's surface the enemy may be for a bolt from the headstock.
    pub reach: f64,
    /// How far from a corner post the enemy may be for a bolt from it.
    pub post_r: f64,
    /// Damage of a bolt.
    pub dmg: f64,
    /// A bolt's knock.
    pub knock: f64,
}

/// The keys a lightning entry may hold: `kind`, `style`, `name` and [`LightningCfg`]'s numbers. (`base`, which
/// `Ball.__init__` reads for every weapon as the reach, means nothing to this kind and is refused.)
const LIGHTNING_KEYS: [&str; 12] = [
    "kind", "style", "name", "every", "n0", "grow", "n_max", "slots", "reach", "post_r", "dmg", "knock",
];

/// Reads a `lightning` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<LightningCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a lightning weapon", &LIGHTNING_KEYS)?;
    let list = e
        .get(w, p, "slots")?
        .as_array()
        .ok_or_else(|| e.err("weapon.slots", "must be a list of numbers"))?;
    let slots = list
        .iter()
        .map(|v| e.number(v, "weapon.slots"))
        .collect::<Result<Vec<_>, _>>()?;
    let cfg = LightningCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "lightning".to_string(),
        },
        name: match w.get("name") {
            Some(_) => Some(e.string(w, p, "name")?),
            None => None,
        },
        every: e.grid(w, p, "every")?,
        n0: match w.get("n0") {
            Some(v) => Some(e.number(v, "weapon.n0")?),
            None => None,
        },
        grow: e.f64(w, p, "grow")?,
        n_max: e.f64(w, p, "n_max")?,
        slots,
        reach: e.f64(w, p, "reach")?,
        post_r: e.f64(w, p, "post_r")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): what the Python fight
/// cannot run. `int(round(4 * every))` raises when `4 * every` is not finite, and `k16 % main16` when it rounds to no
/// 16th; `int(B.n_vol + 1e-9)` raises when the notes are not finite (the engine's counts are 64-bit: at most 2^53).
pub(crate) fn check(e: &Entry, w: &LightningCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    if crate::py::round_half_even(4.0 * w.every) == 0.0 {
        return Err(e.err(
            "weapon.every",
            format!(
                "{} beats rounds to no 16th note (the riff's grid divides by it)",
                w.every
            ),
        ));
    }
    for (field, x) in [
        ("weapon.n0", w.n0.unwrap_or(0.0)),
        ("weapon.n_max", w.n_max),
        ("weapon.grow", w.grow),
    ] {
        if !x.is_finite() || x.abs() > 9_007_199_254_740_992.0 {
            return Err(e.err(
                field,
                format!("{x:e} notes is too many (the riff takes a whole number of them)"),
            ));
        }
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): `n0` (default 0; `lightning_setup` sets
/// `n_vol` again, with the default 2).
pub fn ball_start(w: &LightningCfg) -> BallStart {
    BallStart {
        n0: w.n0.unwrap_or(0.0),
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste: `None`, since the riff's grid is not divided by a haste
/// (`lightning_tick` takes `k16 % main16` with no `ab_mult`).
pub(crate) fn main16(_w: &LightningCfg) -> Option<f64> {
    None
}

/// `POSTS` (3856): the x and y signs of the four corner posts (ring frame), in the engine's order.
pub const POSTS: [(f64, f64); 4] = [(-1.0, 1.0), (1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roster::{FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn electric() -> Json {
        let all: Json = serde_json::from_str(ROSTER).expect("the roster is JSON");
        let list = all["fighters"].as_array().expect("a list of fighters");
        list.iter()
            .find(|f| f["name"] == "ELECTRIC")
            .expect("in the roster")
            .clone()
    }

    #[test]
    fn the_electric_loads_with_the_rosters_numbers() {
        let all = parse_roster(ROSTER).expect("the roster loads");
        let f = all.iter().find(|f| f.name == "ELECTRIC").expect("in the roster");
        let WeaponCfg::Lightning(w) = &f.weapon else {
            panic!("the ELECTRIC's weapon is lightning: {:?}", f.weapon)
        };
        assert_eq!(
            (w.style.as_str(), w.name.as_deref(), w.every),
            ("bolt", Some("LIGHTNING RIFF"), 4.0)
        );
        assert_eq!((w.n0, w.grow, w.n_max), (Some(2.0), 0.5, 8.0));
        assert_eq!(w.slots, vec![0.0, 6.0, 12.0, 3.0, 10.0, 8.0, 14.0, 4.0]);
        assert_eq!((w.reach, w.post_r, w.dmg, w.knock), (1.1, 1.7, 1.5, 0.15));
        assert_eq!((f.weapon.kind(), main16(w)), ("lightning", None));
        assert_eq!(f.weapon.ball_start().n0, 2.0);
        assert!(f.unlock1.kind.is_ported() && f.unlock2.kind.is_ported() && f.low.kind.is_ported());
        assert_eq!(f.unlock1.get("reach_k"), Some(1.4));
        assert_eq!(f.unlock2.get("dmg_k"), Some(0.7));
        assert_eq!(f.low.get("splinter_knock"), Some(0.4));
    }

    #[test]
    fn what_python_cannot_run_is_refused() {
        for (key, value, field) in [
            ("every", serde_json::json!(0.1), "weapon.every"),
            ("slots", serde_json::json!("0 6 12"), "weapon.slots"),
            ("slots", serde_json::json!([0, "6"]), "weapon.slots"),
            ("base", serde_json::json!(0.5), "weapon.base"),
        ] {
            let mut v = electric();
            v["weapon"][key] = value;
            let err = FighterCfg::from_json(&v).expect_err(key);
            assert_eq!(err.field, field, "{err:?}");
        }
        let mut v = electric();
        v["weapon"].as_object_mut().expect("a table").remove("n0");
        let f = FighterCfg::from_json(&v).expect("n0 has defaults");
        let WeaponCfg::Lightning(w) = &f.weapon else {
            panic!("lightning")
        };
        assert_eq!((w.n0, f.weapon.ball_start().n0), (None, 0.0));
    }
}
