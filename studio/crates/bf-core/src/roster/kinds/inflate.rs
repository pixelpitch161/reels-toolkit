//! Weapon kind `inflate`, BAGPIPES's BAG INFLATE: its roster settings. `fight.py` 1763 to 1867 (`inflate_setup`,
//! `inflate_start`, `inflate_step`, `inflate_clash`), its branch of `grid16_fire` (1487 to 1507), and the per-ball
//! radius, mass and speed it gives `move` (3159, 3165, 3179, 3197, 3209 to 3216, 3233 to 3234).

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError, join};

/// Whether the engine runs weapon kind `inflate`.
pub const PORTED: bool = true;

/// Weapon kind `inflate`'s typed settings. Fields without a default are read as `w[name]`; the others are
/// `w.get(name, default)` with `fight.py`'s default.
#[derive(Clone, Debug, PartialEq)]
pub struct InflateCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// Beats between two cycles on the 16th grid (`max(1, int(round(4 * every)))` 16ths: a haste divides it).
    pub every: f64,
    /// Beats the bag swells (a haste divides it). Never 0 in a roster entry (`inflate_step` divides by it).
    pub inflate_beats: f64,
    /// The swollen size (x R) a fight starts with (`inf_scale`).
    pub scale0: f64,
    /// Size added per landed hit.
    pub grow: f64,
    /// The size never grows past this.
    pub scale_max: f64,
    /// How much of the swell goes into the mass (`.get('mass_k', 1.0)`).
    pub mass_k: f64,
    /// The speed factor at the full swell.
    pub slow: f64,
    /// The flight's length in beats a fight starts with (`inf_fly`).
    pub fly_beats: f64,
    /// Flight beats added per landed hit.
    pub fly_grow: f64,
    /// The flight never grows past this.
    pub fly_max: f64,
    /// The speed factor as the bag lets go.
    pub fly_speed: f64,
    /// How much of `fly_speed` the flight loses by its end (`.get('fly_fade', 0.35)`).
    pub fly_fade: f64,
    /// Seconds between two zig-zags, drawn from this range (`uniform(*zig)`).
    pub zig: [f64; 2],
    /// Degrees of a zig-zag's turn, drawn from this range (`uniform(*zig_turn)`).
    pub zig_turn: [f64; 2],
    /// How much a zig-zag turns toward the enemy (`.get('home', 0.3)`).
    pub home: f64,
    /// Degrees either side the release is aimed off the enemy (`.get('aim_wobble', 20.0)`).
    pub aim_wobble: f64,
    /// A body slam's damage at the full swell.
    pub slam_dmg: f64,
    /// The least swell (0..1) that body-slams (`.get('slam_min', 0.3)`).
    pub slam_min: f64,
    /// A flight hit's damage.
    pub fly_dmg: f64,
    /// Seconds between two contact hits.
    pub cool: f64,
    /// Knockback of a flight hit (a body slam: 0.8 of it).
    pub knock: f64,
    /// Beats of rest a cycle keeps before the next (`.get('rest_beats', 0.25)`).
    pub rest_beats: f64,
}

/// The keys an inflate entry may hold: `kind`, `style` and [`InflateCfg`]'s fields.
const INFLATE_KEYS: [&str; 24] = [
    "kind",
    "style",
    "every",
    "inflate_beats",
    "scale0",
    "grow",
    "scale_max",
    "mass_k",
    "slow",
    "fly_beats",
    "fly_grow",
    "fly_max",
    "fly_speed",
    "fly_fade",
    "zig",
    "zig_turn",
    "home",
    "aim_wobble",
    "slam_dmg",
    "slam_min",
    "fly_dmg",
    "cool",
    "knock",
    "rest_beats",
];

/// `w[key]`, a pair of numbers (`uniform(*w[key])`).
fn pair(e: &Entry, w: &Json, p: &str, key: &str) -> Result<[f64; 2], RosterError> {
    let field = join(p, key);
    match e.get(w, p, key)?.as_array().map(|a| a.as_slice()) {
        Some([a, b]) => Ok([e.number(a, &field)?, e.number(b, &field)?]),
        _ => Err(e.err(&field, "must be a pair of numbers [low, high]")),
    }
}

/// The swell's length: `inflate_step` divides by it (Python raises on 0).
fn inflate_beats_value(e: &Entry, x: f64) -> Result<f64, RosterError> {
    if x == 0.0 {
        return Err(e.err(
            "weapon.inflate_beats",
            "0 is not a swell (the bag must take some time to fill)",
        ));
    }
    Ok(x)
}

