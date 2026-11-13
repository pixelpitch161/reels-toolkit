//! Weapon kind `blade` in thrust mode, TROMBONE's slide lance: its roster settings (`fight.py` `Ball.__init__`,
//! `Fight.__init__`, `start_thrust`, `thrust_step`, the thrust grid in the fight loop).
//!
//! A blade with no `mode` spins (VIOLIN's bow, milestone 4: the spinning branch of `blade_step`, 780 to 825, with
//! `spin`, `spin_grow` and `spin_max`). It is a stub until VIOLIN's port: while [`SPIN_PORTED`] is false such an entry
//! loads as `WeaponCfg::Unsupported("blade")`; the port fills [`parse_spin`] (and the fields the spinning branch reads)
//! and sets it.

use serde_json::Value as Json;

use crate::roster::{Entry, RosterError};

/// Whether the engine runs the spinning blade (a `blade` with no `mode`). While false the roster reads one as
/// `WeaponCfg::Unsupported("blade")`, which the engine refuses in words.
pub const SPIN_PORTED: bool = false;

/// A blade's mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BladeMode {
    /// `mode = 'thrust'`: the lance aims at the enemy and shoots out on the grid.
    Thrust,
    /// No `mode`: the blade spins round the ball (VIOLIN's bow; a stub until [`SPIN_PORTED`]).
    Spin,
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
    /// A spinning blade's rate in rad/s (`Ball.__init__`'s `w.get('spin', 0.0)`); 0.0 for a thrust blade, whose entry
    /// may not hold the key.
    pub spin: f64,
    /// A spinning blade's rate added per landed hit (`w.get('spin_grow')`, 0.0 = falsy: no growth); 0.0 for a thrust
    /// blade.
    pub spin_grow: f64,
    /// The rate it never grows past (`w['spin_max']`, read only when `spin_grow` is set); 0.0 for a thrust blade.
    pub spin_max: f64,
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

/// A thrust blade's starting fret count, from 0 to [`MAX_FRETS0`].
fn frets0_value(e: &Entry, n: i64) -> Result<i64, RosterError> {
    if !(0..=MAX_FRETS0).contains(&n) {
        return Err(e.err("weapon.frets0", format!("{n} is not a fret count (0 to {MAX_FRETS0})")));
    }
    Ok(n)
}

/// Reads a thrust-blade weapon entry (a `blade` whose `mode` is `thrust`).
pub(crate) fn parse(e: &Entry, w: &Json, style: String) -> Result<BladeCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a thrust blade", &BLADE_KEYS)?;
    Ok(BladeCfg {
        style,
        mode: BladeMode::Thrust,
        every: e.grid(w, p, "every")?,
        rest: w.get("rest").map(|v| e.number(v, "weapon.rest")).transpose()?,
        base: e.f64_or(w, p, "base", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        max_len: e.f64(w, p, "max_len")?,
        frets0: match w.get("frets0") {
            Some(v) => frets0_value(e, e.int(v, "weapon.frets0")?)?,
            None => 0,
        },
        t_out: e.f64(w, p, "t_out")?,
        t_hold: e.f64(w, p, "t_hold")?,
        t_back: e.f64(w, p, "t_back")?,
        aim_rate: e.f64_or(w, p, "aim_rate", 8.0)?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64_or(w, p, "knock", 0.8)?,
        parry: match w.get("parry") {
            None | Some(Json::Null) => false,
            Some(Json::Bool(b)) => *b,
            Some(_) => return Err(e.err("weapon.parry", "must be true or false")),
        },
        spin: 0.0,
        spin_grow: 0.0,
        spin_max: 0.0,
    })
}

/// Reads a spinning-blade weapon entry (a `blade` with no `mode`); called only when [`SPIN_PORTED`] is true. A stub
/// until VIOLIN's port, which reads the fields the spinning branch of `blade_step` reads (780 to 825).
pub(crate) fn parse_spin(e: &Entry, _w: &Json, _style: String) -> Result<BladeCfg, RosterError> {
    Err(e.err("weapon.mode", "a spinning blade is not ported yet"))
}

/// [`parse`]'s checks made again on typed settings (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &BladeCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    frets0_value(e, w.frets0)?;
    Ok(())
}
