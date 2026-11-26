//! The static scene: a low-poly boxing ring floating in a dark arena. The port of `projects/ballfight/scene.py`,
//! with the runtime patches `kit/picture.py` makes to it (the fighters' colours on the corner posts, the neutral
//! band on the skirt).
//!
//! World: y up, the canvas at y = 0. The fight runs in the ring's own frame `(sx, sz)` in `[-A, A]^2`; the ring
//! is turned 45 degrees so the red corner sits on the screen's left and the blue corner on its right:
//! world `X = (sx - sz) / sqrt 2`, world `Z = (sx + sz) / sqrt 2` (the camera is on +Z, looking toward -Z).
//!
//! Not ported: `TRUSS` and `CANS` (nothing in `kit/picture.py` draws them).

use bf_core::dmath;
use bf_core::rng::Generator;

use crate::camera::V3;
use crate::geometry::prism;
use crate::np;
use crate::v3;

/// `S2 = math.sqrt(2.0)`.
pub const S2: f64 = std::f64::consts::SQRT_2;
/// `A`: the ropes (the walls of the fight).
pub const A: f64 = 3.0;
/// `AP = A + 0.62`: the apron's edge (the canvas).
pub const AP: f64 = A + 0.62;
/// `POST = A + 0.24`: the corner posts.
pub const POST: f64 = A + 0.24;
/// `POST_H`.
pub const POST_H: f64 = 1.42;
/// `ROPE_Y`: the heights of the three ropes.
pub const ROPE_Y: [f64; 3] = [0.42, 0.80, 1.18];
/// `FLOOR_Y`: the arena floor.
pub const FLOOR_Y: f64 = -1.15;
/// `R_BALL`.
pub const R_BALL: f64 = 0.6;
/// `SUBJECT`: the point the lens is focused on.
pub const SUBJECT: V3 = [0.0, 0.4, 0.0];
/// `SPOT`: the key light, a hard spot over the ring.
pub const SPOT: V3 = [0.0, 9.5, -1.4];
/// `SPOT_AIM`.
pub const SPOT_AIM: V3 = [0.0, 0.0, 0.0];

/// `CANVAS`: the slate-blue canvas.
pub const CANVAS: V3 = [104.0, 98.0, 142.0];
/// `CANVAS_RIM`: the apron band outside the ropes.
pub const CANVAS_RIM: V3 = [70.0, 58.0, 104.0];
/// `SKIRT`.
pub const SKIRT: V3 = [34.0, 30.0, 58.0];
/// `SKIRT_BAND` as `kit/picture.py` patches it: a neutral band on the skirt.
pub const SKIRT_BAND: V3 = [150.0, 140.0, 184.0];
/// `ARENA_FLOOR`.
pub const ARENA_FLOOR: V3 = [46.0, 40.0, 62.0];

/// `BARRIER_R`: the LED boards around the ring floor.
pub const BARRIER_R: f64 = 5.55;
/// `BARRIER_H`.
pub const BARRIER_H: f64 = 0.72;
/// `STAND_R0`.
pub const STAND_R0: f64 = 6.2;
/// `ROW_W`.
pub const ROW_W: f64 = 0.82;
/// `ROW_H`.
pub const ROW_H: f64 = 0.40;
/// `N_ROWS`.
pub const N_ROWS: usize = 10;

/// `CROWD_COLS`: the shirt colours.
pub const CROWD_COLS: [V3; 8] = [
    [86.0, 70.0, 118.0],
    [118.0, 74.0, 104.0],
    [70.0, 92.0, 132.0],
    [124.0, 98.0, 84.0],
    [96.0, 70.0, 82.0],
    [74.0, 80.0, 108.0],
    [132.0, 112.0, 70.0],
    [64.0, 104.0, 98.0],
];

/// `ROPE_COL`: the colours of the three ropes (the middle one is replaced by the fighter's side colour).
pub const ROPE_COL: [V3; 3] = [[236.0, 230.0, 222.0], [228.0, 72.0, 78.0], [236.0, 230.0, 222.0]];

