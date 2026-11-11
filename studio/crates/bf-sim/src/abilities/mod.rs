//! The abilities, one module per kind: free functions taking `&mut Fight`, ports of `fight.py`'s `trigger` (2830 to
//! 2988), `ab_dur` (2823 to 2825) and `effects_step` (3055 to 3113) for the five kinds FLUTE and TROMBONE use, and
//! `shield`.
//!
//! - [`haste`]: the fighter's weapon grid runs faster for a while (FLUTTER TONGUE, POWER SLIDE).
//! - [`dance`]: the cobras leave their baskets and chase the enemy (SNAKE DANCE).
//! - [`wave`]: a ring spreads from the fighter and hits the enemy its front passes (PICCOLO).
//! - [`shockwave`]: the same ring under its own name, and the slow it puts on the enemy it hits (SAD TROMBONE).
//! - [`slam`]: a strike called on where the enemy is heading, landing some beats later (PEDAL TONE).
//! - [`shield`]: hits on the fighter are blocked for a while (TUBA, HORN, ORGAN).
//!
//! The kit's other ability kinds each have a module here too (or live in their weapon's module, as `trigger` says
//! below), owned by their fighter's port: stubs until then, already called from [`trigger`]'s dispatch and from
//! every other place `fight.py` calls them.
//!
//! `Fight::abilities_step` decides WHEN an ability fires (levels, the low-HP turn, the unlock timers) and calls
//! [`trigger`]; [`crate::schedule`] calls [`effects_step`] once per step, after the weapons and before
//! `abilities_step`, so a ring or a slam made in a step first moves in the next one.
//!
//! None of this code draws from the fight's generator.

pub mod arc;
pub mod beam;
pub mod copy;
pub mod dance;
pub mod flick;
pub mod haste;
pub mod hook;
pub mod lights_out;
pub mod prelude;
pub mod runoff;
pub mod shield;
pub mod shockwave;
pub mod slam;
pub mod slap;
pub mod smash;
pub mod snip;
pub mod stampede;
pub mod stun;
pub mod sustain;
pub mod swan;
pub mod twin;
pub mod vibrato;
pub mod wave;
pub mod x33;

use bf_core::event::Value;

use crate::ball::Slot;
use crate::fight::Fight;
use crate::kinds::{crowd, drill, jump, lightning, mortar, roll_charge, spotlight, tiptoe, tune};
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

/// Whether fighter `k` has an ability of this kind in any slot (Python's `any(a['kind'] == kind for a in
/// B.cfg['abilities'].values())`, the lists `run()` makes of snippers, hookers, flickers, ...). A kind not ported is
/// [`AbilityKind::Unsupported`] in the roster, so no fighter has it yet.
pub fn has(fight: &Fight, k: usize, kind: &AbilityKind) -> bool {
    let cfg = fight.balls[k].cfg;
    [&cfg.unlock1, &cfg.unlock2, &cfg.low].iter().any(|ab| &ab.kind == kind)
}

/// `trigger(k, slot)`: fighter `k` fires the ability in `slot`. Event `ability`: its name, its kind, and 1 if this is
/// the first time that slot fires, else 0; the ability becomes the fighter's last (`B.last_ab`, what IDENTITY CRISIS
/// copies). Then the kind does its work ([`fire`]).
///
/// A kind the engine has not ported ([`AbilityKind::Unsupported`]) is announced and marked used like any other and
/// then does nothing, as `fight.py` does for a kind it has no branch for.
pub fn trigger(fight: &mut Fight, k: usize, slot: Slot) {
    let ab = slot.of(fight.balls[k].cfg);
    fight.socket("trigger.ability");
    let b = &mut fight.balls[k];
    let first = !b.used[slot.index()];
    b.used[slot.index()] = true;
    let data = vec![
        Value::Str(ab.name.clone()),
        Value::Str(ab.kind.name().to_string()),
        Value::Int(i64::from(first)),
    ];
    fight.ev("ability", k as i64, data);
    // fight.py 2840: what IDENTITY CRISIS would copy
    fight.socket("trigger.last_ab");
    fight.balls[k].last_ab = Some(ab);
    fire(fight, k, ab);
}

/// `trigger(k, 'copy', ab=src, quiet=True)` (`fight.py` 2830 to 2840, from `start_copy` 2815): fighter `k` fires
/// another fighter's ability on its own ball, with no `ability` event (the copy is announced by its own `copy`
/// event), no bookkeeping, and without becoming its last ability.
pub fn trigger_quiet<'a>(fight: &mut Fight<'a>, k: usize, ab: &'a AbilityCfg) {
    fight.socket("trigger.quiet");
    fire(fight, k, ab);
}

