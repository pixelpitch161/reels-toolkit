//! The roster numbers (Task 7): what the fight engine reads about each fighter, from the exported
//! `projects/ballfight/fighters.py` (`bf-content/data/roster.json`, written by `tools/studio_export/roster.py`).
//!
//! Each weapon and ability kind the engine has ported gets typed settings carrying exactly the fields `fight.py`
//! reads for it, with the default `fight.py` gives a field it reads with `.get(name, default)`. A kind not ported yet
//! is [`WeaponCfg::Unsupported`] or [`AbilityKind::Unsupported`] with its kind name, so every fighter loads today;
//! later milestones fill them in. [`FighterCfg::raw`] keeps the whole entry untouched, every number exactly as
//! exported (ints stay ints).

use std::collections::BTreeMap;
use std::fmt;

use serde_json::Value as Json;

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
    /// A kind the engine has not ported yet (a spinning blade, with no `mode`, is `"blade"`).
    Unsupported(String),
}

/// Weapon kind `striker` (`fight.py` `striker_setup`, `striker_beat`, `striker_fire`, `striker_step`,
/// `swat_cobras`, and the start of the fight loop). Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct StrikerCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// Beats between strikes on the 16th grid (`int(round(4 * every))` 16ths; haste divides it). In a roster entry
    /// 4 x `every` is a finite number.
    pub every: f64,
    /// A basket drops every `int(drop_every)` beats; a roster entry must give at least 1.
    pub drop_every: f64,
    /// At most `int(max_baskets)` baskets, at least 1 in a roster entry; the oldest is taken back for a new one.
    pub max_baskets: f64,
    /// Beats a cobra takes to rise out of its basket.
    pub rise: f64,
    /// The starting reach (`blade_len`; `Ball` reads `.get('base', 0.0)`).
    pub base: f64,
    /// Reach added per landed bite.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_reach: f64,
    /// A cobra lunges when the enemy is within reach plus this (`.get('slack', 0.25)`).
    pub slack: f64,
    /// Seconds a lunge takes at the start.
    pub t_strike: f64,
    /// A lunge never gets quicker than this.
    pub t_min: f64,
    /// Seconds taken off a lunge per landed bite (`.get('speed_grow', 0.0)`).
    pub speed_grow: f64,
    /// The fang line's radius.
    pub head_r: f64,
    /// Damage per bite.
    pub dmg: f64,
    /// Knockback per bite.
    pub knock: f64,
    /// Units a second the cobras slither (x 1.6 on the way home).
    pub slither: f64,
}

/// The keys a striker entry may hold: `kind`, `style` and [`StrikerCfg`]'s fields.
const STRIKER_KEYS: [&str; 17] = [
    "kind",
    "style",
    "every",
    "drop_every",
    "max_baskets",
    "rise",
    "base",
    "grow",
    "max_reach",
    "slack",
    "t_strike",
    "t_min",
    "speed_grow",
    "head_r",
    "dmg",
    "knock",
    "slither",
];

/// A blade's mode. Only the thrust lance is ported so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BladeMode {
    /// `mode = 'thrust'`: the lance aims at the enemy and shoots out on the grid.
    Thrust,
}

/// Weapon kind `blade` in thrust mode (`fight.py` `Ball.__init__`, `Fight.__init__`, `start_thrust`,
/// `thrust_step`, the thrust grid in the fight loop, and the hound code that reads an enemy lance). Fields without a
/// default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct BladeCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// The mode.
    pub mode: BladeMode,
    /// Beats between thrusts on the 16th grid (`int(round(4 * every))` 16ths; haste divides it). In a roster entry
    /// 4 x `every` is a finite number.
    pub every: f64,
    /// The closed lance's length beyond the ball, as given. `fight.py` reads it with two defaults: see
    /// [`BladeCfg::rest_idle`] and [`BladeCfg::rest_start`].
    pub rest: Option<f64>,
    /// The starting reach (`blade_len`; `.get('base', 0.0)`).
    pub base: f64,
    /// Reach added per landed thrust.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_len: f64,
    /// The starting fret count (`.get('frets0', 0)`; an int, it goes into hit events). From 0 to [`MAX_FRETS0`] in
    /// a roster entry.
    pub frets0: i64,
    /// Seconds the lance takes to shoot out.
    pub t_out: f64,
    /// Seconds it holds out.
    pub t_hold: f64,
    /// Seconds it takes to pull back.
    pub t_back: f64,
    /// Radians a second the idle lance turns toward the enemy (`.get('aim_rate', 8.0)`).
    pub aim_rate: f64,
    /// Damage per thrust.
    pub dmg: f64,
    /// Knockback per thrust (`.get('knock', 0.8)`).
    pub knock: f64,
    /// Whether the live lance swats enemy projectiles (`.get('parry')`, falsy when absent).
    pub parry: bool,
}

