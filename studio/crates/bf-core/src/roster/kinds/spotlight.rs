//! Weapon kind `spotlight`, ALTO SAX's SPOTLIGHT SOLO: its roster settings. `fight.py` 4369 to 4512 (`spot_setup`,
//! `_spot_line`, `spot_tick`, `spot_pos`, `spot_step`, `start_encore`), and the abilities `octave`, `altissimo` and
//! `encore` (`trigger` 2958 to 2965).

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `spotlight`.
pub const PORTED: bool = true;

/// Weapon kind `spotlight`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct SpotlightCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// The weapon's own name (`cfg['weapon'].get('name')`: what a copier announces), when the entry has one.
    pub name: Option<String>,
    /// Beats between two sweeps on the 16th grid (`int(round(4 * every))` 16ths; a haste divides it). In a roster
    /// entry 4 x `every` is a finite number.
    pub every: f64,
    /// 16ths after the bar line a sweep starts (`int(w.get('off16', 0))`).
    pub off16: f64,
    /// 16ths a sweep lasts.
    pub sweep16: f64,
    /// The length of the line the light glides along.
    pub span: f64,
    /// How far ahead of the enemy the line's middle is put: this times its motion to mid-sweep
    /// (`.get('lead', 1.0)`).
    pub lead: f64,
    /// Degrees the line may be turned off square to the enemy's heading, either way (`.get('jitter', 0.0)`).
    pub jitter: f64,
    /// The light's starting radius (`blade_len`; `Ball` reads `.get('base', 0.0)`, `spot_setup` `w['base']`).
    pub base: f64,
    /// Radius added per sweep that burned.
    pub grow: f64,
    /// The radius never grows past this; ENCORE's chase light is this wide (times its `r_k`).
    pub max_r: f64,
    /// The starting damage of a burn (`B.burn`).
    pub dmg: f64,
    /// Damage added per sweep that burned (`.get('dmg_grow', 0.0)`).
    pub dmg_grow: f64,
    /// The burn never grows past this.
    pub dmg_max: f64,
    /// Knockback per burn (`.get('knock', 0.1)`).
    pub knock: f64,
}

/// The keys a spotlight entry may hold: `kind`, `style`, `name` and [`SpotlightCfg`]'s fields.
const SPOTLIGHT_KEYS: [&str; 16] = [
    "kind", "style", "name", "every", "off16", "sweep16", "span", "lead", "jitter", "base", "grow", "max_r", "dmg",
    "dmg_grow", "dmg_max", "knock",
];

/// Reads a `spotlight` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<SpotlightCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a spotlight weapon", &SPOTLIGHT_KEYS)?;
    Ok(SpotlightCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "spotlight".to_string(),
        },
        name: match w.get("name") {
            Some(_) => Some(e.string(w, p, "name")?),
            None => None,
        },
        every: e.grid(w, p, "every")?,
        off16: e.f64_or(w, p, "off16", 0.0)?,
        sweep16: e.f64(w, p, "sweep16")?,
        span: e.f64(w, p, "span")?,
        lead: e.f64_or(w, p, "lead", 1.0)?,
        jitter: e.f64_or(w, p, "jitter", 0.0)?,
        // `spot_setup` reads `w['base']`: a spotlight without one raises in Python
        base: e.f64(w, p, "base")?,
        grow: e.f64(w, p, "grow")?,
        max_r: e.f64(w, p, "max_r")?,
        dmg: e.f64(w, p, "dmg")?,
        dmg_grow: e.f64_or(w, p, "dmg_grow", 0.0)?,
        dmg_max: e.f64(w, p, "dmg_max")?,
        knock: e.f64_or(w, p, "knock", 0.1)?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &SpotlightCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): only `base`, the light's starting radius.
pub fn ball_start(w: &SpotlightCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float): a haste divides it
/// (`spot_tick` 4433 to 4435).
pub(crate) fn main16(w: &SpotlightCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}

#[cfg(test)]
mod tests {
    use crate::roster::{AbilityKind, WeaponCfg, parse_roster};

