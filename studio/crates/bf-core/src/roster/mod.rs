//! The roster numbers (Task 7): what the fight engine reads about each fighter, from the exported
//! `projects/ballfight/fighters.py` (`bf-content/data/roster.json`, written by `tools/studio_export/roster.py`).
//!
//! Each weapon and ability kind the engine has ported gets typed settings carrying exactly the fields `fight.py`
//! reads for it, with the default `fight.py` gives a field it reads with `.get(name, default)`. A kind not ported yet
//! is [`WeaponCfg::Unsupported`] or [`AbilityKind::Unsupported`] with its kind name, so every fighter loads today;
//! later milestones fill them in. [`FighterCfg::raw`] keeps the whole entry untouched, every number exactly as
//! exported (ints stay ints).
//!
//! The kinds live one module each: [`striker`] (FLUTE) and [`blade`] (TROMBONE), the abilities in [`abilities`],
//! and every further kind of the kit in `kinds::<kind>` and `abilities::<kind>`, each owned by its fighter's port
//! and read as `Unsupported` until its `PORTED` is true.

use std::fmt;

use serde_json::Value as Json;

pub mod abilities;
pub mod blade;
pub mod kinds;
pub mod striker;

pub use abilities::{AbilityCfg, AbilityKind, AbilitySettings, SlowCfg};
pub use blade::{BladeCfg, BladeMode, MAX_FRETS0};
pub use kinds::{
    anchor::AnchorCfg, combo::ComboCfg, crowd::CrowdCfg, drill::DrillCfg, echo::EchoCfg, grapple::GrappleCfg,
    jump::JumpCfg, lightning::LightningCfg, line::LineCfg, mortar::MortarCfg, rake::RakeCfg, roll::RollCfg,
    roll_charge::RollChargeCfg, spotlight::SpotlightCfg, swallow::SwallowCfg, tiptoe::TiptoeCfg, tune::TuneCfg,
    walk::WalkCfg,
};
pub use striker::StrikerCfg;

/// One fighter's roster entry.
#[derive(Clone, Debug, PartialEq)]
pub struct FighterCfg {
    /// The fighter's name, such as `FLUTE`.
    pub name: String,
    /// Its colour (RGB).
    pub color: [u8; 3],
    /// Its dark shade (RGB).
    pub dark: [u8; 3],
    /// Its weapon.
    pub weapon: WeaponCfg,
    /// The ability it unlocks at level 2.
    pub unlock1: AbilityCfg,
    /// The ability it unlocks at level 3.
    pub unlock2: AbilityCfg,
    /// The ability it fires once when its HP drops low.
    pub low: AbilityCfg,
    /// Its MIDI register, lowest and highest note.
    pub register: (i32, i32),
    /// Its musical idiom (the composer's hint), such as `charmer`.
    pub idiom: String,
    /// The Pixel Pitch voice and model it uses.
    pub pp: String,
    /// The roster entry exactly as exported.
    pub raw: Json,
}

/// A weapon, by kind.
#[derive(Clone, Debug, PartialEq)]
pub enum WeaponCfg {
    /// Kind `striker` (FLUTE's cobras).
    Striker(StrikerCfg),
    /// Kind `blade` with `mode = 'thrust'` (TROMBONE's slide lance).
    Blade(BladeCfg),
    /// Kind `roll` (PIANO's PIANO ROLL), typed by `roster::kinds::roll`.
    Roll(kinds::roll::RollCfg),
    /// Kind `swallow` (TUBA's the bell), typed by `roster::kinds::swallow`.
    Swallow(kinds::swallow::SwallowCfg),
    /// Kind `echo` (HORN's the echo call), typed by `roster::kinds::echo`.
    Echo(kinds::echo::EchoCfg),
    /// Kind `line` (ORGAN's the pipes), typed by `roster::kinds::line`.
    Line(kinds::line::LineCfg),
    /// Kind `rake` (ACOUSTIC's SIX-STRING RAKE), typed by `roster::kinds::rake`.
    Rake(kinds::rake::RakeCfg),
    /// Kind `combo` (TRUMPET's the valve combo), typed by `roster::kinds::combo`.
    Combo(kinds::combo::ComboCfg),
    /// Kind `jump` (CLARINET's TWELFTH JUMP), typed by `roster::kinds::jump`.
    Jump(kinds::jump::JumpCfg),
    /// Kind `tune` (OBOE's TUNING A), typed by `roster::kinds::tune`.
    Tune(kinds::tune::TuneCfg),
    /// Kind `anchor` (BASS CLARINET's ENDPIN ANCHOR), typed by `roster::kinds::anchor`.
    Anchor(kinds::anchor::AnchorCfg),
    /// Kind `mortar` (EUPHONIUM's BELL MORTAR), typed by `roster::kinds::mortar`.
    Mortar(kinds::mortar::MortarCfg),
    /// Kind `tiptoe` (BASSOON's TIPTOE), typed by `roster::kinds::tiptoe`.
    Tiptoe(kinds::tiptoe::TiptoeCfg),
    /// Kind `lightning` (ELECTRIC's LIGHTNING RIFF), typed by `roster::kinds::lightning`.
    Lightning(kinds::lightning::LightningCfg),
    /// Kind `grapple` (CELLO's THE EMBRACE), typed by `roster::kinds::grapple`.
    Grapple(kinds::grapple::GrappleCfg),
    /// Kind `walk` (CONTRABASS's WALKING BASS), typed by `roster::kinds::walk`.
    Walk(kinds::walk::WalkCfg),
    /// Kind `spotlight` (ALTO SAX's SPOTLIGHT SOLO), typed by `roster::kinds::spotlight`.
    Spotlight(kinds::spotlight::SpotlightCfg),
    /// Kind `crowd` (TENOR SAX's CROWD PLEASER), typed by `roster::kinds::crowd`.
    Crowd(kinds::crowd::CrowdCfg),
    /// Kind `drill` (SOUSAPHONE's DRILL SET), typed by `roster::kinds::drill`.
    Drill(kinds::drill::DrillCfg),
    /// Kind `roll_charge` (BASS DRUM's STEAMROLLER), typed by `roster::kinds::roll_charge`.
    RollCharge(kinds::roll_charge::RollChargeCfg),
    /// A kind the engine has not ported yet (a spinning blade, with no `mode`, is `"blade"`).
    Unsupported(String),
}

/// What `Ball.__init__` reads from any weapon (`fight.py` 231 to 258), each with Python's default when the weapon
/// has none: the reach `base`, the fret count `frets0`, the closed lance `rest`, the volley size `n0`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BallStart {
    /// `w.get('base', 0.0)`: the starting reach (`blade_len`).
    pub base: f64,
    /// `w.get('frets0', 0)`: the starting fret count.
    pub frets0: i64,
    /// `w.get('rest', 0.0)`: a thrust lance's length before its first substep (`ext`).
    pub rest: f64,
    /// `float(w.get('n0', 0))`: projectiles per volley at the start (`n_vol`).
    pub n0: f64,
}

/// A roster entry the engine cannot read: the fighter, the field and what is wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RosterError {
    /// The fighter (empty when the file itself is wrong).
    pub fighter: String,
    /// The field, such as `weapon.t_strike`.
    pub field: String,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for RosterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.fighter.is_empty() {
            write!(f, "roster: {}: {}", self.field, self.message)
        } else {
            write!(f, "roster: {}: {}: {}", self.fighter, self.field, self.message)
        }
    }
}

impl std::error::Error for RosterError {}