/// The largest `frets0` a roster entry may give. A landed thrust adds one fret, so a count that starts here stays a
/// whole number the engine can hold however long the fight (Python's ints have no end; an `i64` has), and far more
/// frets than this are nothing a picture could draw.
///
/// This limit is the Studio's own, stricter than the kit on purpose: Python accepts any integer here. A roster is
/// Studio-owned data checked when it is loaded, and a `frets0` outside 0 to 1,000,000 is a typing mistake the app
/// names in words. No reference roster comes near it (the TROMBONE starts at 1).
pub const MAX_FRETS0: i64 = 1_000_000;

/// The keys a thrust-blade entry may hold: `kind`, `mode`, `style` and [`BladeCfg`]'s fields.
const BLADE_KEYS: [&str; 16] = [
    "kind", "mode", "style", "every", "rest", "base", "grow", "max_len", "frets0", "t_out", "t_hold", "t_back",
    "aim_rate", "dmg", "knock", "parry",
];

impl BladeCfg {
    /// `thrust_step`'s `w.get('rest', 0.25)`: the closed lance's length while idle and the end of every pull-back.
    pub fn rest_idle(&self) -> f64 {
        self.rest.unwrap_or(0.25)
    }

    /// `Ball.__init__`'s `w.get('rest', 0.0)`: the lance's length before its first substep.
    pub fn rest_start(&self) -> f64 {
        self.rest.unwrap_or(0.0)
    }
}

/// An ability.
#[derive(Clone, Debug, PartialEq)]
pub struct AbilityCfg {
    /// Its name, such as `FLUTTER TONGUE`.
    pub name: String,
    /// Its kind.
    pub kind: AbilityKind,
    /// Every number in the entry (ints as `f64`; all of the roster's are exact), by name: `every_bars`,
    /// `dur_beats`, `mult`, `speed`, `radius`, `dmg`, `knock`, `tele`, ...
    pub params: BTreeMap<String, f64>,
    /// A shockwave's or wave's `slow` (`ab.get('slow')`: the enemy it hits moves at `mult` x speed for `dur` s).
    /// `None` when the entry has none or one Python counts as false (an empty table).
    pub slow: Option<SlowCfg>,
}

impl AbilityCfg {
    /// `params[name]`, when the entry has it.
    pub fn get(&self, name: &str) -> Option<f64> {
        self.params.get(name).copied()
    }
}

/// An ability's `slow` option.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlowCfg {
    /// The speed multiplier while slowed.
    pub mult: f64,
    /// Seconds it lasts.
    pub dur: f64,
}

/// An ability kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AbilityKind {
    /// `haste`: the fighter's grid runs `mult` x faster for `dur` s or `dur_beats` beats.
    Haste,
    /// `dance`: the cobras chase the enemy (SNAKE DANCE).
    Dance,
    /// `wave`: a ring that spreads from the fighter.
    Wave,
    /// `shockwave`: a ring that spreads from the fighter (it may slow).
    Shockwave,
    /// `slam`: a telegraphed strike where the enemy is heading.
    Slam,
    /// A kind the engine has not ported yet, or a ported kind with an option it has not ported, written
    /// `kind+option` (OBOE's `"wave+detune"`).
    Unsupported(String),
}

impl AbilityKind {
    /// The kind's name in `fighters.py` (`ab['kind']`), as an `ability` event carries it. For a kind the engine has
    /// not ported it is the kind alone, without the `+option` an unported option adds.
    pub fn name(&self) -> &str {
        match self {
            AbilityKind::Haste => "haste",
            AbilityKind::Dance => "dance",
            AbilityKind::Wave => "wave",
            AbilityKind::Shockwave => "shockwave",
            AbilityKind::Slam => "slam",
            AbilityKind::Unsupported(kind) => kind.split('+').next().unwrap_or(kind),
        }
    }

    fn from_name(kind: &str) -> AbilityKind {
        match kind {
            "haste" => AbilityKind::Haste,
            "dance" => AbilityKind::Dance,
            "wave" => AbilityKind::Wave,
            "shockwave" => AbilityKind::Shockwave,
            "slam" => AbilityKind::Slam,
            other => AbilityKind::Unsupported(other.to_string()),
        }
    }

