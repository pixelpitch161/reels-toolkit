//! Weapon kind `tether`, SYNTH's PATCH CABLES: its roster settings. `fight.py` 1572 to 1660 (`_tether`, `tether_beat`,
//! `tether_16`, `tether_step`).
//!
//! Every `every` beats, while fewer cables are plugged in (or flying) than its slots (`n0`, + `grow` per landed plug,
//! up to `n_max`), the synth fires a plug at the enemy (`plug_speed`); a plug within the enemy's radius + `plug_r` jabs
//! it (`plug_dmg`) and plugs a cable in for `life_beats` beats. Every `volt16` 16ths (a haste divides it) a voltage
//! pulse runs down every plugged cable and hits for `volt_dmg` on arrival (`volt_speed`). The leash keeps the enemy
//! within `length` unless it is flung harder than `snap` x the cables: then they all snap.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `tether`.
pub const PORTED: bool = true;

/// Weapon kind `tether`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct TetherCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// Beats between two plugs (`kb % every`: a haste does not divide it). Never 0 in a roster entry.
    pub every: f64,
    /// The starting slots (`float(w.get('n0', 0))`, `Ball.n_vol`).
    pub n0: f64,
    /// Slots added per landed plug.
    pub grow: f64,
    /// The slots never grow past this.
    pub n_max: f64,
    /// A plug's speed.
    pub plug_speed: f64,
    /// A plug lands within the enemy's radius plus this (`.get('plug_r', 0.14)`).
    pub plug_r: f64,
    /// A plug's damage.
    pub plug_dmg: f64,
    /// Beats a plugged cable stays in.
    pub life_beats: f64,
    /// 16ths between two pulses (`max(1, int(round(volt16 / ab_mult)))`: a haste divides it).
    pub volt16: f64,
    /// Units a second a pulse runs down a cable. Never 0 in a roster entry (the travel time divides by it).
    pub volt_speed: f64,
    /// A pulse's damage.
    pub volt_dmg: f64,
    /// The leash's length.
    pub length: f64,
    /// A knock harder than this x the cables snaps them.
    pub snap: f64,
}

/// The keys a tether entry may hold: `kind`, `style` and [`TetherCfg`]'s fields.
const TETHER_KEYS: [&str; 15] = [
    "kind",
    "style",
    "every",
    "n0",
    "grow",
    "n_max",
    "plug_speed",
    "plug_r",
    "plug_dmg",
    "life_beats",
    "volt16",
    "volt_speed",
    "volt_dmg",
    "length",
    "snap",
];

/// A number `fight.py` divides by (`kb % every`, `dist / volt_speed`): Python raises on 0.
fn nonzero(e: &Entry, field: &str, x: f64) -> Result<f64, RosterError> {
    if x == 0.0 {
        return Err(e.err(field, "0 is not allowed here (the cables divide by it)"));
    }
    Ok(x)
}

/// Reads a `tether` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<TetherCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a tether weapon", &TETHER_KEYS)?;
    Ok(TetherCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "tether".to_string(),
        },
        every: nonzero(e, "weapon.every", e.f64(w, p, "every")?)?,
        n0: e.f64_or(w, p, "n0", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        n_max: e.f64(w, p, "n_max")?,
        plug_speed: e.f64(w, p, "plug_speed")?,
        plug_r: e.f64_or(w, p, "plug_r", 0.14)?,
        plug_dmg: e.f64(w, p, "plug_dmg")?,
        life_beats: e.f64(w, p, "life_beats")?,
        volt16: e.f64(w, p, "volt16")?,
        volt_speed: nonzero(e, "weapon.volt_speed", e.f64(w, p, "volt_speed")?)?,
        volt_dmg: e.f64(w, p, "volt_dmg")?,
        length: e.f64(w, p, "length")?,
        snap: e.f64(w, p, "snap")?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &TetherCfg) -> Result<(), RosterError> {
    nonzero(e, "weapon.every", w.every)?;
    nonzero(e, "weapon.volt_speed", w.volt_speed)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): `n0`, the starting slots (`n_vol`).