/// `trigger`'s dispatch (`fight.py` 2841 to 2988): the kind's own start, for fighter `k`. Every kit kind has its
/// branch, in Python's order; each goes to the module that owns the kind (its own, or its weapon's).
pub fn fire<'a>(fight: &mut Fight<'a>, k: usize, ab: &'a AbilityCfg) {
    fight.socket("trigger.dispatch");
    match &ab.kind {
        // 2841 to 2846 (a wave's `detune` is the ring's: `wave::ring`)
        AbilityKind::Shockwave => shockwave::fire(fight, k, ab),
        AbilityKind::Wave => wave::fire(fight, k, ab),
        // 2847 to 2848
        AbilityKind::Shield => shield::start(fight, k, ab),
        // 2849 to 2851
        AbilityKind::Haste => haste::fire(fight, k, ab),
        // 2852 to 2858
        AbilityKind::Arc => arc::start(fight, k, ab),
        // 2859 to 2865
        AbilityKind::Slam => slam::fire(fight, k, ab),
        // 2866 to 2869: rewind, lash (kinds older than the kit's). SLOT.
        // 2870 to 2871
        AbilityKind::Beam => beam::start(fight, k, ab),
        // 2872 to 2873 (`spout`, the third kind of this branch, is older than the kit's)
        AbilityKind::Whip => x33::start(fight, k, ab, "whip"),
        AbilityKind::Sweep => x33::start(fight, k, ab, "sweep"),
        // 2874 to 2877: charge, clamp (kinds older than the kit's). SLOT.
        // 2878 to 2879
        AbilityKind::Dance => dance::fire(fight, k, ab),
        // 2880 to 2881
        AbilityKind::Flick => flick::start(fight, k, ab),
        // 2882 to 2883
        AbilityKind::Snip => snip::start(fight, k, ab),
        // 2884 to 2888
        AbilityKind::Endless => tune::start_endless(fight, k, ab),
        // 2889 to 2892
        AbilityKind::Stun => stun::start(fight, k, ab),
        // 2893 to 2894
        AbilityKind::Copy => copy::start(fight, k, ab),
        // 2895 to 2896
        AbilityKind::LightsOut => lights_out::start(fight, k, ab),
        // 2897 to 2899
        AbilityKind::Twin => twin::start(fight, k, ab),
        // 2900 to 2901
        AbilityKind::Barrage => mortar::start_barrage(fight, k, ab),
        // 2902 to 2914: quake, offbeat (kinds older than the kit's). SLOT.
        // 2915 to 2916
        AbilityKind::Hook => hook::start(fight, k, ab),
        // 2917 to 2918
        AbilityKind::Mountain => tiptoe::start_mountain(fight, k, ab),
        // 2919 to 2929: snap, joke, melody, double_stop (kinds older than the kit's). SLOT.
        // 2930 to 2932
        AbilityKind::Rhapsody => jump::start_rhapsody(fight, k, ab),
        // 2933 to 2935
        AbilityKind::Fork => lightning::start_fork(fight, k, ab),
        // 2936 to 2938
        AbilityKind::Tapping => lightning::start_tapping(fight, k, ab),
        // 2939 to 2940
        AbilityKind::Smash => smash::start(fight, k, ab),
        // 2941 to 2943
        AbilityKind::Sustain => sustain::start(fight, k, ab),
        // 2944 to 2945
        AbilityKind::Runoff => runoff::start(fight, k, ab),
        // 2946 to 2947
        AbilityKind::Slap => slap::start(fight, k, ab),
        // 2948 to 2950
        AbilityKind::Prelude => prelude::start(fight, k, ab),
        // 2951 to 2952
        AbilityKind::Swan => swan::start(fight, k, ab),
        // 2953 to 2954
        AbilityKind::Stampede => stampede::start(fight, k, ab),
        // 2955 to 2957
        AbilityKind::Vibrato => vibrato::start(fight, k, ab),
        // 2958 to 2960
        AbilityKind::Octave => spotlight::start_octave(fight, k, ab),
        // 2961 to 2963
        AbilityKind::Altissimo => spotlight::start_altissimo(fight, k, ab),
        // 2964 to 2965
        AbilityKind::Encore => spotlight::start_encore(fight, k, ab),
        // 2966 to 2968 (`if B.dive is None`: the crowd's own test)
        AbilityKind::Runway => crowd::start_runway(fight, k, ab),
        // 2969 to 2972
        AbilityKind::Subtone => crowd::start_subtone(fight, k, ab),
        // 2973 to 2976
        AbilityKind::Stagedive => crowd::start_dive(fight, k, ab),
        // 2977 to 2979 (`if hasattr(B, 'march')`: the drill's own test)
        AbilityKind::FlashSpin => drill::start_flash(fight, k, ab),
        // 2980 to 2982 (the same)
        AbilityKind::CompanyFront => drill::start_front(fight, k, ab),
        // 2983 to 2984
        AbilityKind::Heartbeat => roll_charge::start_heartbeat(fight, k, ab),
        // 2985 to 2986
        AbilityKind::DrumRoll => roll_charge::start_drum_roll(fight, k, ab),
        // 2987 to 2988
        AbilityKind::Cannons => roll_charge::start_cannons(fight, k, ab),
        // no branch: Python announces it and does nothing else
        AbilityKind::Unsupported(_) => {}
    }
}

/// `effects_step(dt)`: what the abilities left in the ring moves on by one step: every ring spreads and hits, then
/// every slam whose time has come lands.
pub fn effects_step(fight: &mut Fight, dt: f64) {
    // fight.py 3057 to 3087
    fight.socket("effects.waves");
    wave::step(fight, dt);
    // fight.py 3088 to 3098
    fight.socket("effects.slams");
    slam::step(fight);
    // fight.py 3099 to 3113: the arcs emit their projectiles, after the slams (PIANO's arc)
    fight.socket("effects.arcs");
    if arc::live(fight) {
        arc::step(fight);
    }
}

#[cfg(test)]
mod tests;