    /// The numbers `fight.py` reads as `ab[name]` for this kind (`every_bars` for an unlock is checked separately);
    /// a group of names means one of them (`dur` or `dur_beats`).
    fn required(&self) -> &'static [&'static [&'static str]] {
        match self {
            AbilityKind::Haste => &[&["mult"], &["dur", "dur_beats"]],
            AbilityKind::Dance => &[&["dur", "dur_beats"]],
            AbilityKind::Wave | AbilityKind::Shockwave => &[&["speed"], &["radius"], &["dmg"], &["knock"]],
            AbilityKind::Slam => &[&["tele"], &["radius"], &["dmg"], &["knock"]],
            AbilityKind::Unsupported(_) => &[],
        }
    }

    /// Every key `fight.py` reads from an ability of this kind: `name`, `kind`, `every_bars` (the timer of an
    /// unlock), then the kind's own. An entry holding anything else is not this kind yet (`Unsupported`).
    fn allowed(&self) -> &'static [&'static str] {
        match self {
            AbilityKind::Haste => &["name", "kind", "every_bars", "mult", "dur", "dur_beats"],
            AbilityKind::Dance => &["name", "kind", "every_bars", "dur", "dur_beats"],
            AbilityKind::Wave | AbilityKind::Shockwave => {
                &["name", "kind", "every_bars", "speed", "radius", "dmg", "knock", "slow"]
            }
            AbilityKind::Slam => &["name", "kind", "every_bars", "tele", "radius", "dmg", "knock"],
            AbilityKind::Unsupported(_) => &[],
        }
    }
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

/// Reads one field of one fighter's entry, naming both in its errors.
struct Entry<'a> {
    fighter: &'a str,
}