pub fn ball_start(w: &TetherCfg) -> BallStart {
    BallStart {
        n0: w.n0,
        ..BallStart::default()
    }
}

/// The 16th grid a haste divides: `tether_16` takes `max(1, int(round(volt16 / ab_mult(k))))`.
pub(crate) fn main16(w: &TetherCfg) -> Option<f64> {
    Some(w.volt16)
}

#[cfg(test)]
mod tests {
    use crate::roster::{WeaponCfg, parse_roster};

    fn synth(weapon: &str) -> String {
        format!(
            r#"{{"format": 1, "fighters": [{{"name": "SYNTH", "color": [70, 228, 255], "dark": [20, 120, 150],
            "corner": "blue", "view": [10, 20, 0], "pitch": 28, "weapon": {weapon},
            "abilities": {{"unlock1": {{"name": "LFO", "kind": "haste", "mult": 2.0, "dur": 3.0, "every_bars": 6}},
            "unlock2": {{"name": "SAWTOOTH", "kind": "lash", "beats": 4, "n_min": 2, "len": 2.2, "spread": 24,
            "sweep": 70, "period_beats": 0.5, "lag": 0.15, "dmg": 1.6, "knock": 0.5, "cool": 0.3, "every_bars": 5}},
            "low": {{"name": "WHITE NOISE", "kind": "wave", "speed": 4.5, "radius": 9.0, "dmg": 8.0, "knock": 1.5}}}},
            "pp": "SYNTH", "register": [57, 81], "idiom": "arp"}}]}}"#
        )
    }

    const KIT: &str = r#"{"kind": "tether", "style": "cables", "every": 2, "n0": 1, "grow": 0.5, "n_max": 4,
        "plug_speed": 6.0, "plug_r": 0.14, "plug_dmg": 1.0, "life_beats": 8, "volt16": 2, "volt_speed": 9.0,
        "volt_dmg": 0.17, "length": 2.4, "snap": 0.15}"#;

    #[test]
    fn the_kit_entry_reads_with_pythons_defaults() {
        let roster = parse_roster(&synth(KIT)).expect("the kit's SYNTH");
        let WeaponCfg::Tether(w) = &roster[0].weapon else {
            panic!("not a tether: {:?}", roster[0].weapon)
        };
        assert_eq!(
            (w.style.as_str(), w.every, w.n0, w.grow, w.n_max),
            ("cables", 2.0, 1.0, 0.5, 4.0)
        );
        assert_eq!(
            (w.plug_speed, w.plug_r, w.plug_dmg, w.life_beats),
            (6.0, 0.14, 1.0, 8.0)
        );
        assert_eq!(
            (w.volt16, w.volt_speed, w.volt_dmg, w.length, w.snap),
            (2.0, 9.0, 0.17, 2.4, 0.15)
        );
        assert_eq!(roster[0].weapon.ball_start().n0, 1.0);
        let bare = parse_roster(&synth(
            &KIT.replace(r#""plug_r": 0.14, "#, "").replace(r#""n0": 1, "#, ""),
        ))
        .expect("defaults");
        let WeaponCfg::Tether(w) = &bare[0].weapon else {
            panic!("not a tether")
        };
        assert_eq!((w.plug_r, w.n0), (0.14, 0.0));
    }

    #[test]
    fn what_the_kind_cannot_run_is_refused_by_name() {
        for (from, to, field) in [
            (r#""every": 2"#, r#""every": 0"#, "weapon.every"),
            (r#""volt_speed": 9.0"#, r#""volt_speed": 0.0"#, "weapon.volt_speed"),
            (r#""snap": 0.15"#, r#""snap": "x""#, "weapon.snap"),
            (r#""snap": 0.15"#, r#""snap": 0.15, "pull": 1.0"#, "weapon.pull"),
            (r#""grow": 0.5, "#, "", "weapon.grow"),
        ] {
            assert!(KIT.contains(from), "{from}");
            let err = parse_roster(&synth(&KIT.replace(from, to))).expect_err(field);
            assert_eq!((err.fighter.as_str(), err.field.as_str()), ("SYNTH", field), "{err}");
        }
    }
}
