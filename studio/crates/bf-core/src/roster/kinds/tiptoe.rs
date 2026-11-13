//! Weapon kind `tiptoe`, BASSOON's TIPTOE: its roster settings. `fight.py` 3280 to 3429 (`tiptoe_setup`, `_behind`,
//! `_hop_launch`, `_hop_land`, `tiptoe_tick`, `tiptoe_step`, `start_mountain`), and the ability `mountain` (`trigger`
//! 2917 to 2918).

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `tiptoe`.
pub const PORTED: bool = true;

/// Weapon kind `tiptoe`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct TiptoeCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// The weapon's name on the card (`w.get('name')`; the fight never reads it).
    pub name: Option<String>,
    /// Beats between two hops on the 16th grid (`int(round(4 * every))`, a haste divides it, at least one 16th). In
    /// a roster entry 4 x `every` is a finite number.
    pub every: f64,
    /// 16ths a hop is in the air (`int(round(air16 / ab_mult))`, at least 1; MOUNTAIN KING takes `int(air16)`). In a
    /// roster entry its size is at most 2^53.
    pub air16: f64,
    /// The longest hop at the start (`blade_len`).
    pub base: f64,
    /// Hop length added per landed stomp.
    pub grow: f64,
    /// The hop length never grows past this; a follow-up hop reaches this far.
    pub max_reach: f64,
    /// A stomp hits an enemy within this beyond contact, at the start.
    pub stomp: f64,
    /// Stomp radius added per landed stomp.
    pub stomp_grow: f64,
    /// The stomp radius never grows past this.
    pub stomp_max: f64,
    /// A hop aims at a spot this far beyond contact behind the enemy.
    pub gap: f64,
    /// How much of the enemy's motion over the air time the aim leads by (`.get('lead', 1.0)`).
    pub lead: f64,
    /// The aim turns by up to this many degrees either way, seeded (`.get('jitter', 0.0)`).
    pub jitter: f64,
    /// Damage per stomp.
    pub dmg: f64,
    /// A stomp from behind deals this many times `dmg` (the backstab).
    pub back: f64,
    /// "Behind" is within this many degrees of straight behind the enemy.
    pub back_half: f64,
    /// Knockback per stomp.
    pub knock: f64,
    /// No hop while its knock boost is above this (`.get('stagger', 99.0)`).
    pub stagger: f64,
}

/// The keys a tiptoe entry may hold: `kind`, `style`, `name` and [`TiptoeCfg`]'s numbers.
const TIPTOE_KEYS: [&str; 19] = [
    "kind",
    "style",
    "name",
    "every",
    "air16",
    "base",
    "grow",
    "max_reach",
    "stomp",
    "stomp_grow",
    "stomp_max",
    "gap",
    "lead",
    "jitter",
    "dmg",
    "back",
    "back_half",
    "knock",
    "stagger",
];

/// The air time in 16ths: Python takes `int(w['air16'])` and `int(round(w['air16'] / mult))`, whole numbers of any
/// size; the engine's are 64-bit, so the entry keeps to what a float holds exactly.
fn air16_value(e: &Entry, x: f64) -> Result<f64, RosterError> {
    if x.abs() > 9_007_199_254_740_992.0 {
        return Err(e.err("weapon.air16", format!("{x:e} sixteenths is too long (at most 2^53)")));
    }
    Ok(x)
}

/// Reads a `tiptoe` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<TiptoeCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a tiptoe weapon", &TIPTOE_KEYS)?;
    Ok(TiptoeCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "tiptoe".to_string(),
        },
        name: match w.get("name") {
            Some(_) => Some(e.string(w, p, "name")?),
            None => None,
        },
        every: e.grid(w, p, "every")?,
        air16: air16_value(e, e.f64(w, p, "air16")?)?,
        base: e.f64(w, p, "base")?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        stomp: e.f64(w, p, "stomp")?,
        stomp_grow: e.f64(w, p, "stomp_grow")?,
        stomp_max: e.f64(w, p, "stomp_max")?,
        gap: e.f64(w, p, "gap")?,
        lead: e.f64_or(w, p, "lead", 1.0)?,
        jitter: e.f64_or(w, p, "jitter", 0.0)?,
        dmg: e.f64(w, p, "dmg")?,
        back: e.f64(w, p, "back")?,
        back_half: e.f64(w, p, "back_half")?,
        knock: e.f64(w, p, "knock")?,
        stagger: e.f64_or(w, p, "stagger", 99.0)?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &TiptoeCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    air16_value(e, w.air16)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): only `base`, the longest hop at the start
/// (`tiptoe_setup` sets `blade_len` to it again).
pub fn ball_start(w: &TiptoeCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float): `tiptoe_tick` divides it by
/// the haste's factor.
pub(crate) fn main16(w: &TiptoeCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}

#[cfg(test)]
mod tests {
    use crate::roster::{WeaponCfg, parse_roster};

