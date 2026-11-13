//! Weapon kind `mortar`, EUPHONIUM's BELL MORTAR: `fight.py` 2659 to 2766 (`mortar_setup`, `predict`, `_shell`,
//! `mortar_fire`, `mortar_tick`, `start_barrage`, `shells_step`), the shells a lance swats (`shell_height`,
//! `swat_shells` 1243 to 1271), and the ability `barrage` (`trigger` 2900 to 2901).
//!
//! Every `every` beats on the 16th grid (haste divides it) the bell lobs a salvo of `int(n_vol)` shells (`n0`, +`grow`
//! per salvo that lands, up to `n_max`; a twin doubles the salvo). They land `flight_beats` later: shell 0 on the
//! enemy's predicted spot then (its motion x `lead`; an anchored enemy is predicted round its whirl), the rest on a
//! ring of `spread` round it (a seeded rotation, one draw a salvo). A landing splashes `splash` (+ R x 0.6 to the
//! ball's centre): `dmg`, knocked away from the impact by `knock` (not while the victim whirls round a peg). Shells
//! cannot be stopped once fired, except by a live lance under the rule `swat_shells`. The ability `barrage` (NOT A
//! TUBA): `n` shells at once, landing over `beats` beats on the 8ths, `aimed` of them at the enemy (jittered), the rest
//! on a jittered grid over the whole ring (a permutation of the grid's cells, then two draws a shell).
//!
//! Events `mortar` (owner, shells, salvo, the aim), `barrage` (owner, shells, salvo), `shell` (owner, where it landed,
//! 1 if it hit, salvo, index, `mortar` or `barrage`), `swat_shell` (the lance's owner, the shell's owner, salvo, index,
//! where it was met); a landing that hits is a `hit` of kind `shell` whose sub-number is the shell's index. Recorded as
//! `SHELLS` while a fighter shells.
//!
//! Everything here is about "ball `k`" and its enemy: IDENTITY CRISIS fires one salvo with the enemy's weapon on the
//! copier's ball ([`mortar_fire`] with `w`), and a copied barrage is [`start_barrage`] with the enemy's ability.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::{Fight, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AbilityCfg, AbilityKind, MortarCfg, WeaponCfg};

/// Weapon kind `mortar`'s per-fight state (`Fight::mortar`): nothing (the volley is the ball's `n_vol`, the shells
/// `Fight::shells`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// Which kind of shell: a salvo's or a barrage's (`Sh['kind']`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellKind {
    /// Fired by `mortar_fire`.
    Mortar,
    /// Fired by `start_barrage`.
    Barrage,
}

impl ShellKind {
    /// Python's name of it.
    pub fn name(self) -> &'static str {
        match self {
            ShellKind::Mortar => "mortar",
            ShellKind::Barrage => "barrage",
        }
    }
}

/// One shell in the air or landed (`_shell` 2686 to 2690): an entry of `Fight::shells`.
#[derive(Clone, Debug, PartialEq)]
pub struct Shell {
    /// The fighter that fired it.
    pub owner: usize,
    /// Where it was fired from (the owner's ball then).
    pub p0: V2,
    /// Where it lands (clipped inside the ropes).
    pub p1: V2,
    /// When it was fired.
    pub t0: f64,
    /// When it lands.
    pub t1: f64,
    /// Its salvo's number.
    pub salvo: i64,
    /// Its index in the salvo.
    pub j: i64,
    /// Damage of its landing.
    pub dmg: f64,
    /// Its splash radius.
    pub splash: f64,
    /// Its knockback.
    pub knock: f64,
    /// A salvo's or a barrage's.
    pub kind: ShellKind,
    /// It has landed or was swatted.
    pub done: bool,
    /// 0 in the air, 1 landed and missed, 2 landed and hit, 3 swatted.
    pub landed: i64,
    /// When a lance swatted it (`Sh['ts']`, rule `swat_shells`).
    pub ts: Option<f64>,
}

/// `B.w['kind'] == 'mortar'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Mortar(_))
}

/// `shelling` (`run()` 5810): a fighter is a mortar, or either fighter has a `barrage` ability. Its shells then land
/// (`shells_step`) and the record has `SHELLS`.
pub fn shelling(f: &Fight) -> bool {
    (0..2).any(|k| is(f, k) || crate::abilities::has(f, k, &AbilityKind::Barrage))
}

/// Fighter `k`'s mortar settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a mortar (Python raises `KeyError` on the first setting it reads).
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a MortarCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Mortar(w) => w,
        other => panic!("fighter {k}'s weapon is {}, not a mortar", other.kind()),
    }
}

