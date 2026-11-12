//! Weapon kind `echo`, HORN's the echo call: its roster settings. `fight.py` 2196 to 2281 (`echo_setup`, `echo_pos`,
//! `_echo_blast`, `echo_tick`, `echo_step`).

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `echo`.
pub const PORTED: bool = true;

/// `Fight.EMAX`: the echo ghost slots the record holds (`ECHO_G`), and so the most ghosts the look can draw.
pub const EMAX: usize = 4;

/// Weapon kind `echo`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct EchoCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// Beats between two calls on the 16th grid (`max(1, int(round(4 * every)))`; a haste does not divide it, it
    /// brings the echoes back faster). In a roster entry 4 x `every` is a finite number.
    pub every: f64,
    /// The blast's reach beyond the ball's surface at the start (`blade_len`; `Ball` reads `.get('base', 0.0)`).
    pub base: f64,
    /// Units a second a blast ring spreads. Never 0 in a roster entry (Python divides by it).
    pub speed: f64,
    /// The call's damage (an echo j deals `dmg * decay ** j`).
    pub dmg: f64,
    /// The call's knockback (an echo's, `knock * decay ** j`).
    pub knock: f64,
    /// Ghosts at the start (`float(w.get('n0', 1))`; `Ball.__init__`'s `n_vol` reads `.get('n0', 0)`).
    pub n0: Option<f64>,
    /// The ghosts never grow past this. At most [`EMAX`] in a roster entry: the record and the look hold four.
    pub n_max: f64,
    /// Ghosts added per landed blast (`.get('grow', 1.0)`).
    pub grow: f64,
    /// Each repeat's damage and knockback over the one before (`.get('decay', 0.8)`).
    pub decay: f64,
    /// An echo reaches this times as far beyond the surface as the call (`.get('echo_reach', 1.0)`).
    pub echo_reach: f64,
    /// Seconds a ring stays after it reached its full size (`.get('linger', 0.25)`).
    pub linger: f64,
    /// Seconds the ghosts' spacing eases toward a beat over the haste (`.get('ease', 0.2)`). Never 0 in a roster
    /// entry (Python divides by it).
    pub ease: f64,
    /// The reach never grows past this (`.get('max_reach', B.blade_len)`: `None` = the reach as it stands).
    pub max_reach: Option<f64>,
    /// Reach added per landed blast (`.get('reach_grow', 0.0)`).
    pub reach_grow: f64,
}

impl EchoCfg {
    /// `float(w.get('n0', 1))`: the ghosts `echo_setup` starts with.
    pub fn ghosts0(&self) -> f64 {
        self.n0.unwrap_or(1.0)
    }
}

/// The keys an echo entry may hold: `kind`, `style` and [`EchoCfg`]'s fields.
const ECHO_KEYS: [&str; 16] = [
    "kind",
    "style",
    "every",
    "base",
    "speed",
    "dmg",
    "knock",
    "n0",
    "n_max",
    "grow",
    "decay",
    "echo_reach",
    "linger",
    "ease",
    "max_reach",
    "reach_grow",
];

/// A number Python divides by (`speed`, `ease`).
fn not_zero(e: &Entry, field: &str, x: f64, what: &str) -> Result<f64, RosterError> {
    if x == 0.0 {
        return Err(e.err(field, format!("0 is not {what} (Python divides by it)")));
    }
    Ok(x)
}

/// `n_max`: at most [`EMAX`] ghosts (the look reads ghost j from the record's four slots; Python raises beyond).
fn n_max_value(e: &Entry, x: f64) -> Result<f64, RosterError> {
    if !(x <= EMAX as f64) {
        return Err(e.err(
            "weapon.n_max",
            format!("{x} ghosts are more than the {EMAX} slots the record and the look hold"),
        ));
    }
    Ok(x)
}

/// Reads an `echo` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<EchoCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "an echo weapon", &ECHO_KEYS)?;
    let opt = |key: &str| -> Result<Option<f64>, RosterError> {
        match w.get(key) {
            Some(_) => Ok(Some(e.f64(w, p, key)?)),
            None => Ok(None),
        }
    };
    Ok(EchoCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "echo".to_string(),
        },
        every: e.grid(w, p, "every")?,
        base: e.f64_or(w, p, "base", 0.0)?,
        speed: not_zero(e, "weapon.speed", e.f64(w, p, "speed")?, "a speed")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
        n0: opt("n0")?,
        n_max: n_max_value(e, e.f64(w, p, "n_max")?)?,
        grow: e.f64_or(w, p, "grow", 1.0)?,
        decay: e.f64_or(w, p, "decay", 0.8)?,
        echo_reach: e.f64_or(w, p, "echo_reach", 1.0)?,
        linger: e.f64_or(w, p, "linger", 0.25)?,
        ease: not_zero(e, "weapon.ease", e.f64_or(w, p, "ease", 0.2)?, "a time to ease over")?,
        max_reach: opt("max_reach")?,
        reach_grow: e.f64_or(w, p, "reach_grow", 0.0)?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &EchoCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    not_zero(e, "weapon.speed", w.speed, "a speed")?;
    not_zero(e, "weapon.ease", w.ease, "a time to ease over")?;
    n_max_value(e, w.n_max)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): `base` (the reach) and `n0` (`n_vol =
/// float(w.get('n0', 0))`, recorded as `NV`).
pub fn ball_start(w: &EchoCfg) -> BallStart {
    BallStart {
        base: w.base,
        n0: w.n0.unwrap_or(0.0),
        ..BallStart::default()
    }
}

