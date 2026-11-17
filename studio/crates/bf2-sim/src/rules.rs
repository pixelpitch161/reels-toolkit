//! The fight's rule numbers, in the current engine's units (ring half-size inside the ropes, ball radius, HP,
//! seconds).

use std::fmt;

/// A refusal: which field (`rules.dt`, `fighter[0].moves[2].hits[0].window`, ...) and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimError2 {
    pub field: String,
    pub message: String,
}

impl SimError2 {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        SimError2 {
            field: field.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for SimError2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for SimError2 {}

/// The rule numbers of one fight.
#[derive(Clone, Debug, PartialEq)]
pub struct Rules2 {
    /// Ring half-size inside the ropes.
    pub a: f64,
    /// The default ball radius.
    pub r: f64,
    /// Seconds per step (120 Hz).
    pub dt: f64,
    /// Units/s², along z.
    pub gravity: f64,
    /// Share of the downward speed kept by a bounce off the canvas.
    pub floor_bounce: f64,
    /// Share of the outward speed kept by a bounce off the ropes.
    pub rope_bounce: f64,
    /// A rope bounce turns the ball by a seeded angle within this many degrees either way.
    pub rope_give_deg: f64,
    /// 1/s, on the canvas only.
    pub friction: f64,
    pub hp: f64,
    /// The low-HP ability fires below this.
    pub low_hp: f64,
    /// Video seconds when the fight starts (`bf_core::grid::Grid::t_fight`).
    pub t_fight: f64,
    /// Video seconds when an undecided fight stops.
    pub t_max: f64,
    pub dmg_scale: f64,
    /// The fight file's handicap: each side's damage multiplier.
    pub dmg_side: [f64; 2],
    /// The longest freeze-on-hit, in steps.
    pub max_hitstop: u32,
    /// Damage multiplier per extra air hit in a juggle.
    pub juggle_decay: f64,
}

impl Rules2 {
    pub fn new(t_fight: f64) -> Self {
        Rules2 {
            a: 3.0,
            r: 0.6,
            dt: 1.0 / 120.0,
            gravity: -18.0,
            floor_bounce: 0.35,
            rope_bounce: 0.8,
            rope_give_deg: 5.0,
            friction: 2.5,
            hp: 100.0,
            low_hp: 25.0,
            t_fight,
            t_max: 58.0,
            dmg_scale: 1.0,
            dmg_side: [1.0, 1.0],
            max_hitstop: 36,
            juggle_decay: 0.8,
        }
    }

    pub fn check(&self) -> Result<(), SimError2> {
        let positive = [
            ("rules.a", self.a),
            ("rules.r", self.r),
            ("rules.dt", self.dt),
            ("rules.hp", self.hp),
        ];
        for (field, v) in positive {
            if !(v.is_finite() && v > 0.0) {
                return Err(SimError2::new(
                    field,
                    format!("must be a finite number above 0, not {v}"),
                ));
            }
        }
        if self.r >= self.a {
            return Err(SimError2::new("rules.r", "the ball must fit inside the ropes"));
        }
        let finite = [
            ("rules.gravity", self.gravity),
            ("rules.floor_bounce", self.floor_bounce),
            ("rules.rope_bounce", self.rope_bounce),
            ("rules.rope_give_deg", self.rope_give_deg),
            ("rules.friction", self.friction),
            ("rules.low_hp", self.low_hp),
            ("rules.t_fight", self.t_fight),
            ("rules.t_max", self.t_max),
            ("rules.dmg_scale", self.dmg_scale),
            ("rules.dmg_side", self.dmg_side[0]),
            ("rules.dmg_side", self.dmg_side[1]),
            ("rules.juggle_decay", self.juggle_decay),
        ];
        for (field, v) in finite {
            if !v.is_finite() {
                return Err(SimError2::new(field, format!("must be a finite number, not {v}")));
            }
        }
        let unit = [
            ("rules.floor_bounce", self.floor_bounce),
            ("rules.rope_bounce", self.rope_bounce),
            ("rules.juggle_decay", self.juggle_decay),
        ];
        for (field, v) in unit {
            if !(0.0..=1.0).contains(&v) {
                return Err(SimError2::new(field, format!("must be from 0 to 1, not {v}")));
            }
        }
        let non_negative = [
            ("rules.rope_give_deg", self.rope_give_deg),
            ("rules.friction", self.friction),
            ("rules.dmg_scale", self.dmg_scale),
            ("rules.dmg_side", self.dmg_side[0]),
            ("rules.dmg_side", self.dmg_side[1]),
        ];
        for (field, v) in non_negative {
            if v < 0.0 {
                return Err(SimError2::new(field, format!("must not be below 0, not {v}")));
            }
        }
        if self.t_max <= self.t_fight {
            return Err(SimError2::new(
                "rules.t_max",
                format!("must be after the fight starts ({}), not {}", self.t_fight, self.t_max),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_spec_values() {
        let r = Rules2::new(4.5);
        assert_eq!((r.a, r.r, r.dt, r.gravity), (3.0, 0.6, 1.0 / 120.0, -18.0));
        assert_eq!(
            (r.floor_bounce, r.rope_bounce, r.rope_give_deg, r.friction),
            (0.35, 0.8, 5.0, 2.5)
        );
        assert_eq!((r.hp, r.low_hp, r.t_fight, r.t_max), (100.0, 25.0, 4.5, 58.0));
        assert_eq!(
            (r.dmg_scale, r.dmg_side, r.max_hitstop, r.juggle_decay),
            (1.0, [1.0, 1.0], 36, 0.8)
        );
        assert!(r.check().is_ok());
    }

    #[test]
    fn check_refuses_bad_values() {
        let field = |f: fn(&mut Rules2)| {
            let mut r = Rules2::new(4.5);
            f(&mut r);
            r.check().unwrap_err().field
        };
        assert_eq!(field(|r| r.dt = 0.0), "rules.dt");
        assert_eq!(field(|r| r.dt = -1.0), "rules.dt");
        assert_eq!(field(|r| r.t_max = f64::NAN), "rules.t_max");
        assert_eq!(field(|r| r.t_max = f64::INFINITY), "rules.t_max");
        assert_eq!(field(|r| r.t_max = 4.5), "rules.t_max");
        let e = SimError2 {
            field: "rules.dt".into(),
            message: "must be above 0".into(),
        };
        assert_eq!(e.to_string(), "rules.dt: must be above 0");
    }
}