/// Reads an `inflate` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<InflateCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "an inflate weapon", &INFLATE_KEYS)?;
    Ok(InflateCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "inflate".to_string(),
        },
        every: e.grid(w, p, "every")?,
        inflate_beats: inflate_beats_value(e, e.f64(w, p, "inflate_beats")?)?,
        scale0: e.f64(w, p, "scale0")?,
        grow: e.f64(w, p, "grow")?,
        scale_max: e.f64(w, p, "scale_max")?,
        mass_k: e.f64_or(w, p, "mass_k", 1.0)?,
        slow: e.f64(w, p, "slow")?,
        fly_beats: e.f64(w, p, "fly_beats")?,
        fly_grow: e.f64(w, p, "fly_grow")?,
        fly_max: e.f64(w, p, "fly_max")?,
        fly_speed: e.f64(w, p, "fly_speed")?,
        fly_fade: e.f64_or(w, p, "fly_fade", 0.35)?,
        zig: pair(e, w, p, "zig")?,
        zig_turn: pair(e, w, p, "zig_turn")?,
        home: e.f64_or(w, p, "home", 0.3)?,
        aim_wobble: e.f64_or(w, p, "aim_wobble", 20.0)?,
        slam_dmg: e.f64(w, p, "slam_dmg")?,
        slam_min: e.f64_or(w, p, "slam_min", 0.3)?,
        fly_dmg: e.f64(w, p, "fly_dmg")?,
        cool: e.f64(w, p, "cool")?,
        knock: e.f64(w, p, "knock")?,
        rest_beats: e.f64_or(w, p, "rest_beats", 0.25)?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &InflateCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    inflate_beats_value(e, w.inflate_beats)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): nothing (`inflate_setup` sets the ball's
/// `blade_len` to the swollen size when the fight starts).
pub fn ball_start(_w: &InflateCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste: `int(round(4 * every))`, as a float.
pub(crate) fn main16(w: &InflateCfg) -> Option<f64> {
    Some(4.0 * w.every)
}

#[cfg(test)]
mod tests {
    use crate::roster::{BallStart, WeaponCfg, parse_roster};

    fn bagpipes(weapon: &str) -> String {
        format!(
            r#"{{"format": 1, "fighters": [{{"name": "BAGPIPES", "color": [64, 196, 88], "dark": [26, 104, 42],
            "corner": "red", "view": [42, 22, 0], "weapon": {weapon},
            "abilities": {{"unlock1": {{"name": "WAR CRY", "kind": "shockwave", "speed": 7.5, "radius": 3.4,
            "dmg": 3.0, "knock": 1.2, "every_bars": 4}},
            "unlock2": {{"name": "HIGHLAND FLING", "kind": "haste", "mult": 2.0, "dur_beats": 8, "every_bars": 6}},
            "low": {{"name": "BAG BURST", "kind": "wave", "speed": 4.5, "radius": 9.0, "dmg": 8.0, "knock": 1.5}}}},
            "pp": "BAGPIPES", "register": [67, 81], "idiom": "drone"}}]}}"#
        )
    }

    const KIT: &str = r#"{"kind": "inflate", "style": "balloon", "every": 4, "inflate_beats": 2, "scale0": 1.4,
        "grow": 0.015, "scale_max": 1.65, "mass_k": 1.0, "slow": 0.55, "fly_beats": 1.25, "fly_grow": 0.06,
        "fly_max": 1.75, "fly_speed": 2.6, "fly_fade": 0.35, "zig": [0.1, 0.17], "zig_turn": [35.0, 70.0],
        "home": 0.35, "aim_wobble": 20.0, "slam_dmg": 5.0, "slam_min": 0.3, "fly_dmg": 6.8, "cool": 0.3,
        "knock": 1.2, "rest_beats": 0.25}"#;

    #[test]
    fn the_kit_entry_reads_with_pythons_defaults() {
        let roster = parse_roster(&bagpipes(KIT)).expect("the kit's BAGPIPES");
        let WeaponCfg::Inflate(w) = &roster[0].weapon else {
            panic!("not an inflate: {:?}", roster[0].weapon)
        };
        assert_eq!((w.style.as_str(), w.every, w.inflate_beats), ("balloon", 4.0, 2.0));
        assert_eq!((w.zig, w.zig_turn, w.home), ([0.1, 0.17], [35.0, 70.0], 0.35));
        assert_eq!((w.slam_dmg, w.fly_dmg, w.cool, w.knock), (5.0, 6.8, 0.3, 1.2));
        assert_eq!(roster[0].weapon.ball_start(), BallStart::default());
        let bare = KIT
            .replace(r#""mass_k": 1.0, "#, "")
            .replace(r#""fly_fade": 0.35, "#, "")
            .replace(r#""home": 0.35, "#, "")
            .replace(r#""aim_wobble": 20.0, "#, "")
            .replace(r#""slam_min": 0.3, "#, "")
            .replace(r#", "rest_beats": 0.25"#, "");
        let roster = parse_roster(&bagpipes(&bare)).expect("without the optional keys");
        let WeaponCfg::Inflate(w) = &roster[0].weapon else {
            panic!("not an inflate")
        };
        assert_eq!(
            (w.mass_k, w.fly_fade, w.home, w.aim_wobble, w.slam_min, w.rest_beats),
            (1.0, 0.35, 0.3, 20.0, 0.3, 0.25)
        );
    }

    #[test]
    fn what_the_kind_cannot_run_is_refused_by_name() {
        for (from, to, field) in [
            (r#""inflate_beats": 2"#, r#""inflate_beats": 0"#, "weapon.inflate_beats"),
            (r#""every": 4"#, r#""every": 1e308"#, "weapon.every"),
            (r#""zig": [0.1, 0.17]"#, r#""zig": [0.1]"#, "weapon.zig"),
            (r#""zig_turn": [35.0, 70.0]"#, r#""zig_turn": 35.0"#, "weapon.zig_turn"),
            (r#""fly_dmg": 6.8"#, r#""fly_dmg": "lots""#, "weapon.fly_dmg"),
            (r#""cool": 0.3"#, r#""cool": 0.3, "pull": 1.0"#, "weapon.pull"),
            (r#""grow": 0.015, "#, "", "weapon.grow"),
        ] {
            assert!(KIT.contains(from), "{from}");
            let err = parse_roster(&bagpipes(&KIT.replace(from, to))).expect_err(field);
            assert_eq!((err.fighter.as_str(), err.field.as_str()), ("BAGPIPES", field), "{err}");
        }
    }
}
