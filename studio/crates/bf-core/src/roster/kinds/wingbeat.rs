//! Weapon kind `wingbeat`, FLUGELHORN's WINGBEAT: its roster settings. A Studio kind (no `fight.py` original; design
//! `docs/studio/2026-11-16-flugelhorn-design.md`).
//!
//! The ball's aim turns toward the enemy at `aim_rate`. Every `every` beats on the beat grid (a haste divides it) two
//! wings sweep out of its sides and clap shut in front of it: an enemy within the reach beyond the two surfaces
//! (`base`, `grow` more per landed clap, up to `max_reach`) and `spread` degrees of the aim is hit for `dmg` and blown
//! back along the aim (knock `knock`).

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `wingbeat`.
pub const PORTED: bool = true;

/// Weapon kind `wingbeat`'s typed settings.
#[derive(Clone, Debug, PartialEq)]
pub struct WingbeatCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// The weapon's name on the card.
    pub name: Option<String>,
    /// Beats between two claps on the beat grid (a haste divides it).
    pub every: f64,
    /// Degrees each side of the aim the wings close over. Above 0 and at most 180.
    pub spread: f64,
    /// The starting reach beyond the two balls' surfaces (`blade_len`).
    pub base: f64,
    /// Reach added per landed clap.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_reach: f64,
    /// Damage of a clap.
    pub dmg: f64,
    /// The knock of a clap (the victim's speed boost).
    pub knock: f64,
    /// Radians a second the aim turns toward the enemy.
    pub aim_rate: f64,
}

/// The keys a wingbeat entry may hold.
const KEYS: [&str; 11] = [
    "kind",
    "style",
    "name",
    "every",
    "spread",
    "base",
    "grow",
    "max_reach",
    "dmg",
    "knock",
    "aim_rate",
];

/// The wings' half-angle: 0 never hits and more than 180 is no angle.
fn spread_value(e: &Entry, x: f64) -> Result<f64, RosterError> {
    if !(x > 0.0 && x <= 180.0) {
        return Err(e.err("weapon.spread", "must be above 0 and at most 180 degrees"));
    }
    Ok(x)
}

/// Reads a `wingbeat` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<WingbeatCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a wingbeat weapon", &KEYS)?;
    Ok(WingbeatCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "wingbeat".to_string(),
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
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64_or(w, p, "knock", 1.0)?,
        aim_rate: e.f64_or(w, p, "aim_rate", 6.0)?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &WingbeatCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    spread_value(e, w.spread)?;
    Ok(())
}

/// What the ball starts with: the reach.
pub fn ball_start(w: &WingbeatCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The number a haste divides: the clap's grid, in beats.
pub(crate) fn main16(w: &WingbeatCfg) -> Option<f64> {
    Some(w.every)
}

#[cfg(test)]
mod tests {
    use crate::roster::{AbilityKind, WeaponCfg, parse_roster};

    const ENTRY: &str = include_str!("../../../../bf-content/data/roster_new.json");

    #[test]
    fn the_studio_entry_reads() {
        let roster = parse_roster(ENTRY).expect("the studio roster");
        let f = roster.iter().find(|f| f.name == "FLUGELHORN").expect("its entry");
        let WeaponCfg::Wingbeat(w) = &f.weapon else {
            panic!("not a wingbeat: {:?}", f.weapon)
        };
        assert_eq!(w.style, "wingbeat");
        assert!(w.spread > 0.0 && w.max_reach >= w.base);
        assert_eq!(f.weapon.kind(), "wingbeat");
        assert_eq!(f.weapon.ball_start().base, w.base);
        assert_eq!(f.unlock1.kind, AbilityKind::Flank);
        assert_eq!(f.low.kind, AbilityKind::TakeWing);
    }

    #[test]
    fn wings_that_cannot_close_are_refused_by_name() {
        let roster = ENTRY.replace(r#""spread": 60.0"#, r#""spread": 0.0"#);
        assert_ne!(roster, ENTRY, "the entry's spread moved");
        assert_eq!(parse_roster(&roster).unwrap_err().field, "weapon.spread");
    }
}
