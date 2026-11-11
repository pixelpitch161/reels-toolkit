//! The fight: `fight.py`'s `class Fight`, its core. Two balls in a square ring, one generator, one event list.
//!
//! Ported here: `Fight.__init__` (lines 264 to 316), `ev`, `damage` and `knock` (319 to 368), `kill_keys` (380 to
//! 387), `move` for the balls (3128 to 3234: walls, rope give, ball against ball), `abilities_step` (3012 to 3053:
//! levels, the low-HP ability and the unlock timers) and `lead_changes` (6225 to 6242). The order of a step is in
//! [`crate::schedule`], the per-frame record in `record.rs`. Weapon kinds and abilities are separate modules of free
//! functions taking `&mut Fight`; their state hangs off [`Fight`] as `Option` fields (or is on the ball, where
//! `Ball.__init__` puts it) and they are called from `schedule.rs`, but for one call: an ability is fired
//! ([`crate::abilities::trigger`]) from [`Fight::abilities_step`], where Python fires it, between one fighter's
//! levels and the next fighter's.
//!
//! The Python code checks, in many places, state that only the kit's other kinds can set (a swallowed, stunned,
//! anchored, hopping, smashed, walking, grabbed, airborne or strutting ball). Those checks are made here already, on
//! the per-ball states of [`Ball`] (their defaults are what Python's `getattr` gives a ball that never had the
//! attribute, so a fight of FLUTE and TROMBONE never takes them), each with the Python lines it stands for; the older
//! kinds' (a rewinding ball, a per-ball radius or mass) are still marked `SLOT`. Where a kit kind's own code runs
//! (the walker's pinned step, the projectiles, the gates of `abilities_step`), the call goes to that kind's module.
//!
//! Every such place is a socket: with `Fight::trace_sockets` on (the engine's own tests), it logs its name, so a test
//! can check that the sockets are reached in `fight.py`'s order (`tests/sockets.rs`).

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::fightfile::FightFile;
use bf_core::py;
use bf_core::recording::Recording;
use bf_core::rng::Generator;

use std::collections::BTreeSet;

use crate::abilities::{self, copy, hook, slap, snip};
use crate::ball::{Ball, Slot};
use crate::geom::{self, V2};
use crate::kinds::striker::Striker;
use crate::kinds::{self, roll};
use crate::record;
use crate::roster::abilities::detune::DetuneCfg;
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
    /// The seconds of stun it puts on the enemy it hits (`W['stun']`: a ring of kind `stun`, CLARINET's SQUEAK),
    /// if any.
    pub stun: Option<f64>,
    /// The detune it puts on the enemy it hits (`W['detune']`, a wave's option: OBOE's ILL WIND), if any.
    pub detune: Option<DetuneCfg>,
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
    /// What a hit by it is called (`S.get('how', 'slam')`, `fight.py` 3097): `slam`, or `cannon` for the BASS DRUM's
    /// 1812 OVERTURE.
    pub how: String,
}