/// Reads the exported roster (`{"about", "format": 1, "fighters": [...]}`), keeping its order.
pub fn parse_roster(text: &str) -> Result<Vec<FighterCfg>, RosterError> {
    let top = |field: &str, message: String| RosterError {
        fighter: String::new(),
        field: field.to_string(),
        message,
    };
    let doc: Json = serde_json::from_str(text).map_err(|e| top("roster.json", format!("not JSON: {e}")))?;
    match doc.get("format") {
        Some(f) if f.as_u64() == Some(1) => {}
        Some(f) => return Err(top("roster.json", format!("format {f}, this app reads 1"))),
        None => return Err(top("roster.json", "no format, this app reads 1".to_string())),
    }
    let fighters = doc
        .get("fighters")
        .and_then(Json::as_array)
        .ok_or_else(|| top("fighters", "missing, or not a list".to_string()))?;
    fighters.iter().map(FighterCfg::from_json).collect()
}

/// Reads one field of one fighter's entry, naming both in its errors. The kind modules read their entries with it.
pub(crate) struct Entry<'a> {
    fighter: &'a str,
}

impl Entry<'_> {
    /// An error at `field` of this fighter.
    pub(crate) fn err(&self, field: &str, message: impl Into<String>) -> RosterError {
        RosterError {
            fighter: self.fighter.to_string(),
            field: field.to_string(),
            message: message.into(),
        }
    }

    /// `obj[key]`, or the error `missing`.
    pub(crate) fn get<'j>(&self, obj: &'j Json, prefix: &str, key: &str) -> Result<&'j Json, RosterError> {
        obj.get(key).ok_or_else(|| self.err(&join(prefix, key), "missing"))
    }

    /// `obj[key]`, a text.
    pub(crate) fn string(&self, obj: &Json, prefix: &str, key: &str) -> Result<String, RosterError> {
        match self.get(obj, prefix, key)? {
            Json::String(s) => Ok(s.clone()),
            _ => Err(self.err(&join(prefix, key), "must be a text")),
        }
    }

    /// A value that must be a number.
    pub(crate) fn number(&self, v: &Json, field: &str) -> Result<f64, RosterError> {
        v.as_f64().ok_or_else(|| self.err(field, "must be a number"))
    }

    /// `obj[key]`, a number.
    pub(crate) fn f64(&self, obj: &Json, prefix: &str, key: &str) -> Result<f64, RosterError> {
        self.number(self.get(obj, prefix, key)?, &join(prefix, key))
    }

    /// A number `fight.py` uses as a count, `int(w[key])`: its whole part must be at least 1. (With a count of 0 the
    /// Python fight raises in its first beat; a negative one means nothing.) The number itself is kept as written.
    pub(crate) fn count(&self, obj: &Json, prefix: &str, key: &str, why: &str) -> Result<f64, RosterError> {
        let x = self.f64(obj, prefix, key)?;
        self.count_value(prefix, key, x, why)
    }

    /// The check of [`Entry::count`] on a number already read.
    pub(crate) fn count_value(&self, prefix: &str, key: &str, x: f64, why: &str) -> Result<f64, RosterError> {
        // `x as i64` truncates toward zero, as `int()` does
        if (x as i64) < 1 {
            return Err(self.err(&join(prefix, key), format!("{x} is less than 1 ({why})")));
        }
        Ok(x)
    }

    /// A weapon's `every`, the beats between two starts on its 16th grid: `fight.py` takes `int(round(4 * every))`,
    /// which raises when 4 x `every` is not a finite number (the engine would carry on with a grid of its own). Any
    /// finite grid runs, as in Python: one that rounds to less than a 16th note is every 16th note.
    pub(crate) fn grid(&self, obj: &Json, prefix: &str, key: &str) -> Result<f64, RosterError> {
        let x = self.f64(obj, prefix, key)?;
        self.grid_value(prefix, key, x)
    }

    /// The check of [`Entry::grid`] on a number already read.
    pub(crate) fn grid_value(&self, prefix: &str, key: &str, x: f64) -> Result<f64, RosterError> {
        if !(4.0 * x).is_finite() {
            return Err(self.err(
                &join(prefix, key),
                format!("{x:e} beats is too long (4 x {key} must be a finite number)"),
            ));
        }
        Ok(x)
    }

    /// `w.get(key, default)`.
    pub(crate) fn f64_or(&self, obj: &Json, prefix: &str, key: &str, default: f64) -> Result<f64, RosterError> {
        match obj.get(key) {
            Some(v) => self.number(v, &join(prefix, key)),
            None => Ok(default),
        }
    }

    /// Refuses any key of a ported kind's entry that the typed settings do not carry: `fight.py` would read it
    /// (some options, such as `pull` or `guard`, on any weapon) and the engine would silently ignore it.
    pub(crate) fn only(&self, obj: &Json, prefix: &str, what: &str, allowed: &[&str]) -> Result<(), RosterError> {
        let unknown = obj
            .as_object()
            .and_then(|m| m.keys().find(|k| !allowed.contains(&k.as_str())));
        match unknown {
            Some(k) => Err(self.err(
                &join(prefix, k),
                format!("not a setting of {what} (the engine would ignore it)"),
            )),
            None => Ok(()),
        }
    }

    /// A value that must be a whole number.
    pub(crate) fn int(&self, v: &Json, field: &str) -> Result<i64, RosterError> {
        v.as_i64().ok_or_else(|| self.err(field, "must be a whole number"))
    }

    fn rgb(&self, obj: &Json, key: &str) -> Result<[u8; 3], RosterError> {
        let list = self.get(obj, "", key)?.as_array().filter(|l| l.len() == 3);
        let list = list.ok_or_else(|| self.err(key, "must be a list of 3 numbers from 0 to 255"))?;
        let mut out = [0u8; 3];
        for (o, v) in out.iter_mut().zip(list) {
            *o = v
                .as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| self.err(key, "must be a list of 3 numbers from 0 to 255"))?;
        }
        Ok(out)
    }

    fn weapon(&self, w: &Json) -> Result<WeaponCfg, RosterError> {
        let p = "weapon";
        let kind = self.string(w, p, "kind")?;
        let style = match w.get("style") {
            Some(_) => self.string(w, p, "style")?,
            None => kind.clone(),
        };
        match kind.as_str() {
            "striker" => Ok(WeaponCfg::Striker(striker::parse(self, w, style)?)),
            "blade" if w.get("mode").and_then(Json::as_str) == Some("thrust") => {
                Ok(WeaponCfg::Blade(blade::parse(self, w, style)?))
            }
            "roll" if kinds::roll::PORTED => Ok(WeaponCfg::Roll(kinds::roll::parse(self, w)?)),
            "swallow" if kinds::swallow::PORTED => Ok(WeaponCfg::Swallow(kinds::swallow::parse(self, w)?)),
            "echo" if kinds::echo::PORTED => Ok(WeaponCfg::Echo(kinds::echo::parse(self, w)?)),
            "line" if kinds::line::PORTED => Ok(WeaponCfg::Line(kinds::line::parse(self, w)?)),
            "rake" if kinds::rake::PORTED => Ok(WeaponCfg::Rake(kinds::rake::parse(self, w)?)),
            "combo" if kinds::combo::PORTED => Ok(WeaponCfg::Combo(kinds::combo::parse(self, w)?)),
            "jump" if kinds::jump::PORTED => Ok(WeaponCfg::Jump(kinds::jump::parse(self, w)?)),
            "tune" if kinds::tune::PORTED => Ok(WeaponCfg::Tune(kinds::tune::parse(self, w)?)),
            "anchor" if kinds::anchor::PORTED => Ok(WeaponCfg::Anchor(kinds::anchor::parse(self, w)?)),
            "mortar" if kinds::mortar::PORTED => Ok(WeaponCfg::Mortar(kinds::mortar::parse(self, w)?)),
            "tiptoe" if kinds::tiptoe::PORTED => Ok(WeaponCfg::Tiptoe(kinds::tiptoe::parse(self, w)?)),
            "lightning" if kinds::lightning::PORTED => Ok(WeaponCfg::Lightning(kinds::lightning::parse(self, w)?)),
            "grapple" if kinds::grapple::PORTED => Ok(WeaponCfg::Grapple(kinds::grapple::parse(self, w)?)),
            "walk" if kinds::walk::PORTED => Ok(WeaponCfg::Walk(kinds::walk::parse(self, w)?)),
            "spotlight" if kinds::spotlight::PORTED => Ok(WeaponCfg::Spotlight(kinds::spotlight::parse(self, w)?)),
            "crowd" if kinds::crowd::PORTED => Ok(WeaponCfg::Crowd(kinds::crowd::parse(self, w)?)),
            "drill" if kinds::drill::PORTED => Ok(WeaponCfg::Drill(kinds::drill::parse(self, w)?)),
            "roll_charge" if kinds::roll_charge::PORTED => {
                Ok(WeaponCfg::RollCharge(kinds::roll_charge::parse(self, w)?))
            }
            _ => Ok(WeaponCfg::Unsupported(kind)),
        }
    }
}