/// The four corners in the order of the Python dict `CORNERS`: red, blue, front, back.
pub const CORNER_NAMES: [&str; 4] = ["red", "blue", "front", "back"];
/// `CORNERS`, in the ring frame, in the order of [`CORNER_NAMES`].
pub const CORNERS: [[f64; 2]; 4] = [[-POST, POST], [POST, -POST], [POST, POST], [-POST, -POST]];
/// The index of a corner in [`CORNERS`].
pub const RED: usize = 0;
pub const BLUE: usize = 1;
pub const FRONT: usize = 2;
pub const BACK: usize = 3;
/// `ROPE_SIDES`: the four sides of the ring as pairs of corners (indices into [`CORNERS`]).
pub const ROPE_SIDES: [(usize, usize); 4] = [(RED, FRONT), (FRONT, BLUE), (BLUE, BACK), (BACK, RED)];

/// `math.radians(x)`.
fn radians(x: f64) -> f64 {
    x * (dmath::PI / 180.0)
}

/// `SPOT_COS = (cos(radians(27)), cos(radians(19)))`: the cone's edge (soft from 19 to 27 degrees).
pub fn spot_cos() -> (f64, f64) {
    (dmath::cos(radians(27.0)), dmath::cos(radians(19.0)))
}

/// `to_world(sx, sz, y)`: a ring-frame point at height `y` in the world.
pub fn to_world(sx: f64, sz: f64, y: f64) -> V3 {
    [(sx - sz) / S2, y, (sx + sz) / S2]
}

/// `to_ring(X, Z)`: a world position in the ring frame, `(sx, sz)`.
pub fn to_ring(x: f64, z: f64) -> (f64, f64) {
    ((x + z) / S2, (z - x) / S2)
}

/// `canvas_albedo(sx, sz)` for one pixel: the canvas, a faint centre ring and star, the apron band.
pub fn canvas_albedo(sx: f64, sz: f64) -> V3 {
    // r = np.hypot(sx, sz): numpy's hypot, the plain root
    let r = np::hypot(sx, sz);
    let mut col = CANVAS;
    if (r - 1.55).abs() < 0.06 || (r - 1.32).abs() < 0.025 {
        col = mix(col, 0.82, [220.0, 200.0, 255.0], 0.18);
    }
    // the centre star (5 points), very faint
    let ang = dmath::atan2(sz, sx);
    let star = r < 0.42 + 0.30 * dmath::cos(2.5 * (ang + 0.3)).abs();
    if star && r < 0.9 {
        col = mix(col, 0.9, [230.0, 210.0, 255.0], 0.1);
    }
    if sx.abs() > A || sz.abs() > A {
        col = CANVAS_RIM;
    }
    if np::maximum(sx.abs(), sz.abs()) > AP - 0.08 {
        col = [224.0, 214.0, 236.0];
    }
    col
}

/// `a * ka + b * kb`, channel by channel.
fn mix(a: V3, ka: f64, b: V3, kb: f64) -> V3 {
    [a[0] * ka + b[0] * kb, a[1] * ka + b[1] * kb, a[2] * ka + b[2] * kb]
}

/// A shaded face of the static scene: its corners, its unit normal and its albedo.
#[derive(Clone, Debug, PartialEq)]
pub struct ColFace {
    pub q: Vec<V3>,
    pub n: V3,
    pub col: V3,
}

/// One person of the crowd: `(feet position on its tier, shirt colour index, phase, row, height)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Person {
    pub p: V3,
    pub ci: usize,
    pub ph: f64,
    pub row: usize,
    pub ht: f64,
}

/// One face of the stands: `(quad, normal, colour, angle)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Stand {
    pub q: [V3; 4],
    pub n: V3,
    pub col: V3,
    pub am: f64,
}

/// One face of the LED boards: `(quad, normal, angle)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Board {
    pub q: [V3; 4],
    pub n: V3,
    pub am: f64,
}