/// A projectile in flight: one row of `fight.py`'s parallel arrays `kp`, `kv`, `kown`, `kid`, `kvol`, `kidx` and
/// `kdmg` (lines 286 to 292).
#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    /// Where it is (`kp`).
    pub p: V2,
    /// Its velocity (`kv`).
    pub v: V2,
    /// The fighter that fired it (`kown`).
    pub own: usize,
    /// Its number, counting up over the fight (`kid`).
    pub id: i64,
    /// The volley it belongs to (`kvol`).
    pub vol: i64,
    /// Its index in that volley (`kidx`).
    pub idx: i64,
    /// The damage it does (`kdmg`).
    pub dmg: f64,
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
    /// The projectiles in flight, in the order they were fired (`kp` and its sister arrays). No ported kind fires
    /// one, so the list is empty in every fight the engine can run today: it is here because the thrust lance's
    /// parry reads it and [`Fight::kill_keys`] empties it. Their flight, their hits and their `K` rows come with
    /// the first kind that fires them (the slots in [`Fight::move_step`] and `record.rs`).
    pub keys: Vec<Key>,
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
    // The thrust blade has no entry here: its state is on the ball (`th`, `ext`, `thr`, `thr_hit`, `thr_k`).
    // FLUTE's and TROMBONE's abilities have no entry here either: their rings and slams are `waves` and `slams`
    // above, the rest is on the ball (`haste`, `slow`, `ready`, `used`) or the striker's (`dance_until`).
    /// The fighters running a snip that IDENTITY CRISIS copied (`self.extra_snip`, a set; `fight.py` 312, 2813 to
    /// 2814, 6154 to 6156), by fighter index (a set of 0 and 1 iterates in that order).
    pub extra_snip: [bool; 2],
    /// When a walker's clash was last announced (`getattr(self, 'wclash_t', -9.0)`, `move` 3227 to 3231): one `clash`
    /// event per push, not one per step while it walks on.
    pub wclash_t: f64,
    /// The mortar shells in the air or landed (`self.shells`, made when a fighter shells: `run()` 5810 to 5812), in
    /// the order they were fired; empty in every other fight (Python has no attribute then, which reads the same).
    pub shells: Vec<kinds::mortar::Shell>,
    /// The next salvo's number (`self.next_salvo`).
    pub next_salvo: i64,
    /// The salvos that have landed a shell (`self.salvo_landed`, a set).
    pub salvo_landed: BTreeSet<i64>,
    // Each kit kind's own state (`fight.py` 286 to 312 list the fight-level Python fields: arcs, beams, hazards,
    // ...), one field per module, empty until its port; `spawn_key`'s `next_id`, `next_vol` and `vol_landed` are
    // the roll's.
    /// Weapon kind `roll`'s state (PIANO).
    pub roll: kinds::roll::State,
    /// Weapon kind `swallow`'s state (TUBA).
    pub swallow: kinds::swallow::State,
    /// Weapon kind `echo`'s state (HORN).
    pub echo: kinds::echo::State,
    /// Weapon kind `line`'s state (ORGAN): the pipes' hazards.
    pub line: kinds::line::State,
    /// Weapon kind `rake`'s state (ACOUSTIC).
    pub rake: kinds::rake::State,
    /// Weapon kind `combo`'s state (TRUMPET).
    pub combo: kinds::combo::State,
    /// Weapon kind `jump`'s state (CLARINET).
    pub jump: kinds::jump::State,
    /// Weapon kind `tune`'s state (OBOE).
    pub tune: kinds::tune::State,
    /// Weapon kind `anchor`'s state (BASS CLARINET).
    pub anchor: kinds::anchor::State,
    /// Weapon kind `mortar`'s state (EUPHONIUM), beside `shells`.
    pub mortar: kinds::mortar::State,
    /// Weapon kind `tiptoe`'s state (BASSOON).
    pub tiptoe: kinds::tiptoe::State,
    /// Weapon kind `lightning`'s state (ELECTRIC).
    pub lightning: kinds::lightning::State,
    /// Weapon kind `grapple`'s state (CELLO).
    pub grapple: kinds::grapple::State,
    /// Weapon kind `walk`'s state (CONTRABASS).
    pub walk: kinds::walk::State,
    /// Weapon kind `spotlight`'s state (ALTO SAX).
    pub spotlight: kinds::spotlight::State,
    /// Weapon kind `crowd`'s state (TENOR SAX).
    pub crowd: kinds::crowd::State,
    /// Weapon kind `drill`'s state (SOUSAPHONE).
    pub drill: kinds::drill::State,
    /// Weapon kind `roll_charge`'s state (BASS DRUM).
    pub roll_charge: kinds::roll_charge::State,
    /// Ability kind `sustain`'s state (PIANO).
    pub sustain: abilities::sustain::State,
    /// Ability kind `arc`'s state (PIANO): the arcs (`self.arcs`).
    pub arc: abilities::arc::State,
    /// Ability kind `runoff`'s state (PIANO).
    pub runoff: abilities::runoff::State,
    /// Ability kind `beam`'s state (TRUMPET): the ability beams (`self.beams`).
    pub beam: abilities::beam::State,
    /// Ability kind `stun`'s state (CLARINET).
    pub stun: abilities::stun::State,
    /// Ability kind `snip`'s state (OBOE, BASS CLARINET, a copy).
    pub snip: abilities::snip::State,
    /// Ability kind `twin`'s state (BASS CLARINET).
    pub twin: abilities::twin::State,
    /// Ability kind `lights_out`'s state (BASS CLARINET).
    pub lights_out: abilities::lights_out::State,
    /// Ability kind `slap`'s state (CONTRABASS).
    pub slap: abilities::slap::State,
    /// Ability kind `stampede`'s state (CONTRABASS).
    pub stampede: abilities::stampede::State,
    /// Ability kind `hook`'s state (BASSOON).
    pub hook: abilities::hook::State,
    /// Ability kind `flick`'s state (HORN).
    pub flick: abilities::flick::State,
    /// Ability kinds `whip` and `sweep`'s state (ACOUSTIC): `self.whips`, `self.sweeps`.
    pub x33: abilities::x33::State,
    /// Ability kind `smash`'s state (ELECTRIC): the splinters.
    pub smash: abilities::smash::State,
    /// Ability kind `vibrato`'s state (CELLO).
    pub vibrato: abilities::vibrato::State,
    /// Ability kind `prelude`'s state (CELLO).
    pub prelude: abilities::prelude::State,
    /// Ability kind `swan`'s state (CELLO).
    pub swan: abilities::swan::State,
    /// Ability kind `copy`'s state (EUPHONIUM).
    pub copy: abilities::copy::State,
    /// The engine's own tests only: `Some` turns on the socket trace (every socket the fight reaches logs its name
    /// here, in order; see [`Fight::socket`]). `None` in every fight the app runs, where a socket costs a test of
    /// this field and nothing else.
    #[cfg(any(test, feature = "testing"))]
    #[doc(hidden)]
    pub trace_sockets: Option<std::cell::RefCell<Vec<&'static str>>>,
}

