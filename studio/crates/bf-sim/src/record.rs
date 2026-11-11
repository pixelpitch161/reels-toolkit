//! What is written down at every frame: `Fight.snap` (`fight.py` 5495 to 5732) for the keys every fight has, and the
//! finished fight as a [`Recording`].
//!
//! The keys, kinds and shapes are the Python recording's (`studio/docs/recording-format.md` section 6.3), so one
//! reader serves both: `P` and `V` are (frames, 2, 2) floats; `TH`, `BL`, `HP`, `SH`, `HA`, `SPIN`, `OM` (frames, 2)
//! floats; `FR`, `NV`, `LV` (frames, 2) whole numbers; `K` (frames, 72, 7) floats, NaN where there is no projectile;
//! `WAVES` and `SLAMS` ragged lists. `EXT`, `SLW` and `CRV` (frames, 2) are there when a fighter thrusts, curves or
//! slows ([`Fight::uses_ext`]). A weapon kind adds its own keys when a fighter has it: `COB`, a ragged list of the
//! baskets and cobras, when a fighter is a striker.

use std::collections::BTreeMap;

use bf_core::event::Value;
use bf_core::recording::{Array, RECORDING_FORMAT, Recording, Source};

use crate::fight::Fight;
use crate::geom;
use crate::kinds::striker;

/// `KMAX`: the most projectiles a frame records.
pub(crate) const KMAX: usize = 72;
/// What a projectile's row holds: x, y, heading, owner, id, index in its volley, volley id.
const K_FIELDS: usize = 7;
/// numpy's `np.nan`, the bits every unused slot of `K` holds.
const NAN: f64 = f64::from_bits(0x7FF8_0000_0000_0000);

/// The frames recorded so far (Python's `rec` dict of lists), flat in C order.
#[derive(Clone, Debug, Default)]
pub(crate) struct Frames {
    n: usize,
    p: Vec<f64>,
    v: Vec<f64>,
    th: Vec<f64>,
    bl: Vec<f64>,
    fr: Vec<i64>,
    nv: Vec<i64>,
    hp: Vec<f64>,
    lv: Vec<i64>,
    sh: Vec<f64>,
    ha: Vec<f64>,
    spin: Vec<f64>,
    om: Vec<f64>,
    k: Vec<f64>,
    waves: Vec<Value>,
    slams: Vec<Value>,
    /// `EXT`, `SLW`, `CRV`: only when a fighter thrusts, curves or slows.
    ext: Option<[Vec<f64>; 3]>,
    /// `COB`: the baskets and cobras, only when a fighter is a striker (`fight.py` 5773 to 5774).
    cob: Option<Vec<Value>>,
    // Later kinds add their keys the same way: an `Option` that is `Some` exactly when Python's `rec` has the key.
}

impl Frames {
    /// The empty record of a fight: the keys `run()` starts `rec` with (`fight.py` 5737), plus the opt-in keys this
    /// fight's fighters bring (`COB`, 5773 to 5774; `EXT`, `SLW` and `CRV`, 5779 to 5780).
    pub(crate) fn new(fight: &Fight) -> Frames {
        Frames {
            ext: fight.uses_ext.then(Default::default),
            cob: (0..2).any(|k| striker::is_striker(fight, k)).then(Vec::new),
            ..Frames::default()
        }
    }

