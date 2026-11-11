//! The fight: `fight.py`'s `class Fight`, its core. Two balls in a square ring, one generator, one event list.
//!
//! Ported here: `Fight.__init__` (lines 264 to 316), `ev`, `damage` and `knock` (319 to 368), `move` for the balls
//! (3128 to 3234: walls, rope give, ball against ball), the level and low-HP bookkeeping of `abilities_step` (3017 to
//! 3032) and `lead_changes` (6225 to 6242). The order of a step is in [`crate::schedule`], the per-frame record in
//! `record.rs`. Weapon kinds and abilities are separate modules of free functions taking `&mut Fight`; their state
//! hangs off [`Fight`] as `Option` fields and they are called only from `schedule.rs`.
//!
//! The Python code checks, in many places, state that only kinds not ported yet can set (a rewinding, swallowed,
//! stunned, anchored, hopping, smashed, walking, grabbed, airborne or strutting ball; a per-ball radius or mass).
//! Each such place is marked with the Python lines it stands for, so the check is added with its kind and nothing
//! else moves.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::fightfile::FightFile;
use bf_core::py;
use bf_core::recording::Recording;
use bf_core::rng::Generator;

use crate::ball::{Ball, Slot};
use crate::geom::{self, V2};
use crate::kinds::striker::Striker;
use crate::record;
use crate::roster::{AbilityKind, FighterCfg, SlowCfg, WeaponCfg};
use crate::rules::{Rules, SimError};
use crate::schedule;

/// Everything that decides a fight: who (left corner first), under which rules, and the seed.
#[derive(Clone, Debug)]
pub struct Setup<'a> {
    /// The two fighters' roster entries, left (red) corner first.
    pub fighters: [&'a FighterCfg; 2],
    /// The rules.
    pub rules: Rules,
    /// The seed of the fight's generator.
    pub seed: u64,
}

/// A ring in flight (an entry of `fight.py`'s `self.waves`, made by `trigger` for the kinds `shockwave` and `wave`).
#[derive(Clone, Debug, PartialEq)]
pub struct Wave {
    /// The fighter that sent it.
    pub owner: usize,
    /// Its centre.
    pub c: V2,
    /// Its radius now.
    pub r: f64,
    /// Units a second it spreads.
    pub speed: f64,
    /// The radius it ends at.
    pub rmax: f64,
    /// Damage to the enemy it passes.
    pub dmg: f64,
    /// Knockback.
    pub knock: f64,
    /// `shockwave` or `wave`: the `hit` event's "what hit".
    pub kind: String,
    /// Whether it has passed the enemy.
    pub hit: bool,
    /// The ability's name.
    pub name: String,
    /// When it started.
    pub t0: f64,
    /// The slow it puts on the enemy it hits, if any.
    pub slow: Option<SlowCfg>,
}

/// A telegraphed slam (an entry of `fight.py`'s `self.slams`, made by `trigger` for the kind `slam`).
#[derive(Clone, Debug, PartialEq)]
pub struct Slam {
    /// The fighter that called it.
    pub owner: usize,
    /// Where it lands.
    pub c: V2,
    /// When it was called.
    pub t0: f64,
    /// When it lands.
    pub t_slam: f64,
    /// Its radius.
    pub radius: f64,
    /// Damage to an enemy inside.
    pub dmg: f64,
    /// Knockback.
    pub knock: f64,
    /// Whether it has landed.
    pub done: bool,
    /// The ability's name.
    pub name: String,
}

/// One fight in progress.
#[derive(Clone, Debug)]
pub struct Fight<'a> {
    /// The rules (`self.R`).
    pub rules: Rules,
    /// The fight's own generator: the launch, then every rope bounce, draw from it in order.
    pub rng: Generator,
    /// The seed.
    pub seed: u64,
    /// The two balls, left corner first.
    pub balls: [Ball<'a>; 2],
    /// Whether per-frame state is recorded.
    pub record: bool,
    /// Where each ball starts: red at (-A + 1.15, A - 1.15), blue mirrored.
    pub corner: [V2; 2],
    /// Whether a fighter thrusts, curves its shots or slows: the record then has `EXT`, `SLW` and `CRV`.
    pub uses_ext: bool,
    /// Rings in flight.
    pub waves: Vec<Wave>,
    /// Slams called and not yet forgotten.
    pub slams: Vec<Slam>,
    /// Every event so far, in the order they happened.
    pub events: Vec<Event>,
    /// The time of the current step, in seconds on the video clock.
    pub t: f64,
    /// The winner, once there is a K.O.
    pub winner: Option<usize>,
    /// The time of the K.O.
    pub t_ko: Option<f64>,
    /// Each striker's baskets and cobras, by fighter (`B.baskets`, `B.next_bk`, `B.dance_until`, `B.t_strike`):
    /// `None` for a fighter that is not a striker, and for every fighter until the fight loop sets the kinds up
    /// ([`crate::kinds::striker::setup`]), as the Python ball has no `baskets` before `striker_setup`.
    pub striker: [Option<Striker>; 2],
    // SLOT (Tasks 13 to 14): each further kind's own state, as `pub <kind>: [Option<...>; 2]` or `Option<...>`,
    // `None` when no fighter has the kind. `fight.py` lines 286 to 312 list the Python fields (projectile arrays,
    // arcs, beams, hazards, ...); FLUTE and TROMBONE need none of those.
}

