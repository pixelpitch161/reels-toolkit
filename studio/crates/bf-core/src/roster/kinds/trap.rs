//! Weapon kind `trap`, HARP's the strings: its roster settings. `fight.py` 1324 to 1332 (`hazard_beat`, shared with
//! `line`), 1363 to 1392 (`lay_string`) and the strings' half of `hazards_step` (1417 to 1434).

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `trap`.
pub const PORTED: bool = true;

/// Weapon kind `trap`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct TrapCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// Beats between two strings on the beat grid (a haste divides it).
    pub every: f64,
    /// Strings kept at the start (`n_vol`; `Ball` reads `float(w.get('n0', 0))`).
    pub n0: f64,
    /// Strings added per landed crossing.
    pub grow: f64,
    /// The count never grows past this.
    pub n_max: f64,
    /// Damage per crossing.
    pub dmg: f64,
    /// Seconds a string waits before it can hit again (`.get('cool', 0.4)`).
    pub cool: f64,
    /// Where the string crosses the line between the balls, as a share of the way (`.get('at', 0.5)`).
    pub at: f64,
    /// The stumble forward on a crossing (`.get('knock', 0.0)`; none when 0).
    pub knock: f64,
}

/// The keys a trap entry may hold: `kind`, `style` and [`TrapCfg`]'s fields.
const TRAP_KEYS: [&str; 10] = [
    "kind", "style", "every", "n0", "grow", "n_max", "dmg", "cool", "at", "knock",
];

/// Reads a `trap` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<TrapCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a trap weapon", &TRAP_KEYS)?;
    Ok(TrapCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "trap".to_string(),
        },
        every: e.grid(w, p, "every")?,
        n0: e.f64_or(w, p, "n0", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        n_max: e.f64(w, p, "n_max")?,
        dmg: e.f64(w, p, "dmg")?,
        cool: e.f64_or(w, p, "cool", 0.4)?,
        at: e.f64_or(w, p, "at", 0.5)?,
        knock: e.f64_or(w, p, "knock", 0.0)?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &TrapCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): only `n0`, the strings kept at the start.
pub fn ball_start(w: &TrapCfg) -> BallStart {
    BallStart {
        n0: w.n0,
        ..BallStart::default()
    }
}

/// The number a haste divides: `hazard_beat`'s `int(round(every / ab_mult(k)))`, as for `line`.
pub(crate) fn main16(w: &TrapCfg) -> Option<f64> {
    Some(w.every)
}

#[cfg(test)]
mod tests {
    use crate::roster::{WeaponCfg, parse_roster};

    const HARP: &str = r#"{"format": 1, "fighters": [{"name": "HARP", "color": [244, 228, 190],
        "dark": [166, 146, 104], "corner": "blue", "view": [8, 26, 0],
        "weapon": {"kind": "trap", "style": "strings", "every": 2, "n0": 1, "grow": 0.5, "n_max": 7, "dmg": 1.8,
        "cool": 0.4, "at": 0.5, "knock": 0.25},
        "abilities": {"unlock1": {"name": "PLUCK", "kind": "arc", "n": 7, "sweep": 70.0, "beats": 1, "speed": 6.0,
        "dmg": 1.2, "every_bars": 3}, "unlock2": {"name": "ANGEL WINGS", "kind": "shield", "dur_beats": 6,
        "every_bars": 6}, "low": {"name": "47 STRINGS", "kind": "wave", "speed": 4.5, "radius": 9.0, "dmg": 8.0,
        "knock": 1.5}}, "pp": "HARP", "register": [50, 84], "idiom": "harp"}]}"#;

    #[test]
    fn the_roster_entry_reads() {
        let roster = parse_roster(HARP).expect("HARP");
        let WeaponCfg::Trap(w) = &roster[0].weapon else {
            panic!("not a trap: {:?}", roster[0].weapon)
        };
        assert_eq!(
            (w.style.as_str(), w.every, w.n0, w.grow, w.n_max),
            ("strings", 2.0, 1.0, 0.5, 7.0)
        );
        assert_eq!((w.dmg, w.cool, w.at, w.knock), (1.8, 0.4, 0.5, 0.25));
        assert_eq!(roster[0].weapon.ball_start().n0, 1.0);
        let err = parse_roster(&HARP.replace(r#""dmg": 1.8"#, r#""dmg": 1.8, "pull": 1"#)).expect_err("pull");
        assert_eq!(err.field, "weapon.pull");
    }
}