/// `prefix.key`, or `key` alone without a prefix.
pub(crate) fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    }
}

impl FighterCfg {
    /// One entry of the exported roster.
    pub fn from_json(v: &Json) -> Result<FighterCfg, RosterError> {
        let name = match v.get("name").and_then(Json::as_str) {
            Some(n) => n.to_string(),
            None => {
                return Err(RosterError {
                    fighter: String::new(),
                    field: "fighters[].name".into(),
                    message: "missing, or not a text".into(),
                });
            }
        };
        let e = Entry { fighter: &name };
        let register = e.get(v, "", "register")?.as_array().filter(|r| r.len() == 2);
        let register = register.ok_or_else(|| e.err("register", "must be a list of 2 notes"))?;
        let note = |j: &Json| {
            e.int(j, "register")
                .and_then(|n| i32::try_from(n).map_err(|_| e.err("register", "out of range")))
        };
        let abilities = e.get(v, "", "abilities")?;
        let f = FighterCfg {
            color: e.rgb(v, "color")?,
            dark: e.rgb(v, "dark")?,
            weapon: e.weapon(e.get(v, "", "weapon")?)?,
            unlock1: e.ability(abilities, "unlock1")?,
            unlock2: e.ability(abilities, "unlock2")?,
            low: e.ability(abilities, "low")?,
            register: (note(&register[0])?, note(&register[1])?),
            idiom: e.string(v, "", "idiom")?,
            pp: e.string(v, "", "pp")?,
            raw: v.clone(),
            name,
        };
        // what the entry's fields hold together (the haste against the weapon's grid), and so that every entry this
        // reader gives passes the check the engine makes again
        f.check()?;
        Ok(f)
    }