impl<'a> Fight<'a> {
    /// `Fight(a, b, seed, rules, record)` (`fight.py` 264 to 316): both fighters in their corners, launched toward
    /// the centre within `LAUNCH_SPREAD` degrees either side, each weapon at a seeded angle. Four draws from the
    /// generator, in this order: fighter 0's launch angle, its weapon angle, then fighter 1's two.
    ///
    /// The setup is checked first ([`Setup::check`]): rules the engine cannot run, a fighter whose weapon or
    /// abilities are of a kind the engine has not ported, or a roster entry whose numbers cannot run, are an error
    /// that says what is wrong, and no fight is made. A fight this returns cannot panic on its numbers.
    pub fn new(setup: &Setup<'a>, record: bool) -> Result<Fight<'a>, SimError> {
        setup.check()?;
        Ok(Fight::launch(setup, record))
    }

    /// [`Fight::new`] that also lets in a fighter whose weapon or abilities are of a kind the engine has not ported:
    /// such a weapon does nothing, and such an ability is announced (its `ability` event, its timer) and does
    /// nothing else, as `fight.py` does for a kind it has no branch for. That is not a fight of the series: it is
    /// for the parity tests whose Python fights use a punching bag (a fighter with no weapon, and abilities of kind
    /// `none`, or the shield bag's `shield`). Everything else is checked as [`Fight::new`] checks it, and an unlock of
    /// a kind not ported must have its `every_bars` (the engine reads it when the unlock fires).
    ///
    /// It exists only in the engine's own tests: under `cfg(test)` and the crate feature `testing`, which bf-sim's
    /// dev-dependency on itself turns on for its integration tests. No other crate enables it, so the app cannot
    /// call it.
    #[cfg(any(test, feature = "testing"))]
    #[doc(hidden)]
    pub fn with_unported(setup: &Setup<'a>, record: bool) -> Result<Fight<'a>, SimError> {
        setup.check_with(true)?;
        Ok(Fight::launch(setup, record))
    }

    /// The launch itself, on a checked setup.
    fn launch(setup: &Setup<'a>, record: bool) -> Fight<'a> {
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
            keys: Vec::new(),
            waves: Vec::new(),
            slams: Vec::new(),
            events: Vec::new(),
            t: 0.0,
            winner: None,
            t_ko: None,
            striker: [None, None],
            extra_snip: [false, false],
            wclash_t: -9.0,
            shells: Vec::new(),
            next_salvo: 0,
            salvo_landed: BTreeSet::new(),
            roll: Default::default(),
            swallow: Default::default(),
            echo: Default::default(),
            line: Default::default(),
            rake: Default::default(),
            combo: Default::default(),
            jump: Default::default(),
            tune: Default::default(),
            anchor: Default::default(),
            mortar: Default::default(),
            tiptoe: Default::default(),
            lightning: Default::default(),
            grapple: Default::default(),
            walk: Default::default(),
            spotlight: Default::default(),
            crowd: Default::default(),
            drill: Default::default(),
            roll_charge: Default::default(),
            sustain: Default::default(),
            arc: Default::default(),
            runoff: Default::default(),
            beam: Default::default(),
            stun: Default::default(),
            snip: Default::default(),
            twin: Default::default(),
            lights_out: Default::default(),
            slap: Default::default(),
            stampede: Default::default(),
            hook: Default::default(),
            flick: Default::default(),
            x33: Default::default(),
            smash: Default::default(),
            vibrato: Default::default(),
            prelude: Default::default(),
            swan: Default::default(),
            copy: Default::default(),
            #[cfg(any(test, feature = "testing"))]
            trace_sockets: None,
        }
    }

    /// A socket: a place where `fight.py` calls a kind's code or tests a kind's state. It does nothing, unless the
    /// engine's own tests turned on `Fight::trace_sockets`: then it logs `name`.
    #[inline]
    pub fn socket(&self, name: &'static str) {
        #[cfg(any(test, feature = "testing"))]
        if let Some(trace) = &self.trace_sockets {
            trace.borrow_mut().push(name);
        }
        #[cfg(not(any(test, feature = "testing")))]
        let _ = name;
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
        self.socket("damage.skips");
        // fight.py 325 to 334: a victim sunk in shadow (#38), smashed (#44) or airborne (#47 swan, #51 dive) cannot be
        // hit. (325 to 326, `rw_left`: the kind `rewind`, older than the kit's. SLOT.)
        let v = &self.balls[vic];
        if v.shadow > 0.0 || v.smashed > 0.0 || v.swan.is_some() || v.dive.is_some() {
            return false;
        }
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
        // fight.py 342 to 343: a resting victim's `guard` (#42, the kind `rests`, older than the kit's). SLOT.
        self.socket("damage.detuned");
        // fight.py 344 to 346: an out-of-tune attacker's hits land weaker (#37)
        let a = &self.balls[att];
        if a.detune_t > 0.0 {
            d *= 1.0 - a.detune_w * a.detune_d;
        }
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
        self.socket("knock.skips");
        // fight.py 355 to 360: a walking bass, an airborne swan, a strut or a stage dive cannot be knocked
        let v = &self.balls[vic];
        if v.walking || v.swan.is_some() || v.strut.is_some() || v.dive.is_some() {
            return;
        }
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

    /// `kill_keys(mask, why, by)` (`fight.py` 380 to 387): takes the projectiles the mask marks out of the fight, with
    /// one event each in the order they were fired: `why` (`parry` when a lance killed it), the projectile's owner,
    /// then `by` (the fighter that killed it; Python's default is -1, nobody), where it was and its id. The others
    /// keep their order.
    ///
    /// # Panics
    /// When the mask is not one entry per projectile.
    pub fn kill_keys(&mut self, mask: &[bool], why: &str, by: i64) {
        assert_eq!(mask.len(), self.keys.len(), "one mask entry per projectile");
        if !mask.contains(&true) {
            return;
        }
        for i in (0..mask.len()).filter(|&i| mask[i]) {
            let key = &self.keys[i];
            let data = vec![
                Value::Int(by),
                Value::Float(key.p[0]),
                Value::Float(key.p[1]),
                Value::Int(key.id),
            ];
            let own = key.own as i64;
            self.ev(why, own, data);
        }
        let mut killed = mask.iter();
        self.keys
            .retain(|_| !killed.next().expect("one mask entry per projectile"));
    }

    /// `move(dt)` for the balls (`fight.py` 3128 to 3234): each ball flies at `V * (1 + boost)` (times its slow),
    /// bounces off the ropes with a seeded give, and the two clash when they overlap.
    ///
    /// The generator is drawn once per rope bounce, in ball order and, for a ball that crosses two ropes in one step,
    /// the x rope first.
    pub fn move_step(&mut self, dt: f64) {
        let (a, r) = (self.rules.a, self.rules.r);
        for k in 0..2 {
            self.socket("move.skips");
            // fight.py 3131 to 3150: a ball that another step drives is skipped here: swallowed, stunned, anchored,
            // mid-hop, smashed, mid-step, grabbed or pinned, airborne, strutting or diving. (3131 to 3132, a rewinding
            // ball: the kind `rewind`, older than the kit's. SLOT.)
            let b = &self.balls[k];
            if b.held
                || b.stun > 0.0
                || b.anc.is_some()
                || b.air
                || b.smashed > 0.0
                || b.walking
                || b.grabbed
                || b.pinned
                || b.swan.is_some()
                || b.strut.is_some()
                || b.dive.is_some()
            {
                continue;
            }
            self.socket("move.speeds");
            let b = &mut self.balls[k];
            b.boost *= dmath::exp(-dt / self.rules.knock_t);
            let mut s = self.rules.v * (1.0 + b.boost);
            // fight.py 3153 to 3156: a hopper (#39) or a planted walker (#47) stands still; it only slides when knocked
            if b.hopper {
                s = self.rules.v * b.boost;
            }
            if b.walker {
                s = self.rules.v * b.boost;
            }
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
        // fight.py 3179: per-ball radii (inflate, older than the kit's kinds) would replace R + R. SLOT.
        let (r_p, r_q) = (r, r);
        self.socket("move.clash");
        // fight.py 3180 to 3187: no clash with a ball that is swallowed, in shadow, mid-hop, smashed, grabbed,
        // airborne or diving (or rewinding: the kind `rewind`, older than the kit's; SLOT). Python's `and` tests
        // these only when the balls overlap; they read state and change nothing, so the order does not matter.
        let apart = |b: &Ball| {
            b.held || b.shadow > 0.0 || b.air || b.smashed > 0.0 || b.grabbed || b.swan.is_some() || b.dive.is_some()
        };
        if dist < r_p + r_q && !apart(&self.balls[0]) && !apart(&self.balls[1]) {
            let m = geom::py_max(dist, 1e-9);
            let n = [d[0] / m, d[1] / m];
            let over = r_p + r_q - dist;
            self.socket("move.walker");
            // fight.py 3190 to 3193: a walker (#47) or a strutting tenor (#51) does not give way
            let walks = |b: &Ball| b.walker || b.strut.is_some();
            let (w_p, w_q) = (walks(&self.balls[0]), walks(&self.balls[1]));
            let v_shared = self.rules.v;
            if w_p != w_q {
                // fight.py 3194 to 3208: the other ball takes the whole overlap and bounces off it; pinned against
                // the ropes, the walker stops short instead (no squeezing through them)
                let (w, o, sg) = if w_p { (0, 1, 1.0) } else { (1, 0, -1.0) };
                // O_.p = O_.p + sg_ * n * over
                let ob = &mut self.balls[o];
                ob.p = [ob.p[0] + sg * n[0] * over, ob.p[1] + sg * n[1] * over];
                // lim_ = A - getattr(O_, 'rad', R): per-ball radii are the older kind `inflate`'s (SLOT)
                let lim = a - r;
                let cl = [geom::np_clip(ob.p[0], -lim, lim), geom::np_clip(ob.p[1], -lim, lim)];
                let back = geom::sub(ob.p, cl);
                // `back_.any()`: a coordinate that is not zero (NaN counts, as numpy's truth test does)
                if back[0] != 0.0 || back[1] != 0.0 {
                    ob.p = cl;
                    let wb = &mut self.balls[w];
                    wb.p = geom::sub(wb.p, back);
                    self.socket("move.walker_pinned");
                    kinds::walk::clash_pinned(self, w);
                }
                // vo = float(O_.v @ n) * sg_; if vo < 0: O_.v = O_.v - 2 * vo * sg_ * n
                let ob = &mut self.balls[o];
                let vo = geom::dot(ob.v, n) * sg;
                if vo < 0.0 {
                    ob.v = [ob.v[0] - 2.0 * vo * sg * n[0], ob.v[1] - 2.0 * vo * sg * n[1]];
                }
            } else {
                // fight.py 3209 to 3216: unequal masses (`mass`: the kind `inflate`, older than the kit's). SLOT.
                // What follows is the equal-mass branch (3217 to 3223).
                let [p, q] = &mut self.balls;
                p.p = [p.p[0] - n[0] * over / 2.0, p.p[1] - n[1] * over / 2.0];
                q.p = [q.p[0] + n[0] * over / 2.0, q.p[1] + n[1] * over / 2.0];
                let (vp, vq) = (geom::dot(p.v, n), geom::dot(q.v, n));
                if vp - vq > 0.0 {
                    p.v = [p.v[0] + (vq - vp) * n[0], p.v[1] + (vq - vp) * n[1]];
                    q.v = [q.v[0] + (vp - vq) * n[0], q.v[1] + (vp - vq) * n[1]];
                }
            }
            let [p, q] = &mut self.balls;
            for b in [&mut *p, &mut *q] {
                // B.v = B.v / (np.linalg.norm(B.v) + 1e-12) * self.R.V * (1 + B.boost)
                let nb = geom::norm(b.v) + 1e-12;
                b.v = [
                    b.v[0] / nb * v_shared * (1.0 + b.boost),
                    b.v[1] / nb * v_shared * (1.0 + b.boost),
                ];
            }
            let mid = [(p.p[0] + q.p[0]) / 2.0, (p.p[1] + q.p[1]) / 2.0];
            self.socket("move.wclash");
            // fight.py 3227 to 3232: a walker's clash is announced once per push, not once per step while it walks
            // on (`wclash_t`)
            if w_p != w_q && self.t - self.wclash_t < 0.2 {
                // pass
            } else {
                if w_p != w_q {
                    self.wclash_t = self.t;
                }
                self.ev("clash", -1, vec![Value::Float(mid[0]), Value::Float(mid[1])]);
            }
            // fight.py 3233 to 3234: `clash_hooks` (inflate's contact damage, older than the kit's kinds). SLOT.
        }
        // fight.py 3235 to 3278: the projectiles (`steer`, their flight, `ricochet`, those that leave the ring, those
        // that hit a ball, the volley and roll growth, the nudge), on `self.keys`. The kit fires them from the roll
        // and its arc (PIANO); FLUTE and TROMBONE fire none. The roll's module owns the code.
        self.socket("move.keys");
        if !self.keys.is_empty() {
            roll::keys_move(self, dt);
        }
    }

    /// `abilities_step(on_beat)` (`fight.py` 3012 to 3053), for each fighter in turn: its level follows the damage it
    /// has dealt (each level-up makes its unlock ready from now and emits `level`); then, on a beat, it fires one
    /// ability ([`abilities::trigger`]): its `low` ability, once, when its hit points are below `LOW_HP`; else the
    /// first of `unlock1` and `unlock2` that is ready (`t >= ready - 1e-6`), which is then ready again `every_bars`
    /// bars later.
    ///
    /// An unlock's `every_bars` is there: [`Fight::new`] (and the test entry `Fight::with_unported`) refuse an
    /// unlock without one, where Python raises `KeyError` when it fires.
    pub fn abilities_step(&mut self, on_beat: bool) {
        // fight.py 3013 to 3016: a charged ability beam (#29 v3, TRUMPET's HIGH C) fires when its beat comes, for both
        // fighters, before the loop below
        for k in 0..2 {
            self.socket("abilities.charged_beam");
            if abilities::has(self, k, &AbilityKind::Beam) {
                abilities::beam::check_charge(self, k);
            }
        }
        for k in 0..2 {
            self.socket("abilities.levels");
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
            self.socket("abilities.smashed");
            // fight.py 3027 to 3028: a smashed fighter (#44) starts nothing
            if self.balls[k].smashed > 0.0 {
                continue;
            }
            let b = &mut self.balls[k];
            if !b.low_done && b.hp < self.rules.low_hp {
                b.low_done = true;
                abilities::trigger(self, k, Slot::Low);
                continue;
            }
            for slot in [Slot::Unlock1, Slot::Unlock2] {
                self.socket("abilities.gates");
                // fight.py 3034 to 3049: an unlock of kind snip, hook, slap or copy waits for its moment (one of kind
                // rewind, older than the kit's, too: SLOT)
                let Some(r) = self.balls[k].ready[slot.index()] else {
                    continue;
                };
                if self.waits(k, slot) {
                    continue;
                }
                if self.t >= r - 1e-6 {
                    abilities::trigger(self, k, slot);
                    // B.ready[slot] = self.t + B.cfg['abilities'][slot]['every_bars'] * 4 * self.R.beat
                    let every_bars = abilities::param(slot.of(self.balls[k].cfg), "every_bars");
                    self.balls[k].ready[slot.index()] = Some(self.t + every_bars * 4.0 * self.rules.beat);
                    // one ability per fighter per beat
                    break;
                }
            }
        }
    }

    /// `abilities_step`'s gates (`fight.py` 3035 to 3049), for an unlock that is not locked: a snip (3037 to 3039), a
    /// hook (3040 to 3042) or a slap (3043 to 3045) waits until its `*_ready` says so, and a copy waits while what it
    /// would copy is a snip that is not ready (3046 to 3049). The ready tests are the kinds' own.
    fn waits(&self, k: usize, slot: Slot) -> bool {
        let ab = slot.of(self.balls[k].cfg);
        match ab.kind {
            AbilityKind::Snip => !snip::ready(self, k, ab),
            AbilityKind::Hook => !hook::ready(self, k, ab),
            AbilityKind::Slap => !slap::ready(self, k, ab),
            AbilityKind::Copy => match copy::source(self, k) {
                Some(src) if src.kind == AbilityKind::Snip => !snip::ready(self, k, src),
                _ => false,
            },
            _ => false,
        }
    }
}

/// `_stunned(k)` (`fight.py` 2355 to 2357): fighter `k` is stunned (#37, SQUEAK) or sunk in shadow (#38, LIGHTS OUT),
/// so it starts no attack (the beat grid and the 16th grids that ask).
pub fn stunned(f: &Fight, k: usize) -> bool {
    f.balls[k].stun > 0.0 || f.balls[k].shadow > 0.0
}

/// Runs one fight to its K.O. (or to `t_max`) and returns its recording, made by this app
/// ([`bf_core::recording::Source::studio`]). With `record` false the recording has no per-frame arrays: only the
/// events, the winner and the K.O. time (the fight itself is the same).
///
/// A setup the engine cannot run is an error in words, before anything is run ([`Setup::check`]): no rules and no
/// roster entry can make this panic.
pub fn run(setup: &Setup, record: bool) -> Result<Recording, SimError> {
    let mut fight = Fight::new(setup, record)?;
    let frames = schedule::run(&mut fight);
    Ok(record::recording(fight, frames))
}

/// [`run`] through the test entry `Fight::with_unported`: a fighter of a kind not ported is let in (and that kind
/// does nothing). For the parity tests against Python fights with a punching bag; not a fight of the series.
#[cfg(any(test, feature = "testing"))]
#[doc(hidden)]
pub fn run_with_unported(setup: &Setup, record: bool) -> Result<Recording, SimError> {
    let mut fight = Fight::with_unported(setup, record)?;
    let frames = schedule::run(&mut fight);
    Ok(record::recording(fight, frames))
}

/// [`run`] with `Fight::trace_sockets` on: the recording, and the name of every socket the fight reached, in order
/// (the setup's, frame `f0`'s, then each step's, each step starting with `clock`, and each frame's after its step).
/// For `tests/sockets.rs`; not a fight of the series (the fight itself is the same).
#[cfg(any(test, feature = "testing"))]
#[doc(hidden)]
pub fn run_traced(setup: &Setup, record: bool) -> Result<(Recording, Vec<&'static str>), SimError> {
    let mut fight = Fight::new(setup, record)?;
    fight.trace_sockets = Some(std::cell::RefCell::new(Vec::new()));
    let frames = schedule::run(&mut fight);
    let trace = fight
        .trace_sockets
        .take()
        .map(std::cell::RefCell::into_inner)
        .unwrap_or_default();
    Ok((record::recording(fight, frames), trace))
}

impl Setup<'_> {
    /// Whether the engine can run this fight, and if not, what is wrong in plain words (one line, naming the field):
    ///
    /// - the rules ([`Rules::check`]): `rules.<NAME>: ...`;
    /// - each fighter (`fighters[0]` is the left corner): a weapon or an ability of a kind the engine has not ported
    ///   yet; or a number of its roster entry the engine cannot run (`FighterCfg::check`, the checks the roster
    ///   reader makes, made again for an entry built or changed in code).
    ///
    /// A setup that passes cannot make the engine panic or run for ever: every number a ported kind reads is there
    /// and in range.
    pub fn check(&self) -> Result<(), SimError> {
        self.check_with(false)
    }

    /// [`Setup::check`], letting in kinds not ported when `unported` is true (the test entry
    /// `Fight::with_unported`).
    fn check_with(&self, unported: bool) -> Result<(), SimError> {
        self.rules.check()?;
        for (k, cfg) in self.fighters.iter().enumerate() {
            check_fighter(cfg, k, unported)?;
        }
        Ok(())
    }
}

/// One fighter of a setup, as [`Setup::check`] says; the error's field is `fighters[k]`.
fn check_fighter(cfg: &FighterCfg, k: usize, unported: bool) -> Result<(), SimError> {
    let field = format!("fighters[{k}]");
    let name = &cfg.name;
    if !unported {
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
    }
    cfg.check()
        .map_err(|e| SimError::new(&field, format!("{}: {}: {}", e.fighter, e.field, e.message)))?;
    // An unlock's timer reads `every_bars` whatever its kind (the roster reader asks for it only of a ported kind).
    for (slot, ab) in [("unlock1", &cfg.unlock1), ("unlock2", &cfg.unlock2)] {
        if ab.get("every_bars").is_none() {
            return Err(SimError::new(
                &field,
                format!(
                    "{name}: abilities.{slot}.every_bars: missing (the bars between two uses of {})",
                    ab.name
                ),
            ));
        }
    }
    Ok(())
}

/// The setup of a fight file's fight with this seed, as the kit builds it (`kit/story.py` `rules` and `run_seed`):
/// the rules at the file's tempo grid, its damage scale and handicap, then its pair rules.
///
/// Errors name the fight file's field: a fighter that is not in `roster`, a fighter whose weapon or abilities the
/// engine has not ported or whose roster numbers it cannot run, a pair rule the engine does not have.
///
/// A handicap of 1.0 for each fighter is no handicap: the rules then have no `dmg_side`, as the kit's rules have
/// none for an episode without `DMG_SIDE`. So a recording's `rules` are the kit's for a file with no `handicap`
/// line. A fight file cannot tell that from an explicit `handicap = [1.0, 1.0]`, which in Python (`DMG_SIDE =
/// (1.0, 1.0)`) DOES record `dmg_side`; the Studio records none for it. That cannot change a fight: the only reader
/// of `dmg_side` (`damage`) multiplies by it, and a damage times 1.0 is that damage. Only the content hash differs.
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
        check_fighter(cfg, k, false)?;
        Ok(cfg)
    };
    let fighters = [find(0)?, find(1)?];

    // kit/story.py rules(): Rules(bpm=T.BPM, t_fight=T.T_FIGHT, fps=T.FPS, dmg_scale=T.DMG_SCALE,
    //                             dmg_side=tuple(T.DMG_SIDE), **T.RULES)
    let grid = ff.grid();
    let mut rules = Rules::new(grid.bpm, grid.t_fight);
    rules.fps = grid.fps;
    rules.dmg_scale = ff.dmg_scale;
    // story.py: `if side is not None: kw['dmg_side'] = tuple(side)`; 1.0 each is the file's "no handicap"
    let side = [ff.handicap[0], ff.handicap[1]];
    rules.dmg_side = (side != [1.0, 1.0]).then_some(side);
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
    // kit/story.py 33 to 35: a roll fighter's rows are its line (`roll_rows`, `roll_start8`). The roll's own code.
    roll::feed_rules(ff, fighters, &mut rules)?;
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
        let fight = testing::fight(&testing::episode_setup(&roster, 392), true);
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
        let a = testing::fight(&testing::episode_setup(&roster, 1), false);
        let b = testing::fight(&testing::episode_setup(&roster, 1), false);
        let c = testing::fight(&testing::episode_setup(&roster, 2), false);
        assert_eq!(a.balls[0].v, b.balls[0].v);
        assert_eq!(a.balls[0].th.to_bits(), b.balls[0].th.to_bits());
        assert_ne!(a.balls[0].v, c.balls[0].v);
        // within LAUNCH_SPREAD (38 degrees) of the line to the centre, which is 45 degrees below the x axis for red
        let ang = dmath::atan2(a.balls[0].v[1], a.balls[0].v[0]);
        assert!((ang + 0.25 * dmath::PI).abs() <= geom::radians(38.0) + 1e-12);
        // bare fighters neither thrust nor slow
        let bare = testing::bare_pair();
        assert!(!testing::fight(&testing::bare_setup(&bare, 1), false).uses_ext);
    }