/// The number a haste divides: `echo_tick` spaces the repeats `max(1, int(round(4 / ab_mult(k))))` 16ths apart,
/// which raises in Python when the quotient is not finite (the call's own grid is not divided).
pub(crate) fn main16(_w: &EchoCfg) -> Option<f64> {
    Some(4.0)
}

#[cfg(test)]
mod tests {
    use crate::roster::{WeaponCfg, parse_roster};

    fn horn(weapon: &str) -> String {
        format!(
            r#"{{"format": 1, "fighters": [{{"name": "HORN", "color": [200, 112, 64], "dark": [120, 58, 30],
            "corner": "blue", "view": [0, 22, 0], "weapon": {weapon},
            "abilities": {{"unlock1": {{"name": "STOPPED HORN", "kind": "shield", "dur_beats": 6, "every_bars": 6}},
            "unlock2": {{"name": "HORN RIP", "kind": "haste", "mult": 2.0, "dur_beats": 8, "every_bars": 6}},
            "low": {{"name": "FINAL FLICK", "kind": "flick", "wind_beats": 1, "reach": 2.4, "glove_r": 0.4,
            "dmg": 14.0, "boost": 2.6, "rope_t": 0.9, "dmg_rope": 10.0}}}},
            "pp": "HORN", "register": [53, 77], "idiom": "horn_call"}}]}}"#
        )
    }

    const KIT: &str = r#"{"kind": "echo", "style": "echo_call", "every": 1, "base": 0.9, "speed": 7.0, "dmg": 4.3,
        "knock": 0.5, "n0": 0, "n_max": 4, "grow": 1.0, "decay": 0.85, "echo_reach": 0.85}"#;

    #[test]
    fn the_kit_entry_reads_with_pythons_defaults() {
        let roster = parse_roster(&horn(KIT)).expect("the kit's HORN");
        let WeaponCfg::Echo(w) = &roster[0].weapon else {
            panic!("not an echo: {:?}", roster[0].weapon)
        };
        assert_eq!(
            (w.style.as_str(), w.every, w.base, w.speed, w.dmg, w.knock),
            ("echo_call", 1.0, 0.9, 7.0, 4.3, 0.5)
        );
        assert_eq!(
            (w.ghosts0(), w.n_max, w.grow, w.decay, w.echo_reach),
            (0.0, 4.0, 1.0, 0.85, 0.85)
        );
        assert_eq!((w.linger, w.ease, w.max_reach, w.reach_grow), (0.25, 0.2, None, 0.0));
        let start = roster[0].weapon.ball_start();
        assert_eq!((start.base, start.n0), (0.9, 0.0));
        // the options at Python's `.get` defaults: no `n0` starts one ghost (`echo_setup`) and no volley
        let bare = KIT
            .replace(r#""n0": 0, "#, "")
            .replace(r#", "decay": 0.85, "echo_reach": 0.85"#, "")
            .replace(r#""style": "echo_call", "#, "");
        let roster = parse_roster(&horn(&bare)).expect("without the options");
        let WeaponCfg::Echo(w) = &roster[0].weapon else {
            panic!("not an echo")
        };
        assert_eq!((w.style.as_str(), w.ghosts0(), w.decay, w.echo_reach), ("echo", 1.0, 0.8, 1.0));
        assert_eq!(roster[0].weapon.ball_start().n0, 0.0);
        let with = KIT.replace(r#""n0": 0"#, r#""n0": 0, "max_reach": 2.5, "reach_grow": 0.1, "ease": 0.4"#);
        let roster = parse_roster(&horn(&with)).expect("with the options");
        let WeaponCfg::Echo(w) = &roster[0].weapon else {
            panic!("not an echo")
        };
        assert_eq!((w.max_reach, w.reach_grow, w.ease), (Some(2.5), 0.1, 0.4));
    }

    #[test]
    fn what_the_kind_cannot_run_is_refused_by_name() {
        for (from, to, field) in [
            (r#""speed": 7.0"#, r#""speed": 0.0"#, "weapon.speed"),
            (r#""n0": 0"#, r#""n0": 0, "ease": 0"#, "weapon.ease"),
            (r#""n_max": 4"#, r#""n_max": 5"#, "weapon.n_max"),
            (r#""every": 1"#, r#""every": 1e308"#, "weapon.every"),
            (r#""dmg": 4.3"#, r#""dmg": "four""#, "weapon.dmg"),
            (r#""dmg": 4.3"#, r#""dmg": 4.3, "pull": 1.0"#, "weapon.pull"),
            (r#""knock": 0.5, "#, "", "weapon.knock"),
        ] {
            assert!(KIT.contains(from), "{from}");
            let err = parse_roster(&horn(&KIT.replace(from, to))).expect_err(field);
            assert_eq!((err.fighter.as_str(), err.field.as_str()), ("HORN", field), "{err}");
        }
    }
}
