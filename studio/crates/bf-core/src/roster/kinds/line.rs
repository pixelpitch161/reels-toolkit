//! Weapon kind `line`, ORGAN's the pipes: its roster settings. `fight.py` 1323 to 1362 and 1393 to 1416
//! (`hazard_beat`, `_to_ropes`, `fire_line`, the lines' half of `hazards_step`; `lay_string` and `strings` belong to
//! the older `trap` kind and are not ported).

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `line`.
pub const PORTED: bool = true;

/// Weapon kind `line`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct LineCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// Beats between two rows on the beat grid (`max(1, int(round(every / ab_mult)))`: a haste divides it). In a
    /// roster entry 4 x `every` is a finite number.
    pub every: f64,
    /// The starting length of a row (`blade_len`; `Ball` reads `.get('base', 0.0)`).
    pub base: f64,
    /// Length added per landed row.
    pub grow: f64,
    /// The length never grows past this.
    pub max_len: f64,
    /// Units a second the eruption front runs out. Never 0 in a roster entry (the look divides by it).
    pub speed: f64,
    /// Seconds a pipe stays up.
    pub linger: f64,
    /// The row's width: a hit when the enemy's centre is within its radius plus half of this.
    pub width: f64,
    /// Damage per row.
    pub dmg: f64,
    /// Knockback per row (`.get('knock', 0.8)`).
    pub knock: f64,
    /// The row starts this far beyond the ball's surface (`.get('gap', 0.05)`).
    pub gap: f64,
}

/// The keys a line entry may hold: `kind`, `style` and [`LineCfg`]'s fields.
const LINE_KEYS: [&str; 12] = [
    "kind", "style", "every", "base", "grow", "max_len", "speed", "linger", "width", "dmg", "knock", "gap",
];

/// A row's speed: the kit's look divides by it (`organ.py` `row_pipes`: Python raises on 0).
fn speed_value(e: &Entry, x: f64) -> Result<f64, RosterError> {
    if x == 0.0 {
        return Err(e.err("weapon.speed", "0 is not a speed (the row's front must move)"));
    }
    Ok(x)
}

/// Reads a `line` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<LineCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a line weapon", &LINE_KEYS)?;
    Ok(LineCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "line".to_string(),
        },
        every: e.grid(w, p, "every")?,
        base: e.f64_or(w, p, "base", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        max_len: e.f64(w, p, "max_len")?,
        speed: speed_value(e, e.f64(w, p, "speed")?)?,
        linger: e.f64(w, p, "linger")?,
        width: e.f64(w, p, "width")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64_or(w, p, "knock", 0.8)?,
        gap: e.f64_or(w, p, "gap", 0.05)?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &LineCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    speed_value(e, w.speed)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): only `base`, the starting row length.
pub fn ball_start(w: &LineCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The number a haste divides: `hazard_beat` takes `int(round(every / ab_mult(k)))`, which raises in Python when the
/// quotient is not finite. The line's grid is in beats, not 16ths; the roster's check is the same division.
pub(crate) fn main16(w: &LineCfg) -> Option<f64> {
    Some(w.every)
}

#[cfg(test)]
mod tests {
    use crate::roster::{WeaponCfg, parse_roster};

    fn organ(weapon: &str) -> String {
        format!(
            r#"{{"format": 1, "fighters": [{{"name": "ORGAN", "color": [196, 52, 104], "dark": [112, 24, 58],
            "corner": "red", "view": [0, 22, 0], "weapon": {weapon},
            "abilities": {{"unlock1": {{"name": "CATHEDRAL", "kind": "shield", "dur_beats": 6, "every_bars": 6}},
            "unlock2": {{"name": "PULL ALL STOPS", "kind": "haste", "mult": 2.0, "dur_beats": 8, "every_bars": 6}},
            "low": {{"name": "TOCCATA", "kind": "slam", "tele": 2, "radius": 1.5, "dmg": 11.0, "knock": 1.6}}}},
            "pp": "ORGAN", "register": [36, 84], "idiom": "organ"}}]}}"#
        )
    }

    const KIT: &str = r#"{"kind": "line", "style": "pipes", "every": 2, "base": 1.6, "grow": 0.06, "max_len": 5.6,
        "speed": 6.0, "linger": 0.55, "width": 0.3, "dmg": 4.0, "knock": 0.8}"#;

    #[test]
    fn the_kit_entry_reads_with_pythons_defaults() {
        let roster = parse_roster(&organ(KIT)).expect("the kit's ORGAN");
        let WeaponCfg::Line(w) = &roster[0].weapon else {
            panic!("not a line: {:?}", roster[0].weapon)
        };
        assert_eq!(
            (w.style.as_str(), w.every, w.base, w.grow, w.max_len),
            ("pipes", 2.0, 1.6, 0.06, 5.6)
        );
        assert_eq!(
            (w.speed, w.linger, w.width, w.dmg, w.knock, w.gap),
            (6.0, 0.55, 0.3, 4.0, 0.8, 0.05)
        );
        assert_eq!(roster[0].weapon.ball_start().base, 1.6);
    }

    #[test]
    fn what_the_kind_cannot_run_is_refused_by_name() {
        for (from, to, field) in [
            (r#""speed": 6.0"#, r#""speed": 0.0"#, "weapon.speed"),
            (r#""every": 2"#, r#""every": 1e308"#, "weapon.every"),
            (r#""dmg": 4.0"#, r#""dmg": "four""#, "weapon.dmg"),
            (r#""dmg": 4.0"#, r#""dmg": 4.0, "pull": 1.0"#, "weapon.pull"),
            (r#""grow": 0.06, "#, "", "weapon.grow"),
        ] {
            assert!(KIT.contains(from), "{from}");
            let err = parse_roster(&organ(&KIT.replace(from, to))).expect_err(field);
            assert_eq!((err.fighter.as_str(), err.field.as_str()), ("ORGAN", field), "{err}");
        }
    }
}