    fn bassoon(weapon: &str) -> String {
        format!(
            r#"{{"format": 1, "fighters": [{{"name": "BASSOON", "color": [30, 130, 76], "dark": [14, 70, 40],
            "corner": "red", "view": [8, 20, 0], "weapon": {weapon},
            "abilities": {{"unlock1": {{"name": "STACCATO", "kind": "haste", "mult": 2.0, "dur_beats": 8,
            "every_bars": 6}},
            "unlock2": {{"name": "BOCAL HOOK", "kind": "hook", "ready": 1.0, "reach": 1.1, "half": 60.0, "open16": 2,
            "track": 6.0, "dmg": 4.0, "knock": 0.4, "every_bars": 4}},
            "low": {{"name": "MOUNTAIN KING", "kind": "mountain", "seq16": [3, 3, 2, 1], "step": 75.0, "gap": 0.1,
            "reach": 3.5, "dmg": 0.4, "knock": 0.3}}}},
            "pp": "BASSOON", "register": [40, 76], "idiom": "staccato"}}]}}"#
        )
    }

    const KIT: &str = r#"{"kind": "tiptoe", "style": "hop", "name": "TIPTOE", "every": 1, "air16": 2, "base": 1.6,
        "grow": 0.04, "max_reach": 3.0, "stomp": 0.2, "stomp_grow": 0.008, "stomp_max": 0.5, "gap": 0.12, "lead": 0.6,
        "jitter": 30.0, "dmg": 1.2, "back": 2.5, "back_half": 55.0, "knock": 0.45, "stagger": 0.45}"#;

    #[test]
    fn the_kit_entry_reads_with_pythons_defaults() {
        let roster = parse_roster(&bassoon(KIT)).expect("the kit's BASSOON");
        let WeaponCfg::Tiptoe(w) = &roster[0].weapon else {
            panic!("not a tiptoe: {:?}", roster[0].weapon)
        };
        assert_eq!((w.style.as_str(), w.name.as_deref()), ("hop", Some("TIPTOE")));
        assert_eq!(
            (w.every, w.air16, w.base, w.grow, w.max_reach),
            (1.0, 2.0, 1.6, 0.04, 3.0)
        );
        assert_eq!(
            (w.stomp, w.stomp_grow, w.stomp_max, w.gap, w.lead, w.jitter),
            (0.2, 0.008, 0.5, 0.12, 0.6, 30.0)
        );
        assert_eq!(
            (w.dmg, w.back, w.back_half, w.knock, w.stagger),
            (1.2, 2.5, 55.0, 0.45, 0.45)
        );
        assert_eq!(roster[0].weapon.ball_start().base, 1.6);
        // the options at Python's `.get` defaults
        let bare = KIT
            .replace(r#""lead": 0.6,"#, "")
            .replace(r#""jitter": 30.0, "#, "")
            .replace(r#", "stagger": 0.45"#, "")
            .replace(r#""style": "hop", "name": "TIPTOE", "#, "");
        let roster = parse_roster(&bassoon(&bare)).expect("without the options");
        let WeaponCfg::Tiptoe(w) = &roster[0].weapon else {
            panic!("not a tiptoe")
        };
        assert_eq!((w.lead, w.jitter, w.stagger), (1.0, 0.0, 99.0));
        assert_eq!((w.style.as_str(), w.name.as_deref()), ("tiptoe", None));
    }

    #[test]
    fn what_the_kind_cannot_run_is_refused_by_name() {
        for (from, to, field) in [
            (r#""every": 1"#, r#""every": 1e308"#, "weapon.every"),
            (r#""air16": 2"#, r#""air16": 1e300"#, "weapon.air16"),
            (r#""dmg": 1.2"#, r#""dmg": "some""#, "weapon.dmg"),
            (r#""dmg": 1.2"#, r#""dmg": 1.2, "pull": 1.0"#, "weapon.pull"),
            (r#""grow": 0.04,"#, "", "weapon.grow"),
            (r#""back_half": 55.0, "#, "", "weapon.back_half"),
        ] {
            assert!(KIT.contains(from), "{from}");
            let err = parse_roster(&bassoon(&KIT.replace(from, to))).expect_err(field);
            assert_eq!((err.fighter.as_str(), err.field.as_str()), ("BASSOON", field), "{err}");
        }
    }

    #[test]
    fn a_mountain_keeps_its_hops_and_a_hook_its_numbers() {
        let roster = parse_roster(&bassoon(KIT)).expect("the kit's BASSOON");
        let f = &roster[0];
        assert_eq!(f.unlock2.kind.name(), "hook");
        assert_eq!(
            (f.unlock2.get("ready"), f.unlock2.get("open16"), f.unlock2.get("track")),
            (Some(1.0), Some(2.0), Some(6.0))
        );
        assert_eq!(f.low.kind.name(), "mountain");
        assert_eq!(crate::roster::abilities::mountain::seq16(&f.low), &[3, 3, 2, 1]);
        for (from, to, field) in [
            (r#""seq16": [3, 3, 2, 1], "#, "", "abilities.low.seq16"),
            (
                r#""seq16": [3, 3, 2, 1]"#,
                r#""seq16": [3, 0.5]"#,
                "abilities.low.seq16",
            ),
            (r#""step": 75.0, "#, "", "abilities.low.step"),
            (r#""ready": 1.0, "#, "", "abilities.unlock2.ready"),
        ] {
            let text = bassoon(KIT);
            assert!(text.contains(from), "{from}");
            let err = parse_roster(&text.replace(from, to)).expect_err(field);
            assert_eq!((err.fighter.as_str(), err.field.as_str()), ("BASSOON", field), "{err}");
        }
    }
}