    /// Whether the fight engine can run this entry's numbers: every check [`parse_roster`] makes on a ported weapon
    /// or ability kind, made again on the typed settings. Every entry the reader gives passes it; it is for an entry
    /// built or changed in code (the fields are public), which the engine checks before a fight
    /// (`bf_sim::Fight::new`) so that no number of a roster entry can make it panic or run for ever. A kind not
    /// ported is not looked at here: the engine refuses it on its own.
    ///
    /// The striker's counts, the grids and `frets0` are the Studio's own load-time limits, as [`MAX_FRETS0`] says:
    /// where these refuse an entry the Python fight raises mid-fight, never finishes, or (for `frets0`) runs on with
    /// integers an `i64` cannot hold.
    pub fn check(&self) -> Result<(), RosterError> {
        let e = Entry { fighter: &self.name };
        match &self.weapon {
            WeaponCfg::Striker(w) => striker::check(&e, w)?,
            WeaponCfg::Blade(w) => blade::check(&e, w)?,
            WeaponCfg::Roll(w) => kinds::roll::check(&e, w)?,
            WeaponCfg::Swallow(w) => kinds::swallow::check(&e, w)?,
            WeaponCfg::Echo(w) => kinds::echo::check(&e, w)?,
            WeaponCfg::Line(w) => kinds::line::check(&e, w)?,
            WeaponCfg::Rake(w) => kinds::rake::check(&e, w)?,
            WeaponCfg::Combo(w) => kinds::combo::check(&e, w)?,
            WeaponCfg::Jump(w) => kinds::jump::check(&e, w)?,
            WeaponCfg::Tune(w) => kinds::tune::check(&e, w)?,
            WeaponCfg::Anchor(w) => kinds::anchor::check(&e, w)?,
            WeaponCfg::Mortar(w) => kinds::mortar::check(&e, w)?,
            WeaponCfg::Tiptoe(w) => kinds::tiptoe::check(&e, w)?,
            WeaponCfg::Lightning(w) => kinds::lightning::check(&e, w)?,
            WeaponCfg::Grapple(w) => kinds::grapple::check(&e, w)?,
            WeaponCfg::Walk(w) => kinds::walk::check(&e, w)?,
            WeaponCfg::Spotlight(w) => kinds::spotlight::check(&e, w)?,
            WeaponCfg::Crowd(w) => kinds::crowd::check(&e, w)?,
            WeaponCfg::Drill(w) => kinds::drill::check(&e, w)?,
            WeaponCfg::RollCharge(w) => kinds::roll_charge::check(&e, w)?,
            WeaponCfg::Unsupported(_) => {}
        }
        let slots = [
            ("unlock1", &self.unlock1),
            ("unlock2", &self.unlock2),
            ("low", &self.low),
        ];
        for (slot, ab) in slots {
            if ab.kind.is_ported() {
                let prefix = format!("abilities.{slot}");
                e.ability_numbers(&prefix, slot, &ab.kind, &ab.params)?;
                abilities::kind_check(&e, &prefix, ab)?;
            }
        }
        // A haste divides the weapon's 16th grid by its factor (`int(round(main16 / ab_mult(k)))`): Python raises
        // `OverflowError` when the quotient is not a finite number, where the engine would saturate and run on.
        if let Some(main16) = self.weapon.main16() {
            for (slot, ab) in slots {
                if let (AbilityKind::Haste, Some(mult)) = (&ab.kind, ab.get("mult"))
                    && !(main16 / mult).is_finite()
                {
                    return Err(e.err(
                        &format!("abilities.{slot}.mult"),
                        format!(
                            "{mult:e} is too small for the weapon's grid ({main16:e} sixteenths / mult must be a finite \
                             number)"
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

impl WeaponCfg {
    /// The weapon's 16th grid before any haste, `main16 = int(round(4 * w['every']))` (as a float: Python divides it
    /// by the haste factor as one), for the kinds that start on a grid; `None` for a kind not ported yet.
    fn main16(&self) -> Option<f64> {
        let every = match self {
            WeaponCfg::Striker(w) => w.every,
            WeaponCfg::Blade(w) => w.every,
            WeaponCfg::Roll(w) => return kinds::roll::main16(w),
            WeaponCfg::Swallow(w) => return kinds::swallow::main16(w),
            WeaponCfg::Echo(w) => return kinds::echo::main16(w),
            WeaponCfg::Line(w) => return kinds::line::main16(w),
            WeaponCfg::Rake(w) => return kinds::rake::main16(w),
            WeaponCfg::Combo(w) => return kinds::combo::main16(w),
            WeaponCfg::Jump(w) => return kinds::jump::main16(w),
            WeaponCfg::Tune(w) => return kinds::tune::main16(w),
            WeaponCfg::Anchor(w) => return kinds::anchor::main16(w),
            WeaponCfg::Mortar(w) => return kinds::mortar::main16(w),
            WeaponCfg::Tiptoe(w) => return kinds::tiptoe::main16(w),
            WeaponCfg::Lightning(w) => return kinds::lightning::main16(w),
            WeaponCfg::Grapple(w) => return kinds::grapple::main16(w),
            WeaponCfg::Walk(w) => return kinds::walk::main16(w),
            WeaponCfg::Spotlight(w) => return kinds::spotlight::main16(w),
            WeaponCfg::Crowd(w) => return kinds::crowd::main16(w),
            WeaponCfg::Drill(w) => return kinds::drill::main16(w),
            WeaponCfg::RollCharge(w) => return kinds::roll_charge::main16(w),
            WeaponCfg::Unsupported(_) => return None,
        };
        Some(crate::py::round_half_even(4.0 * every))
    }

    /// What `Ball.__init__` reads from this weapon (`base`, `frets0`, `rest`, `n0`), with Python's defaults.
    pub fn ball_start(&self) -> BallStart {
        match self {
            WeaponCfg::Striker(w) => BallStart {
                base: w.base,
                ..BallStart::default()
            },
            WeaponCfg::Blade(w) => BallStart {
                base: w.base,
                frets0: w.frets0,
                rest: w.rest_start(),
                n0: 0.0,
            },
            WeaponCfg::Roll(w) => kinds::roll::ball_start(w),
            WeaponCfg::Swallow(w) => kinds::swallow::ball_start(w),
            WeaponCfg::Echo(w) => kinds::echo::ball_start(w),
            WeaponCfg::Line(w) => kinds::line::ball_start(w),
            WeaponCfg::Rake(w) => kinds::rake::ball_start(w),
            WeaponCfg::Combo(w) => kinds::combo::ball_start(w),
            WeaponCfg::Jump(w) => kinds::jump::ball_start(w),
            WeaponCfg::Tune(w) => kinds::tune::ball_start(w),
            WeaponCfg::Anchor(w) => kinds::anchor::ball_start(w),
            WeaponCfg::Mortar(w) => kinds::mortar::ball_start(w),
            WeaponCfg::Tiptoe(w) => kinds::tiptoe::ball_start(w),
            WeaponCfg::Lightning(w) => kinds::lightning::ball_start(w),
            WeaponCfg::Grapple(w) => kinds::grapple::ball_start(w),
            WeaponCfg::Walk(w) => kinds::walk::ball_start(w),
            WeaponCfg::Spotlight(w) => kinds::spotlight::ball_start(w),
            WeaponCfg::Crowd(w) => kinds::crowd::ball_start(w),
            WeaponCfg::Drill(w) => kinds::drill::ball_start(w),
            WeaponCfg::RollCharge(w) => kinds::roll_charge::ball_start(w),
            WeaponCfg::Unsupported(_) => BallStart::default(),
        }
    }

    /// The kind's name in `fighters.py` (`w['kind']`).
    pub fn kind(&self) -> &str {
        match self {
            WeaponCfg::Striker(_) => "striker",
            WeaponCfg::Blade(_) => "blade",
            WeaponCfg::Roll(_) => "roll",
            WeaponCfg::Swallow(_) => "swallow",
            WeaponCfg::Echo(_) => "echo",
            WeaponCfg::Line(_) => "line",
            WeaponCfg::Rake(_) => "rake",
            WeaponCfg::Combo(_) => "combo",
            WeaponCfg::Jump(_) => "jump",
            WeaponCfg::Tune(_) => "tune",
            WeaponCfg::Anchor(_) => "anchor",
            WeaponCfg::Mortar(_) => "mortar",
            WeaponCfg::Tiptoe(_) => "tiptoe",
            WeaponCfg::Lightning(_) => "lightning",
            WeaponCfg::Grapple(_) => "grapple",
            WeaponCfg::Walk(_) => "walk",
            WeaponCfg::Spotlight(_) => "spotlight",
            WeaponCfg::Crowd(_) => "crowd",
            WeaponCfg::Drill(_) => "drill",
            WeaponCfg::RollCharge(_) => "roll_charge",
            WeaponCfg::Unsupported(kind) => kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROSTER: &str = include_str!("../../../bf-content/data/roster.json");

    fn fighter(name: &str) -> FighterCfg {
        parse_roster(ROSTER)
            .expect("the roster loads")
            .into_iter()
            .find(|f| f.name == name)
            .expect("in the roster")
    }

    /// `json` with `key` removed from the weapon (or set to `value`).
    fn weapon_with(name: &str, key: &str, value: Option<Json>) -> Json {
        let mut raw = fighter(name).raw;
        let w = raw["weapon"].as_object_mut().unwrap();
        match value {
            Some(v) => _ = w.insert(key.into(), v),
            None => _ = w.remove(key),
        }
        raw
    }

    #[track_caller]
    fn bits(got: f64, want: f64) {
        assert_eq!(got.to_bits(), want.to_bits(), "{got:?} vs {want:?}");
    }

    #[test]
    fn the_whole_roster_loads_in_order() {
        let r = parse_roster(ROSTER).expect("loads");
        assert_eq!(r.len(), 20);
        assert_eq!(r[0].name, "PIANO");
        assert_eq!(r[2].name, "FLUTE");
        assert_eq!(r[3].name, "TROMBONE");
        let doc: Json = serde_json::from_str(ROSTER).unwrap();
        for (f, raw) in r.iter().zip(doc["fighters"].as_array().unwrap()) {
            assert_eq!(&f.raw, raw, "{} keeps its entry untouched", f.name);
        }
    }

    #[test]
    fn flute_is_a_striker_with_its_roster_numbers() {
        let f = fighter("FLUTE");
        assert_eq!(f.color, [222, 232, 246]);
        assert_eq!(f.dark, [120, 134, 162]);
        assert_eq!(f.register, (72, 93));
        assert_eq!(f.idiom, "charmer");
        assert_eq!(f.pp, "FLUTE");
        let WeaponCfg::Striker(w) = &f.weapon else {
            panic!("{:?}", f.weapon)
        };
        assert_eq!(w.style, "cobra");
        for (got, want) in [
            (w.every, 1.0),
            (w.drop_every, 4.0),
            (w.max_baskets, 2.0),
            (w.rise, 1.0),
            (w.base, 1.4),
            (w.grow, 0.07),
            (w.max_reach, 2.4),
            (w.slack, 0.25),
            (w.t_strike, 0.12),
            (w.t_min, 0.07),
            (w.speed_grow, 0.003),
            (w.head_r, 0.14),
            (w.dmg, 1.08),
            (w.knock, 0.5),
            (w.slither, 2.2),
        ] {
            bits(got, want);
        }
        assert_eq!(f.unlock1.name, "FLUTTER TONGUE");
        assert_eq!(f.unlock1.kind, AbilityKind::Haste);
        assert_eq!(f.unlock1.get("mult"), Some(2.0));
        assert_eq!(f.unlock1.get("dur_beats"), Some(8.0));
        assert_eq!(f.unlock1.get("every_bars"), Some(4.0));
        assert_eq!(f.unlock2.kind, AbilityKind::Dance);
        assert_eq!(f.unlock2.get("every_bars"), Some(5.0));
        assert_eq!(f.low.name, "PICCOLO");
        assert_eq!(f.low.kind, AbilityKind::Wave);
        assert_eq!(f.low.params.len(), 4);
        assert_eq!(f.low.get("radius"), Some(9.0));
        assert_eq!(f.low.slow, None);
    }

    #[test]
    fn trombone_is_a_thrust_lance_with_its_roster_numbers() {
        let f = fighter("TROMBONE");
        assert_eq!(f.color, [120, 240, 184]);
        assert_eq!(f.register, (40, 65));
        let WeaponCfg::Blade(w) = &f.weapon else {
            panic!("{:?}", f.weapon)
        };
        assert_eq!(w.style, "slide");
        assert_eq!(w.mode, BladeMode::Thrust);
        assert_eq!(w.rest, Some(0.3));
        bits(w.rest_idle(), 0.3);
        bits(w.rest_start(), 0.3);
        assert_eq!(w.frets0, 1);
        assert!(w.parry);
        for (got, want) in [
            (w.every, 1.0),
            (w.base, 0.95),
            (w.grow, 0.08),
            (w.max_len, 2.5),
            (w.t_out, 0.08),
            (w.t_hold, 0.06),
            (w.t_back, 0.18),
            (w.aim_rate, 7.0),
            (w.dmg, 3.5),
            (w.knock, 0.9),
        ] {
            bits(got, want);
        }
        assert_eq!(f.unlock1.kind, AbilityKind::Shockwave);
        assert_eq!(f.unlock1.slow, Some(SlowCfg { mult: 0.6, dur: 1.0 }));
        assert_eq!(f.unlock2.kind, AbilityKind::Haste);
        assert_eq!(f.low.kind, AbilityKind::Slam);
        assert_eq!(f.low.get("dmg"), Some(24.0));
        assert_eq!(f.low.get("tele"), Some(2.0));
    }

    #[test]
    fn missing_optional_fields_take_the_fight_py_defaults() {
        let mut raw = fighter("FLUTE").raw;
        for k in ["slack", "speed_grow", "base", "style"] {
            raw["weapon"].as_object_mut().unwrap().remove(k);
        }
        let WeaponCfg::Striker(w) = FighterCfg::from_json(&raw).unwrap().weapon else {
            panic!()
        };
        assert_eq!((w.slack, w.speed_grow, w.base), (0.25, 0.0, 0.0));
        assert_eq!(w.style, "striker");

        let mut raw = fighter("TROMBONE").raw;
        for k in ["rest", "base", "frets0", "aim_rate", "knock", "parry", "style"] {
            raw["weapon"].as_object_mut().unwrap().remove(k);
        }
        let WeaponCfg::Blade(w) = FighterCfg::from_json(&raw).unwrap().weapon else {
            panic!()
        };
        assert_eq!(w.rest, None);
        assert_eq!((w.rest_idle(), w.rest_start()), (0.25, 0.0));
        assert_eq!(
            (w.base, w.frets0, w.aim_rate, w.knock, w.parry),
            (0.0, 0, 8.0, 0.8, false)
        );
        assert_eq!(w.style, "blade");
    }

    #[test]
    fn kinds_not_ported_yet_load_as_unsupported() {
        let piano = fighter("PIANO");
        assert_eq!(piano.weapon, WeaponCfg::Unsupported("roll".into()));
        assert_eq!(piano.unlock1.kind, AbilityKind::Unsupported("sustain".into()));
        assert_eq!(piano.unlock2.kind, AbilityKind::Unsupported("arc".into()));
        assert_eq!(piano.unlock2.get("sweep"), Some(150.0));
        // TUBA's haste is a ported kind even though its swallow weapon is not
        let tuba = fighter("TUBA");
        assert_eq!(tuba.weapon, WeaponCfg::Unsupported("swallow".into()));
        assert_eq!(tuba.unlock1.kind, AbilityKind::Haste);
        // a spinning blade (no mode) is not the thrust lance
        let spin = weapon_with("TROMBONE", "mode", None);
        assert_eq!(
            FighterCfg::from_json(&spin).unwrap().weapon,
            WeaponCfg::Unsupported("blade".into())
        );
    }

    /// The 18 weapon kinds of the kit's other fighters each have their module (`roster::kinds::<kind>`), and while
    /// it is not ported the kind loads as `Unsupported` with its name: a roster entry of every one of them, as the
    /// kit exports it, is no fighter the engine runs.
    #[test]
    fn every_kit_weapon_kind_loads_as_unsupported_until_ported() {
        let ported = [
            kinds::roll::PORTED,
            kinds::swallow::PORTED,
            kinds::echo::PORTED,
            kinds::line::PORTED,
            kinds::rake::PORTED,
            kinds::combo::PORTED,
            kinds::jump::PORTED,
            kinds::tune::PORTED,
            kinds::anchor::PORTED,
            kinds::mortar::PORTED,
            kinds::tiptoe::PORTED,
            kinds::lightning::PORTED,
            kinds::grapple::PORTED,
            kinds::walk::PORTED,
            kinds::spotlight::PORTED,
            kinds::crowd::PORTED,
            kinds::drill::PORTED,
            kinds::roll_charge::PORTED,
        ];
        assert_eq!(ported, [false; 18]);
        let mut seen = Vec::new();
        for f in parse_roster(ROSTER).expect("loads") {
            let kind = f.raw["weapon"]["kind"].as_str().expect("a kind").to_string();
            if ["striker", "blade"].contains(&kind.as_str()) {
                assert_eq!(f.weapon.kind(), kind, "{}", f.name);
                continue;
            }
            assert_eq!(f.weapon, WeaponCfg::Unsupported(kind.clone()), "{}", f.name);
            assert_eq!(f.weapon.kind(), kind);
            assert_eq!(f.weapon.ball_start(), BallStart::default(), "{}", f.name);
            seen.push(kind);
        }
        seen.sort();
        assert_eq!(
            seen,
            [
                "anchor",
                "combo",
                "crowd",
                "drill",
                "echo",
                "grapple",
                "jump",
                "lightning",
                "line",
                "mortar",
                "rake",
                "roll",
                "roll_charge",
                "spotlight",
                "swallow",
                "tiptoe",
                "tune",
                "walk"
            ]
        );
    }

    /// The kit's 36 further ability kinds load as `Unsupported` with their name until their module is ported; the
    /// `shield` (TUBA, HORN, ORGAN) is ported and typed, with `dur` or `dur_beats`.
    #[test]
    fn the_shield_is_typed_and_the_other_kit_ability_kinds_wait_for_their_ports() {
        let roster = parse_roster(ROSTER).expect("loads");
        let mut shields = Vec::new();
        let mut waiting = Vec::new();
        for f in &roster {
            for (ab, slot) in [(&f.unlock1, "unlock1"), (&f.unlock2, "unlock2"), (&f.low, "low")] {
                let kind = f.raw["abilities"][slot]["kind"].as_str().expect("a kind");
                match &ab.kind {
                    AbilityKind::Shield => {
                        assert!(ab.get("dur_beats").is_some(), "{}", f.name);
                        assert_eq!(ab.settings, AbilitySettings::Plain);
                        shields.push(format!("{} {slot}", f.name));
                    }
                    AbilityKind::Unsupported(k) => {
                        assert!(k == kind || k.starts_with(&format!("{kind}+")), "{} {k}", f.name);
                        waiting.push(k.clone());
                    }
                    other => assert_eq!(other.name(), kind, "{}", f.name),
                }
            }
        }
        assert_eq!(shields, ["TUBA unlock2", "HORN unlock1", "ORGAN unlock1"]);
        waiting.sort();
        waiting.dedup();
        // OBOE's wave waits for its `detune` (`wave+detune`); the other 36 are the kit's further kinds
        assert_eq!(waiting.len(), 37, "{waiting:?}");
        assert!(waiting.contains(&"wave+detune".to_string()));
        // a shield needs its time, and holds nothing else
        let mut raw = fighter("TUBA").raw;
        raw["abilities"]["unlock2"].as_object_mut().unwrap().remove("dur_beats");
        assert_eq!(
            FighterCfg::from_json(&raw).unwrap_err().to_string(),
            "roster: TUBA: abilities.unlock2.dur or dur_beats: missing"
        );
        let mut raw = fighter("TUBA").raw;
        raw["abilities"]["unlock2"]["radius"] = Json::from(1.0);
        assert_eq!(
            FighterCfg::from_json(&raw).unwrap().unlock2.kind,
            AbilityKind::Unsupported("shield+radius".into())
        );
    }

    #[test]
    fn a_missing_required_field_is_named() {
        let raw = weapon_with("FLUTE", "t_strike", None);
        assert_eq!(
            FighterCfg::from_json(&raw).unwrap_err().to_string(),
            "roster: FLUTE: weapon.t_strike: missing"
        );
        let raw = weapon_with("TROMBONE", "dmg", Some(Json::String("lots".into())));
        assert_eq!(
            FighterCfg::from_json(&raw).unwrap_err().to_string(),
            "roster: TROMBONE: weapon.dmg: must be a number"
        );
        let mut raw = fighter("FLUTE").raw;
        raw["abilities"]["unlock1"].as_object_mut().unwrap().remove("mult");
        assert_eq!(
            FighterCfg::from_json(&raw).unwrap_err().to_string(),
            "roster: FLUTE: abilities.unlock1.mult: missing"
        );
        assert!(
            parse_roster("{")
                .unwrap_err()
                .to_string()
                .starts_with("roster: roster.json: not JSON")
        );
        assert_eq!(
            parse_roster(r#"{"format": 2, "fighters": []}"#)
                .unwrap_err()
                .to_string(),
            "roster: roster.json: format 2, this app reads 1"
        );
        assert_eq!(
            parse_roster(r#"{"fighters": []}"#).unwrap_err().to_string(),
            "roster: roster.json: no format, this app reads 1"
        );
    }

    /// A striker drops a basket every `int(drop_every)` beats and keeps `int(max_baskets)`: with a whole part
    /// below 1 the Python fight raises in its first beat (a remainder by zero, a pop from an empty list), so such an
    /// entry is refused when the roster is read.
    #[test]
    fn a_striker_needs_a_beat_count_and_room_for_a_basket() {
        let load = |key: &str, value: f64| FighterCfg::from_json(&weapon_with("FLUTE", key, Some(Json::from(value))));
        for (key, value, why) in [
            ("drop_every", 0.0, "a basket drops every that many beats"),
            ("drop_every", 0.5, "a basket drops every that many beats"),
            ("drop_every", -4.0, "a basket drops every that many beats"),
            ("max_baskets", 0.0, "there must be room for a basket"),
            ("max_baskets", 0.99, "there must be room for a basket"),
            ("max_baskets", -1.0, "there must be room for a basket"),
        ] {
            assert_eq!(
                load(key, value).unwrap_err().to_string(),
                format!("roster: FLUTE: weapon.{key}: {value} is less than 1 ({why})")
            );
        }
        // the whole part counts, as Python's int(): 1.9 beats is every beat, 1.5 baskets is one basket
        for (key, value) in [
            ("drop_every", 1.0),
            ("drop_every", 1.9),
            ("max_baskets", 1.5),
            ("max_baskets", 7.0),
        ] {
            let WeaponCfg::Striker(w) = load(key, value).expect("loads").weapon else {
                panic!("a striker")
            };
            // the number is kept as written
            assert_eq!(
                if key == "drop_every" {
                    w.drop_every
                } else {
                    w.max_baskets
                },
                value
            );
        }
        // a whole number in the file is read the same way
        let raw = weapon_with("FLUTE", "max_baskets", Some(Json::from(0)));
        assert_eq!(
            FighterCfg::from_json(&raw).unwrap_err().to_string(),
            "roster: FLUTE: weapon.max_baskets: 0 is less than 1 (there must be room for a basket)"
        );
        // and a missing one is still named as missing
        let raw = weapon_with("FLUTE", "drop_every", None);
        assert_eq!(
            FighterCfg::from_json(&raw).unwrap_err().to_string(),
            "roster: FLUTE: weapon.drop_every: missing"
        );
    }

    /// What the thrust lance cannot run on. A haste factor of 0: Python divides the thrust grid and the thrust's time
    /// by it and raises on the first 16th note of the haste (the engine would start a thrust that never ends);
    /// below 0 a thrust would run backwards. An `every` whose fourfold is not a finite number: Python raises in the
    /// fight's first step. A `frets0` out of range: the count goes up by one per landed thrust in a whole number
    /// that, unlike Python's, has an end. Each is refused when the roster is read, with the fighter and the field.
    #[test]
    fn a_thrust_blade_needs_a_haste_above_zero_a_finite_grid_and_a_fret_count() {
        let err = |raw: &Json| FighterCfg::from_json(raw).unwrap_err().to_string();
        // the haste factor, on whichever fighter and slot the haste sits: the lance's own POWER SLIDE, and the
        // striker's FLUTTER TONGUE, whose strike grid and lunge time it divides the same way
        for (name, slot) in [("TROMBONE", "unlock2"), ("FLUTE", "unlock1")] {
            for mult in [0.0, -2.0, -0.0] {
                let mut raw = fighter(name).raw;
                raw["abilities"][slot]["mult"] = Json::from(mult);
                assert_eq!(
                    err(&raw),
                    format!(
                        "roster: {name}: abilities.{slot}.mult: {mult} is not above 0 (a haste divides the weapon's \
                         grid by it)"
                    )
                );
            }
            for mult in [0.5, 2.0, 1e-9] {
                let mut raw = fighter(name).raw;
                raw["abilities"][slot]["mult"] = Json::from(mult);
                let f = FighterCfg::from_json(&raw).expect("loads");
                let ab = if slot == "unlock1" { &f.unlock1 } else { &f.unlock2 };
                assert_eq!((ab.kind.clone(), ab.get("mult")), (AbilityKind::Haste, Some(mult)));
            }
        }
        // a factor above 0 so small that Python's `int(round(main16 / ab_mult(k)))` raises `OverflowError` (the
        // quotient is infinite) where the engine would saturate and run on: the smallest positive double, whose
        // inverse is not finite either, and 1e-308, whose inverse is finite but whose grid of 4 sixteenths is not
        // (Python on the reference PC: both raise; the 1e-9 above, a small factor Python runs, still loads)
        for (name, slot) in [("TROMBONE", "unlock2"), ("FLUTE", "unlock1")] {
            let mut raw = fighter(name).raw;
            raw["abilities"][slot]["mult"] = Json::from(5e-324);
            assert_eq!(
                err(&raw),
                format!(
                    "roster: {name}: abilities.{slot}.mult: 5e-324 is too small (a haste divides the weapon's grid by \
                     it, and 1 / mult must be a finite number)"
                )
            );
            let mut raw = fighter(name).raw;
            raw["abilities"][slot]["mult"] = Json::from(1e-308);
            assert_eq!(
                err(&raw),
                format!(
                    "roster: {name}: abilities.{slot}.mult: 1e-308 is too small for the weapon's grid (4e0 sixteenths \
                     / mult must be a finite number)"
                )
            );
        }
        // the grid's side of it: a huge but finite grid that a haste of 0.5 takes past the largest double (Python
        // raises), and the same grid under a haste of 0.9, which stays finite (Python runs it)
        let mut raw = weapon_with("TROMBONE", "every", Some(Json::from(4e307)));
        raw["abilities"]["unlock2"]["mult"] = Json::from(0.5);
        assert_eq!(
            err(&raw),
            "roster: TROMBONE: abilities.unlock2.mult: 5e-1 is too small for the weapon's grid (1.6e308 sixteenths / \
             mult must be a finite number)"
        );
        raw["abilities"]["unlock2"]["mult"] = Json::from(0.9);
        assert_eq!(
            FighterCfg::from_json(&raw).expect("loads").unlock2.get("mult"),
            Some(0.9)
        );
        // a haste on a fighter whose weapon is not ported yet: only its own factor is checked
        let mut raw = fighter("TUBA").raw;
        raw["abilities"]["unlock1"]["mult"] = Json::from(1e-308);
        assert_eq!(
            FighterCfg::from_json(&raw).expect("loads").unlock1.get("mult"),
            Some(1e-308)
        );
        raw["abilities"]["unlock1"]["mult"] = Json::from(5e-324);
        assert!(err(&raw).ends_with("1 / mult must be a finite number)"));

        // a whole number in the file is read the same way
        let mut raw = fighter("TROMBONE").raw;
        raw["abilities"]["unlock2"]["mult"] = Json::from(0);
        assert_eq!(
            err(&raw),
            "roster: TROMBONE: abilities.unlock2.mult: 0 is not above 0 (a haste divides the weapon's grid by it)"
        );
        // `mult` on an ability that is no haste is that kind's own business
        let mut raw = fighter("TROMBONE").raw;
        raw["abilities"]["low"]["mult"] = Json::from(0.0);
        assert_eq!(
            FighterCfg::from_json(&raw).expect("loads").low.kind,
            AbilityKind::Unsupported("slam+mult".into())
        );

        // the grid: 4 x every must be a finite number, for the lance and for the cobras
        for name in ["TROMBONE", "FLUTE"] {
            for every in [1e308, -1e308] {
                let raw = weapon_with(name, "every", Some(Json::from(every)));
                assert_eq!(
                    err(&raw),
                    format!(
                        "roster: {name}: weapon.every: {every:e} beats is too long (4 x every must be a finite number)"
                    )
                );
            }
            // any finite grid runs, as in Python: a huge one starts once, a tiny or negative one on every 16th
            for every in [4e307, 0.25, 0.0, -1.0, 2.5] {
                let raw = weapon_with(name, "every", Some(Json::from(every)));
                let got = match FighterCfg::from_json(&raw).expect("loads").weapon {
                    WeaponCfg::Blade(w) => w.every,
                    WeaponCfg::Striker(w) => w.every,
                    other => panic!("{other:?}"),
                };
                assert_eq!(got, every, "kept as written");
            }
        }
        assert_eq!(
            err(&weapon_with("TROMBONE", "every", Some(Json::from(1e308)))),
            "roster: TROMBONE: weapon.every: 1e308 beats is too long (4 x every must be a finite number)"
        );

        // the fret count
        for frets0 in [-1, -20, MAX_FRETS0 + 1, i64::MAX] {
            let raw = weapon_with("TROMBONE", "frets0", Some(Json::from(frets0)));
            assert_eq!(
                err(&raw),
                format!("roster: TROMBONE: weapon.frets0: {frets0} is not a fret count (0 to 1000000)")
            );
        }
        for frets0 in [0, 1, 24, MAX_FRETS0] {
            let raw = weapon_with("TROMBONE", "frets0", Some(Json::from(frets0)));
            let WeaponCfg::Blade(w) = FighterCfg::from_json(&raw).expect("loads").weapon else {
                panic!("a blade")
            };
            assert_eq!(w.frets0, frets0);
        }
        // and a count that is no whole number is still named as such
        let raw = weapon_with("TROMBONE", "frets0", Some(Json::from(1.5)));
        assert_eq!(err(&raw), "roster: TROMBONE: weapon.frets0: must be a whole number");
    }

    #[test]
    fn a_ported_kind_refuses_keys_the_engine_would_ignore() {
        // fight.py reads `pull` and `guard` on any weapon: a striker that gained one must not load silently
        let raw = weapon_with("FLUTE", "pull", Some(Json::from(0.5)));
        assert_eq!(
            FighterCfg::from_json(&raw).unwrap_err().to_string(),
            "roster: FLUTE: weapon.pull: not a setting of a striker weapon (the engine would ignore it)"
        );
        let raw = weapon_with("TROMBONE", "guard", Some(Json::from(0.5)));
        assert_eq!(
            FighterCfg::from_json(&raw).unwrap_err().to_string(),
            "roster: TROMBONE: weapon.guard: not a setting of a thrust blade (the engine would ignore it)"
        );
        // a ported ability kind with an option the engine has not ported is not that kind yet: it loads as
        // Unsupported naming the option, never typed with the option dropped (OBOE's real wave has `detune`)
        let mut raw = fighter("FLUTE").raw;
        raw["abilities"]["low"]["detune"] = serde_json::json!({"w": 0.3, "dur": 2.0});
        let f = FighterCfg::from_json(&raw).unwrap();
        assert_eq!(f.low.kind, AbilityKind::Unsupported("wave+detune".into()));
        let mut raw = fighter("TROMBONE").raw;
        raw["abilities"]["unlock1"]["slow"]["ramp"] = Json::from(1);
        let f = FighterCfg::from_json(&raw).unwrap();
        assert_eq!(f.unlock1.kind, AbilityKind::Unsupported("shockwave+slow.ramp".into()));
        assert_eq!(fighter("OBOE").low.kind, AbilityKind::Unsupported("wave+detune".into()));
        // a kind not ported yet still loads, whatever it holds
        let mut raw = fighter("PIANO").raw;
        raw["weapon"]["pull"] = Json::from(0.5);
        raw["abilities"]["unlock1"]["odd"] = Json::from(true);
        assert!(FighterCfg::from_json(&raw).is_ok());
    }

    /// `fight.py` tests an ability's `slow` for truth (`if ab.get('slow')`, `if W.get('slow')`): an empty table is
    /// no slow at all, and the fight runs (checked in Python: a wave with `slow={}` lands without a `slow` event and
    /// does not make the recording hold `EXT`, `SLW` and `CRV`). A table with only one of its two numbers is true in
    /// Python and raises when the ring lands, so it is refused here.
    #[test]
    fn an_empty_slow_is_no_slow() {
        for empty in [
            serde_json::json!({}),
            Json::Null,
            Json::Bool(false),
            Json::from(0),
            Json::from(0.0),
            serde_json::json!([]),
            Json::from(""),
        ] {
            let mut raw = fighter("TROMBONE").raw;
            raw["abilities"]["unlock1"]["slow"] = empty.clone();
            let f = FighterCfg::from_json(&raw).unwrap_or_else(|e| panic!("slow = {empty}: {e}"));
            assert_eq!(
                (f.unlock1.kind.clone(), f.unlock1.slow),
                (AbilityKind::Shockwave, None),
                "slow = {empty}"
            );
        }
        // a wave may slow too, as in Python (the option is the ring's, not the shockwave's)
        let mut raw = fighter("FLUTE").raw;
        raw["abilities"]["low"]["slow"] = serde_json::json!({"mult": 0.5, "dur": 0.75});
        let f = FighterCfg::from_json(&raw).expect("loads");
        assert_eq!(
            (f.low.kind.clone(), f.low.slow),
            (AbilityKind::Wave, Some(SlowCfg { mult: 0.5, dur: 0.75 }))
        );
        // half a slow, or one that is no table
        for odd in [
            serde_json::json!({"mult": 0.5}),
            serde_json::json!({"dur": 1.0}),
            Json::from(2),
            Json::Bool(true),
        ] {
            let mut raw = fighter("TROMBONE").raw;
            raw["abilities"]["unlock1"]["slow"] = odd.clone();
            assert_eq!(
                FighterCfg::from_json(&raw).unwrap_err().to_string(),
                "roster: TROMBONE: abilities.unlock1.slow: must be a table with mult and dur",
                "slow = {odd}"
            );
        }
    }

    /// An ability kind's name is the one `fighters.py` gives it: what an `ability` event carries.
    #[test]
    fn an_ability_kind_has_its_python_name() {
        let (flute, trombone) = (fighter("FLUTE"), fighter("TROMBONE"));
        let names: Vec<&str> = [
            &flute.unlock1,
            &flute.unlock2,
            &flute.low,
            &trombone.unlock1,
            &trombone.low,
        ]
        .iter()
        .map(|ab| ab.kind.name())
        .collect();
        assert_eq!(names, ["haste", "dance", "wave", "shockwave", "slam"]);
        for f in [&flute, &trombone] {
            for (ab, slot) in [(&f.unlock1, "unlock1"), (&f.unlock2, "unlock2"), (&f.low, "low")] {
                assert_eq!(
                    Some(ab.kind.name()),
                    f.raw["abilities"][slot]["kind"].as_str(),
                    "{}",
                    ab.name
                );
            }
        }
        // a kind not ported, and a ported kind with an option that is not
        assert_eq!(fighter("PIANO").unlock1.kind.name(), "sustain");
        let oboe = fighter("OBOE");
        assert_eq!(oboe.low.kind, AbilityKind::Unsupported("wave+detune".into()));
        assert_eq!(oboe.low.kind.name(), "wave");
        // every ability of the roster: the name is the entry's own kind
        for f in parse_roster(ROSTER).expect("loads") {
            for (ab, slot) in [(&f.unlock1, "unlock1"), (&f.unlock2, "unlock2"), (&f.low, "low")] {
                assert_eq!(
                    Some(ab.kind.name()),
                    f.raw["abilities"][slot]["kind"].as_str(),
                    "{} {}",
                    f.name,
                    ab.name
                );
            }
        }
    }

    /// `FighterCfg::check` makes the reader's checks again on typed settings changed in code: every entry the reader
    /// gives passes, and each change names the fighter and the field as the reader does.
    #[test]
    fn check_refuses_typed_settings_changed_by_hand() {
        for f in parse_roster(ROSTER).expect("loads") {
            assert_eq!(f.check(), Ok(()), "{}", f.name);
        }
        let err = |name: &str, change: &dyn Fn(&mut FighterCfg)| {
            let mut f = fighter(name);
            change(&mut f);
            f.check().expect_err("refused").to_string()
        };
        let striker = |f: &mut FighterCfg| match &mut f.weapon {
            WeaponCfg::Striker(w) => w.clone(),
            other => panic!("{other:?}"),
        };
        let set_striker = |f: &mut FighterCfg, w: StrikerCfg| f.weapon = WeaponCfg::Striker(w);
        assert_eq!(
            err("FLUTE", &|f| {
                let mut w = striker(f);
                w.drop_every = 0.9;
                set_striker(f, w);
            }),
            "roster: FLUTE: weapon.drop_every: 0.9 is less than 1 (a basket drops every that many beats)"
        );
        assert_eq!(
            err("FLUTE", &|f| {
                let mut w = striker(f);
                w.max_baskets = 0.0;
                set_striker(f, w);
            }),
            "roster: FLUTE: weapon.max_baskets: 0 is less than 1 (there must be room for a basket)"
        );
        assert_eq!(
            err("FLUTE", &|f| {
                let mut w = striker(f);
                w.every = f64::MAX;
                set_striker(f, w);
            }),
            "roster: FLUTE: weapon.every: 1.7976931348623157e308 beats is too long (4 x every must be a finite number)"
        );
        let blade = |f: &mut FighterCfg, change: &dyn Fn(&mut BladeCfg)| match &mut f.weapon {
            WeaponCfg::Blade(w) => change(w),
            other => panic!("{other:?}"),
        };
        assert_eq!(
            err("TROMBONE", &|f| blade(f, &|w| w.frets0 = -1)),
            "roster: TROMBONE: weapon.frets0: -1 is not a fret count (0 to 1000000)"
        );
        assert_eq!(
            err("TROMBONE", &|f| blade(f, &|w| w.every = f64::NAN)),
            "roster: TROMBONE: weapon.every: NaN beats is too long (4 x every must be a finite number)"
        );
        assert_eq!(
            err("FLUTE", &|f| _ = f.unlock1.params.remove("every_bars")),
            "roster: FLUTE: abilities.unlock1.every_bars: missing"
        );
        assert_eq!(
            err("FLUTE", &|f| _ = f.unlock1.params.insert("mult".into(), 0.0)),
            "roster: FLUTE: abilities.unlock1.mult: 0 is not above 0 (a haste divides the weapon's grid by it)"
        );
        assert_eq!(
            err("TROMBONE", &|f| _ = f.low.params.remove("tele")),
            "roster: TROMBONE: abilities.low.tele: missing"
        );
        assert_eq!(
            err("FLUTE", &|f| {
                f.low.params.remove("speed");
            }),
            "roster: FLUTE: abilities.low.speed: missing"
        );
        // a ported kind changed into one that reads other numbers
        assert_eq!(
            err("FLUTE", &|f| f.unlock2.kind = AbilityKind::Haste),
            "roster: FLUTE: abilities.unlock2.mult: missing"
        );
        // a kind not ported is not looked at
        let mut f = fighter("FLUTE");
        f.unlock1.kind = AbilityKind::Unsupported("shield".into());
        f.unlock1.params.clear();
        f.weapon = WeaponCfg::Unsupported("roll".into());
        assert_eq!(f.check(), Ok(()));
    }

    #[test]
    fn flute_and_trombone_hold_only_typed_keys() {
        // the real roster loads (all 20), and the two ported fighters pass the unknown-key check
        assert_eq!(parse_roster(ROSTER).expect("loads").len(), 20);
        for name in ["FLUTE", "TROMBONE"] {
            let f = fighter(name);
            assert!(!matches!(f.weapon, WeaponCfg::Unsupported(_)), "{name}");
            for ab in [&f.unlock1, &f.unlock2, &f.low] {
                assert!(!matches!(ab.kind, AbilityKind::Unsupported(_)), "{name} {}", ab.name);
            }
        }
    }
}