impl<'a> Fight<'a> {
    /// `Fight(a, b, seed, rules, record)` (`fight.py` 264 to 316): both fighters in their corners, launched toward
    /// the centre within `LAUNCH_SPREAD` degrees either side, each weapon at a seeded angle. Four draws from the
    /// generator, in this order: fighter 0's launch angle, its weapon angle, then fighter 1's two.
    pub fn new(setup: &Setup<'a>, record: bool) -> Fight<'a> {
        let rules = setup.rules.clone();
        let mut rng = Generator::new(setup.seed);
        let mut balls = [
            Ball::new(setup.fighters[0], 0, &rules),
            Ball::new(setup.fighters[1], 1, &rules),
        ];
        let a = rules.a;
        // corners: red = (-A, +A) (screen left), blue = (+A, -A) (screen right) in the ring frame
        let corner = [[-a + 1.15, a - 1.15], [a - 1.15, -a + 1.15]];
        for (bl, c) in balls.iter_mut().zip(corner) {
            bl.p = c;
            // to_c = -bl.p / np.linalg.norm(bl.p)
            let n = geom::norm(bl.p);
            let to_c = [-bl.p[0] / n, -bl.p[1] / n];
            let ang = dmath::atan2(to_c[1], to_c[0]) + geom::radians(rng.uniform(-1.0, 1.0) * rules.launch_spread);
            bl.v = [rules.v * dmath::cos(ang), rules.v * dmath::sin(ang)];
            bl.th = rng.uniform(0.0, 2.0 * dmath::PI);
        }
        // thrust: the closed lance starts aimed at the enemy
        for k in 0..2 {
            if balls[k].thrusts() {
                let d = geom::sub(balls[1 - k].p, balls[k].p);
                balls[k].th = py::fmod_py(dmath::atan2(d[1], d[0]), 2.0 * dmath::PI);
            }
        }
        // fight.py 283 to 284. The third reason, a volley with `curve_grow`, is an option of a kind not ported.
        let uses_ext = balls.iter().any(|bl| {
            bl.thrusts()
                || [Slot::Unlock1, Slot::Unlock2, Slot::Low]
                    .iter()
                    .any(|s| s.of(bl.cfg).slow.is_some())
        });
        Fight {
            rules,
            rng,
            seed: setup.seed,
            balls,
            record,
            corner,
            uses_ext,
            waves: Vec::new(),
            slams: Vec::new(),
            events: Vec::new(),
            t: 0.0,
            winner: None,
            t_ko: None,
            striker: [None, None],
        }
    }

    /// `ab_mult(k)` (`fight.py` 2827 to 2828): how many times faster fighter `k`'s weapon grid runs: its
    /// `haste_mult` while its haste lasts, else 1.0.
    pub fn ab_mult(&self, k: usize) -> f64 {
        if self.balls[k].haste > 0.0 {
            self.balls[k].haste_mult
        } else {
            1.0
        }
    }

    /// `ev(kind, who, *data)`: appends the event `(round(t, 5), kind, who, *data)`. `who` is a fighter index, or -1
    /// for a `clash`. The data keep Python's types: a `Value::Int` where Python has an int, a `Value::Float` where
    /// it has a float.
    pub fn ev(&mut self, kind: &str, who: i64, data: Vec<Value>) {
        let mut fields = Vec::with_capacity(1 + data.len());
        fields.push(Value::Int(who));
        fields.extend(data);
        self.events.push(Event {
            t: py::round_nd(self.t, 5),
            kind: kind.to_string(),
            fields,
        });
    }

    /// `damage(att, vic, dmg, kind, x, y, sub)`: applies a hit (unless shielded) and returns whether it landed. A
    /// landed hit takes `dmg * dmg_scale` (times the attacker's `dmg_side` entry) off the victim, adds it to the
    /// attacker's `dealt`, and emits `hit`; a shielded victim emits `block` and loses nothing.
    ///
    /// `at` is the contact point, Python's `x, y`. `sub` is the hit's sub-number (a blade's fret count, a cobra's
    /// basket id), 0 where Python leaves it out.
    pub fn damage(&mut self, att: usize, vic: usize, dmg: f64, kind: &str, at: V2, sub: i64) -> bool {
        let [x, y] = at;
        // fight.py 325 to 334: a victim that is rewinding, sunk in shadow, smashed or airborne (swan, dive) cannot be
        // hit. Kinds not ported.
        if self.balls[vic].shield > 0.0 {
            let data = vec![
                Value::Int(att as i64),
                Value::Float(x),
                Value::Float(y),
                Value::Str(kind.to_string()),
            ];
            self.ev("block", vic as i64, data);
            return false;
        }
        let mut d = dmg * self.rules.dmg_scale;
        if let Some(hc) = self.rules.dmg_side {
            // opt-in (#36, crossovers): per-fighter damage handicap
            d *= hc[att];
        }
        // fight.py 342 to 346: a resting victim's `guard` (#42) and a detuned attacker (#37). Kinds not ported.
        self.balls[vic].hp -= d;
        self.balls[att].dealt += d;
        let data = vec![
            Value::Int(vic as i64),
            Value::Float(d),
            Value::Float(x),
            Value::Float(y),
            Value::Str(kind.to_string()),
            Value::Int(sub),
        ];
        self.ev("hit", att as i64, data);
        true
    }

    /// `knock(vic, away, k)`: bends the victim's heading toward `away` (0.8 of the unit `away`, 0.2 of its own
    /// heading), back at the shared speed, and raises its speed boost to `k` if that is more.
    pub fn knock(&mut self, vic: usize, away: V2, k: f64) {
        // fight.py 355 to 360: a walking bass, an airborne swan, a strut or a stage dive cannot be knocked. Kinds
        // not ported.
        let v_shared = self.rules.v;
        let b = &mut self.balls[vic];
        let n = geom::norm(away);
        if n < 1e-9 {
            return;
        }
        let away = geom::div(away, n);
        let cur = geom::div(b.v, geom::norm(b.v) + 1e-12);
        let d = geom::add(geom::scale(away, 0.8), geom::scale(cur, 0.2));
        b.v = geom::scale(geom::div(d, geom::norm(d) + 1e-12), v_shared);
        b.boost = geom::py_max(b.boost, k);
    }

    /// `move(dt)` for the balls (`fight.py` 3128 to 3234): each ball flies at `V * (1 + boost)` (times its slow),
    /// bounces off the ropes with a seeded give, and the two clash when they overlap.
    ///
    /// The generator is drawn once per rope bounce, in ball order and, for a ball that crosses two ropes in one step,
    /// the x rope first.
    pub fn move_step(&mut self, dt: f64) {
        let (a, r) = (self.rules.a, self.rules.r);
        for k in 0..2 {
            // fight.py 3131 to 3150: a ball that another step drives is skipped here (rewinding, swallowed, stunned,
            // anchored, mid-hop, smashed, mid-step, grabbed or pinned, airborne, strutting or diving). Kinds not
            // ported.
            let b = &mut self.balls[k];
            b.boost *= dmath::exp(-dt / self.rules.knock_t);
            let mut s = self.rules.v * (1.0 + b.boost);
            // fight.py 3153 to 3156: a hopper or a planted walker only slides when knocked. Kinds not ported.
            if b.slow > 0.0 {
                // #34: slowed (e.g. SAD TROMBONE)
                s *= b.slow_mult;
            }
            // fight.py 3159 to 3160: a per-ball speed factor `vmul` (inflate). Kind not ported.
            // v = B.v / (np.linalg.norm(B.v) + 1e-12) * s
            let n = geom::norm(b.v) + 1e-12;
            let v = [b.v[0] / n * s, b.v[1] / n * s];
            b.p = [b.p[0] + v[0] * dt, b.p[1] + v[1] * dt];
            b.v = v;
            for ax in 0..2 {
                let b = &mut self.balls[k];
                // fight.py 3165: a per-ball radius `rad` (inflate) would replace R here. Kind not ported.
                let lim = a - r;
                if b.p[ax].abs() > lim {
                    b.p[ax] = lim.copysign(b.p[ax]) * 2.0 - b.p[ax];
                    b.v[ax] = -b.v[ax];
                    let g = geom::radians(self.rng.normal(0.0, self.rules.rope_give * 0.6));
                    let (c, s_) = (dmath::cos(g), dmath::sin(g));
                    let nv = [c * b.v[0] - s_ * b.v[1], s_ * b.v[0] + c * b.v[1]];
                    if geom::sign(nv[ax]) == geom::sign(b.v[ax]) {
                        // never bend back into the rope
                        b.v = nv;
                    }
                    let side = if b.p[ax].is_sign_negative() { -1 } else { 1 };
                    let data = vec![
                        Value::Int(ax as i64),
                        Value::Int(side),
                        Value::Float(b.p[0]),
                        Value::Float(b.p[1]),
                        Value::Float(s),
                    ];
                    self.ev("rope", k as i64, data);
                }
            }
        }
        // ball vs ball
        let d = geom::sub(self.balls[1].p, self.balls[0].p);
        let dist = geom::hypot(d);
        // fight.py 3179: per-ball radii (inflate) would replace R + R. Lines 3180 to 3187: no clash with a ball that
        // is rewinding, swallowed, in shadow, mid-hop, smashed, grabbed, airborne or diving. Kinds not ported.
        let (r_p, r_q) = (r, r);
        if dist < r_p + r_q {
            let m = geom::py_max(dist, 1e-9);
            let n = [d[0] / m, d[1] / m];
            let over = r_p + r_q - dist;
            // fight.py 3190 to 3216: a walker (or a strutter) that does not give way, and unequal masses (inflate).
            // Kinds not ported; what follows is the equal-mass branch (3217 to 3223).
            let v_shared = self.rules.v;
            let [p, q] = &mut self.balls;
            p.p = [p.p[0] - n[0] * over / 2.0, p.p[1] - n[1] * over / 2.0];
            q.p = [q.p[0] + n[0] * over / 2.0, q.p[1] + n[1] * over / 2.0];
            let (vp, vq) = (geom::dot(p.v, n), geom::dot(q.v, n));
            if vp - vq > 0.0 {
                p.v = [p.v[0] + (vq - vp) * n[0], p.v[1] + (vq - vp) * n[1]];
                q.v = [q.v[0] + (vp - vq) * n[0], q.v[1] + (vp - vq) * n[1]];
            }
            for b in [&mut *p, &mut *q] {
                // B.v = B.v / (np.linalg.norm(B.v) + 1e-12) * self.R.V * (1 + B.boost)
                let nb = geom::norm(b.v) + 1e-12;
                b.v = [
                    b.v[0] / nb * v_shared * (1.0 + b.boost),
                    b.v[1] / nb * v_shared * (1.0 + b.boost),
                ];
            }
            let mid = [(p.p[0] + q.p[0]) / 2.0, (p.p[1] + q.p[1]) / 2.0];
            // fight.py 3227 to 3231: a walker's clash is announced once per push (`wclash_t`). Kind not ported.
            self.ev("clash", -1, vec![Value::Float(mid[0]), Value::Float(mid[1])]);
            // fight.py 3233 to 3234: `clash_hooks` (inflate's contact damage). Kind not ported.
        }
        // SLOT (projectile kinds: volley, arc, roll): fight.py 3235 to 3278, in this order: `steer`, the projectiles'
        // flight, `ricochet`, projectiles that leave the ring (`kill_keys(out, 'wall')`), projectiles that hit a
        // ball (`damage(..., 'key', ...)`, volley growth, the nudge). FLUTE and TROMBONE fire none.
    }

    /// `abilities_step(on_beat)` (`fight.py` 3012 to 3053), the bookkeeping: levels follow the damage dealt (each
    /// level-up makes its unlock ready from now and emits `level`), and on a beat a fighter below `LOW_HP` has its
    /// `low` turn, once.
    pub fn abilities_step(&mut self, on_beat: bool) {
        // SLOT (ability kind `beam`, not one of FLUTE's or TROMBONE's): fight.py 3013 to 3016, a charged ability beam
        // fires first, for both fighters, before the loop below.
        for k in 0..2 {
            // unlocks by damage dealt
            let (lv2, lv3) = self.rules.lv_dmg;
            let dealt = self.balls[k].dealt;
            let lv = 1 + i64::from(dealt >= lv2) + i64::from(dealt >= lv3);
            while self.balls[k].level < lv {
                let b = &mut self.balls[k];
                b.level += 1;
                let slot = if b.level == 2 { Slot::Unlock1 } else { Slot::Unlock2 };
                b.ready[slot.index()] = Some(self.t);
                let level = b.level;
                self.ev("level", k as i64, vec![Value::Int(level)]);
            }
            if !on_beat {
                continue;
            }
            // fight.py 3027 to 3028: a smashed fighter (#44) starts nothing. Kind not ported.
            let b = &mut self.balls[k];
            if !b.low_done && b.hp < self.rules.low_hp {
                b.low_done = true;
                // SLOT (Task 14): `self.trigger(k, 'low')`, fight.py 3031, goes here, before the `continue`.
                continue;
            }
            // SLOT (Task 14): the unlock loop, fight.py 3033 to 3053. For `slot` in (unlock1, unlock2), in that
            // order: `r = B.ready[slot]`; if `r` is set and `self.t >= r - 1e-6`: `trigger(k, slot)`, then
            // `B.ready[slot] = self.t + every_bars * 4 * beat`, then stop (one ability per fighter per beat). The
            // waits in between (3035 to 3049: rewind, snip, hook, slap, copy) belong to kinds not ported.
        }
    }
}

/// Runs one fight to its K.O. (or to `t_max`) and returns its recording, made by this app
/// ([`bf_core::recording::Source::studio`]). With `record` false the recording has no per-frame arrays: only the
/// events, the winner and the K.O. time (the fight itself is the same).
///
/// # Panics
/// When the rules cannot be run ([`Rules::check`]): [`setup_from`] and [`Rules::apply`] refuse such rules, so this
/// needs rules whose fields were set by hand.
pub fn run(setup: &Setup, record: bool) -> Recording {
    if let Err(e) = setup.rules.check() {
        panic!("the engine cannot run these rules: {e}");
    }
    let mut fight = Fight::new(setup, record);
    let frames = schedule::run(&mut fight);
    record::recording(fight, frames)
}

/// The setup of a fight file's fight with this seed, as the kit builds it (`kit/story.py` `rules` and `run_seed`):
/// the rules at the file's tempo grid, its damage scale and handicap, then its pair rules.
///
/// Errors name the fight file's field: a fighter that is not in `roster`, a fighter whose weapon or abilities the
/// engine has not ported, a pair rule the engine does not have.
pub fn setup_from<'a>(ff: &FightFile, roster: &'a [FighterCfg], seed: u64) -> Result<Setup<'a>, SimError> {
    if ff.fighters.len() != 2 {
        return Err(SimError::new(
            "fighters",
            format!("a fight has 2 fighters, this file has {}", ff.fighters.len()),
        ));
    }
    if ff.handicap.len() != 2 {
        return Err(SimError::new(
            "handicap",
            format!("one entry per fighter (2), this file has {}", ff.handicap.len()),
        ));
    }
    let find = |k: usize| -> Result<&'a FighterCfg, SimError> {
        let name = &ff.fighters[k];
        let field = format!("fighters[{k}]");
        let cfg = roster
            .iter()
            .find(|f| &f.name == name)
            .ok_or_else(|| SimError::new(&field, format!("{name:?} is not in the roster")))?;
        if let WeaponCfg::Unsupported(kind) = &cfg.weapon {
            return Err(SimError::new(
                &field,
                format!("the engine has not ported {name}'s weapon kind {kind:?} yet"),
            ));
        }
        for ab in [&cfg.unlock1, &cfg.unlock2, &cfg.low] {
            if let AbilityKind::Unsupported(kind) = &ab.kind {
                return Err(SimError::new(
                    &field,
                    format!(
                        "the engine has not ported {name}'s ability {} (kind {kind:?}) yet",
                        ab.name
                    ),
                ));
            }
        }
        Ok(cfg)
    };
    let fighters = [find(0)?, find(1)?];

    // kit/story.py rules(): Rules(bpm=T.BPM, t_fight=T.T_FIGHT, fps=T.FPS, dmg_scale=T.DMG_SCALE,
    //                             dmg_side=tuple(T.DMG_SIDE), **T.RULES)
    let grid = ff.grid();
    let mut rules = Rules::new(grid.bpm, grid.t_fight);
    rules.fps = grid.fps;
    rules.dmg_scale = ff.dmg_scale;
    rules.dmg_side = Some([ff.handicap[0], ff.handicap[1]]);
    // an error names the fight file's field, not the engine's rule
    rules.check().map_err(|e| {
        let field = match e.field.as_str() {
            "rules.dmg_side" => "handicap",
            "rules.dmg_scale" => "dmg_scale",
            _ => "tempo_bpm",
        };
        SimError::new(field, e.message)
    })?;
    rules.apply(&ff.pair_rules).map_err(|e| {
        let rule = e.field.strip_prefix("rules.").unwrap_or(&e.field);
        if ff.pair_rules.contains_key(rule) {
            SimError::new(format!("pair_rules.{rule}"), e.message)
        } else {
            e
        }
    })?;
    Ok(Setup { fighters, rules, seed })
}

