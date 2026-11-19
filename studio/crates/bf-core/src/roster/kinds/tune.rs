//! Weapon kind `tune`, OBOE's TUNING A: its roster settings. `fight.py` 2425 to 2525 (`tune_setup`, `_detune`,
//! `tune_tick`, `tune_step`), and the ability `endless` (`trigger` 2884 to 2888).
//!
//! Every `every` beats the oboe sounds its A as a narrow beam: the aim locks `lock16` 16ths before the shot and the
//! beam fires `off16` 16ths after the bar line, `len` long and `width` wide, live for `hold` seconds. A landed beam
//! puts the enemy out of tune for `dur_beats` beats at the oboe's depth (`depth0`, plus `grow` per landed beam, up
//! to `depth_max`): its hits land `weak` x depth weaker, and each note of the oboe's `sting16` grid stings it for
//! `sting` x depth.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `tune`. While false the roster reads it as `WeaponCfg::Unsupported("tune")`,
/// which the engine refuses in words.
pub const PORTED: bool = true;

/// Weapon kind `tune`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct TuneCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// Beats between two shots (`int(round(4 * every))` 16ths, not 0).
    pub every: f64,
    /// The shot falls `int(off16)` 16ths after the grid's line (`w.get('off16', 0)`).
    pub off16: f64,
    /// The aim locks `int(lock16)` 16ths before the shot (`w.get('lock16', 1)`; 0 or less: no lock ahead).
    pub lock16: f64,
    /// How far the lock leads the enemy's motion (`w.get('lead', 0.0)`).
    pub lead: f64,
    /// The beam's length beyond the ball's surface.
    pub len: f64,
    /// The beam's width.
    pub width: f64,
    /// Seconds a beam stays live.
    pub hold: f64,
    /// Damage of a beam.
    pub dmg: f64,
    /// Knockback of a beam.
    pub knock: f64,
    /// Beats the enemy stays out of tune.
    pub dur_beats: f64,
    /// The depth of the first detune.
    pub depth0: f64,
    /// Depth added per landed beam.
    pub grow: f64,
    /// The depth never grows past this (`w.get('depth_max', 1.0)`).
    pub depth_max: f64,
    /// How much weaker a detuned enemy's hits land, per unit of depth.
    pub weak: f64,
    /// Damage of a sting, per unit of depth.
    pub sting: f64,
    /// The stings fall every `int(sting16)` 16ths (`w.get('sting16', 2)`, not 0).
    pub sting16: f64,
    /// Radians a second the bell turns to the enemy between shots (`w.get('track', 5.0)`).
    pub track: f64,
}

/// The keys a tune entry may hold: `kind`, `style` and [`TuneCfg`]'s fields.
const TUNE_KEYS: [&str; 19] = [
    "kind",
    "style",
    "every",
    "off16",
    "lock16",
    "lead",
    "len",
    "width",
    "hold",
    "dmg",
    "knock",
    "dur_beats",
    "depth0",
    "grow",
    "depth_max",
    "weak",
    "sting",
    "sting16",
    "track",
];

/// A number `fight.py` takes `int()` of: it must be finite (`int(inf)` and `int(nan)` raise).
fn whole(e: &Entry, key: &str, x: f64) -> Result<f64, RosterError> {
    if !x.is_finite() {
        return Err(e.err(&format!("weapon.{key}"), format!("{x} is not a number of 16th notes")));
    }
    Ok(x)
}

/// Reads a `tune` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<TuneCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a tune weapon", &TUNE_KEYS)?;
    let cfg = TuneCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "tune".to_string(),
        },
        every: e.grid(w, p, "every")?,
        off16: e.f64_or(w, p, "off16", 0.0)?,
        lock16: e.f64_or(w, p, "lock16", 1.0)?,
        lead: e.f64_or(w, p, "lead", 0.0)?,
        len: e.f64(w, p, "len")?,
        width: e.f64(w, p, "width")?,
        hold: e.f64(w, p, "hold")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
        dur_beats: e.f64(w, p, "dur_beats")?,
        depth0: e.f64(w, p, "depth0")?,
        grow: e.f64(w, p, "grow")?,
        depth_max: e.f64_or(w, p, "depth_max", 1.0)?,
        weak: e.f64(w, p, "weak")?,
        sting: e.f64(w, p, "sting")?,
        sting16: e.f64_or(w, p, "sting16", 2.0)?,
        track: e.f64_or(w, p, "track", 5.0)?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): the grids `tune_tick`
