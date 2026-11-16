//! Weapon kind `plumbing`, CONTRABASSOON's U-BEND: its roster settings. A fighter made from scratch (Nov 16, 2026,
//! `docs/studio/2026-11-16-contrabassoon.md`), so no `fight.py` lines stand behind it: Rust is the source.
//!
//! Every `every` beats (a haste divides it) the fighter lays a U-shaped piece of tube in the ring, `lead` ahead of the
//! enemy, its bend toward the enemy (legs `leg` long, `gap` apart, `thick` thick). At most `max_pipes` stand; the
//! oldest goes. Both balls bounce off pipes. An enemy bounce hurts (`dmg`, + `dmg_grow` a landed hit up to `dmg_max`,
//! once per `cool` s a pipe) and boosts the enemy by `knock`; the owner's bounce boosts it by `own_boost`.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `plumbing`.
pub const PORTED: bool = true;

/// Weapon kind `plumbing`'s typed settings.
#[derive(Clone, Debug, PartialEq)]
pub struct PlumbingCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// The weapon's name (default `U-BEND`).
    pub name: String,
    /// Beats between pipes.
    pub every: f64,
    /// The most pipes that stand at once.
    pub max_pipes: f64,
    /// A leg's length.
    pub leg: f64,
    /// The distance between the two legs.
    pub gap: f64,
    /// The tube's half-thickness (what a ball bounces off).
    pub thick: f64,
    /// How far ahead of the enemy, along its heading, a pipe is laid.
    pub lead: f64,
    /// The first bounce's damage.
    pub dmg: f64,
    /// The damage a landed bounce adds to the next.
    pub dmg_grow: f64,
    /// The bounce damage never grows past this.
    pub dmg_max: f64,
    /// Seconds before the same pipe can hurt again.
    pub cool: f64,
    /// The speed boost an enemy bounce gives the enemy.
    pub knock: f64,
    /// The speed boost the owner's own bounce gives it.
    pub own_boost: f64,
}

/// The keys a `plumbing` entry may hold.
const KEYS: [&str; 16] = [
    "kind",
    "style",
    "name",
    "every",
    "max_pipes",
    "leg",
    "gap",
    "thick",
    "lead",
    "dmg",
    "dmg_grow",
    "dmg_max",
    "cool",
    "knock",
    "own_boost",
    "n0",
];

/// Reads a `plumbing` weapon entry.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<PlumbingCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a plumbing weapon", &KEYS)?;
    Ok(PlumbingCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "plumbing".to_string(),
        },
        name: match w.get("name") {
            Some(_) => e.string(w, p, "name")?,
            None => "U-BEND".to_string(),
        },
        every: e.grid(w, p, "every")?,
        max_pipes: e.f64_or(w, p, "max_pipes", 3.0)?,
        leg: e.f64_or(w, p, "leg", 0.8)?,
        gap: e.f64_or(w, p, "gap", 0.5)?,
        thick: e.f64_or(w, p, "thick", 0.09)?,
        lead: e.f64_or(w, p, "lead", 1.6)?,
        dmg: e.f64(w, p, "dmg")?,
        dmg_grow: e.f64_or(w, p, "dmg_grow", 0.0)?,
        dmg_max: e.f64_or(w, p, "dmg_max", f64::INFINITY)?,
        cool: e.f64_or(w, p, "cool", 0.4)?,
        knock: e.f64_or(w, p, "knock", 0.0)?,
        own_boost: e.f64_or(w, p, "own_boost", 0.0)?,
    })
}

/// The kind's checks: a grid `every`, at least one pipe, a positive size.
pub(crate) fn check(e: &Entry, w: &PlumbingCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    if !(w.max_pipes >= 1.0) {
        return Err(e.err("weapon.max_pipes", "must be at least 1"));
    }
    for (name, v) in [("leg", w.leg), ("gap", w.gap), ("thick", w.thick)] {
        if !(v.is_finite() && v > 0.0) {
            return Err(e.err(&format!("weapon.{name}"), "must be a finite number above 0"));
        }
    }
    Ok(())
}

/// What `Ball.__init__` reads: nothing of its own.
pub fn ball_start(_w: &PlumbingCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's grid, in beats.
pub(crate) fn main16(w: &PlumbingCfg) -> Option<f64> {
    Some(w.every)
}

#[cfg(test)]
mod tests {
    use crate::roster::{AbilityKind, WeaponCfg, parse_roster};

    const ENTRY: &str = r#"{"format": 1, "fighters": [{"name": "CONTRABASSOON", "color": [150, 34, 44],
        "dark": [84, 16, 22], "corner": "red", "view": [20, 18, 0],
        "weapon": {"kind": "plumbing", "style": "ubend", "name": "U-BEND", "every": 4, "max_pipes": 3, "leg": 0.8,
        "gap": 0.5, "thick": 0.09, "lead": 1.6, "dmg": 2.6, "dmg_grow": 0.15, "dmg_max": 4.0, "cool": 0.4,
        "knock": 0.6, "own_boost": 0.5},
        "abilities": {"unlock1": {"name": "WATER KEY", "kind": "water_key", "beats": 1, "reach": 0.9, "dmg": 2.0,
        "knock": 1.2, "every_bars": 3}, "unlock2": {"name": "HOT PIPES", "kind": "hot_pipes", "dur_beats": 8,
        "mult": 2.0, "every_bars": 6}, "low": {"name": "FIVE AND A HALF METRES", "kind": "pipe_box", "beats": 4,
        "half": 1.0, "dmg": 2.0, "cool": 0.2}}, "pp": "BASSOON", "register": [22, 58], "idiom": "reed"}]}"#;

    #[test]
    fn the_roster_entry_reads() {
        let roster = parse_roster(ENTRY).expect("CONTRABASSOON");
        let WeaponCfg::Plumbing(w) = &roster[0].weapon else {
            panic!("not plumbing: {:?}", roster[0].weapon)
        };
        assert_eq!(
            (w.style.as_str(), w.name.as_str(), w.every, w.max_pipes),
            ("ubend", "U-BEND", 4.0, 3.0)
        );
        assert_eq!(
            (w.dmg, w.dmg_grow, w.dmg_max, w.knock, w.own_boost),
            (2.6, 0.15, 4.0, 0.6, 0.5)
        );
        let ab = &roster[0];
        assert_eq!(ab.unlock1.kind, AbilityKind::WaterKey);
        assert_eq!(ab.unlock2.kind, AbilityKind::HotPipes);
        assert_eq!(ab.low.kind, AbilityKind::PipeBox);
        let err = parse_roster(&ENTRY.replace(r#""dmg": 2.6"#, r#""dmg": 2.6, "pull": 1"#)).expect_err("pull");
        assert_eq!(err.field, "weapon.pull");
    }
}