/// The scene of one picture: everything `scene.py` builds at import, with the two fighters' colours on it.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    /// `CORNER_COL`, in the order of [`CORNER_NAMES`]: the fighters' colours times 0.9 on the red and blue
    /// corners.
    pub corner_col: [V3; 4],
    /// `RED_LIGHT`.
    pub red_light: V3,
    /// `BLUE_LIGHT`.
    pub blue_light: V3,
    /// `SKIRT_FACES`.
    pub skirt_faces: Vec<ColFace>,
    /// `POSTS`, in the order of [`CORNER_NAMES`].
    pub posts: [Vec<ColFace>; 4],
    /// `CROWD`.
    pub crowd: Vec<Person>,
    /// `STANDS`.
    pub stands: Vec<Stand>,
    /// `BARRIER`.
    pub barrier: Vec<Board>,
}

impl Scene {
    /// The scene for two fighters whose colours are `col` (`COL[0]`, `COL[1]`).
    pub fn new(col: [V3; 2]) -> Scene {
        // S.CORNER_COL['red'] = np.clip(COL[0] * 0.9, 0, 255)
        let side = |c: V3| c.map(|v| np::clip(v * 0.9, 0.0, 255.0));
        let neutral = [236.0, 232.0, 222.0];
        let corner_col = [side(col[0]), side(col[1]), neutral, neutral];
        let (stands, barrier) = (stands(), barrier());
        Scene {
            corner_col,
            red_light: to_world(CORNERS[RED][0], CORNERS[RED][1], POST_H + 0.35),
            blue_light: to_world(CORNERS[BLUE][0], CORNERS[BLUE][1], POST_H + 0.35),
            skirt_faces: platform_faces(),
            posts: std::array::from_fn(|c| post_faces(c, corner_col[c])),
            crowd: crowd(),
            stands,
            barrier,
        }
    }
}