impl Entry<'_> {
    fn err(&self, field: &str, message: impl Into<String>) -> RosterError {
        RosterError {
            fighter: self.fighter.to_string(),
            field: field.to_string(),
            message: message.into(),
        }
    }

    fn get<'j>(&self, obj: &'j Json, prefix: &str, key: &str) -> Result<&'j Json, RosterError> {
        obj.get(key).ok_or_else(|| self.err(&join(prefix, key), "missing"))
    }

    fn string(&self, obj: &Json, prefix: &str, key: &str) -> Result<String, RosterError> {
        match self.get(obj, prefix, key)? {
            Json::String(s) => Ok(s.clone()),
            _ => Err(self.err(&join(prefix, key), "must be a text")),
        }
    }

    fn number(&self, v: &Json, field: &str) -> Result<f64, RosterError> {
        v.as_f64().ok_or_else(|| self.err(field, "must be a number"))
    }

    fn f64(&self, obj: &Json, prefix: &str, key: &str) -> Result<f64, RosterError> {
        self.number(self.get(obj, prefix, key)?, &join(prefix, key))
    }

    /// A number `fight.py` uses as a count, `int(w[key])`: its whole part must be at least 1. (With a count of 0 the
    /// Python fight raises in its first beat; a negative one means nothing.) The number itself is kept as written.
    fn count(&self, obj: &Json, prefix: &str, key: &str, why: &str) -> Result<f64, RosterError> {
        let x = self.f64(obj, prefix, key)?;
        // `x as i64` truncates toward zero, as `int()` does
        if (x as i64) < 1 {
            return Err(self.err(&join(prefix, key), format!("{x} is less than 1 ({why})")));
        }
        Ok(x)
    }

    /// A weapon's `every`, the beats between two starts on its 16th grid: `fight.py` takes `int(round(4 * every))`,
    /// which raises when 4 x `every` is not a finite number (the engine would carry on with a grid of its own). Any
    /// finite grid runs, as in Python: one that rounds to less than a 16th note is every 16th note.
    fn grid(&self, obj: &Json, prefix: &str, key: &str) -> Result<f64, RosterError> {
        let x = self.f64(obj, prefix, key)?;
        if !(4.0 * x).is_finite() {
            return Err(self.err(
                &join(prefix, key),
                format!("{x:e} beats is too long (4 x {key} must be a finite number)"),
            ));
        }
        Ok(x)
    }

    /// `w.get(key, default)`.
    fn f64_or(&self, obj: &Json, prefix: &str, key: &str, default: f64) -> Result<f64, RosterError> {
        match obj.get(key) {
            Some(v) => self.number(v, &join(prefix, key)),
            None => Ok(default),
        }
    }

    /// Refuses any key of a ported kind's entry that the typed settings do not carry: `fight.py` would read it
    /// (some options, such as `pull` or `guard`, on any weapon) and the engine would silently ignore it.
    fn only(&self, obj: &Json, prefix: &str, what: &str, allowed: &[&str]) -> Result<(), RosterError> {
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

    fn int(&self, v: &Json, field: &str) -> Result<i64, RosterError> {
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
            "striker" => {
                self.only(w, p, "a striker weapon", &STRIKER_KEYS)?;
                Ok(WeaponCfg::Striker(StrikerCfg {
                    style,
                    every: self.grid(w, p, "every")?,
                    drop_every: self.count(w, p, "drop_every", "a basket drops every that many beats")?,
                    max_baskets: self.count(w, p, "max_baskets", "there must be room for a basket")?,
                    rise: self.f64(w, p, "rise")?,
                    base: self.f64_or(w, p, "base", 0.0)?,
                    grow: self.f64(w, p, "grow")?,
                    max_reach: self.f64(w, p, "max_reach")?,
                    slack: self.f64_or(w, p, "slack", 0.25)?,
                    t_strike: self.f64(w, p, "t_strike")?,
                    t_min: self.f64(w, p, "t_min")?,
                    speed_grow: self.f64_or(w, p, "speed_grow", 0.0)?,
                    head_r: self.f64(w, p, "head_r")?,
                    dmg: self.f64(w, p, "dmg")?,
                    knock: self.f64(w, p, "knock")?,
                    slither: self.f64(w, p, "slither")?,
                }))
            }
            "blade" if w.get("mode").and_then(Json::as_str) == Some("thrust") => {
                self.only(w, p, "a thrust blade", &BLADE_KEYS)?;
                Ok(WeaponCfg::Blade(BladeCfg {
                    style,
                    mode: BladeMode::Thrust,
                    every: self.grid(w, p, "every")?,
                    rest: w.get("rest").map(|v| self.number(v, "weapon.rest")).transpose()?,
                    base: self.f64_or(w, p, "base", 0.0)?,
                    grow: self.f64(w, p, "grow")?,
                    max_len: self.f64(w, p, "max_len")?,
                    frets0: match w.get("frets0") {
                        Some(v) => {
                            let n = self.int(v, "weapon.frets0")?;
                            if !(0..=MAX_FRETS0).contains(&n) {
                                return Err(
                                    self.err("weapon.frets0", format!("{n} is not a fret count (0 to {MAX_FRETS0})"))
                                );
                            }
                            n
                        }
                        None => 0,
                    },
                    t_out: self.f64(w, p, "t_out")?,
                    t_hold: self.f64(w, p, "t_hold")?,
                    t_back: self.f64(w, p, "t_back")?,
                    aim_rate: self.f64_or(w, p, "aim_rate", 8.0)?,
                    dmg: self.f64(w, p, "dmg")?,
                    knock: self.f64_or(w, p, "knock", 0.8)?,
                    parry: match w.get("parry") {
                        None | Some(Json::Null) => false,
                        Some(Json::Bool(b)) => *b,
                        Some(_) => return Err(self.err("weapon.parry", "must be true or false")),
                    },
                }))
            }
            _ => Ok(WeaponCfg::Unsupported(kind)),
        }
    }

    fn ability(&self, abilities: &Json, slot: &str) -> Result<AbilityCfg, RosterError> {
        let prefix = format!("abilities.{slot}");
        let ab = self.get(abilities, "abilities", slot)?;
        if !ab.is_object() {
            return Err(self.err(&prefix, "must be a table"));
        }
        let name = self.string(ab, &prefix, "name")?;
        let kind_name = self.string(ab, &prefix, "kind")?;
        let mut kind = AbilityKind::from_name(&kind_name);
        // A ported kind carrying an option the engine has not ported (OBOE's wave has `detune`) is not that kind:
        // it loads as `Unsupported("wave+detune")`, so it is never typed with the option silently dropped.
        if let Some(extra) = unported_option(ab, kind.allowed()) {
            kind = AbilityKind::Unsupported(format!("{kind_name}+{extra}"));
        }
        let mut params = BTreeMap::new();
        if let Some(obj) = ab.as_object() {
            for (k, v) in obj {
                if let Some(x) = v.as_f64() {
                    params.insert(k.clone(), x);
                }
            }
        }
        let supported = !matches!(kind, AbilityKind::Unsupported(_));
        // `fight.py` asks `if ab.get('slow')` (and `if W.get('slow')` when the ring lands): Python's truth test, by
        // which an empty table, like no table at all, is no slow.
        let slow = match ab.get("slow") {
            Some(s) if !py_truthy(s) => None,
            None => None,
            Some(s) => {
                let mult = s.get("mult").and_then(Json::as_f64);
                let dur = s.get("dur").and_then(Json::as_f64);
                match (mult, dur) {
                    (Some(mult), Some(dur)) => Some(SlowCfg { mult, dur }),
                    _ if supported => {
                        return Err(self.err(&join(&prefix, "slow"), "must be a table with mult and dur"));
                    }
                    _ => None,
                }
            }
        };
        if supported {
            for group in kind.required() {
                if !group.iter().any(|k| params.contains_key(*k)) {
                    return Err(self.err(&join(&prefix, &group.join(" or ")), "missing"));
                }
            }
            if slot != "low" && !params.contains_key("every_bars") {
                return Err(self.err(&join(&prefix, "every_bars"), "missing"));
            }
            // A haste divides its fighter's weapon grid, the time of a thrust and the time of a lunge by `mult`
            // (`ab_mult`): with 0 the Python fight raises (a division by zero) on the first 16th note of the haste,
            // where the engine would start a thrust that never ends; below 0 a thrust runs backwards for ever.
            if let (AbilityKind::Haste, Some(&mult)) = (&kind, params.get("mult")) {
                if mult <= 0.0 {
                    return Err(self.err(
                        &join(&prefix, "mult"),
                        format!("{mult} is not above 0 (a haste divides the weapon's grid by it)"),
                    ));
                }
                // So small that its inverse (a thrust's `thr_k = 1 / mult`) is infinite: Python's
                // `int(round(main16 / mult))` then raises `OverflowError` for any grid but 0, where the engine would
                // saturate and run on. The weapon's own grid is checked against it in `FighterCfg::from_json`.
                if !(1.0 / mult).is_finite() {
                    return Err(self.err(
                        &join(&prefix, "mult"),
                        format!(
                            "{mult:e} is too small (a haste divides the weapon's grid by it, and 1 / mult must be a \
                             finite number)"
                        ),
                    ));
                }
            }
        }
        Ok(AbilityCfg {
            name,
            kind,
            params,
            slow,
        })
    }
}

