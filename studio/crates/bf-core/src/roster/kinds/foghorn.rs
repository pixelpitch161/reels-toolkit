//! Weapon kind `foghorn`, CONTRABASS CLARINET's FOGHORN: its roster settings. The first kind made in the Studio (no
//! `fight.py` original; design `docs/studio/2026-11-16-contrabass-clarinet-design.md`).
//!
//! Every `every` beats on the beat grid (a haste divides it) the bell, which turns toward the enemy at `aim_rate`,
//! blows a cone `spread` degrees each side of its aim and `reach` beyond the balls' surfaces (`base`, `grow` more per
//! landed wall slam, up to `max_reach`), live for `dur` seconds. Its first touch hits for `dmg` and shoves the enemy
//! along the blast (a speed boost of `push`); while it lasts it keeps steering the enemy away. A rope hit of the shoved
//! enemy within `slam_t` seconds is the wall slam (`dmg_wall`).

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `foghorn`.
pub const PORTED: bool = true;

/// Weapon kind `foghorn`'s typed settings.
#[derive(Clone, Debug, PartialEq)]
pub struct FoghornCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// The weapon's name on the card.
    pub name: Option<String>,
    /// Beats between two blasts on the beat grid (a haste divides it).
    pub every: f64,
    /// Degrees each side of the aim the cone covers. Above 0 and below 90.
    pub spread: f64,
    /// The starting reach beyond the two balls' surfaces (`blade_len`).
    pub base: f64,
    /// Reach added per landed wall slam.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_reach: f64,
    /// Seconds a blast is live. Above 0.
    pub dur: f64,
    /// The shoved enemy's speed boost (at least this).
    pub push: f64,
    /// How hard a live blast steers the enemy's heading along it, in units a second squared.
    pub steer: f64,
    /// Damage of a blast's first touch.
    pub dmg: f64,
    /// Damage of the wall slam.
    pub dmg_wall: f64,
    /// Seconds after the shove in which a rope hit is the wall slam.
    pub slam_t: f64,
    /// Radians a second the bell turns toward the enemy.
    pub aim_rate: f64,
}

/// The keys a foghorn entry may hold.
const KEYS: [&str; 15] = [
    "kind",
    "style",
    "name",
    "every",
    "spread",
    "base",
    "grow",
    "max_reach",
    "dur",
    "push",
    "steer",
    "dmg",
    "dmg_wall",
    "slam_t",
    "aim_rate",
];

/// The cone's half-angle: a cone of 0 never hits and one of 90 or more is no cone.
fn spread_value(e: &Entry, x: f64) -> Result<f64, RosterError> {
    if !(x > 0.0 && x < 90.0) {
        return Err(e.err("weapon.spread", "must be above 0 and below 90 degrees"));
    }
    Ok(x)
}

/// A blast's length: it must last.
fn dur_value(e: &Entry, x: f64) -> Result<f64, RosterError> {
    if !(x > 0.0 && x.is_finite()) {
        return Err(e.err("weapon.dur", "must be a number of seconds above 0"));
    }
    Ok(x)
}

/// Reads a `foghorn` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<FoghornCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a foghorn weapon", &KEYS)?;
    Ok(FoghornCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "foghorn".to_string(),
        },
        name: match w.get("name") {
            Some(_) => Some(e.string(w, p, "name")?),
            None => None,
        },
        every: e.grid(w, p, "every")?,
        spread: spread_value(e, e.f64(w, p, "spread")?)?,
        base: e.f64_or(w, p, "base", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        dur: dur_value(e, e.f64(w, p, "dur")?)?,
        push: e.f64_or(w, p, "push", 2.0)?,
        steer: e.f64_or(w, p, "steer", 18.0)?,
        dmg: e.f64(w, p, "dmg")?,
        dmg_wall: e.f64(w, p, "dmg_wall")?,
        slam_t: e.f64_or(w, p, "slam_t", 0.8)?,
        aim_rate: e.f64_or(w, p, "aim_rate", 5.0)?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &FoghornCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    spread_value(e, w.spread)?;
    dur_value(e, w.dur)?;
    Ok(())
}

/// What the ball starts with: the reach.
pub fn ball_start(w: &FoghornCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The number a haste divides: the blast's grid, in beats.
pub(crate) fn main16(w: &FoghornCfg) -> Option<f64> {
    Some(w.every)
}

#[cfg(test)]
mod tests {
    use crate::roster::{WeaponCfg, parse_roster};

    const ENTRY: &str = include_str!("../../../../bf-content/data/roster_new.json");

    #[test]
    fn the_studio_entry_reads() {
        let roster = parse_roster(ENTRY).expect("the studio roster");
        let f = roster
            .iter()
            .find(|f| f.name == "CONTRABASS CLARINET")
            .expect("its entry");
        let WeaponCfg::Foghorn(w) = &f.weapon else {
            panic!("not a foghorn: {:?}", f.weapon)
        };
        assert_eq!(w.style, "foghorn");
        assert!(w.spread > 0.0 && w.dur > 0.0 && w.max_reach >= w.base);
        assert_eq!(f.weapon.kind(), "foghorn");
        assert_eq!(f.weapon.ball_start().base, w.base);
    }

    #[test]
    fn a_cone_that_cannot_work_is_refused_by_name() {
        let roster = ENTRY.replace(r#""spread": 28.0"#, r#""spread": 95.0"#);
        assert_ne!(roster, ENTRY, "the entry's spread moved");
        let err = parse_roster(&roster).unwrap_err();
        assert_eq!(err.field, "weapon.spread");
        let roster = ENTRY.replace(r#""dur": 0.55"#, r#""dur": 0.0"#);
        assert_eq!(parse_roster(&roster).unwrap_err().field, "weapon.dur");
    }
}