    fn alto(weapon: &str, low: &str) -> String {
        format!(
            r#"{{"format": 1, "fighters": [{{"name": "ALTO SAX", "color": [200, 80, 240], "dark": [110, 36, 140],
            "corner": "red", "view": [10, 18, 0], "weapon": {weapon},
            "abilities": {{"unlock1": {{"name": "OCTAVE KEY", "kind": "octave", "dur_beats": 8, "every_bars": 5}},
            "unlock2": {{"name": "ALTISSIMO", "kind": "altissimo", "dur_beats": 8, "narrow": 0.5, "hot": 2.5,
            "span_k": 0.75, "every_bars": 6}},
            "low": {low}}},
            "pp": "ALTO SAX", "register": [49, 81], "idiom": "lead"}}]}}"#
        )
    }

    const KIT: &str = r#"{"kind": "spotlight", "style": "spotlight", "name": "SPOTLIGHT SOLO", "every": 4, "off16": 0,
        "sweep16": 6, "span": 2.6, "lead": 1.0, "jitter": 15.0, "base": 0.42, "grow": 0.03, "max_r": 0.75,
        "dmg": 1.3, "dmg_grow": 0.06, "dmg_max": 2.1, "knock": 0.12}"#;

    const ENCORE: &str = r#"{"name": "ENCORE", "kind": "encore", "beats": 8, "speed": 0.8, "dmg": 0.9, "r_k": 1.0,
        "n_phone": 6, "spread": 1.4, "phone_r": 0.45, "phone_dmg": 0.6}"#;

    #[test]
    fn the_kit_entry_reads_with_its_numbers() {
        let roster = parse_roster(&alto(KIT, ENCORE)).expect("the kit's ALTO SAX");
        let WeaponCfg::Spotlight(w) = &roster[0].weapon else {
            panic!("not a spotlight: {:?}", roster[0].weapon)
        };
        assert_eq!(
            (w.style.as_str(), w.name.as_deref()),
            ("spotlight", Some("SPOTLIGHT SOLO"))
        );
        assert_eq!(
            (w.every, w.off16, w.sweep16, w.span, w.lead, w.jitter),
            (4.0, 0.0, 6.0, 2.6, 1.0, 15.0)
        );
        assert_eq!((w.base, w.grow, w.max_r), (0.42, 0.03, 0.75));
        assert_eq!((w.dmg, w.dmg_grow, w.dmg_max, w.knock), (1.3, 0.06, 2.1, 0.12));
        assert_eq!(roster[0].weapon.ball_start().base, 0.42);
        assert_eq!(roster[0].unlock1.kind, AbilityKind::Octave);
        assert_eq!(roster[0].unlock2.kind, AbilityKind::Altissimo);
        assert_eq!(roster[0].low.kind, AbilityKind::Encore);
        assert_eq!(roster[0].low.get("n_phone"), Some(6.0));
    }

    #[test]
    fn the_options_take_pythons_defaults() {
        let bare = r#"{"kind": "spotlight", "every": 4, "sweep16": 6, "span": 2.6, "base": 0.42, "grow": 0.03,
            "max_r": 0.75, "dmg": 1.3, "dmg_max": 2.1}"#;
        let roster = parse_roster(&alto(bare, ENCORE)).expect("a spotlight with no options");
        let WeaponCfg::Spotlight(w) = &roster[0].weapon else {
            panic!("not a spotlight: {:?}", roster[0].weapon)
        };
        assert_eq!((w.style.as_str(), w.name.as_deref()), ("spotlight", None));
        assert_eq!(
            (w.off16, w.lead, w.jitter, w.dmg_grow, w.knock),
            (0.0, 1.0, 0.0, 0.0, 0.1)
        );
    }

    #[test]
    fn what_the_kind_cannot_run_is_refused_by_name() {
        for (from, to, field) in [
            (r#""every": 4"#, r#""every": 1e308"#, "weapon.every"),
            (r#""dmg": 1.3"#, r#""dmg": "hot""#, "weapon.dmg"),
            (r#""dmg": 1.3"#, r#""dmg": 1.3, "pull": 1.0"#, "weapon.pull"),
            (r#""grow": 0.03, "#, "", "weapon.grow"),
            (r#""base": 0.42, "#, "", "weapon.base"),
            (r#""sweep16": 6, "#, "", "weapon.sweep16"),
            (r#""span": 2.6, "#, "", "weapon.span"),
            (r#""max_r": 0.75,"#, "", "weapon.max_r"),
            (r#""dmg_max": 2.1, "#, "", "weapon.dmg_max"),
        ] {
            assert!(KIT.contains(from), "{from}");
            let err = parse_roster(&alto(&KIT.replace(from, to), ENCORE)).expect_err(field);
            assert_eq!((err.fighter.as_str(), err.field.as_str()), ("ALTO SAX", field), "{err}");
        }
    }

    #[test]
    fn an_ability_without_a_number_it_reads_is_refused_by_name() {
        for (from, field) in [
            (r#""beats": 8, "#, "abilities.low.beats"),
            (r#""speed": 0.8, "#, "abilities.low.speed"),
            (r#""dmg": 0.9, "#, "abilities.low.dmg"),
            (r#""n_phone": 6, "#, "abilities.low.n_phone"),
            (r#""spread": 1.4, "#, "abilities.low.spread"),
            (r#""phone_r": 0.45, "#, "abilities.low.phone_r"),
            (r#", "phone_dmg": 0.6"#, "abilities.low.phone_dmg"),
        ] {
            assert!(ENCORE.contains(from), "{from}");
            let err = parse_roster(&alto(KIT, &ENCORE.replace(from, ""))).expect_err(field);
            assert_eq!((err.fighter.as_str(), err.field.as_str()), ("ALTO SAX", field), "{err}");
        }
        // `r_k` is `ab.get('r_k', 1.0)`: an ENCORE without it loads
        parse_roster(&alto(KIT, &ENCORE.replace(r#""r_k": 1.0, "#, ""))).expect("no r_k");
        for (from, to, field) in [
            (r#""narrow": 0.5, "#, "", "abilities.unlock2.narrow"),
            (r#""hot": 2.5,"#, "", "abilities.unlock2.hot"),
            (r#""span_k": 0.75, "#, "", "abilities.unlock2.span_k"),
            (
                r#""dur_beats": 8, "every_bars": 5"#,
                r#""every_bars": 5"#,
                "abilities.unlock1.dur or dur_beats",
            ),
        ] {
            let text = alto(KIT, ENCORE);
            assert!(text.contains(from), "{from}");
            let err = parse_roster(&text.replace(from, to)).expect_err(field);
            assert_eq!((err.fighter.as_str(), err.field.as_str()), ("ALTO SAX", field), "{err}");
        }
    }
}