    /// Brief, step 1: an event's time is Python's `round(t, 5)`, which rounds the exact decimal value (ties to
    /// even), not `floor(t * 1e5 + 0.5) / 1e5`. The expected values are what Python gave.
    #[test]
    fn event_times_are_rounded_to_5_places() {
        let roster = testing::roster();
        let mut fight = testing::fight(&testing::episode_setup(&roster, 392), false);
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
        let mut fight = testing::fight(&testing::episode_setup(&roster, 392), false);
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
        let mut fight = testing::fight(&setup, false);
        assert!(fight.damage(0, 1, 2.0, "wave", [0.0, 0.0], 0));
        assert_eq!((fight.balls[1].hp, fight.balls[0].dealt), (94.0, 6.0));
    }

    /// The numbers are what `Fight.knock` gave in Python.
    #[test]
    fn knock_bends_the_heading_and_keeps_the_larger_boost() {
        let roster = testing::roster();
        let mut fight = testing::fight(&testing::episode_setup(&roster, 392), false);
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
        let mut fight = testing::fight(&testing::bare_setup(&bare, 1), false);
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
        let mut fight = testing::fight(&testing::bare_setup(&bare, 1), false);
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
        let mut fight = testing::fight(&testing::bare_setup(&bare, 1), false);
        assert_eq!((fight.ab_mult(0), fight.ab_mult(1)), (1.0, 1.0));
        fight.balls[1].haste_mult = 2.0;
        assert_eq!(fight.ab_mult(1), 1.0, "a factor without haste time left does nothing");
        fight.balls[1].haste = 0.25;
        assert_eq!((fight.ab_mult(0), fight.ab_mult(1)), (1.0, 2.0));
        fight.balls[1].haste = 0.0;
        assert_eq!(fight.ab_mult(1), 1.0);
    }