    /// The number of frames recorded.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.n
    }

    /// `snap(rec)`: appends the state of the fight as one frame.
    pub(crate) fn snap(&mut self, fight: &Fight) {
        self.n += 1;
        for b in &fight.balls {
            self.p.extend(b.p);
            self.v.extend(b.v);
            self.th.push(b.th);
            self.bl.push(b.blade_len);
            self.fr.push(b.frets);
            // int(B.n_vol + 1e-9)
            self.nv.push((b.n_vol + 1e-9) as i64);
            self.hp.push(geom::py_max(0.0, b.hp));
            self.lv.push(b.level);
            self.sh.push(b.shield);
            self.ha.push(b.haste);
            self.spin.push(b.spin_dir);
            self.om.push(b.spin);
        }
        // K = np.full((KMAX, 7), np.nan)
        self.k.extend(std::iter::repeat_n(NAN, KMAX * K_FIELDS));
        // SLOT (projectile kinds), from `fight.keys`: fight.py 5510 to 5514 fills the first min(KMAX, len(kp)) rows
        // of this frame's K with [x, y, atan2(vy, vx), owner, id, index in its volley, volley id]. FLUTE and TROMBONE
        // fire none.
        let waves = fight.waves.iter().map(|w| {
            Value::List(vec![
                Value::Int(w.owner as i64),
                Value::Float(w.c[0]),
                Value::Float(w.c[1]),
                Value::Float(w.r),
                Value::Float(w.rmax),
                Value::Str(w.kind.clone()),
                Value::Float(w.t0),
            ])
        });
        self.waves.push(Value::List(waves.collect()));
        // a slam stays listed until 0.6 s after it lands
        let slams = fight.slams.iter().filter(|s| fight.t < s.t_slam + 0.6).map(|s| {
            Value::List(vec![
                Value::Int(s.owner as i64),
                Value::Float(s.c[0]),
                Value::Float(s.c[1]),
                Value::Float(s.t0),
                Value::Float(s.t_slam),
                Value::Float(s.radius),
            ])
        });
        self.slams.push(Value::List(slams.collect()));
        // fight.py 5520 to 5521: `RAD` (inflate). Kind not ported.
        if let Some(cob) = &mut self.cob {
            // #32 v2, 5522 to 5524: for each fighter the list of its baskets (none for a fighter without `baskets`),
            // each (id, home x, home y, cobra x, cobra y, t_drop)
            let sides = fight.striker.iter().map(|st| {
                let baskets = st.iter().flat_map(|st| &st.baskets).map(|bk| {
                    Value::List(vec![
                        Value::Int(bk.id),
                        Value::Float(bk.home[0]),
                        Value::Float(bk.home[1]),
                        Value::Float(bk.cp[0]),
                        Value::Float(bk.cp[1]),
                        Value::Float(bk.t_drop),
                    ])
                });
                Value::List(baskets.collect())
            });
            cob.push(Value::List(sides.collect()));
        }
        if let Some([ext, slw, crv]) = &mut self.ext {
            // #34, only recorded when a fighter uses the options
            for b in &fight.balls {
                ext.push(b.ext);
                slw.push(b.slow);
                crv.push(b.curve_now);
            }
        }
        // fight.py 5529 to 5732: the keys of the kinds not ported (HOUNDS ... RCHG), in that order.
    }

    /// The per-frame arrays of a recording, each under its Python key.
    fn into_arrays(self) -> BTreeMap<String, Array> {
        let n = self.n;
        let mut arrays = BTreeMap::new();
        let mut put = |key: &str, a: Array| _ = arrays.insert(key.to_string(), a);
        let pair = |data: Vec<f64>| Array::F64 {
            shape: vec![n, 2],
            data,
        };
        let whole = |data: Vec<i64>| Array::I64 {
            shape: vec![n, 2],
            data,
        };
        put(
            "P",
            Array::F64 {
                shape: vec![n, 2, 2],
                data: self.p,
            },
        );
        put(
            "V",
            Array::F64 {
                shape: vec![n, 2, 2],
                data: self.v,
            },
        );
        put("TH", pair(self.th));
        put("BL", pair(self.bl));
        put("FR", whole(self.fr));
        put("NV", whole(self.nv));
        put("HP", pair(self.hp));
        put("LV", whole(self.lv));
        put("SH", pair(self.sh));
        put("HA", pair(self.ha));
        put("SPIN", pair(self.spin));
        put("OM", pair(self.om));
        put(
            "K",
            Array::F64 {
                shape: vec![n, KMAX, K_FIELDS],
                data: self.k,
            },
        );
        put("WAVES", Array::Nested(Value::List(self.waves)));
        put("SLAMS", Array::Nested(Value::List(self.slams)));
        if let Some(cob) = self.cob {
            put("COB", Array::Nested(Value::List(cob)));
        }
        if let Some([ext, slw, crv]) = self.ext {
            put("EXT", pair(ext));
            put("SLW", pair(slw));
            put("CRV", pair(crv));
        }
        arrays
    }
}