/// `platform_faces()`: the ring platform's skirt (4 sides) with a band.
fn platform_faces() -> Vec<ColFace> {
    let c = [(AP, AP), (AP, -AP), (-AP, -AP), (-AP, AP)];
    let mut out = Vec::with_capacity(12);
    for k in 0..4 {
        let (a, b) = (c[k], c[(k + 1) % 4]);
        for (y0, y1, col) in [(FLOOR_Y, -0.32, SKIRT), (-0.32, -0.18, SKIRT_BAND), (-0.18, 0.0, SKIRT)] {
            let q = vec![
                to_world(a.0, a.1, y0),
                to_world(b.0, b.1, y0),
                to_world(b.0, b.1, y1),
                to_world(a.0, a.1, y1),
            ];
            let mid = to_world((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, 0.0);
            // n = mid / (np.linalg.norm(mid) + 1e-9); n[1] = 0; n / np.linalg.norm(n)
            let mut n = v3::div(mid, v3::norm(mid) + 1e-9);
            n[1] = 0.0;
            out.push(ColFace {
                q,
                n: v3::div(n, v3::norm(n)),
                col,
            });
        }
    }
    out
}

/// `post_faces(name)`: a padded corner post: a steel pole and three turnbuckle cushions in the corner's colour.
fn post_faces(corner: usize, corner_col: V3) -> Vec<ColFace> {
    let w = to_world(CORNERS[corner][0], CORNERS[corner][1], 0.0);
    let mut out = Vec::new();
    let mut add = |r: f64, y0: f64, y1: f64, col: V3| {
        for (q, n) in prism(w[0], w[2], r, y0, y1, 6, None, 0.0, (0.0, 0.0)) {
            out.push(ColFace { q, n, col });
        }
    };
    add(0.085, 0.0, POST_H, [190.0, 190.0, 204.0]);
    for y in ROPE_Y {
        add(0.16, y - 0.11, y + 0.11, corner_col);
    }
    add(0.11, POST_H, POST_H + 0.06, [220.0, 220.0, 230.0]);
    out
}

/// `CROWD`: the people on the stands, placed by `np.random.default_rng(27)`.
fn crowd() -> Vec<Person> {
    let mut rng = Generator::new(27);
    let mut out = Vec::new();
    for row in 0..N_ROWS {
        let rr = STAND_R0 + row as f64 * ROW_W + 0.42;
        let yy = FLOOR_Y + (row + 1) as f64 * ROW_H;
        // n = int(2 * math.pi * rr / 0.40)
        let n = (2.0 * dmath::PI * rr / 0.40) as usize;
        for k in 0..n {
            // a = 2 * math.pi * (k + 0.5 * (row % 2)) / n + rng.uniform(-0.035, 0.035)
            let a = 2.0 * dmath::PI * (k as f64 + 0.5 * (row % 2) as f64) / n as f64 + rng.uniform(-0.035, 0.035);
            if rng.random() < 0.10 {
                continue;
            }
            let p = [rr * dmath::sin(a), yy, -rr * dmath::cos(a)];
            // (p, int(rng.integers(0, len(CROWD_COLS))), float(rng.random()), row, float(rng.uniform(0.92, 1.1)))
            let ci = rng.integers(CROWD_COLS.len() as u64) as usize;
            let ph = rng.random();
            let ht = rng.uniform(0.92, 1.1);
            out.push(Person { p, ci, ph, row, ht });
        }
    }
    out
}

/// `STANDS`: the stepped tiers under the crowd.
fn stands() -> Vec<Stand> {
    let mut out = Vec::with_capacity(N_ROWS * 56 * 2);
    let p = |r: f64, a: f64, yv: f64| -> V3 { [r * dmath::sin(a), yv, -r * dmath::cos(a)] };
    for row in 0..N_ROWS {
        let (r0, r1) = (STAND_R0 + row as f64 * ROW_W, STAND_R0 + (row + 1) as f64 * ROW_W);
        let y = FLOOR_Y + row as f64 * ROW_H;
        for k in 0..56 {
            let (a0, a1) = (
                2.0 * dmath::PI * k as f64 / 56.0,
                2.0 * dmath::PI * (k + 1) as f64 / 56.0,
            );
            let top = [
                p(r0, a0, y + ROW_H),
                p(r0, a1, y + ROW_H),
                p(r1, a1, y + ROW_H),
                p(r1, a0, y + ROW_H),
            ];
            let riser = [p(r0, a0, y), p(r0, a1, y), p(r0, a1, y + ROW_H), p(r0, a0, y + ROW_H)];
            let am = (a0 + a1) / 2.0;
            out.push(Stand {
                q: top,
                n: [0.0, 1.0, 0.0],
                col: [54.0, 46.0, 74.0],
                am,
            });
            out.push(Stand {
                q: riser,
                // -np.array([math.sin(am), 0, -math.cos(am)])
                n: v3::neg([dmath::sin(am), 0.0, -dmath::cos(am)]),
                col: [40.0, 34.0, 58.0],
                am,
            });
        }
    }
    out
}

/// `BARRIER`: both faces of the LED boards.
fn barrier() -> Vec<Board> {
    let mut out = Vec::with_capacity(128);
    let p = |a: f64, yv: f64| -> V3 { [BARRIER_R * dmath::sin(a), yv, -BARRIER_R * dmath::cos(a)] };
    for k in 0..64 {
        let (a0, a1) = (
            2.0 * dmath::PI * k as f64 / 64.0,
            2.0 * dmath::PI * (k + 1) as f64 / 64.0,
        );
        let q = [
            p(a0, FLOOR_Y),
            p(a1, FLOOR_Y),
            p(a1, FLOOR_Y + BARRIER_H),
            p(a0, FLOOR_Y + BARRIER_H),
        ];
        let am = (a0 + a1) / 2.0;
        let n_out = [dmath::sin(am), 0.0, -dmath::cos(am)];
        out.push(Board { q, n: n_out, am });
        let mut back = q;
        back.reverse();
        out.push(Board {
            q: back,
            n: v3::neg(n_out),
            am,
        });
    }
    out
}