/// `mortar_setup(k)` (2668 to 2671), from `run()` 5813 to 5814 (after 5810 to 5812 made `shells`, `next_salvo`,
/// `salvo_landed`: `Fight::shells` and its neighbours, empty by default): the volley starts at `float(w['n0'])`.
pub fn setup(f: &mut Fight, k: usize) {
    f.balls[k].n_vol = weapon(f, k).n0;
}

/// `predict(o, tl)` (2672 to 2684): where ball `o` will be in `tl` s: straight-line motion, or round its whirl while
/// it is anchored (then off along the tangent at the shared speed once the whirl has ended).
pub fn predict(f: &Fight, o: usize, tl: f64) -> V2 {
    let b = &f.balls[o];
    let Some(an) = &b.anc else {
        // O.p + O.v * tl
        return geom::add(b.p, geom::scale(b.v, tl));
    };
    let t_in = py::py_min(tl, py::py_max(0.0, an.t_end - f.t));
    let th = an.th + an.om * t_in;
    let (cs, sn) = (dmath::cos(th), dmath::sin(th));
    // An['c'] + An['orbit'] * np.array([cos(th), sin(th)])
    let mut p = [an.c[0] + an.orbit * cs, an.c[1] + an.orbit * sn];
    if tl > t_in {
        let dir = if an.om >= 0.0 { 1.0 } else { -1.0 };
        // tang = np.array([-sin(th), cos(th)]) * (1.0 if om >= 0 else -1.0); p = p + tang * self.R.V * (tl - t_in)
        let tang = [-sn * dir, cs * dir];
        p = geom::add(p, geom::scale(geom::scale(tang, f.rules.v), tl - t_in));
    }
    p
}

/// `_shell(k, land, t1, salvo, j, dmg, splash, knock, kind)` (2686 to 2690): a shell from ball `k`'s spot now to
/// `land` (clipped 0.2 inside the ropes), landing at `t1`.
#[allow(clippy::too_many_arguments)]
fn shell(f: &mut Fight, k: usize, land: V2, t1: f64, salvo: i64, j: i64, hit: [f64; 3], kind: ShellKind) {
    let lim = f.rules.a - 0.2;
    let [dmg, splash, knock] = hit;
    let sh = Shell {
        owner: k,
        p0: f.balls[k].p,
        p1: [geom::np_clip(land[0], -lim, lim), geom::np_clip(land[1], -lim, lim)],
        t0: f.t,
        t1,
        salvo,
        j,
        dmg,
        splash,
        knock,
        kind,
        done: false,
        landed: 0,
        ts: None,
    };
    f.shells.push(sh);
}

/// `mortar_tick(k, k16)` (2712 to 2720), from `run()` 6025 to 6029: on its grid (haste divides it) a fighter that is
/// not stunned (or sunk in shadow) fires a salvo.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    if stunned(f, k) {
        return;
    }
    let w = weapon(f, k);
    let main16 = py::round_half_even(4.0 * w.every) as i64;
    let ev16 = (py::round_half_even(main16 as f64 / f.ab_mult(k)) as i64).max(1);
    if py::imod_py(k16, ev16) == 0 {
        mortar_fire(f, k, None, None);
    }
}

/// `shells_step()` (2741 to 2766), from `run()` 6157 to 6158 (`if shelling`: a mortar or a `barrage`; see [`shelling`]):
/// every shell whose time has come lands: it hits the enemy of its owner within its splash (+ R x 0.6), knocks it
/// away from the impact unless it whirls round a peg, and the first hit of a salvo grows the owner's volley when the
/// owner is a mortar. Shells landed 0.6 s ago or more are dropped.
pub fn shells_step(f: &mut Fight) {
    let r = f.rules.r;
    for i in 0..f.shells.len() {
        let sh = &f.shells[i];
        if sh.done || f.t + 1e-9 < sh.t1 {
            continue;
        }
        let (owner, p1, dmg, splash, knock, salvo, j, kind) =
            (sh.owner, sh.p1, sh.dmg, sh.splash, sh.knock, sh.salvo, sh.j, sh.kind);
        f.shells[i].done = true;
        let o = 1 - owner;
        let op = f.balls[o].p;
        let rel = geom::sub(op, p1);
        let dist = geom::hypot(rel);
        let hit = dist < splash + r * 0.6;
        let mut landed = false;
        if hit {
            landed = f.damage(owner, o, dmg, "shell", op, j);
            if f.balls[o].anc.is_none() {
                let away = if dist > 1e-3 { rel } else { f.balls[o].v };
                f.knock(o, away, knock);
            }
        }
        f.shells[i].landed = 1 + i64::from(landed);
        let data = vec![
            Value::Float(p1[0]),
            Value::Float(p1[1]),
            Value::Int(i64::from(landed)),
            Value::Int(salvo),
            Value::Int(j),
            Value::Str(kind.name().to_string()),
        ];
        f.ev("shell", owner as i64, data);
        if landed && kind == ShellKind::Mortar && f.salvo_landed.insert(salvo) {
            // the owner's volley grows when its weapon is a mortar (a copied salvo grows nobody's)
            if let WeaponCfg::Mortar(w) = &f.balls[owner].cfg.weapon {
                let b = &mut f.balls[owner];
                b.n_vol = py::py_min(w.n_max, b.n_vol + w.grow);
            }
        }
    }
    let t = f.t;
    f.shells.retain(|sh| t < sh.t1 + 0.6);
}