/// `lead_changes(events, hp0=100.0, hyst=5.0, t_from=4.0)` (`fight.py` 6225 to 6242): how often the HP lead flips,
/// from the event list, and when. A fighter leads when it is more than `hyst` HP ahead; the first lead and any flip
/// before `t_from` are not counted. Python's defaults are [`LEAD_HP0`], [`LEAD_HYST`] and [`LEAD_T_FROM`].
///
/// A `hit` or `heal` event the engine could not have made (no fighter index 0 or 1, no amount) is passed over,
/// where Python raises.
pub fn lead_changes(events: &[Event], hp0: f64, hyst: f64, t_from: f64) -> (u32, Vec<f64>) {
    let side = |v: Option<&Value>| match v {
        Some(Value::Int(k @ 0..=1)) => Some(*k as usize),
        _ => None,
    };
    let amount = |v: Option<&Value>| match v {
        Some(Value::Float(x)) => Some(*x),
        Some(Value::Int(n)) => Some(*n as f64),
        _ => None,
    };
    let mut hp = [hp0, hp0];
    let (mut lead, mut n, mut times) = (0i32, 0u32, Vec::new());
    for e in events {
        // e[2] is fields[0]
        if e.kind == "heal" {
            // ability kind 'rewind' gives back damage taken
            if let (Some(who), Some(x)) = (side(e.fields.first()), amount(e.fields.get(1))) {
                hp[who] += x;
            }
            continue;
        }
        if e.kind == "hit" {
            let (Some(vic), Some(d)) = (side(e.fields.get(1)), amount(e.fields.get(2))) else {
                continue;
            };
            hp[vic] -= d;
            let d = hp[0] - hp[1];
            let new = if d > hyst {
                1
            } else if d < -hyst {
                -1
            } else {
                lead
            };
            if new != lead && new != 0 {
                if lead != 0 && e.t >= t_from {
                    n += 1;
                    times.push(e.t);
                }
                lead = new;
            }
        }
    }
    (n, times)
}

