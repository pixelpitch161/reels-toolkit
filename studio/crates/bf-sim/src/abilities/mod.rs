//! The abilities, one module per kind: free functions taking `&mut Fight`, ports of `fight.py`'s `trigger` (2830 to
//! 2988), `ab_dur` (2823 to 2825) and `effects_step` (3055 to 3113) for the five kinds FLUTE and TROMBONE use.
//!
//! - [`haste`]: the fighter's weapon grid runs faster for a while (FLUTTER TONGUE, POWER SLIDE).
//! - [`dance`]: the cobras leave their baskets and chase the enemy (SNAKE DANCE).
//! - [`wave`]: a ring spreads from the fighter and hits the enemy its front passes (PICCOLO).
//! - [`shockwave`]: the same ring under its own name, and the slow it puts on the enemy it hits (SAD TROMBONE).
//! - [`slam`]: a strike called on where the enemy is heading, landing some beats later (PEDAL TONE).
//!
//! `Fight::abilities_step` decides WHEN an ability fires (levels, the low-HP turn, the unlock timers) and calls
//! [`trigger`]; [`crate::schedule`] calls [`effects_step`] once per step, after the weapons and before
//! `abilities_step`, so a ring or a slam made in a step first moves in the next one.
//!
//! None of this code draws from the fight's generator.

pub mod dance;
pub mod haste;
pub mod shockwave;
pub mod slam;
pub mod wave;

use bf_core::event::Value;

use crate::ball::Slot;
use crate::fight::Fight;
use crate::roster::{AbilityCfg, AbilityKind};

/// `ab[name]`: a number of an ability's roster entry that its kind cannot do without.
///
/// In a fight the number is always there: `Fight::new` (through `FighterCfg::check`) refuses an ability of a ported
/// kind that lacks one of its kind's numbers, and an unlock of any kind without `every_bars`, where Python raises
/// `KeyError`. Only an entry changed by a test after the check could get to the panic.
pub(crate) fn param(ab: &AbilityCfg, name: &str) -> f64 {
    ab.get(name)
        .unwrap_or_else(|| panic!("the ability {} has no {name} (Fight::new refuses that)", ab.name))
}

/// `ab_dur(ab)`: an ability's duration in seconds: its `dur`, else its `dur_beats` beats at this fight's tempo.
///
/// # Panics
/// When the entry has neither (Python raises `KeyError`). Never for an ability of the fight's own fighters: a haste
/// or a dance without either is refused by `Fight::new`.
pub fn ab_dur(fight: &Fight, ab: &AbilityCfg) -> f64 {
    match ab.get("dur") {
        Some(dur) => dur,
        None => param(ab, "dur_beats") * fight.rules.beat,
    }
}

/// `trigger(k, slot)`: fighter `k` fires the ability in `slot`. Event `ability`: its name, its kind, and 1 if this is
/// the first time that slot fires, else 0. Then the kind does its work.
///
/// A kind the engine has not ported ([`AbilityKind::Unsupported`]) is announced and marked used like any other and
/// then does nothing, as `fight.py` does for a kind it has no branch for.
pub fn trigger(fight: &mut Fight, k: usize, slot: Slot) {
    let ab = slot.of(fight.balls[k].cfg);
    let b = &mut fight.balls[k];
    let first = !b.used[slot.index()];
    b.used[slot.index()] = true;
    let data = vec![
        Value::Str(ab.name.clone()),
        Value::Str(ab.kind.name().to_string()),
        Value::Int(i64::from(first)),
    ];
    fight.ev("ability", k as i64, data);
    // fight.py 2840: `B.last_ab = ab`, what the kind `copy` (IDENTITY CRISIS) would fire. Kind not ported.
    match &ab.kind {
        AbilityKind::Wave => wave::fire(fight, k, ab),
        AbilityKind::Shockwave => shockwave::fire(fight, k, ab),
        AbilityKind::Haste => haste::fire(fight, k, ab),
        AbilityKind::Slam => slam::fire(fight, k, ab),
        AbilityKind::Dance => dance::fire(fight, k, ab),
        // fight.py 2847 to 2988 has a branch for every other kind (shield, arc, rewind, lash, beam, ...): each comes
        // with its kind.
        AbilityKind::Unsupported(_) => {}
    }
}

/// `effects_step(dt)`: what the abilities left in the ring moves on by one step: every ring spreads and hits, then
/// every slam whose time has come lands.
pub fn effects_step(fight: &mut Fight, dt: f64) {
    // fight.py 3057 to 3087
    wave::step(fight, dt);
    // fight.py 3088 to 3098
    slam::step(fight);
    // SLOT (ability kind `arc`): fight.py 3099 to 3113, the arcs emit their projectiles, after the slams.
}

#[cfg(test)]
mod tests;