/// `shell_height(Sh, t)` (1243 to 1249): a shell's height at `t` on its arc and how far along it is (0 to 1): launched
/// from the top of the bell (R x 1.7), the apex growing with the range, down to the canvas at `t1`.
pub fn shell_height(f: &Fight, sh: &Shell, t: f64) -> (f64, f64) {
    let s = py::py_min(1.0, py::py_max(0.0, (t - sh.t0) / py::py_max(1e-6, sh.t1 - sh.t0)));
    let rng = geom::hypot(geom::sub(sh.p1, sh.p0));
    let apex = 2.2 + 0.35 * rng;
    // self.R.R * 1.7 * (1 - s_) + 0.05 * s_ + 4 * apex * s_ * (1 - s_)
    (f.rules.r * 1.7 * (1.0 - s) + 0.05 * s + 4.0 * apex * s * (1.0 - s), s)
}

/// `swat_shells(k, a, d, L_)` (1251 to 1271), from `thrust_step` 755 to 756 (a live lance, the rule `swat_shells`,
/// shells in the air): ball `k`'s lance (base `a`, direction `d`, length `l`) swats every enemy shell coming down
/// through its height band (below `swat_shell_h`) within `swat_shell_r` of it in plan. A swatted shell never lands.
pub fn swat_shells(f: &mut Fight, k: usize, a: V2, d: V2, l: f64) {
    let h_max = f.rules.swat_shell_h();
    let r_max = f.rules.swat_shell_r();
    for i in 0..f.shells.len() {
        let sh = &f.shells[i];
        if sh.owner == k || sh.done || f.t + 1e-9 >= sh.t1 {
            continue;
        }
        let (h, s) = shell_height(f, sh, f.t);
        if s < 0.5 || h > h_max {
            continue;
        }
        // p = Sh['p0'] + (Sh['p1'] - Sh['p0']) * s_
        let p = geom::add(sh.p0, geom::scale(geom::sub(sh.p1, sh.p0), s));
        let rel = geom::sub(p, a);
        // c = a + d * min(max(float(rel @ d), 0.0), L_)
        let c = geom::add(a, geom::scale(d, py::py_min(py::py_max(geom::dot(rel, d), 0.0), l)));
        if geom::hypot(geom::sub(p, c)) < r_max {
            let (owner, salvo, j) = (sh.owner, sh.salvo, sh.j);
            let t = f.t;
            let sh = &mut f.shells[i];
            sh.done = true;
            sh.landed = 3;
            sh.ts = Some(t);
            let data = vec![
                Value::Int(owner as i64),
                Value::Int(salvo),
                Value::Int(j),
                Value::Float(p[0]),
                Value::Float(p[1]),
            ];
            f.ev("swat_shell", k as i64, data);
        }
    }
}

/// `mortar_fire(k, w=None, n=None)` (2691 to 2711): ball `k` fires a salvo of `n` shells (its own volley when `None`),
/// with the weapon settings `w` (its own when `None`); IDENTITY CRISIS's signature copy fires one with the enemy's
/// weapon and volley (2820 to 2821). One draw: the ring's rotation.
pub fn mortar_fire(f: &mut Fight, k: usize, w: Option<&MortarCfg>, n: Option<i64>) {
    let w = match w {
        Some(w) => w.clone(),
        None => weapon(f, k).clone(),
    };
    // int(B.n_vol + 1e-9): toward zero
    let mut n = n.unwrap_or((f.balls[k].n_vol + 1e-9) as i64);
    if f.twin.until[k] > f.t {
        // MULTIPHONICS: two notes at once
        n *= 2;
    }
    let tl = w.flight_beats * f.rules.beat;
    let o = 1 - k;
    let tgt = if f.balls[o].anc.is_some() {
        predict(f, o, tl * w.lead)
    } else {
        // O.p + O.v * tl * w.get('lead', 1.0)
        let b = &f.balls[o];
        geom::add(b.p, geom::scale(geom::scale(b.v, tl), w.lead))
    };
    let salvo = f.next_salvo;
    f.next_salvo += 1;
    let rot = f.rng.uniform(0.0, 2.0 * dmath::PI);
    let t1 = f.t + tl;
    for j in 0..n.max(0) {
        let land = if j == 0 {
            tgt
        } else {
            // a = rot + 2 * math.pi * (j - 1) / max(1, n - 1)
            let a = rot + 2.0 * dmath::PI * (j - 1) as f64 / (n - 1).max(1) as f64;
            geom::add(tgt, geom::scale([dmath::cos(a), dmath::sin(a)], w.spread))
        };
        shell(f, k, land, t1, salvo, j, [w.dmg, w.splash, w.knock], ShellKind::Mortar);
    }
    let data = vec![
        Value::Int(n),
        Value::Int(salvo),
        Value::Float(tgt[0]),
        Value::Float(tgt[1]),
    ];
    f.ev("mortar", k as i64, data);
}