/// Python's truth value of a JSON value (`if x:`): `None`, `False`, zero, `""`, `[]` and `{}` are false.
fn py_truthy(v: &Json) -> bool {
    match v {
        Json::Null => false,
        Json::Bool(b) => *b,
        Json::Number(n) => n.as_f64() != Some(0.0),
        Json::String(s) => !s.is_empty(),
        Json::Array(a) => !a.is_empty(),
        Json::Object(o) => !o.is_empty(),
    }
}

/// The first key of a ported ability kind's entry outside `allowed` (or inside its `slow`, outside `mult` and
/// `dur`), as `key` or `slow.key`. `None` for an unported kind (its `allowed` is empty and nothing is checked).
fn unported_option(ab: &Json, allowed: &[&str]) -> Option<String> {
    if allowed.is_empty() {
        return None;
    }
    let obj = ab.as_object()?;
    if let Some(k) = obj.keys().find(|k| !allowed.contains(&k.as_str())) {
        return Some(k.clone());
    }
    let slow = obj.get("slow").and_then(Json::as_object)?;
    slow.keys()
        .find(|k| !["mult", "dur"].contains(&k.as_str()))
        .map(|k| format!("slow.{k}"))
}

fn join(prefix: &str, key: &str) -> String {
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
        // A haste divides the weapon's 16th grid by its factor (`int(round(main16 / ab_mult(k)))`): Python raises
        // `OverflowError` when the quotient is not a finite number, where the engine would saturate and run on.
        if let Some(main16) = f.weapon.main16() {
            let e = Entry { fighter: &f.name };
            for (slot, ab) in [("unlock1", &f.unlock1), ("unlock2", &f.unlock2), ("low", &f.low)] {
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
        Ok(f)
    }
}

impl WeaponCfg {
    /// The weapon's 16th grid before any haste, `main16 = int(round(4 * w['every']))` (as a float: Python divides it
    /// by the haste factor as one), for the kinds that start on a grid; `None` for a kind not ported yet.
    fn main16(&self) -> Option<f64> {
        let every = match self {
            WeaponCfg::Striker(w) => w.every,
            WeaponCfg::Blade(w) => w.every,
            WeaponCfg::Unsupported(_) => return None,
        };
        Some(crate::py::round_half_even(4.0 * every))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROSTER: &str = include_str!("../../bf-content/data/roster.json");

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
