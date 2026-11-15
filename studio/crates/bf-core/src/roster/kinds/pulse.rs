//! Weapon kind `pulse`, DRUMS's the drumhead: its roster settings. `fight.py` 411 to 424 (`fire_pulse`), the pulse
//! counter of `run()` (5946 to 5963) and its rings in `effects_step` (kind `pulse`, with `grow` and `clears`).

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `pulse`.
pub const PORTED: bool = true;

/// Weapon kind `pulse`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct PulseCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// Beats between two main pulses (a haste adds pulses between them).
    pub every: f64,
    /// The starting reach beyond the ball's surface (`w.get('base', 0.0)`).
    pub base: f64,
    /// Reach added per landed main pulse.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_reach: f64,
    /// Units a second the ring spreads.
    pub speed: f64,
    /// Damage of a main pulse.
    pub dmg: f64,
    /// A haste pulse's share of `dmg` (`.get('sub_dmg', 0.5)`).
    pub sub_dmg: f64,
    /// Knockback of a main pulse (a haste pulse knocks half).
    pub knock: f64,
    /// Whether its rings clear the enemy's projectiles (`.get('clears', False)`).
    pub clears: bool,
}

/// The keys a pulse entry may hold: `kind`, `style` and [`PulseCfg`]'s fields.
const PULSE_KEYS: [&str; 11] = [
    "kind",
    "style",
    "every",
    "base",
    "grow",
    "max_reach",
    "speed",
    "dmg",
    "sub_dmg",
    "knock",
    "clears",
];

/// Reads a `pulse` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<PulseCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a pulse weapon", &PULSE_KEYS)?;
    let clears = match w.get("clears") {
        None => false,
        Some(v) => v
            .as_bool()
            .ok_or_else(|| e.err("weapon.clears", "must be true or false"))?,
    };
    Ok(PulseCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "pulse".to_string(),
        },
        every: e.grid(w, p, "every")?,
        base: e.f64_or(w, p, "base", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        speed: e.f64(w, p, "speed")?,
        dmg: e.f64(w, p, "dmg")?,
        sub_dmg: e.f64_or(w, p, "sub_dmg", 0.5)?,
        knock: e.f64(w, p, "knock")?,
        clears,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &PulseCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    if w.every <= 0.0 {
        return Err(e.err(
            "weapon.every",
            format!("{} beats: a pulse needs a grid above 0", w.every),
        ));
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): only `base`, the starting reach.
pub fn ball_start(w: &PulseCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float).
pub(crate) fn main16(w: &PulseCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}

#[cfg(test)]
mod tests {
    use crate::roster::{WeaponCfg, parse_roster};

    const DRUMS: &str = r#"{"format": 1, "fighters": [{"name": "DRUMS", "color": [255, 146, 40],
        "dark": [160, 84, 16], "corner": "red", "view": [12, 30, 0],
        "weapon": {"kind": "pulse", "style": "drumhead", "every": 1, "base": 0.9, "grow": 0.07, "max_reach": 2.3,
        "speed": 9.0, "dmg": 2.5, "sub_dmg": 0.45, "knock": 0.75, "clears": true},
        "abilities": {"unlock1": {"name": "DRUM ROLL", "kind": "haste", "mult": 4.0, "dur_beats": 4,
        "every_bars": 4}, "unlock2": {"name": "CRASH CYMBAL", "kind": "shield", "dur_beats": 6, "every_bars": 6},
        "low": {"name": "DOUBLE KICK", "kind": "slam", "tele": 2, "radius": 1.35, "dmg": 11.0, "knock": 1.6}},
        "pp": "DRUMS", "register": [36, 50], "idiom": "kit"}]}"#;

    #[test]
    fn the_roster_entry_reads() {
        let roster = parse_roster(DRUMS).expect("DRUMS");
        let WeaponCfg::Pulse(w) = &roster[0].weapon else {
            panic!("not a pulse: {:?}", roster[0].weapon)
        };
        assert_eq!(
            (w.style.as_str(), w.every, w.base, w.grow),
            ("drumhead", 1.0, 0.9, 0.07)
        );
        assert_eq!(
            (w.max_reach, w.speed, w.dmg, w.sub_dmg, w.knock, w.clears),
            (2.3, 9.0, 2.5, 0.45, 0.75, true)
        );
        assert_eq!(roster[0].weapon.ball_start().base, 0.9);
        let err = parse_roster(&DRUMS.replace(r#""dmg": 2.5"#, r#""dmg": 2.5, "pull": 1"#)).expect_err("pull");
        assert_eq!(err.field, "weapon.pull");
    }
}