/// divides by. Python raises `ZeroDivisionError` on the first 16th note of the fight for a grid of 0
/// (`% main16`, `% int(sting16)`), and `int()` raises for a number that is not finite.
pub(crate) fn check(e: &Entry, w: &TuneCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    if crate::py::round_half_even(4.0 * w.every) == 0.0 {
        return Err(e.err(
            "weapon.every",
            format!(
                "{} beats is less than a 16th note (the beam's grid would be 0)",
                w.every
            ),
        ));
    }
    whole(e, "off16", w.off16)?;
    whole(e, "lock16", w.lock16)?;
    whole(e, "sting16", w.sting16)?;
    if (w.sting16 as i64) == 0 {
        return Err(e.err(
            "weapon.sting16",
            format!("{} is no grid (the stings fall every that many 16th notes)", w.sting16),
        ));
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`): a tune entry
/// holds none of them, so each is at its Python default.
pub fn ball_start(_w: &TuneCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste, when a haste divides it: no haste divides the tune's grid
/// (`tune_tick` takes `int(round(4 * every))` itself), so `None`.
pub(crate) fn main16(_w: &TuneCfg) -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    //! What Python does with each refused entry is printed by
    //! `tools/studio_export/unit_checks/tune_unit_vectors.py` (its last section).
    use serde_json::{Value as Json, json};

    use super::*;
    use crate::roster::{AbilityKind, FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn oboe() -> Json {
        let doc: Json = serde_json::from_str(ROSTER).expect("the roster is JSON");
        let fighters = doc["fighters"].as_array().expect("fighters");
        fighters
            .iter()
            .find(|f| f["name"] == "OBOE")
            .expect("OBOE is in the roster")
            .clone()
    }

    /// OBOE's entry with one edit, read.
    fn edited(edit: impl Fn(&mut Json)) -> Result<FighterCfg, RosterError> {
        let mut raw = oboe();
        edit(&mut raw);
        FighterCfg::from_json(&raw)
    }

    /// OBOE's entry without `key` of the table at `path` (`weapon`, or `abilities` and a slot).
    fn without(path: &[&str], key: &str) -> Result<FighterCfg, RosterError> {
        edited(|raw| {
            let mut at = &mut *raw;
            for p in path {
                at = &mut at[*p];
            }
            at.as_object_mut().expect("a table").remove(key);
        })
    }

    #[track_caller]
    fn refused(got: Result<FighterCfg, RosterError>, field: &str, says: &str) {
        let e = got.expect_err("refused");
        assert_eq!((e.fighter.as_str(), e.field.as_str()), ("OBOE", field), "{e}");
        assert!(e.message.contains(says), "{e}");
    }

    #[test]
    fn oboe_is_a_tuner_with_its_roster_numbers() {
        let roster = parse_roster(ROSTER).expect("the roster loads");
        let f = roster.iter().find(|f| f.name == "OBOE").expect("OBOE");
        let WeaponCfg::Tune(w) = &f.weapon else {
            panic!("{:?}", f.weapon)
        };
        let want = TuneCfg {
            style: "a_beam".into(),
            every: 4.0,
            off16: 3.0,
            lock16: 3.0,
            lead: 0.0,
            len: 6.0,
            width: 0.12,
            hold: 0.18,
            dmg: 2.0,
            knock: 0.35,
            dur_beats: 8.0,
            depth0: 0.4,
            grow: 0.1,
            depth_max: 1.0,
            weak: 0.6,
            sting: 0.75,
            sting16: 2.0,
            track: 4.0,
        };
        assert_eq!(w, &want);
        assert_eq!(
            (f.weapon.kind(), main16(w), ball_start(w)),
            ("tune", None, BallStart::default())
        );
        assert_eq!(
            (&f.unlock1.kind, &f.unlock2.kind, &f.low.kind),
            (&AbilityKind::Snip, &AbilityKind::Endless, &AbilityKind::Wave)
        );
        let detune = f.low.detune.as_ref().expect("ILL WIND detunes");
        assert_eq!((detune.depth, detune.dur_beats, detune.weak), (1.0, 8.0, 0.45));
        assert_eq!(f.unlock1.get("half"), Some(55.0));
        assert_eq!(f.unlock2.get("track"), Some(1.4));
    }

    /// The defaults are `fight.py`'s `w.get(name, default)`.
    #[test]
    fn the_optional_numbers_take_pythons_defaults() {
        let f = edited(|raw| {
            let w = raw["weapon"].as_object_mut().expect("weapon");
            for key in ["style", "off16", "lock16", "lead", "depth_max", "sting16", "track"] {
                w.remove(key);
            }
        })
        .expect("loads");
        let WeaponCfg::Tune(w) = &f.weapon else {
            panic!("{:?}", f.weapon)
        };
        assert_eq!(
            (
                w.style.as_str(),
                w.off16,
                w.lock16,
                w.lead,
                w.depth_max,
                w.sting16,
                w.track
            ),
            ("tune", 0.0, 1.0, 0.0, 1.0, 2.0, 5.0)
        );
    }

    /// Python raises `ZeroDivisionError` on the fight's first 16th for a grid of 0 (`% main16`, `% int(sting16)`)
    /// and `KeyError` mid-fight for a number it reads as `w[name]`; an unknown key it would ignore in silence.
    #[test]
    fn numbers_the_tune_cannot_run_are_refused_by_name() {
        refused(
            edited(|raw| raw["weapon"]["every"] = json!(0.1)),
            "weapon.every",
            "less than a 16th note",
        );
        refused(
            edited(|raw| raw["weapon"]["sting16"] = json!(0)),
            "weapon.sting16",
            "no grid",
        );
        refused(
            edited(|raw| raw["weapon"]["sting16"] = json!(0.5)),
            "weapon.sting16",
            "no grid",
        );
        for key in [
            "every",
            "len",
            "width",
            "hold",
            "dmg",
            "knock",
            "dur_beats",
            "depth0",
            "grow",
            "weak",
            "sting",
        ] {
            refused(without(&["weapon"], key), &format!("weapon.{key}"), "missing");
        }
        refused(
            edited(|raw| raw["weapon"]["hold"] = json!("long")),
            "weapon.hold",
            "must be a number",
        );
        refused(
            edited(|raw| raw["weapon"]["pull"] = json!(1.0)),
            "weapon.pull",
            "not a setting of a tune weapon",
        );
    }

    /// ILL WIND's `detune` is read as `dd_['depth']`, `dd_['dur_beats']`, `dd_['weak']` when the ring lands
    /// (`KeyError` there for a missing one); an empty table is no detune at all (`if ab.get('detune')`).
    #[test]
    fn a_waves_detune_needs_its_three_numbers() {
        for key in ["depth", "dur_beats", "weak"] {
            refused(
                without(&["abilities", "low", "detune"], key),
                &format!("abilities.low.detune.{key}"),
                "missing",
            );
        }
        refused(
            edited(|raw| raw["abilities"]["low"]["detune"]["depth2"] = json!(1.0)),
            "abilities.low.detune.depth2",
            "not a setting of a wave's detune",
        );
        refused(
            edited(|raw| raw["abilities"]["low"]["detune"] = json!(3)),
            "abilities.low.detune",
            "must be a table",
        );
        let plain = edited(|raw| raw["abilities"]["low"]["detune"] = json!({})).expect("loads");
        assert_eq!((&plain.low.kind, &plain.low.detune), (&AbilityKind::Wave, &None));
    }

    /// The snip and the held note need every number `fight.py` reads as `ab[name]`.
    #[test]
    fn the_snip_and_the_endless_note_need_their_numbers() {
        for key in [
            "ready",
            "reach",
            "half",
            "open16",
            "track",
            "dmg",
            "knock",
            "every_bars",
        ] {
            refused(
                without(&["abilities", "unlock1"], key),
                &format!("abilities.unlock1.{key}"),
                "missing",
            );
        }
        for key in ["len", "width", "track", "dmg", "every_bars"] {
            refused(
                without(&["abilities", "unlock2"], key),
                &format!("abilities.unlock2.{key}"),
                "missing",
            );
        }
        refused(
            without(&["abilities", "unlock2"], "dur_beats"),
            "abilities.unlock2.dur or dur_beats",
            "missing",
        );
    }
}