/// `lead_changes`' default `hp0`.
pub const LEAD_HP0: f64 = 100.0;
/// `lead_changes`' default `hyst`.
pub const LEAD_HYST: f64 = 5.0;
/// `lead_changes`' default `t_from`.
pub const LEAD_T_FROM: f64 = 4.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    /// Brief, step 1: after `Fight::new` with seed 392, both balls' `p`, `v` and `th` equal frame `f0` of the
    /// Python recording to 1e-12.
    #[test]
    fn launch_matches_python() {
        let roster = testing::roster();
        let py = testing::python_recording("ep392.bfrec");
        let fight = Fight::new(&testing::episode_setup(&roster, 392), true);
        let (p, v, th) = (
            testing::f64s(&py, "P"),
            testing::f64s(&py, "V"),
            testing::f64s(&py, "TH"),
        );
        for k in 0..2 {
            let b = &fight.balls[k];
            for i in 0..2 {
                assert!((b.p[i] - p[2 * k + i]).abs() <= 1e-12, "p[{k}][{i}] = {}", b.p[i]);
                assert!((b.v[i] - v[2 * k + i]).abs() <= 1e-12, "v[{k}][{i}] = {}", b.v[i]);
            }
            assert!((b.th - th[k]).abs() <= 1e-12, "th[{k}] = {}", b.th);
        }
        // the corners are exact, and they are what the recording holds
        assert_eq!(fight.corner, [[-1.85, 1.85], [1.85, -1.85]]);
        assert_eq!(fight.corner.to_vec(), py.corner);
        assert_eq!([fight.balls[0].p, fight.balls[1].p], fight.corner);
        // the lance starts aimed at the enemy (3 pi / 4 from the blue corner); the flute keeps its seeded angle
        assert!((fight.balls[1].th - 0.75 * dmath::PI).abs() <= 1e-15);
        // both fly at the shared speed
        for b in &fight.balls {
            assert!((geom::norm(b.v) - 2.6).abs() <= 1e-12);
        }
        assert!(fight.uses_ext, "the lance thrusts and SAD TROMBONE slows");
        assert_eq!((fight.t, fight.winner, fight.t_ko), (0.0, None, None));
        assert!(fight.events.is_empty());
    }

    #[test]
    fn the_launch_is_seeded() {
        let roster = testing::roster();
        let a = Fight::new(&testing::episode_setup(&roster, 1), false);
        let b = Fight::new(&testing::episode_setup(&roster, 1), false);
        let c = Fight::new(&testing::episode_setup(&roster, 2), false);
        assert_eq!(a.balls[0].v, b.balls[0].v);
        assert_eq!(a.balls[0].th.to_bits(), b.balls[0].th.to_bits());
        assert_ne!(a.balls[0].v, c.balls[0].v);
        // within LAUNCH_SPREAD (38 degrees) of the line to the centre, which is 45 degrees below the x axis for red
        let ang = dmath::atan2(a.balls[0].v[1], a.balls[0].v[0]);
        assert!((ang + 0.25 * dmath::PI).abs() <= geom::radians(38.0) + 1e-12);
        // bare fighters neither thrust nor slow
        let bare = testing::bare_pair();
        assert!(!Fight::new(&testing::bare_setup(&bare, 1), false).uses_ext);
    }

    /// Brief, step 1: an event's time is Python's `round(t, 5)`, which rounds the exact decimal value (ties to
    /// even), not `floor(t * 1e5 + 0.5) / 1e5`. The expected values are what Python gave.
    #[test]
    fn event_times_are_rounded_to_5_places() {
        let roster = testing::roster();
        let mut fight = Fight::new(&testing::episode_setup(&roster, 392), false);
        for (t, want) in [
            (3.59656862745098, 3.59657),
            (4.471568627450978, 4.47157),
            (1.000005, 1.00001),
            (12.345675, 12.34567),
            (3.0000050000000003, 3.00001),
            (5e-06, 1e-05),
            (57.99999999, 58.0),
            (2.675, 2.675),
        ] {
            fight.t = t;
            fight.ev("rope", 0, vec![]);
            let e = fight.events.last().expect("an event was added");
            assert_eq!(e.t.to_bits(), f64::to_bits(want), "round({t}, 5) gave {}", e.t);
            assert_eq!((e.kind.as_str(), e.fields.as_slice()), ("rope", &[Value::Int(0)][..]));
        }
        // `who` comes first, then the data in order, with their types kept
        fight.t = 4.0;
        fight.ev("clash", -1, vec![Value::Float(0.5), Value::Float(-0.25)]);
        let e = fight.events.last().expect("an event was added");
        assert_eq!(e.fields, vec![Value::Int(-1), Value::Float(0.5), Value::Float(-0.25)]);
        assert_eq!(e.who(), None);
        assert_eq!(fight.events.len(), 9);
    }

    fn ev(t: f64, kind: &str, fields: Vec<Value>) -> Event {
        Event {
            t,
            kind: kind.into(),
            fields,
        }
    }

    /// The numbers are what `Fight.damage` gave in Python under the episode's rules (`dmg_side = (2.0, 0.4)`).
    #[test]
    fn damage_scales_by_side_and_a_shield_blocks() {
        use Value::{Float as F, Int as I};
        let roster = testing::roster();
        let mut fight = Fight::new(&testing::episode_setup(&roster, 392), false);
        fight.t = 5.0;
        assert!(fight.damage(0, 1, 1.08, "fang", [0.25, -0.5], 3));
        assert!(fight.damage(1, 0, 3.5, "blade", [0.1, 0.2], 1));
        fight.balls[0].shield = 0.5;
        assert!(!fight.damage(1, 0, 3.5, "blade", [0.1, 0.2], 1));
        let s = |x: &str| Value::Str(x.into());
        assert_eq!(
            fight.events,
            vec![
                ev(5.0, "hit", vec![I(0), I(1), F(2.16), F(0.25), F(-0.5), s("fang"), I(3)]),
                ev(
                    5.0,
                    "hit",
                    vec![I(1), I(0), F(1.4000000000000001), F(0.1), F(0.2), s("blade"), I(1)]
                ),
                ev(5.0, "block", vec![I(0), I(1), F(0.1), F(0.2), s("blade")]),
            ]
        );
        assert_eq!((fight.balls[0].hp, fight.balls[1].hp), (98.6, 97.84));
        assert_eq!((fight.balls[0].dealt, fight.balls[1].dealt), (2.16, 1.4000000000000001));
        // without a handicap only the shared scale counts
        let bare = testing::bare_pair();
        let mut setup = testing::bare_setup(&bare, 1);
        setup.rules.dmg_scale = 3.0;
        let mut fight = Fight::new(&setup, false);
        assert!(fight.damage(0, 1, 2.0, "wave", [0.0, 0.0], 0));
        assert_eq!((fight.balls[1].hp, fight.balls[0].dealt), (94.0, 6.0));
    }

    /// The numbers are what `Fight.knock` gave in Python.
    #[test]
    fn knock_bends_the_heading_and_keeps_the_larger_boost() {
        let roster = testing::roster();
        let mut fight = Fight::new(&testing::episode_setup(&roster, 392), false);
        fight.balls[1].v = [1.0, -2.4];
        fight.knock(1, [0.3, 0.4], 0.9);
        let close = |got: V2, want: V2| (got[0] - want[0]).abs() <= 1e-12 && (got[1] - want[1]).abs() <= 1e-12;
        assert!(
            close(fight.balls[1].v, [2.0127843481929473, 1.6458126162023128]),
            "{:?}",
            fight.balls[1].v
        );
        assert_eq!(fight.balls[1].boost, 0.9);
        fight.knock(1, [-3.0, 0.5], 0.5);
        assert!(
            close(fight.balls[1].v, [-2.40822796135027, 0.9800194315170394]),
            "{:?}",
            fight.balls[1].v
        );
        assert_eq!(fight.balls[1].boost, 0.9, "a weaker knock does not lower the boost");
        // back at the shared speed (to the 1e-12 the division guards with)
        assert!((geom::norm(fight.balls[1].v) - 2.6).abs() <= 1e-11);
        // no direction, no knock
        let before = fight.balls[1].clone();
        fight.knock(1, [0.0, 1e-10], 5.0);
        assert_eq!((fight.balls[1].v, fight.balls[1].boost), (before.v, before.boost));
    }

    #[test]
    fn levels_follow_the_damage_dealt() {
        use Value::Int as I;
        let bare = testing::bare_pair();
        let mut fight = Fight::new(&testing::bare_setup(&bare, 1), false);
        fight.t = 7.0;
        fight.balls[0].dealt = 13.999;
        fight.abilities_step(false);
        assert_eq!(fight.balls[0].level, 1);
        assert!(fight.events.is_empty());
        fight.balls[0].dealt = 14.0;
        fight.abilities_step(false);
        assert_eq!((fight.balls[0].level, fight.balls[0].ready), (2, [Some(7.0), None]));
        // a second look changes nothing
        fight.abilities_step(false);
        assert_eq!(fight.events, vec![ev(7.0, "level", vec![I(0), I(2)])]);
        // one big hit can cross both thresholds in one step: two events, both unlocks ready from now
        fight.t = 8.25;
        fight.balls[1].dealt = 40.0;
        fight.abilities_step(false);
        assert_eq!(
            (fight.balls[1].level, fight.balls[1].ready),
            (3, [Some(8.25), Some(8.25)])
        );
        assert_eq!(
            fight.events[1..],
            [ev(8.25, "level", vec![I(1), I(2)]), ev(8.25, "level", vec![I(1), I(3)])]
        );
        assert_eq!(fight.balls[0].level, 2);
    }

    #[test]
    fn low_hp_is_noted_once_and_only_on_a_beat() {
        let bare = testing::bare_pair();
        let mut fight = Fight::new(&testing::bare_setup(&bare, 1), false);
        fight.balls[0].hp = 25.0;
        fight.abilities_step(true);
        assert!(!fight.balls[0].low_done, "25.0 is not below LOW_HP");
        fight.balls[0].hp = 24.9;
        fight.abilities_step(false);
        assert!(!fight.balls[0].low_done, "not between beats");
        fight.abilities_step(true);
        assert!(fight.balls[0].low_done);
        assert!(!fight.balls[1].low_done);
    }

    /// `ab_mult`: the haste factor counts only while haste lasts, and only for the hasted fighter.
    #[test]
    fn ab_mult_is_the_haste_factor_while_haste_lasts() {
        let bare = testing::bare_pair();
        let mut fight = Fight::new(&testing::bare_setup(&bare, 1), false);
        assert_eq!((fight.ab_mult(0), fight.ab_mult(1)), (1.0, 1.0));
        fight.balls[1].haste_mult = 2.0;
        assert_eq!(fight.ab_mult(1), 1.0, "a factor without haste time left does nothing");
        fight.balls[1].haste = 0.25;
        assert_eq!((fight.ab_mult(0), fight.ab_mult(1)), (1.0, 2.0));
        fight.balls[1].haste = 0.0;
        assert_eq!(fight.ab_mult(1), 1.0);
    }

    /// A free ball flies straight at the shared speed. (The step divides by `norm + 1e-12`, so the speed is the
    /// shared one to about 1e-12, not to the last bit.)
    #[test]
    fn a_free_ball_flies_straight() {
        let bare = testing::bare_pair();
        let mut fight = Fight::new(&testing::bare_setup(&bare, 5), false);
        let (p0, v0) = (fight.balls[0].p, fight.balls[0].v);
        let dt = fight.rules.dt;
        fight.move_step(dt);
        let b = &fight.balls[0];
        assert_ne!(b.p, p0, "the ball moved");
        for i in 0..2 {
            assert!((b.v[i] - v0[i]).abs() <= 1e-11);
            assert!((b.p[i] - (p0[i] + v0[i] * dt)).abs() <= 1e-13);
        }
        assert!(fight.events.is_empty());
    }

    /// A ball past a rope is mirrored back inside, turned round, bent by the seeded give, and a `rope` event says
    /// where. The give is `radians(normal(0, ROPE_GIVE * 0.6))` from the fight's own generator.
    #[test]
    fn a_rope_bounce_mirrors_the_ball_and_draws_the_give() {
        use Value::{Float as F, Int as I};
        let bare = testing::bare_pair();
        let mut fight = Fight::new(&testing::bare_setup(&bare, 5), false);
        let mut rng = fight.rng.clone();
        fight.t = 9.0;
        // far from the other ball, heading straight at the right rope (lim = A - R = 2.4)
        fight.balls[1].p = [-2.0, -2.0];
        fight.balls[1].v = [-2.6, 0.0];
        fight.balls[0].p = [2.39, 1.0];
        fight.balls[0].v = [2.6, 0.0];
        let dt = fight.rules.dt;
        fight.move_step(dt);
        let b = &fight.balls[0];
        let x = 2.39 + 2.6 * dt;
        let want_x = 2.4 * 2.0 - x;
        assert!((b.p[0] - want_x).abs() <= 1e-13 && b.p[0] < 2.4, "{}", b.p[0]);
        assert_eq!(b.p[1], 1.0);
        let g = geom::radians(rng.normal(0.0, fight.rules.rope_give * 0.6));
        assert!(g != 0.0 && g.abs() < 0.5, "a small bend: {g}");
        let want_v = [-2.6 * dmath::cos(g), -2.6 * dmath::sin(g)];
        assert!(
            (b.v[0] - want_v[0]).abs() <= 1e-11 && (b.v[1] - want_v[1]).abs() <= 1e-11,
            "{:?} vs {want_v:?}",
            b.v
        );
        assert_eq!(
            fight.events,
            vec![ev(9.0, "rope", vec![I(0), I(0), I(1), F(b.p[0]), F(b.p[1]), F(2.6)])]
        );
        // exactly one draw was taken
        assert_eq!(fight.rng.random().to_bits(), rng.random().to_bits());
        // a slowed ball moves at V * slow_mult, and the event carries that speed
        fight.balls[0].p = [0.0, -2.39];
        fight.balls[0].v = [0.0, -2.6];
        fight.balls[0].slow = 1.0;
        fight.balls[0].slow_mult = 0.5;
        fight.move_step(dt);
        let e = fight.events.last().expect("a second bounce");
        assert_eq!(e.fields[..3], [I(0), I(1), I(-1)]);
        assert_eq!(e.fields[5], F(1.3));
        assert!((fight.balls[0].p[1] - (-4.8 - (-2.39 - 1.3 * dt))).abs() <= 1e-13);
        assert!((geom::norm(fight.balls[0].v) - 1.3).abs() <= 1e-11);
    }

    /// Two balls that overlap are pushed apart by half the overlap each, swap their speeds along the line between
    /// them, go back to the shared speed, and a `clash` event (who = -1) marks the midpoint.
    #[test]
    fn overlapping_balls_clash() {
        use Value::{Float as F, Int as I};
        let bare = testing::bare_pair();
        let mut fight = Fight::new(&testing::bare_setup(&bare, 5), false);
        fight.t = 6.0;
        let dt = fight.rules.dt;
        // head on along x, meeting at the origin: after the step they overlap
        let x = 0.6 + 2.6 * dt - 0.01;
        fight.balls[0].p = [-x, 0.5];
        fight.balls[0].v = [2.6, 0.0];
        fight.balls[1].p = [x, 0.5];
        fight.balls[1].v = [-2.6, 0.0];
        fight.move_step(dt);
        let (a, b) = (&fight.balls[0], &fight.balls[1]);
        assert!(
            (b.p[0] - a.p[0] - 1.2).abs() <= 1e-12,
            "apart by R + R: {}",
            b.p[0] - a.p[0]
        );
        assert!((a.p[0] + b.p[0]).abs() <= 1e-12 && a.p[1] == 0.5 && b.p[1] == 0.5);
        assert!(
            (a.v[0] + 2.6).abs() <= 1e-11 && (b.v[0] - 2.6).abs() <= 1e-11,
            "{:?} {:?}",
            a.v,
            b.v
        );
        assert_eq!(fight.events.len(), 1);
        let e = &fight.events[0];
        assert_eq!((e.t, e.kind.as_str(), &e.fields[0]), (6.0, "clash", &I(-1)));
        assert_eq!((e.who(), e.fields.len()), (None, 3));
        let (F(mx), F(my)) = (&e.fields[1], &e.fields[2]) else {
            panic!("the midpoint is two floats: {e:?}")
        };
        assert!(mx.abs() <= 1e-12 && *my == 0.5);
        // balls that are already moving apart keep their velocities (but are still pushed apart and renormalised)
        fight.balls[0].p = [-0.5, 0.0];
        fight.balls[0].v = [-2.6, 0.0];
        fight.balls[1].p = [0.5, 0.0];
        fight.balls[1].v = [2.6, 0.0];
        fight.move_step(dt);
        assert!(fight.balls[0].v[0] < 0.0 && fight.balls[1].v[0] > 0.0);
        assert_eq!(fight.events.len(), 2);
        // a knocked ball leaves the clash at V * (1 + boost)
        fight.balls[0].p = [-0.5, 0.0];
        fight.balls[0].v = [2.6, 0.0];
        fight.balls[0].boost = 1.0;
        fight.balls[1].p = [0.5, 0.0];
        fight.balls[1].v = [-2.6, 0.0];
        fight.move_step(dt);
        let boost = fight.balls[0].boost;
        assert!(
            (boost - dmath::exp(-dt / 0.38)).abs() <= 1e-15,
            "the boost decays by exp(-dt / KNOCK_T): {boost}"
        );
        assert!((geom::norm(fight.balls[0].v) - 2.6 * (1.0 + boost)).abs() <= 1e-11);
        assert!((geom::norm(fight.balls[1].v) - 2.6).abs() <= 1e-11);
        assert_eq!(fight.events.len(), 3);
    }

    /// `lead_changes` gives the count and the times `kit/story.py` stored for every committed recording.
    #[test]
    fn lead_changes_match_the_kit() {
        let stories = testing::stories();
        let mut total = 0;
        for name in testing::ALL_RECORDINGS {
            let py = testing::python_recording(name);
            let (n, times) = lead_changes(&py.events, LEAD_HP0, LEAD_HYST, LEAD_T_FROM);
            let story = &stories["stories"][name]["story"];
            assert_eq!(u64::from(n), story["lead_changes"].as_u64().expect("a count"), "{name}");
            let want: Vec<f64> = story["lc_t"]
                .as_array()
                .expect("a list")
                .iter()
                .map(|t| t.as_f64().expect("a time"))
                .collect();
            assert_eq!(times, want, "{name}");
            total += n;
        }
        assert_eq!(total, 14, "the 21 recordings hold 14 lead changes between them");
    }

    #[test]
    fn lead_changes_follow_the_python_rules() {
        use Value::{Float as F, Int as I};
        let hit = |t: f64, att: i64, vic: i64, d: f64| {
            ev(
                t,
                "hit",
                vec![I(att), I(vic), F(d), F(0.0), F(0.0), Value::Str("x".into()), I(0)],
            )
        };
        let events = vec![
            hit(1.0, 0, 1, 6.0),                  // left leads by 6: the first lead is not a change
            hit(2.0, 1, 0, 12.0), // right leads by 6, but before t_from: the lead flips without being counted
            hit(5.0, 0, 1, 10.0), // left leads by 4: within the hysteresis, no change
            hit(6.0, 0, 1, 2.0),  // left leads by 6: counted
            ev(7.0, "heal", vec![I(1), F(20.0)]), // a heal moves HP but is not looked at by itself
            hit(8.0, 0, 1, 1.0),  // right leads by 13: counted
            ev(9.0, "rope", vec![I(0)]),
        ];
        assert_eq!(lead_changes(&events, 100.0, 5.0, 4.0), (2, vec![6.0, 8.0]));
        assert_eq!(lead_changes(&events, 100.0, 5.0, 0.0), (3, vec![2.0, 6.0, 8.0]));
        assert_eq!(lead_changes(&[], 100.0, 5.0, 4.0), (0, vec![]));
        // an event the engine could not have made is passed over, not a panic
        let odd = vec![
            ev(5.0, "hit", vec![I(0), I(7), F(50.0)]),
            ev(5.0, "hit", vec![I(0)]),
            ev(5.0, "heal", vec![F(1.0), F(1.0)]),
        ];
        assert_eq!(lead_changes(&odd, 100.0, 5.0, 4.0), (0, vec![]));
    }

    #[test]
    fn setup_from_builds_the_episodes_rules() {
        let roster = testing::roster();
        let ff = testing::example_fight_file();
        let setup = setup_from(&ff, &roster, 392).expect("the example fight file sets up");
        assert_eq!(
            (setup.fighters[0].name.as_str(), setup.fighters[1].name.as_str()),
            ("FLUTE", "TROMBONE")
        );
        assert_eq!(setup.seed, 392);
        // exactly the rules the Python episode fought under
        assert_eq!(setup.rules, testing::episode_rules());
        assert_eq!(setup.rules.to_values(), testing::python_recording("ep392.bfrec").rules);
        // the seed is the caller's, not the file's
        assert_eq!(setup_from(&ff, &roster, 7).expect("sets up").seed, 7);
        // sides are the file's order
        let mut swapped = ff.clone();
        swapped.fighters.reverse();
        swapped.handicap.reverse();
        let setup = setup_from(&swapped, &roster, 1).expect("sets up");
        assert_eq!(setup.fighters[0].name, "TROMBONE");
        assert_eq!(
            setup.rules.to_values(),
            testing::python_recording("scenarios/swapped_s1.bfrec").rules
        );
    }

    #[test]
    fn setup_from_names_what_it_cannot_run() {
        let roster = testing::roster();
        let ff = testing::example_fight_file();
        let err = |change: &dyn Fn(&mut FightFile)| {
            let mut ff = ff.clone();
            change(&mut ff);
            setup_from(&ff, &roster, 1).expect_err("refused").to_string()
        };
        assert_eq!(
            err(&|f| f.fighters[1] = "KAZOO".into()),
            r#"fighters[1]: "KAZOO" is not in the roster"#
        );
        assert_eq!(
            err(&|f| f.fighters[0] = "PIANO".into()),
            r#"fighters[0]: the engine has not ported PIANO's weapon kind "roll" yet"#
        );
        assert_eq!(
            err(&|f| _ = f.fighters.pop()),
            "fighters: a fight has 2 fighters, this file has 1"
        );
        assert_eq!(
            err(&|f| f.handicap.push(1.0)),
            "handicap: one entry per fighter (2), this file has 3"
        );
        assert_eq!(
            err(&|f| _ = f.pair_rules.insert("swat_shells".into(), Value::Bool(true))),
            "pair_rules.swat_shells: the engine has no rule of this name"
        );
        assert_eq!(
            err(&|f| _ = f.pair_rules.insert("swat_cobras".into(), Value::Int(1))),
            "pair_rules.swat_cobras: must be true or false"
        );
        assert_eq!(
            err(&|f| f.dmg_scale = f64::NAN),
            "dmg_scale: NaN is not a finite number"
        );
        // a fighter whose weapon is ported but an ability is not
        let mut odd = roster.clone();
        let flute = odd.iter_mut().find(|f| f.name == "FLUTE").expect("in the roster");
        flute.unlock2.kind = AbilityKind::Unsupported("stun".into());
        assert_eq!(
            setup_from(&ff, &odd, 1).expect_err("refused").to_string(),
            r#"fighters[0]: the engine has not ported FLUTE's ability SNAKE DANCE (kind "stun") yet"#
        );
    }
}
