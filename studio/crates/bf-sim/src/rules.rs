//! The rules of a fight: `fight.py`'s `Rules` (lines 144 to 164), the same for both fighters.
//!
//! The Python object is a bag of attributes: the engine's constants, the episode's settings, and whatever opt-in rules
//! an episode passes as keywords. Here it is a struct with one field per attribute the ported engine reads.
//! [`Rules::apply`] is Python's `setattr` loop for the keyword overrides (a pair rule from the fight file, a scenario
//! override such as `LV_DMG`), keyed by the Python attribute names, and [`Rules::to_values`] gives the attributes back
//! under those names for a recording's `rules` (`studio/docs/recording-format.md` section 6.1).

use std::collections::BTreeMap;
use std::fmt;

use bf_core::event::Value;
use bf_core::py;

/// Something the engine cannot run: the field, and what is wrong with it, in one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimError {
    /// The field, as a path such as `rules.LV_DMG` or `fighters[0]`.
    pub field: String,
    /// What is wrong, in one line.
    pub message: String,
}

impl SimError {
    pub(crate) fn new(field: impl Into<String>, message: impl Into<String>) -> SimError {
        SimError {
            field: field.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for SimError {}

/// The most steps a fight may take (`(t_max - t_fight) / DT`): a fight of the series takes about 6,500. Rules that
/// ask for more are refused, so a mistyped `DT` cannot make the engine run for hours.
pub const MAX_STEPS: f64 = 1_000_000.0;

/// The rules of one fight. Field names are the Python attribute names in lower case; the comment gives the Python
/// name and the engine's default.
#[derive(Clone, Debug, PartialEq)]
pub struct Rules {
    /// `A` (3.0): half-size of the ring inside the ropes, in ring units.
    pub a: f64,
    /// `R` (0.6): ball radius (both the same size: fairness).
    pub r: f64,
    /// `V` (2.6): the shared ball speed.
    pub v: f64,
    /// `DT` (1/120): the step, 4 substeps per frame.
    pub dt: f64,
    /// `FPS` (30): recorded frames per second.
    pub fps: u32,
    /// `ROPE_GIVE` (5.0): degrees of seeded jitter on every rope bounce.
    pub rope_give: f64,
    /// `HP` (100.0): starting hit points.
    pub hp: f64,
    /// `LOW_HP` (25.0): below this the `low` ability fires, once.
    pub low_hp: f64,
    /// `LV_DMG` ((14.0, 36.0)): damage dealt to reach level 2 and level 3.
    pub lv_dmg: (f64, f64),
    /// `KNOCK_T` (0.38): seconds a knockback speed boost takes to decay.
    pub knock_t: f64,
    /// `LAUNCH_SPREAD` (38.0): degrees either side of the corner-to-centre line at the start, seeded.
    pub launch_spread: f64,
    /// `KEY_R` (0.11): projectile hit radius.
    pub key_r: f64,
    /// `bpm`: the tempo.
    pub bpm: f64,
    /// `beat`: `60.0 / bpm` when the rules are made; an override of `bpm` alone does not change it (as in Python).
    pub beat: f64,
    /// `t_fight`: when the fight starts, in seconds on the video clock.
    pub t_fight: f64,
    /// `t_max` (58.0): the fight stops here if nobody is knocked out.
    pub t_max: f64,
    /// `dmg_scale` (1.0): the shared damage scale.
    pub dmg_scale: f64,
    /// `dmg_side` (absent): the damage each fighter deals is scaled by its entry (left, right).
    pub dmg_side: Option<[f64; 2]>,
    /// `swat_cobras` (absent, which is false): a live thrust lance knocks a risen enemy cobra flat (#41).
    pub swat_cobras: bool,
}

impl Rules {
    /// `Rules(bpm=bpm, t_fight=t_fight)`: the engine's constants at this tempo and start time.
    pub fn new(bpm: f64, t_fight: f64) -> Rules {
        Rules {
            a: 3.0,
            r: 0.6,
            v: 2.6,
            dt: 1.0 / 120.0,
            fps: 30,
            rope_give: 5.0,
            hp: 100.0,
            low_hp: 25.0,
            lv_dmg: (14.0, 36.0),
            knock_t: 0.38,
            launch_spread: 38.0,
            key_r: 0.11,
            bpm,
            beat: 60.0 / bpm,
            t_fight,
            t_max: 58.0,
            dmg_scale: 1.0,
            dmg_side: None,
            swat_cobras: false,
        }
    }

    /// Sets rules by their Python attribute name, as `Rules(**overrides)` does: `swat_cobras = true` from a fight
    /// file's pair rules, `LV_DMG = [2.0, 5.0]` from a scenario, or the whole `rules` of a recording. Each entry sets
    /// exactly that attribute (`bpm` does not move `beat`). A whole number is accepted where a decimal is expected.
    ///
    /// An unknown name, a value of the wrong type, or rules the engine cannot run ([`Rules::check`]) is an error
    /// that names the entry, and then nothing is changed.
    pub fn apply(&mut self, overrides: &BTreeMap<String, Value>) -> Result<(), SimError> {
        let mut next = self.clone();
        for (key, value) in overrides {
            let field = format!("rules.{key}");
            match key.as_str() {
                "A" => next.a = number(value, &field)?,
                "R" => next.r = number(value, &field)?,
                "V" => next.v = number(value, &field)?,
                "DT" => next.dt = number(value, &field)?,
                "FPS" => {
                    next.fps = match value {
                        Value::Int(n) => {
                            u32::try_from(*n).map_err(|_| SimError::new(&field, format!("{n} is not a frame rate")))?
                        }
                        _ => return Err(SimError::new(&field, "must be a whole number")),
                    }
                }
                "ROPE_GIVE" => next.rope_give = number(value, &field)?,
                "HP" => next.hp = number(value, &field)?,
                "LOW_HP" => next.low_hp = number(value, &field)?,
                "LV_DMG" => {
                    let [lv2, lv3] = pair(value, &field)?;
                    next.lv_dmg = (lv2, lv3);
                }
                "KNOCK_T" => next.knock_t = number(value, &field)?,
                "LAUNCH_SPREAD" => next.launch_spread = number(value, &field)?,
                "KEY_R" => next.key_r = number(value, &field)?,
                "bpm" => next.bpm = number(value, &field)?,
                "beat" => next.beat = number(value, &field)?,
                "t_fight" => next.t_fight = number(value, &field)?,
                "t_max" => next.t_max = number(value, &field)?,
                "dmg_scale" => next.dmg_scale = number(value, &field)?,
                "dmg_side" => {
                    next.dmg_side = match value {
                        Value::Null => None,
                        _ => Some(pair(value, &field)?),
                    }
                }
                "swat_cobras" => {
                    next.swat_cobras = match value {
                        Value::Bool(on) => *on,
                        _ => return Err(SimError::new(&field, "must be true or false")),
                    }
                }
                _ => return Err(SimError::new(&field, "the engine has no rule of this name")),
            }
        }
        next.check()?;
        *self = next;
        Ok(())
    }

    /// Whether the engine can run these rules: every number finite; a positive ring, ball, step, beat, knock decay
    /// and frame rate; a rope give and a start time that are not negative; at least one step per recorded frame; and
    /// at most [`MAX_STEPS`] steps. Python has no such check: there such rules raise mid-fight or never finish.
    pub fn check(&self) -> Result<(), SimError> {
        let numbers = [
            ("A", self.a),
            ("R", self.r),
            ("V", self.v),
            ("DT", self.dt),
            ("ROPE_GIVE", self.rope_give),
            ("HP", self.hp),
            ("LOW_HP", self.low_hp),
            ("LV_DMG", self.lv_dmg.0),
            ("LV_DMG", self.lv_dmg.1),
            ("KNOCK_T", self.knock_t),
            ("LAUNCH_SPREAD", self.launch_spread),
            ("KEY_R", self.key_r),
            ("bpm", self.bpm),
            ("beat", self.beat),
            ("t_fight", self.t_fight),
            ("t_max", self.t_max),
            ("dmg_scale", self.dmg_scale),
        ];
        let sides = self.dmg_side.unwrap_or([1.0, 1.0]);
        for (name, x) in numbers.into_iter().chain(sides.map(|x| ("dmg_side", x))) {
            if !x.is_finite() {
                return Err(SimError::new(
                    format!("rules.{name}"),
                    format!("{x} is not a finite number"),
                ));
            }
        }
        // The ropes give by a normal draw whose deviation is ROPE_GIVE * 0.6, and numpy refuses a negative deviation,
        // minus zero included (the generator panics where numpy raises).
        if self.rope_give.is_sign_negative() {
            return Err(SimError::new(
                "rules.ROPE_GIVE",
                format!("{} is negative", self.rope_give),
            ));
        }
        if self.t_fight < 0.0 {
            return Err(SimError::new("rules.t_fight", format!("{} is negative", self.t_fight)));
        }
        for (name, x) in [
            ("A", self.a),
            ("R", self.r),
            ("DT", self.dt),
            ("KNOCK_T", self.knock_t),
            ("beat", self.beat),
        ] {
            if x <= 0.0 {
                return Err(SimError::new(format!("rules.{name}"), format!("{x} is not above 0")));
            }
        }
        if self.fps == 0 {
            return Err(SimError::new("rules.FPS", "0 is not a frame rate"));
        }
        if self.steps_per_frame() < 1 {
            return Err(SimError::new(
                "rules.FPS",
                format!(
                    "{} frames a second is faster than the step ({} s): no step to record",
                    self.fps, self.dt
                ),
            ));
        }
        let steps = (self.t_max - self.t_fight) / self.dt;
        if steps > MAX_STEPS {
            return Err(SimError::new(
                "rules.DT",
                format!("the fight would take {steps:.0} steps (at most {MAX_STEPS:.0})"),
            ));
        }
        Ok(())
    }

    /// `int(round(1.0 / (FPS * DT)))`: the steps between two recorded frames (4).
    pub fn steps_per_frame(&self) -> i64 {
        py::round_half_even(1.0 / (f64::from(self.fps) * self.dt)) as i64
    }

    /// `int(round(t_fight * FPS))`: the video frame the fight's first recorded frame belongs to.
    pub fn f0(&self) -> usize {
        py::round_half_even(self.t_fight * f64::from(self.fps)) as usize
    }

    /// The rules as a recording holds them: `vars(rules)` in Python, each attribute under its Python name. `dmg_side`
    /// and `swat_cobras` are attributes only an episode adds, so they are left out when absent (`None`, `false`).
    pub fn to_values(&self) -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        let mut put = |key: &str, v: Value| _ = m.insert(key.to_string(), v);
        put("A", Value::Float(self.a));
        put("R", Value::Float(self.r));
        put("V", Value::Float(self.v));
        put("DT", Value::Float(self.dt));
        put("FPS", Value::Int(i64::from(self.fps)));
        put("ROPE_GIVE", Value::Float(self.rope_give));
        put("HP", Value::Float(self.hp));
        put("LOW_HP", Value::Float(self.low_hp));
        put(
            "LV_DMG",
            Value::List(vec![Value::Float(self.lv_dmg.0), Value::Float(self.lv_dmg.1)]),
        );
        put("KNOCK_T", Value::Float(self.knock_t));
        put("LAUNCH_SPREAD", Value::Float(self.launch_spread));
        put("KEY_R", Value::Float(self.key_r));
        put("bpm", Value::Float(self.bpm));
        put("beat", Value::Float(self.beat));
        put("t_fight", Value::Float(self.t_fight));
        put("t_max", Value::Float(self.t_max));
        put("dmg_scale", Value::Float(self.dmg_scale));
        if let Some([left, right]) = self.dmg_side {
            put("dmg_side", Value::List(vec![Value::Float(left), Value::Float(right)]));
        }
        if self.swat_cobras {
            put("swat_cobras", Value::Bool(true));
        }
        m
    }
}

/// A number: a float, or a whole number widened as Python arithmetic would.
fn number(value: &Value, field: &str) -> Result<f64, SimError> {
    match value {
        Value::Float(x) => Ok(*x),
        Value::Int(n) => Ok(*n as f64),
        _ => Err(SimError::new(field, "must be a number")),
    }
}

/// A list of exactly two numbers.
fn pair(value: &Value, field: &str) -> Result<[f64; 2], SimError> {
    match value {
        Value::List(items) if items.len() == 2 => Ok([number(&items[0], field)?, number(&items[1], field)?]),
        _ => Err(SimError::new(field, "must be a list of 2 numbers")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    #[test]
    fn new_has_the_engine_constants() {
        let r = Rules::new(129.84, 1.8485);
        assert_eq!(
            (r.a, r.r, r.v, r.fps, r.rope_give, r.hp, r.low_hp),
            (3.0, 0.6, 2.6, 30, 5.0, 100.0, 25.0)
        );
        assert_eq!(r.dt.to_bits(), (1.0f64 / 120.0).to_bits());
        assert_eq!(r.lv_dmg, (14.0, 36.0));
        assert_eq!((r.knock_t, r.launch_spread, r.key_r), (0.38, 38.0, 0.11));
        assert_eq!(r.beat.to_bits(), (60.0f64 / 129.84).to_bits());
        assert_eq!((r.t_fight, r.t_max, r.dmg_scale), (1.8485, 58.0, 1.0));
        assert_eq!((r.dmg_side, r.swat_cobras), (None, false));
        assert_eq!(r.steps_per_frame(), 4);
        assert_eq!(r.check(), Ok(()));
    }

    /// Every reference recording's `rules` (the episode's, the scenario overrides, the swapped handicap) applies,
    /// and comes back out with the same names, types and bits.
    #[test]
    fn a_recordings_rules_apply_and_come_back_the_same() {
        let mut seen_overrides = 0;
        for name in testing::ALL_RECORDINGS {
            let py = testing::python_recording(name);
            let mut rules = Rules::new(1.0, 0.0);
            rules.apply(&py.rules).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(rules.to_values(), py.rules, "{name}");
            if rules.lv_dmg != (14.0, 36.0) || rules.low_hp != 25.0 || rules.dmg_scale != 1.0 {
                seen_overrides += 1;
            }
            assert_eq!(rules.f0(), py.f0, "{name}");
        }
        assert_eq!(seen_overrides, 9, "the early_levels, early_low and early_ko scenarios");
        let ep = testing::episode_rules();
        assert_eq!((ep.dmg_side, ep.swat_cobras), (Some([2.0, 0.4]), true));
        assert_eq!((ep.bpm, ep.beat), (133.7704918032787, 0.44852941176470584));
    }

    #[test]
    fn apply_sets_only_the_named_rules() {
        let mut r = Rules::new(120.0, 4.0);
        let over = BTreeMap::from([
            ("swat_cobras".to_string(), Value::Bool(true)),
            ("LOW_HP".to_string(), Value::Int(92)),
            ("bpm".to_string(), Value::Float(60.0)),
        ]);
        r.apply(&over).unwrap();
        let mut want = Rules::new(120.0, 4.0);
        want.swat_cobras = true;
        want.low_hp = 92.0;
        want.bpm = 60.0; // and beat stays 0.5, as setattr leaves it
        assert_eq!(r, want);
        assert_eq!(r.beat, 0.5);
        r.apply(&BTreeMap::from([("dmg_side".to_string(), Value::Null)]))
            .unwrap();
        assert_eq!(r.dmg_side, None);
        // absent attributes are left out of a recording's rules
        assert!(!Rules::new(120.0, 4.0).to_values().contains_key("swat_cobras"));
        assert!(!Rules::new(120.0, 4.0).to_values().contains_key("dmg_side"));
    }

    #[test]
    fn a_bad_override_is_named_and_changes_nothing() {
        let before = Rules::new(120.0, 4.0);
        let cases: [(&str, Value, &str); 12] = [
            ("ROPE_GIVE", Value::Float(-1.0), "rules.ROPE_GIVE: -1 is negative"),
            ("ROPE_GIVE", Value::Float(-0.0), "rules.ROPE_GIVE: -0 is negative"),
            ("t_fight", Value::Float(-2.0), "rules.t_fight: -2 is negative"),
            (
                "swat_shells",
                Value::Bool(true),
                "rules.swat_shells: the engine has no rule of this name",
            ),
            ("LOW_HP", Value::Str("low".into()), "rules.LOW_HP: must be a number"),
            (
                "LV_DMG",
                Value::List(vec![Value::Float(2.0)]),
                "rules.LV_DMG: must be a list of 2 numbers",
            ),
            ("swat_cobras", Value::Int(1), "rules.swat_cobras: must be true or false"),
            ("FPS", Value::Float(30.0), "rules.FPS: must be a whole number"),
            ("FPS", Value::Int(0), "rules.FPS: 0 is not a frame rate"),
            ("DT", Value::Float(0.0), "rules.DT: 0 is not above 0"),
            (
                "t_max",
                Value::Float(f64::INFINITY),
                "rules.t_max: inf is not a finite number",
            ),
            (
                "DT",
                Value::Float(1e-9),
                "rules.DT: the fight would take 54000000000 steps (at most 1000000)",
            ),
        ];
        for (key, value, want) in cases {
            let mut r = before.clone();
            // a good entry beside the bad one must not be applied either
            let over = BTreeMap::from([("A".to_string(), Value::Float(4.0)), (key.to_string(), value)]);
            assert_eq!(r.apply(&over).unwrap_err().to_string(), want);
            assert_eq!(r, before, "{key}");
        }
        // 300 frames a second at 120 steps a second: no step to record
        let mut r = before.clone();
        let err = r
            .apply(&BTreeMap::from([("FPS".to_string(), Value::Int(300))]))
            .unwrap_err();
        assert_eq!(err.field, "rules.FPS");
        assert!(err.message.contains("no step to record"), "{err}");
    }
}