    /// `kill_keys`: the marked projectiles leave the fight, each with an event (its owner, who killed it, where it
    /// was, its id) in the order they were fired; the others stay as they are. The events are what Python gave for
    /// four projectiles put in by `spawn_key`.
    #[test]
    fn kill_keys_takes_the_marked_projectiles_out_in_order() {
        use Value::{Float as F, Int as I};
        let bare = testing::bare_pair();
        let mut fight = testing::fight(&testing::bare_setup(&bare, 1), false);
        assert!(fight.keys.is_empty(), "a fight starts with no projectile");
        fight.t = 7.123456;
        let key = |own: usize, id: i64, p: V2| Key {
            p,
            v: [0.0, 0.0],
            own,
            id,
            vol: 7,
            idx: id,
            dmg: 2.0 + id as f64,
        };
        fight.keys = vec![
            key(0, 0, [0.5, -0.25]),
            key(1, 1, [1.5, 2.0]),
            key(0, 2, [-1.0, 0.125]),
            key(1, 3, [0.0, 0.0]),
        ];
        // nothing marked: nothing happens
        fight.kill_keys(&[false; 4], "wall", -1);
        assert!(fight.events.is_empty() && fight.keys.len() == 4);
        // Python's default `by` is -1: nobody
        fight.kill_keys(&[true, false, true, true], "wall", -1);
        assert_eq!(
            fight.events,
            vec![
                ev(7.12346, "wall", vec![I(0), I(-1), F(0.5), F(-0.25), I(0)]),
                ev(7.12346, "wall", vec![I(0), I(-1), F(-1.0), F(0.125), I(2)]),
                ev(7.12346, "wall", vec![I(1), I(-1), F(0.0), F(0.0), I(3)]),
            ]
        );
        assert_eq!(fight.keys, vec![key(1, 1, [1.5, 2.0])]);
        fight.kill_keys(&[true], "parry", 0);
        assert_eq!(
            fight.events.last(),
            Some(&ev(7.12346, "parry", vec![I(1), I(0), F(1.5), F(2.0), I(1)]))
        );
        assert!(fight.keys.is_empty());
    }