/// `trigger`, kind `barrage` (2900 to 2901): `start_barrage(k, ab)` (2721 to 2740): `n` shells at once from ball `k`,
/// landing `flight_beats` beats later and on over `beats` beats on the 8ths; the first `aimed` at the enemy's
/// predicted spot (two normal draws of jitter each), the rest on the cells of a jittered grid over the whole ring
/// (one permutation of the cells, then two uniform draws a shell).
pub fn start_barrage(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let param = crate::abilities::param;
    // int(ab['n']), max(1, int(round(2 * ab['beats'])))
    let n = param(ab, "n") as i64;
    let steps = (py::round_half_even(2.0 * param(ab, "beats")) as i64).max(1);
    let tl = param(ab, "flight_beats") * f.rules.beat;
    let lim = f.rules.a - 0.45;
    // int(math.ceil(math.sqrt(n)))
    let g = (n as f64).sqrt().ceil() as i64;
    let cells = f.rng.permutation((g * g) as usize);
    let salvo = f.next_salvo;
    f.next_salvo += 1;
    let aimed = ab.get("aimed").unwrap_or(0.0);
    let lead = ab.get("lead").unwrap_or(1.0);
    let hit = [param(ab, "dmg"), param(ab, "splash"), param(ab, "knock")];
    let beat = f.rules.beat;
    for j in 0..n {
        // self.t + tl + (j % steps) * self.R.beat / 2
        let t1 = f.t + tl + py::imod_py(j, steps) as f64 * beat / 2.0;
        let land = if (j as f64) < aimed {
            let p = predict(f, 1 - k, (t1 - f.t) * lead);
            let mut jit = [0.0; 2];
            f.rng.fill_normal(0.0, 0.25, &mut jit);
            geom::add(p, jit)
        } else {
            let c = cells[py::imod_py(j, g * g) as usize] as i64;
            let (cx, cy) = (c / g, c % g);
            let span = |f: &mut Fight, cell: i64| {
                // -lim + (cx + self.rng.uniform(.15, .85)) * 2 * lim / g
                -lim + (cell as f64 + f.rng.uniform(0.15, 0.85)) * 2.0 * lim / g as f64
            };
            let x = span(f, cx);
            let y = span(f, cy);
            [x, y]
        };
        shell(f, k, land, t1, salvo, j, hit, ShellKind::Barrage);
    }
    f.ev("barrage", k as i64, vec![Value::Int(n), Value::Int(salvo)]);
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5816 to 5817: `SHELLS` when shelling.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if shelling(f) { &["SHELLS"] } else { &[] }
}

/// This kind's rows of one frame: `snap`, `SHELLS` (5564 to 5568), into `out` (only the keys [`record_keys`] gave): a
/// ragged list of every shell in the air or landed less than 0.6 s ago, each `(owner, x0, y0, x1, y1, t0, t1, salvo,
/// j, landed)` and, for a swatted shell, its swat time as an 11th field.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let shells = f.shells.iter().map(|sh| {
        let mut row = vec![
            Value::Int(sh.owner as i64),
            Value::Float(sh.p0[0]),
            Value::Float(sh.p0[1]),
            Value::Float(sh.p1[0]),
            Value::Float(sh.p1[1]),
            Value::Float(sh.t0),
            Value::Float(sh.t1),
            Value::Int(sh.salvo),
            Value::Int(sh.j),
            Value::Int(sh.landed),
        ];
        if let Some(ts) = sh.ts {
            row.push(Value::Float(ts));
        }
        Value::List(row)
    });
    out.push_nested("SHELLS", Value::List(shells.collect()));
}