/// The finished fight as a recording made by this app: what `run()` returns (`fight.py` 6216 to 6222), with the
/// rules it was fought under. Without frames (a fight that did not record) the recording has no arrays.
pub(crate) fn recording(fight: Fight, frames: Option<Frames>) -> Recording {
    Recording {
        format: RECORDING_FORMAT,
        source: Source::studio(),
        names: fight.balls.iter().map(|b| b.name.to_string()).collect(),
        seed: fight.seed,
        winner: fight.winner,
        t_ko: fight.t_ko,
        f0: fight.rules.f0(),
        fps: fight.rules.fps,
        corner: fight.corner.to_vec(),
        rules: fight.rules.to_values(),
        events: fight.events,
        arrays: frames.map(Frames::into_arrays).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fight::{Slam, Wave};
    use crate::testing;

    fn floats(arrays: &BTreeMap<String, Array>, key: &str) -> (Vec<usize>, Vec<f64>) {
        match &arrays[key] {
            Array::F64 { shape, data } => (shape.clone(), data.clone()),
            other => panic!("{key} is {other:?}"),
        }
    }

    fn wholes(arrays: &BTreeMap<String, Array>, key: &str) -> (Vec<usize>, Vec<i64>) {
        match &arrays[key] {
            Array::I64 { shape, data } => (shape.clone(), data.clone()),
            other => panic!("{key} is {other:?}"),
        }
    }

    #[test]
    fn a_frame_holds_the_balls_state() {
        let roster = testing::roster();
        let mut fight = testing::fight(&testing::episode_setup(&roster, 392), true);
        let mut frames = Frames::new(&fight);
        frames.snap(&fight);
        // change everything a frame holds, then take a second frame
        fight.balls[0].hp = -3.5; // below zero on the last hit: recorded as 0
        fight.balls[1].hp = 61.25;
        fight.balls[0].level = 3;
        fight.balls[1].frets = 4;
        fight.balls[1].blade_len = 1.19;
        fight.balls[0].n_vol = 2.9999999999; // int(n_vol + 1e-9): a volley of 3 that float noise left just below
        fight.balls[1].n_vol = 2.5;
        fight.balls[0].shield = 0.25;
        fight.balls[1].haste = 1.5;
        fight.balls[1].ext = 0.95;
        fight.balls[0].slow = 0.75;
        fight.balls[0].p = [0.5, -0.25];
        fight.balls[1].v = [-1.0, 2.0];
        frames.snap(&fight);
        assert_eq!(frames.len(), 2);
        let a = frames.into_arrays();

        let (shape, p) = floats(&a, "P");
        assert_eq!(shape, [2, 2, 2]);
        assert_eq!(p[..4], [-1.85, 1.85, 1.85, -1.85]);
        assert_eq!(p[4..6], [0.5, -0.25]);
        let (shape, v) = floats(&a, "V");
        assert_eq!((shape, &v[6..]), (vec![2, 2, 2], &[-1.0, 2.0][..]));
        assert_eq!(floats(&a, "HP"), (vec![2, 2], vec![100.0, 100.0, 0.0, 61.25]));
        assert_eq!(wholes(&a, "LV"), (vec![2, 2], vec![1, 1, 3, 1]));
        assert_eq!(wholes(&a, "FR"), (vec![2, 2], vec![0, 1, 0, 4]));
        assert_eq!(wholes(&a, "NV"), (vec![2, 2], vec![0, 0, 3, 2]));
        assert_eq!(floats(&a, "BL").1, [1.4, 0.95, 1.4, 1.19]);
        assert_eq!(floats(&a, "SH").1, [0.0, 0.0, 0.25, 0.0]);
        assert_eq!(floats(&a, "HA").1, [0.0, 0.0, 0.0, 1.5]);
        assert_eq!(floats(&a, "SPIN").1, [1.0, -1.0, 1.0, -1.0]);
        assert_eq!(floats(&a, "OM").1, [0.0; 4]);
        assert_eq!(floats(&a, "TH").0, [2, 2]);
        assert_eq!(floats(&a, "EXT"), (vec![2, 2], vec![0.0, 0.3, 0.0, 0.95]));
        assert_eq!(floats(&a, "SLW").1, [0.0, 0.0, 0.75, 0.0]);
        assert_eq!(floats(&a, "CRV").1, [0.0; 4]);
        let (shape, k) = floats(&a, "K");
        assert_eq!((shape, k.len()), (vec![2, 72, 7], 2 * 72 * 7));
        assert!(k.iter().all(|x| x.to_bits() == 0x7FF8_0000_0000_0000));
        let empty = Array::Nested(Value::List(vec![Value::List(vec![]), Value::List(vec![])]));
        assert_eq!((&a["WAVES"], &a["SLAMS"]), (&empty, &empty));
    }

    /// Fighters that neither thrust, curve nor slow have no `EXT`, `SLW`, `CRV`, and without a striker there is no
    /// `COB`.
    #[test]
    fn the_opt_in_keys_follow_the_fighters() {
        let bare = testing::bare_pair();
        let fight = testing::fight(&testing::bare_setup(&bare, 1), true);
        let mut frames = Frames::new(&fight);
        frames.snap(&fight);
        let a = frames.into_arrays();
        let keys: Vec<&str> = a.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            [
                "BL", "FR", "HA", "HP", "K", "LV", "NV", "OM", "P", "SH", "SLAMS", "SPIN", "TH", "V", "WAVES"
            ]
        );
        // FLUTE against TROMBONE has the 19 keys of the Python recordings
        let roster = testing::roster();
        let fight = testing::fight(&testing::episode_setup(&roster, 1), true);
        let mut frames = Frames::new(&fight);
        frames.snap(&fight);
        let a = frames.into_arrays();
        let py = testing::python_recording("seed1.bfrec");
        assert_eq!(a.keys().collect::<Vec<_>>(), py.arrays.keys().collect::<Vec<_>>());
        assert_eq!(a.len(), 19);
    }

    /// `COB`: per frame, for each fighter the list of its baskets, each `[id, home x, home y, cobra x, cobra y,
    /// t_drop]` with the id a whole number; a fighter without baskets (the TROMBONE; the FLUTE before the fight loop
    /// has set it up) has an empty list.
    #[test]
    fn cob_lists_each_fighters_baskets() {
        use crate::kinds::striker::{Basket, setup};
        use Value::{Float as F, Int as I, List as L};
        let roster = testing::roster();
        let ep = testing::episode_setup(&roster, 1);
        // TROMBONE on the left, so the baskets are the second list
        let swapped = crate::Setup {
            fighters: [ep.fighters[1], ep.fighters[0]],
            ..ep
        };
        let mut fight = testing::fight(&swapped, true);
        let mut frames = Frames::new(&fight);
        frames.snap(&fight); // the ball has no `baskets` yet
        setup(&mut fight, 1);
        frames.snap(&fight); // none on the floor
        let st = fight.striker[1].as_mut().expect("set up");
        st.baskets.push(Basket {
            id: 3,
            home: [0.5, -0.25],
            cp: [0.5, -0.25],
            t_drop: 8.97156862745098,
            strike: None,
            down: None,
        });
        st.baskets.push(Basket {
            id: 4,
            home: [1.5, 2.0],
            cp: [1.25, 1.75], // out dancing
            t_drop: 10.77156862745098,
            strike: None,
            down: Some(11.0),
        });
        frames.snap(&fight);
        let a = frames.into_arrays();
        let none = || L(vec![L(vec![]), L(vec![])]);
        let rows = L(vec![
            L(vec![I(3), F(0.5), F(-0.25), F(0.5), F(-0.25), F(8.97156862745098)]),
            L(vec![I(4), F(1.5), F(2.0), F(1.25), F(1.75), F(10.77156862745098)]),
        ]);
        assert_eq!(
            a["COB"],
            Array::Nested(L(vec![none(), none(), L(vec![L(vec![]), rows])]))
        );
    }

    /// The rows are the tuples `snap` builds: a wave is (owner, x, y, radius now, final radius, kind, start time); a
    /// slam is (owner, x, y, time called, time it lands, radius) and is listed until 0.6 s after it lands.
    #[test]
    fn waves_and_slams_are_listed_as_python_lists_them() {
        use Value::{Float as F, Int as I};
        let bare = testing::bare_pair();
        let mut fight = testing::fight(&testing::bare_setup(&bare, 1), true);
        fight.waves.push(Wave {
            owner: 1,
            c: [0.5, -1.5],
            r: 0.6,
            speed: 6.0,
            rmax: 3.4,
            dmg: 3.0,
            knock: 1.0,
            kind: "shockwave".into(),
            hit: false,
            name: "SAD TROMBONE".into(),
            t0: 16.25,
            slow: None,
        });
        fight.slams.push(Slam {
            owner: 0,
            c: [1.0, 2.0],
            t0: 20.0,
            t_slam: 20.5,
            radius: 1.35,
            dmg: 24.0,
            knock: 1.6,
            done: false,
            name: "PEDAL TONE".into(),
        });
        let mut frames = Frames::new(&fight);
        for t in [20.25, 21.0, 21.1, 21.125] {
            fight.t = t;
            frames.snap(&fight);
        }
        let a = frames.into_arrays();
        let wave = Value::List(vec![
            I(1),
            F(0.5),
            F(-1.5),
            F(0.6),
            F(3.4),
            Value::Str("shockwave".into()),
            F(16.25),
        ]);
        let slam = Value::List(vec![I(0), F(1.0), F(2.0), F(20.0), F(20.5), F(1.35)]);
        let rows = |v: &[Value]| Value::List(v.to_vec());
        assert_eq!(
            a["WAVES"],
            Array::Nested(Value::List(vec![rows(std::slice::from_ref(&wave)); 4]))
        );
        // 20.5 + 0.6 = 21.1: listed while t < 21.1
        assert_eq!(
            a["SLAMS"],
            Array::Nested(Value::List(vec![
                rows(std::slice::from_ref(&slam)),
                rows(std::slice::from_ref(&slam)),
                rows(&[]),
                rows(&[])
            ]))
        );
    }

    #[test]
    fn the_recording_carries_the_fights_header() {
        let roster = testing::roster();
        let setup = testing::episode_setup(&roster, 392);
        let mut fight = testing::fight(&setup, false);
        fight.winner = Some(1);
        fight.t_ko = Some(50.5);
        fight.t = 50.5;
        fight.ev("ko", 0, vec![Value::Int(1)]);
        let rec = recording(fight, None);
        assert_eq!((rec.format, &rec.source), (RECORDING_FORMAT, &Source::studio()));
        assert_eq!(rec.names, ["FLUTE", "TROMBONE"]);
        assert_eq!(
            (rec.seed, rec.winner, rec.t_ko, rec.f0, rec.fps),
            (392, Some(1), Some(50.5), 108, 30)
        );
        assert_eq!(rec.corner, vec![[-1.85, 1.85], [1.85, -1.85]]);
        assert_eq!(rec.rules, setup.rules.to_values());
        assert_eq!(rec.events.len(), 1);
        assert!(rec.arrays.is_empty());
    }
}