    /// A free ball flies straight at the shared speed. (The step divides by `norm + 1e-12`, so the speed is the
    /// shared one to about 1e-12, not to the last bit.)
    #[test]
    fn a_free_ball_flies_straight() {
        let bare = testing::bare_pair();
        let mut fight = testing::fight(&testing::bare_setup(&bare, 5), false);
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
        let mut fight = testing::fight(&testing::bare_setup(&bare, 5), false);
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
        let mut fight = testing::fight(&testing::bare_setup(&bare, 5), false);
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
            err(&|f| _ = f.pair_rules.insert("swat_drums".into(), Value::Bool(true))),
            "pair_rules.swat_drums: the engine has no rule of this name"
        );
        assert_eq!(
            err(&|f| _ = f.pair_rules.insert("swat_shells".into(), Value::Int(1))),
            "pair_rules.swat_shells: must be true or false"
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

#[cfg(test)]
mod error_tests {
    //! Task 14b: bad rules and bad roster entries are errors in words, never a panic.

    use super::*;
    use crate::roster::StrikerCfg;
    use crate::testing;

    /// The error of `run` and of `Fight::new` (the same), as one line.
    fn refused(setup: &Setup) -> String {
        let run_err = run(setup, false).expect_err("refused").to_string();
        let new_err = Fight::new(setup, false).map(|_| ()).expect_err("refused").to_string();
        assert_eq!(run_err, new_err);
        run_err
    }

    /// A change to the rules, and the error it must give.
    type RuleCase<'a> = (&'a dyn Fn(&mut Rules), &'a str);

    #[test]
    fn rules_the_engine_cannot_run_are_an_error() {
        let roster = testing::roster();
        let base = testing::episode_setup(&roster, 1);
        let cases: [RuleCase; 8] = [
            (&|r| r.dt = 0.0, "rules.DT: 0 is not above 0"),
            (
                &|r| r.dt = 1e-9,
                "rules.DT: the fight would take 54411764706 steps (at most 1000000)",
            ),
            (&|r| r.fps = 0, "rules.FPS: 0 is not a frame rate"),
            (&|r| r.t_max = f64::INFINITY, "rules.t_max: inf is not a finite number"),
            (&|r| r.rope_give = -0.0, "rules.ROPE_GIVE: -0 is negative"),
            (&|r| r.t_fight = -1.0, "rules.t_fight: -1 is negative"),
            (&|r| r.beat = 0.0, "rules.beat: 0 is not above 0"),
            (
                &|r| r.dmg_side = Some([1.0, f64::NAN]),
                "rules.dmg_side: NaN is not a finite number",
            ),
        ];
        for (change, want) in cases {
            let mut setup = base.clone();
            change(&mut setup.rules);
            assert_eq!(refused(&setup), want);
            // the test entry checks the rules too
            let err = Fight::with_unported(&setup, false).map(|_| ()).expect_err("refused");
            assert_eq!(err.to_string(), want);
        }
        // counters that would overflow (fix round 1): a beat so short, or a start so late, that the beat grid's
        // whole numbers pass 2^53 (the counters would saturate, then `next_beat += 1` overflow)
        let mut setup = base.clone();
        setup.rules.beat = 1e-300;
        assert_eq!(
            refused(&setup),
            "rules.beat: a beat of 1e-300 s is too short for a fight that runs to 5.8e1 s: the beat grid would count \
             2.32e302 sixteenth notes (at most 2^53)"
        );
        let mut setup = base.clone();
        setup.rules.beat = 1e-15;
        assert_eq!(
            refused(&setup),
            "rules.beat: a beat of 1e-15 s is too short for a fight that runs to 5.8e1 s: the beat grid would count \
             2.3199999999999997e17 sixteenth notes (at most 2^53)"
        );
        let mut setup = base.clone();
        setup.rules.t_fight = 1e18;
        setup.rules.t_max = 1e18 + 1024.0;
        assert!((setup.rules.t_max - setup.rules.t_fight) / setup.rules.dt <= crate::rules::MAX_STEPS);
        assert_eq!(
            refused(&setup),
            "rules.beat: a beat of 4.4852941176470584e-1 s is too short for a fight that runs to \
             1.000000000000001e18 s: the beat grid would count 8.918032786885256e18 sixteenth notes (at most 2^53)"
        );
        // a start the grid can count, but whose frame number cannot be
        let mut setup = base.clone();
        setup.rules.t_fight = 1e15;
        setup.rules.t_max = 1e15 + 1.0;
        assert_eq!(
            refused(&setup),
            "rules.t_fight: 1e15 s is too late a start: its frame number 3e16 is past 2^53"
        );
        // and through a fight file's pair rules, as a person would type them
        let roster = testing::roster();
        let mut ff = testing::example_fight_file();
        ff.pair_rules.insert("beat".into(), Value::Float(1e-300));
        let err = setup_from(&ff, &roster, 1).expect_err("refused").to_string();
        assert!(
            err.starts_with("pair_rules.beat: a beat of 1e-300 s is too short"),
            "{err}"
        );

        // the setup itself says so before anything runs
        let mut setup = base.clone();
        setup.rules.knock_t = 0.0;
        assert_eq!(
            setup.check().expect_err("refused").to_string(),
            "rules.KNOCK_T: 0 is not above 0"
        );
        assert_eq!(base.check(), Ok(()));
        assert!(run(&base, false).is_ok());
    }

    #[test]
    fn a_kind_not_ported_is_an_error() {
        let roster = testing::roster();
        let flute = testing::fighter(&roster, "FLUTE");
        let piano = testing::fighter(&roster, "PIANO");
        let mut setup = testing::episode_setup(&roster, 1);
        setup.fighters[1] = piano;
        assert_eq!(
            refused(&setup),
            r#"fighters[1]: the engine has not ported PIANO's weapon kind "roll" yet"#
        );
        // an ability of a kind not ported, on a fighter whose weapon is ported
        let mut odd = flute.clone();
        odd.low.kind = AbilityKind::Unsupported("stun".into());
        let mut setup = testing::episode_setup(&roster, 1);
        setup.fighters[0] = &odd;
        assert_eq!(
            refused(&setup),
            r#"fighters[0]: the engine has not ported FLUTE's ability PICCOLO (kind "stun") yet"#
        );
        // the bare pair of the unit tests has no weapon, and abilities of kind `none`
        let bare = testing::bare_pair();
        assert_eq!(
            refused(&testing::bare_setup(&bare, 1)),
            r#"fighters[0]: the engine has not ported LEFT's weapon kind "none" yet"#
        );
        // the test entry lets it in
        assert!(Fight::with_unported(&testing::bare_setup(&bare, 1), false).is_ok());
        assert!(run_with_unported(&testing::bare_setup(&bare, 1), false).is_ok());
    }

    /// A roster entry changed in code, with numbers the roster reader would refuse: the panics and endless fights
    /// they would cause are errors, in the reader's words, with the fighter's corner.
    #[test]
    fn a_roster_entry_the_engine_cannot_run_is_an_error() {
        let roster = testing::roster();
        let flute = testing::fighter(&roster, "FLUTE");
        let trombone = testing::fighter(&roster, "TROMBONE");
        let striker = |change: &dyn Fn(&mut StrikerCfg)| {
            let mut f = flute.clone();
            match &mut f.weapon {
                WeaponCfg::Striker(w) => change(w),
                other => panic!("{other:?}"),
            }
            f
        };
        let with_left = |f: &FighterCfg| {
            let mut setup = testing::episode_setup(&roster, 1);
            setup.fighters[0] = f;
            refused(&setup)
        };
        // a striker that would divide by zero on its first beat, or pop an empty list
        assert_eq!(
            with_left(&striker(&|w| w.drop_every = 0.5)),
            "fighters[0]: FLUTE: weapon.drop_every: 0.5 is less than 1 (a basket drops every that many beats)"
        );
        assert_eq!(
            with_left(&striker(&|w| w.max_baskets = 0.0)),
            "fighters[0]: FLUTE: weapon.max_baskets: 0 is less than 1 (there must be room for a basket)"
        );
        // an ability without a number its kind reads (Python: KeyError when it fires)
        let mut f = flute.clone();
        f.low.params.remove("speed");
        assert_eq!(with_left(&f), "fighters[0]: FLUTE: abilities.low.speed: missing");
        let mut f = flute.clone();
        f.unlock2.params.remove("every_bars");
        assert_eq!(
            with_left(&f),
            "fighters[0]: FLUTE: abilities.unlock2.every_bars: missing"
        );
        let mut f = flute.clone();
        f.unlock1.params.insert("mult".into(), 0.0);
        assert_eq!(
            with_left(&f),
            "fighters[0]: FLUTE: abilities.unlock1.mult: 0 is not above 0 (a haste divides the weapon's grid by it)"
        );
        // a lance whose fret count an i64 cannot carry
        let mut t = trombone.clone();
        if let WeaponCfg::Blade(w) = &mut t.weapon {
            w.frets0 = i64::MAX;
        }
        let mut setup = testing::episode_setup(&roster, 1);
        setup.fighters[1] = &t;
        assert_eq!(
            refused(&setup),
            "fighters[1]: TROMBONE: weapon.frets0: 9223372036854775807 is not a fret count (0 to 1000000)"
        );
        // the test entry: an unlock of a kind not ported still needs its timer
        let mut bare = testing::bare_pair();
        bare[1].unlock2.params.clear();
        let err = Fight::with_unported(&testing::bare_setup(&bare, 1), false)
            .map(|_| ())
            .expect_err("refused");
        assert_eq!(
            err.to_string(),
            "fighters[1]: RIGHT: abilities.unlock2.every_bars: missing (the bars between two uses of NOTHING)"
        );
    }
}
