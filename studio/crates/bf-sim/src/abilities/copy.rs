//! Ability kind `copy` (EUPHONIUM's IDENTITY CRISIS): `fight.py` `trigger` 2893 to 2894; 2801 to 2822 (`COPYABLE`,
//! `copy_source`, `start_copy`); the gate in `abilities_step` 3046 to 3049; `trigger`'s quiet path 2830 to 2840.
//!
//! The fighter fires the enemy's last-used ability back as its own, same kind and numbers, through
//! [`trigger_quiet`] (no `ability` event, no bookkeeping), when its kind is one of [`COPYABLE`]; a copied snip runs
//! from the fight loop as the copier's extra snip (`Fight::extra_snip`) and waits for the enemy to be close, as the
//! original does (`Fight::waits`). If the enemy has used no copyable ability yet: one round of its signature, an
//! anchor whirl (with the enemy's settings and orbit, when the copier is not whirling already) or a mortar salvo
//! (the enemy's weapon and volley); any other weapon is announced and does nothing. Event `copy`: the owner, the
//! name of what is copied (the ability's, or the enemy weapon's), its kind.
//!
//! The copier is "ball `k`", whichever fighter it is; nothing here reads ball `k`'s own roster entry. No draw from the
//! fight's generator here (the copied kind draws what it draws).

use bf_core::event::Value;

use crate::abilities::trigger_quiet;
use crate::fight::Fight;
use crate::kinds::{anchor, mortar};
use crate::roster::{AbilityCfg, AbilityKind, WeaponCfg};

/// `COPYABLE` (2802): the kinds IDENTITY CRISIS fires back, in the kit's order.
pub const COPYABLE: [&str; 11] = [
    "shockwave",
    "wave",
    "shield",
    "haste",
    "arc",
    "slam",
    "snip",
    "twin",
    "lights_out",
    "barrage",
    "stun",
];

/// This ability's per-fight state (`Fight::copy`): nothing (what it copies is the enemy's `Ball::last_ab`, a copied
/// snip `Fight::extra_snip`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `copy`: `trigger` 2893 to 2894: `start_copy(k)` (2808 to 2821).
pub fn start<'a>(f: &mut Fight<'a>, k: usize, _ab: &AbilityCfg) {
    let o = 1 - k;
    if let Some(src) = source(f, k) {
        let data = vec![Value::Str(src.name.clone()), Value::Str(src.kind.name().to_string())];
        f.ev("copy", k as i64, data);
        if src.kind == AbilityKind::Snip {
            f.extra_snip[k] = true;
        }
        trigger_quiet(f, k, src);
        return;
    }
    // its signature, once: O.cfg['weapon'].get('name', O.w['kind'].upper())
    let ocfg = f.balls[o].cfg;
    let kind = ocfg.weapon.kind().to_string();
    let name = match ocfg.raw["weapon"].get("name").and_then(|n| n.as_str()) {
        Some(n) => n.to_string(),
        None => kind.to_uppercase(),
    };
    f.ev("copy", k as i64, vec![Value::Str(name), Value::Str(kind)]);
    match &ocfg.weapon {
        WeaponCfg::Anchor(w) if f.balls[k].anc.is_none() => {
            let orbit = f.balls[o].blade_len;
            anchor::anchor_plant(f, k, w, orbit);
        }
        WeaponCfg::Mortar(w) => {
            // int(O.n_vol + 1e-9): toward zero
            let n = (f.balls[o].n_vol + 1e-9) as i64;
            mortar::mortar_fire(f, k, Some(w), Some(n));
        }
        _ => {}
    }
}

/// `copy_source(k)` (2804 to 2806): the enemy's last ability if its kind is one of [`COPYABLE`], else none.
pub fn source<'a>(f: &Fight<'a>, k: usize) -> Option<&'a AbilityCfg> {
    f.balls[1 - k].last_ab.filter(|src| COPYABLE.contains(&src.kind.name()))
}
