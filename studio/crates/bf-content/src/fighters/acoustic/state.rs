//! What ACOUSTIC's look reads from the recording's events (`Look.setup` of `kit/fighters/acoustic.py`, 230 to 249):
//! its strokes (`RAKES`, `RAKE_T`, `RAKE_HIT`), its FINGERPICK strings (`WHIPS`) and its BIG STRUMs (`SWEEPS`). The
//! part reads the same events straight from the run, as the kit's `Sound.play` does.

use bf_core::dmath;
use bf_core::event::Event;

/// One stroke, from its `rake` event: `dict(t0, owner, main, L, c, aim, u, n, dir, S, dur)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Rake {
    /// `e[0]`.
    pub t0: f64,
    /// `e[2]`.
    pub owner: usize,
    /// `bool(e[3])`: on the weapon's own grid.
    pub main: bool,
    /// `e[4]`: the strings' length.
    pub l: f64,
    /// `np.array([e[5], e[6]])`: where the ball was.
    pub c: [f64; 2],
    /// `e[7]`: the aim.
    pub aim: f64,
    /// `np.array([math.cos(e[7]), math.sin(e[7])])`.
    pub u: [f64; 2],
    /// `np.array([-math.sin(e[7]), math.cos(e[7])])`.
    pub n: [f64; 2],
    /// `e[8]`: +1 down, -1 up.
    pub dir: f64,
    /// `e[9]`: the sweep to either side.
    pub s: f64,
    /// `e[10]`: the stroke's seconds.
    pub dur: f64,
}

/// One FINGERPICK string, from its `whip` event: `dict(t0, owner, j, ang, len)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Whip {
    /// `e[0]`.
    pub t0: f64,
    /// `e[2]`.
    pub owner: usize,
    /// `e[3]`: its place in the whip.
    pub j: i64,
    /// `e[4]`: its aim.
    pub ang: f64,
    /// `e[5]`: its length.
    pub len: f64,
}

/// One BIG STRUM, from its `sweep` event: `dict(t0, owner, c, u, aim, reach, dur)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Sweep {
    /// `e[0]`.
    pub t0: f64,
    /// `e[2]`.
    pub owner: usize,
    /// `np.array([e[3], e[4]])`.
    pub c: [f64; 2],
    /// `np.array([math.cos(e[5]), math.sin(e[5])])`.
    pub u: [f64; 2],
    /// `e[5]`.
    pub aim: f64,
    /// `e[6]`.
    pub reach: f64,
    /// `e[7]`.
    pub dur: f64,
}

/// Whether the event is fighter `k`'s of this kind (`e[1] == kind and e[2] == k`).
fn mine(e: &Event, kind: &str, k: usize) -> bool {
    e.kind == kind && e.who() == Some(k)
}

/// `self.RAKES`: fighter `k`'s strokes, in event order.
pub fn rakes(ev: &[Event], k: usize) -> Vec<Rake> {
    ev.iter()
        .filter(|e| mine(e, "rake", k))
        .map(|e| {
            let aim = e.f(7);
            let (s, c) = (dmath::sin(aim), dmath::cos(aim));
            Rake {
                t0: e.t,
                owner: k,
                main: e.i(3) != 0,
                l: e.f(4),
                c: [e.f(5), e.f(6)],
                aim,
                u: [c, s],
                n: [-s, c],
                dir: e.f(8),
                s: e.f(9),
                dur: e.f(10),
            }
        })
        .collect()
}

/// `self.RAKE_HIT`: the strokes (indices into [`rakes`]) that landed: for each of fighter `k`'s `hit` events with
/// `e[7] == 'rake'`, the last stroke not after it (`np.searchsorted(RAKE_T, e[0], 'right') - 1`), if any.
pub fn rake_hits(ev: &[Event], k: usize, rakes: &[Rake]) -> Vec<bool> {
    let mut hit = vec![false; rakes.len()];
    for e in ev.iter().filter(|e| mine(e, "hit", k) && e.s(7) == "rake") {
        let after = rakes.partition_point(|r| r.t0 <= e.t);
        if let Some(j) = after.checked_sub(1) {
            hit[j] = true;
        }
    }
    hit
}

/// `self.WHIPS`: fighter `k`'s FINGERPICK strings, in event order.
pub fn whips(ev: &[Event], k: usize) -> Vec<Whip> {
    ev.iter()
        .filter(|e| mine(e, "whip", k))
        .map(|e| Whip {
            t0: e.t,
            owner: k,
            j: e.i(3),
            ang: e.f(4),
            len: e.f(5),
        })
        .collect()
}

/// `self.SWEEPS`: fighter `k`'s BIG STRUMs, in event order.
pub fn sweeps(ev: &[Event], k: usize) -> Vec<Sweep> {
    ev.iter()
        .filter(|e| mine(e, "sweep", k))
        .map(|e| {
            let aim = e.f(5);
            Sweep {
                t0: e.t,
                owner: k,
                c: [e.f(3), e.f(4)],
                u: [dmath::cos(aim), dmath::sin(aim)],
                aim,
                reach: e.f(6),
                dur: e.f(7),
            }
        })
        .collect()
}
