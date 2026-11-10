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
    /// Beats between strikes on the 16th grid (`int(round(4 * every))` 16ths; haste divides it).
    pub every: f64,
    /// A basket drops every `int(drop_every)` beats.
    pub drop_every: f64,
    /// At most `int(max_baskets)` baskets; the oldest is taken back for a new one.
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
    /// Beats between thrusts on the 16th grid (`int(round(4 * every))` 16ths; haste divides it).
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
    /// The starting fret count (`.get('frets0', 0)`; an int, it goes into hit events).
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
    /// A kind the engine has not ported yet.
    Unsupported(String),
}

impl AbilityKind {
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
    match doc.get("format").and_then(Json::as_u64) {
        Some(1) => {}
        other => {
            return Err(top(
                "format",
                format!("this app reads roster format 1, found {other:?}"),
            ));
        }
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

    /// `w.get(key, default)`.
    fn f64_or(&self, obj: &Json, prefix: &str, key: &str, default: f64) -> Result<f64, RosterError> {
        match obj.get(key) {
            Some(v) => self.number(v, &join(prefix, key)),
            None => Ok(default),
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
            "striker" => Ok(WeaponCfg::Striker(StrikerCfg {
                style,
                every: self.f64(w, p, "every")?,
                drop_every: self.f64(w, p, "drop_every")?,
                max_baskets: self.f64(w, p, "max_baskets")?,
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
            })),
            "blade" if w.get("mode").and_then(Json::as_str) == Some("thrust") => Ok(WeaponCfg::Blade(BladeCfg {
                style,
                mode: BladeMode::Thrust,
                every: self.f64(w, p, "every")?,
                rest: w.get("rest").map(|v| self.number(v, "weapon.rest")).transpose()?,
                base: self.f64_or(w, p, "base", 0.0)?,
                grow: self.f64(w, p, "grow")?,
                max_len: self.f64(w, p, "max_len")?,
                frets0: match w.get("frets0") {
                    Some(v) => self.int(v, "weapon.frets0")?,
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
            })),
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
        let kind = AbilityKind::from_name(&self.string(ab, &prefix, "kind")?);
        let mut params = BTreeMap::new();
        if let Some(obj) = ab.as_object() {
            for (k, v) in obj {
                if let Some(x) = v.as_f64() {
                    params.insert(k.clone(), x);
                }
            }
        }
        let supported = !matches!(kind, AbilityKind::Unsupported(_));
        let slow = match ab.get("slow") {
            None | Some(Json::Null) => None,
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
        }
        Ok(AbilityCfg {
            name,
            kind,
            params,
            slow,
        })
    }
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
        Ok(FighterCfg {
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
        })
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
            "roster: format: this app reads roster format 1, found Some(2)"
        );
    }
}
