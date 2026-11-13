//! Weapon kind `scratch_floor`, TURNTABLES's THE RING IS THE RECORD: its roster settings. `fight.py` 1713 to 1762
//! (`scratch_beat`, `scratch_step`).
//!
//! Every `every` beats (a haste divides it) the DJ scratches the record the ring sits on: over `dur_beats` the floor
//! jerks forward, then back, and each jerk flings the opponent round the ring centre with a knock of the scratch's
//! strength (`blade_len`, from `base`). The first rope the opponent hits within `window` s is a rope slam (`dmg` x the
//! impact speed / the shared speed, capped at `cap`); a landed slam adds `grow` strength, up to `max`.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `scratch_floor`.
pub const PORTED: bool = true;

/// Weapon kind `scratch_floor`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct ScratchFloorCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// Beats between two scratches (`max(1, int(round(every / ab_mult)))`: a haste divides it). In a roster entry
    /// 4 x `every` is a finite number.
    pub every: f64,
    /// A scratch's length in beats (the forward jerk, then the back jerk half way).
    pub dur_beats: f64,
    /// The starting strength (`blade_len`: the jerk's knock).
    pub base: f64,
    /// Strength added per landed rope slam.
    pub grow: f64,
    /// The strength never grows past this (`w['max']`).
    pub max: f64,
    /// How much of the fling points outward (`.get('out', 0.4)`).
    pub out: f64,
    /// Seconds after a jerk in which a rope hit is a slam.
    pub window: f64,
    /// A slam's damage at the shared speed.
    pub dmg: f64,
    /// The impact speed's factor is capped here (`.get('cap', 1.6)`).
    pub cap: f64,
}

/// The keys a scratch_floor entry may hold: `kind`, `style` and [`ScratchFloorCfg`]'s fields.
const KEYS: [&str; 11] = [
    "kind",
    "style",
    "every",
    "dur_beats",
    "base",
    "grow",
    "max",
    "out",
    "window",
    "dmg",
    "cap",
];

/// Reads a `scratch_floor` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<ScratchFloorCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a scratch_floor weapon", &KEYS)?;
    Ok(ScratchFloorCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "scratch_floor".to_string(),
        },
        every: e.grid(w, p, "every")?,
        dur_beats: e.f64(w, p, "dur_beats")?,
        base: e.f64(w, p, "base")?,
        grow: e.f64(w, p, "grow")?,
        max: e.f64(w, p, "max")?,
        out: e.f64_or(w, p, "out", 0.4)?,
        window: e.f64(w, p, "window")?,
        dmg: e.f64(w, p, "dmg")?,
        cap: e.f64_or(w, p, "cap", 1.6)?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &ScratchFloorCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): only `base`, the starting strength.
pub fn ball_start(w: &ScratchFloorCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The number a haste divides: `scratch_beat` takes `int(round(every / ab_mult(k)))` beats (a grid in beats, as the
/// line's).
pub(crate) fn main16(w: &ScratchFloorCfg) -> Option<f64> {
    Some(w.every)
}

#[cfg(test)]
mod tests {
    use crate::roster::{WeaponCfg, parse_roster};

    fn turntables(weapon: &str) -> String {
        format!(
            r#"{{"format": 1, "fighters": [{{"name": "TURNTABLES", "color": [190, 245, 60], "dark": [96, 140, 20],
            "corner": "red", "view": [20, 22, 0], "pitch": 45, "weapon": {weapon},
            "abilities": {{"unlock1": {{"name": "SCRATCH", "kind": "haste", "mult": 2.0, "dur_beats": 8, "every_bars": 6}},
            "unlock2": {{"name": "REWIND", "kind": "rewind", "window": 2.0, "zip_beats": 1, "min_undo": 5.0, "every_bars": 6}},
            "low": {{"name": "BASS DROP", "kind": "slam", "tele": 2, "radius": 1.35, "dmg": 11.0, "knock": 1.6}}}},
            "pp": "TURNTABLES", "register": [50, 74], "idiom": "scratch"}}]}}"#
        )
    }

    const KIT: &str = r#"{"kind": "scratch_floor", "style": "record", "every": 4, "dur_beats": 1, "base": 1.3,
        "grow": 0.06, "max": 2.2, "out": 0.4, "window": 0.8, "dmg": 1.6, "cap": 1.6}"#;

    #[test]
    fn the_kit_entry_reads_with_pythons_defaults() {
        let roster = parse_roster(&turntables(KIT)).expect("the kit's TURNTABLES");
        let WeaponCfg::ScratchFloor(w) = &roster[0].weapon else {
            panic!("not a scratch_floor: {:?}", roster[0].weapon)
        };
        assert_eq!(
            (w.style.as_str(), w.every, w.dur_beats, w.base),
            ("record", 4.0, 1.0, 1.3)
        );
        assert_eq!(
            (w.grow, w.max, w.out, w.window, w.dmg, w.cap),
            (0.06, 2.2, 0.4, 0.8, 1.6, 1.6)
        );
        assert_eq!(roster[0].weapon.ball_start().base, 1.3);
        let bare = KIT.replace(r#", "out": 0.4"#, "").replace(r#", "cap": 1.6"#, "");
        let roster = parse_roster(&turntables(&bare)).expect("defaults");
        let WeaponCfg::ScratchFloor(w) = &roster[0].weapon else {
            panic!()
        };
        assert_eq!((w.out, w.cap), (0.4, 1.6));
    }

    #[test]
    fn what_the_kind_cannot_run_is_refused_by_name() {
        for (from, to, field) in [
            (r#""every": 4"#, r#""every": 1e308"#, "weapon.every"),
            (r#""dmg": 1.6"#, r#""dmg": "four""#, "weapon.dmg"),
            (r#""cap": 1.6"#, r#""cap": 1.6, "pull": 1.0"#, "weapon.pull"),
            (r#""window": 0.8, "#, "", "weapon.window"),
        ] {
            assert!(KIT.contains(from), "{from}");
            let err = parse_roster(&turntables(&KIT.replace(from, to))).expect_err(field);
            assert_eq!(
                (err.fighter.as_str(), err.field.as_str()),
                ("TURNTABLES", field),
                "{err}"
            );
        }
    }
}
