/// The first use of an ability in a fight.
#[derive(Clone, Debug, PartialEq)]
pub struct AbilityMark {
    /// Seconds into the fight.
    pub t: f32,
    /// Which fighter (0 or 1).
    pub who: usize,
    /// The ability's name as shown ("Snake dance").
    pub name: String,
}

/// What the interface shows about one seed's fight.
#[derive(Clone, Debug, PartialEq)]
pub struct StoryView {
    pub seed: u64,
    pub winner: Option<usize>,
    pub t_ko: Option<f64>,
    pub winner_hp: f64,
    pub lead_changes: u32,
    pub score: f64,
    pub fits: bool,
    pub misses: Vec<String>,
    pub t_fight: f64,
    /// (seconds, hp[0] - hp[1]) after each hit.
    pub lead: Vec<(f32, f32)>,
    pub firsts: Vec<AbilityMark>,
}
