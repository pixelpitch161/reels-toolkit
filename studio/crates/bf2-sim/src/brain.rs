//! Behaviour: seeded move choice and movement.

/// A move the brain may pick, and when.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub mv: usize,
    /// Centre distance, from and to.
    pub range: (f64, f64),
    pub weight: f64,
    /// Steps after it starts before it can start again.
    pub cooldown: u32,
    /// Needs a full meter (100), which it spends.
    pub needs_meter: bool,
    /// `Some(true)` only when airborne, `Some(false)` only on the canvas.
    pub air: Option<bool>,
}

/// A behaviour profile.
#[derive(Clone, Debug, PartialEq)]
pub struct Brain {
    pub choices: Vec<Choice>,
    /// Move speed (2.6 like today).
    pub speed: f64,
    /// Steps between decisions (6).
    pub think: u32,
    /// The move fired once when HP drops below `low_hp`.
    pub low: Option<usize>,
    /// 0 to 1: how much it circles instead of approaching.
    pub orbit: f64,
}
