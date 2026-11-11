//! The 2-D convex hull in the order `scipy.spatial.ConvexHull(points).vertices` gives.
//!
//! The ring's spot beam is `pts[ConvexHull(pts).vertices]` drawn as a polygon, so the Studio needs the hull's
//! corners in scipy's order: counter-clockwise (for x to the right and y up), **starting at a corner that only
//! Qhull's own bookkeeping decides**. scipy walks Qhull's facet list from its first facet
//! (`_Qhull.get_extremes_2d` in `scipy/spatial/_qhull.pyx`), and which facet is first at the end depends on how
//! Qhull 8.0.2 built the hull: the first triangle it picks, the order it adds points in, where each new edge
//! goes in the list. No rule on the points alone gives it (over random inputs the start is the leftmost point
//! in a quarter of the cases, the rightmost in a fifth, neither in another fifth).
//!
//! So [`qhull_order`] runs Qhull's algorithm for 2-D input step for step, from `libqhull_r` (`qh_maxmin`,
//! `qh_maxsimplex`, `qh_initialhull`, `qh_partitionall`, `qh_furthestnext`, then per added point
//! `qh_nextfurthest`, `qh_findhorizon`, `qh_makenewfacets`, `qh_partitionvisible` with `qh_findbest`,
//! `qh_findbestnew` and `qh_findbesthorizon`, and the narrow-hull handling for a sliver of a first triangle),
//! with Qhull's own distances and thresholds. Every decision is a comparison of plain `f64` arithmetic (one
//! square root per edge), so the order is the same on every PC. Besides the reference cases
//! (`studio/reference/picture_kit/hull.npz`) it was compared with scipy 1.18.1 on 40,000 seeded point sets
//! (clouds from 1e-6 to 1e6 across, ellipses, the beam's shape, near-lines, tight clusters, whole-number grids,
//! repeated points): it gave an order for 36,979 of them, and every one was scipy's.
//!
//! What is not ported is Qhull's merging. When a new point is within rounding of the line of an edge it does not
//! replace (three points in a row, met in an awkward order), or the new edges are not clearly convex, Qhull
//! merges facets; when the points do not span a plane it stops with an error. [`qhull_order`] reports those
//! inputs instead of guessing (the other 3,021 of the 40,000: 2,903 sets drawn from a 6 x 6 grid of whole
//! numbers, with many points in a row, and 118 that scipy itself refuses), and [`convex_hull`] then returns the
//! hull by a plain scan: the hull's corners, counter-clockwise, but starting at the leftmost point. Repeated
//! points and points along an edge are fine as long as Qhull happens to add the end points of a row before a
//! middle one; the beam (a point and 32 points of an ellipse) never comes near a merge.

/// Why [`qhull_order`] did not follow Qhull: the input is one Qhull itself treats specially.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unusual {
    /// Fewer than three points, or a coordinate that is not a finite number (scipy raises).
    NotEnoughPoints,
    /// All points on one line: Qhull finds no first triangle (scipy raises `QhullError`).
    Flat,
    /// A point within rounding of an edge's line, or new edges that are not clearly convex: Qhull merges.
    Merge,
}

/// The corners of the convex hull of `pts`, as indices into `pts`, in the order
/// `scipy.spatial.ConvexHull(pts).vertices` gives them: counter-clockwise for x right and y up (clockwise on a
/// screen), from Qhull's own starting corner. Points inside the hull or along its edges are left out.
///
/// For an input where Qhull would merge or fail (see [`Unusual`]) the result is the same hull from a plain scan,
/// counter-clockwise from the leftmost point (the lower one of two); fewer than three distinct points come back
/// as they are.
pub fn convex_hull(pts: &[(f64, f64)]) -> Vec<usize> {
    qhull_order(pts).unwrap_or_else(|_| scan_hull(pts))
}

/// [`convex_hull`] where Qhull's order is certain, else the reason it is not.
pub fn qhull_order(pts: &[(f64, f64)]) -> Result<Vec<usize>, Unusual> {
    if pts.len() < 3 || pts.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
        return Err(Unusual::NotEnoughPoints);
    }
    Qhull::new(pts)?.build()
}

/// The hull by Andrew's monotone chain: counter-clockwise from the leftmost (then lowest) point, corners only.
fn scan_hull(pts: &[(f64, f64)]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..pts.len())
        .filter(|&i| pts[i].0.is_finite() && pts[i].1.is_finite())
        .collect();
    order.sort_by(|&a, &b| {
        pts[a]
            .0
            .total_cmp(&pts[b].0)
            .then(pts[a].1.total_cmp(&pts[b].1))
            .then(a.cmp(&b))
    });
    order.dedup_by(|a, b| pts[*a] == pts[*b]);
    if order.len() < 3 {
        return order;
    }
    let turn = |o: usize, a: usize, b: usize| {
        (pts[a].0 - pts[o].0) * (pts[b].1 - pts[o].1) - (pts[a].1 - pts[o].1) * (pts[b].0 - pts[o].0)
    };
    let mut hull: Vec<usize> = Vec::with_capacity(order.len() + 1);
    for pass in 0..2 {
        let base = hull.len();
        let walk: Vec<usize> = if pass == 0 {
            order.clone()
        } else {
            order.iter().rev().copied().collect()
        };
        for i in walk {
            while hull.len() >= base + 2 && turn(hull[hull.len() - 2], hull[hull.len() - 1], i) <= 0.0 {
                hull.pop();
            }
            hull.push(i);
        }
        // The last point of each chain starts the next one.
        hull.pop();
    }
    hull
}

// ---------------------------------------------------------------- Qhull, two dimensions
const REAL_EPSILON: f64 = f64::EPSILON;
const REAL_MAX: f64 = f64::MAX;
/// `qh_RATIOmaxsimplex`.
const RATIO_MAXSIMPLEX: f64 = 1.0e-3;
/// `qh_MAXnarrow`.
const MAX_NARROW: f64 = -0.99999999;

/// A place in Qhull's facet list: nowhere, the sentinel after the last facet, or a facet.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ptr {
    Null,
    Tail,
    At(usize),
}

/// `facetT` for 2-D: an edge of the hull.
struct Facet {
    /// The two corners (indices into the vertex list), the later-made vertex first, as Qhull sorts them.
    v: [usize; 2],
    /// `toporient`: which way round the corners go.
    top: bool,
    /// The neighbouring edges: `nb[k]` is across the corner `v[k]`, so it shares the other one. `None` once the
    /// facet is visible and its cone is made.
    nb: [Option<usize>; 2],
    normal: (f64, f64),
    offset: f64,
    /// `outsideset`: the points above this edge still to be added, the furthest last.
    outside: Option<Vec<usize>>,
    furthest_dist: f64,
    visible: bool,
    new: bool,
    replace: Option<usize>,
    visit: u32,
}

struct Qhull {
    /// The input points, then the interior point of the first triangle.
    pts: Vec<(f64, f64)>,
    /// The point of each vertex, in the order the vertices were made.
    vertices: Vec<usize>,
    facets: Vec<Facet>,
    /// `qh.facet_list`, without its sentinel.
    list: Vec<usize>,
    facet_next: Ptr,
    newfacet_list: Ptr,
    visible_list: Ptr,
    visit_id: u32,
    max_outside: f64,
    findbestnew: bool,
    findbest_notsharp: bool,
    /// `qh.NARROWhull`: the first triangle is a sliver, so points within rounding of an edge are kept as its
    /// outside points for a while instead of being dropped.
    narrow: bool,
    /// The points that started the first triangle.
    simplex: [usize; 3],
    // The precision constants of `qh_detroundoff`.
    dist_round: f64,
    min_denom: f64,
    min_visible: f64,
    max_coplanar: f64,
    min_outside: f64,
}

impl Qhull {
    /// `qh_maxmin`, `qh_detroundoff`, `qh_maxsimplex`: the constants and the three points of the first triangle.
    fn new(input: &[(f64, f64)]) -> Result<Qhull, Unusual> {
        let n = input.len();
        let coord = |i: usize, k: usize| if k == 0 { input[i].0 } else { input[i].1 };
        // qh_maxmin: the points with the smallest and largest x, then y, and the sizes the roundoff depends on.
        let (mut max_abs, mut max_width, mut max_sum) = (0.0f64, -REAL_MAX, 0.0f64);
        let mut maxpoints = [0usize; 4];
        let mut near_zero = [0.0f64; 2];
        for k in 0..2 {
            let (mut minimum, mut maximum) = (0usize, 0usize);
            for i in 0..n {
                if coord(maximum, k) < coord(i, k) {
                    maximum = i;
                } else if coord(minimum, k) > coord(i, k) {
                    minimum = i;
                }
            }
            let maxcoord = fmax(coord(maximum, k), -coord(minimum, k));
            max_width = fmax(max_width, coord(maximum, k) - coord(minimum, k));
            max_abs = fmax(max_abs, maxcoord);
            max_sum += maxcoord;
            maxpoints[2 * k] = minimum;
            maxpoints[2 * k + 1] = maximum;
            near_zero[k] = 80.0 * max_sum * REAL_EPSILON;
        }
        // qh_distround and qh_detroundoff, for a hull with merging at its defaults ('C-0').
        let mut maxdistsum = 2.0f64.sqrt() * max_abs;
        if maxdistsum > max_sum {
            maxdistsum = max_sum;
        }
        let dist_round = REAL_EPSILON * (2.0 * maxdistsum * 1.01 + max_abs);
        let min_visible = 2.0 * dist_round;
        let mut q = Qhull {
            pts: input.to_vec(),
            vertices: Vec::new(),
            facets: Vec::new(),
            list: Vec::new(),
            facet_next: Ptr::Null,
            newfacet_list: Ptr::Tail,
            visible_list: Ptr::Null,
            visit_id: 0,
            max_outside: 0.0,
            findbestnew: false,
            findbest_notsharp: false,
            narrow: false,
            simplex: [0; 3],
            dist_round,
            min_denom: fmax(1.0 / REAL_MAX, f64::MIN_POSITIVE) * max_abs,
            min_visible,
            max_coplanar: min_visible,
            min_outside: 2.0 * min_visible,
        };
        // qh_maxsimplex: the extreme points in x, then the point that makes the largest triangle with them.
        let (mut maxcoord, mut mincoord) = (-REAL_MAX, REAL_MAX);
        let (mut maxx, mut minx) = (None, None);
        for &p in &maxpoints {
            if maxcoord < input[p].0 {
                maxcoord = input[p].0;
                maxx = Some(p);
            }
            if mincoord > input[p].0 {
                mincoord = input[p].0;
                minx = Some(p);
            }
        }
        let (Some(minx), Some(maxx)) = (minx, maxx) else {
            return Err(Unusual::Flat);
        };
        if minx == maxx {
            return Err(Unusual::Flat);
        }
        let det_simplex = |apex: usize| -> (f64, bool) {
            let r0 = (input[minx].0 - input[apex].0, input[minx].1 - input[apex].1);
            let r1 = (input[maxx].0 - input[apex].0, input[maxx].1 - input[apex].1);
            let det = r0.0 * r1.1 - r0.1 * r1.0;
            (det.abs(), det.abs() < 10.0 * near_zero[1])
        };
        let prevdet = maxcoord - mincoord;
        let (mut maxdet, mut maxpoint, mut maxnearzero) = (-1.0f64, None, false);
        for &p in &maxpoints {
            if p != minx && p != maxx && Some(p) != maxpoint {
                let (det, nearzero) = det_simplex(p);
                if det > maxdet {
                    (maxdet, maxpoint, maxnearzero) = (det, Some(p), nearzero);
                }
            }
        }
        let targetdet = prevdet * max_width;
        let mindet = 10.0 * RATIO_MAXSIMPLEX * targetdet;
        let maybe_falsenarrow = maxdet > 0.0 && maxdet / targetdet < RATIO_MAXSIMPLEX;
        if maxpoint.is_none() || maxnearzero || maybe_falsenarrow {
            for p in 0..n {
                if !maxpoints.contains(&p) && p != minx && p != maxx {
                    let (det, nearzero) = det_simplex(p);
                    if det > maxdet {
                        (maxdet, maxpoint, maxnearzero) = (det, Some(p), nearzero);
                        if !maxnearzero && maxdet > mindet {
                            break;
                        }
                    }
                }
            }
        }
        q.simplex = [minx, maxx, maxpoint.ok_or(Unusual::Flat)?];
        Ok(q)
    }

    // ---- the facet list (qh_appendfacet, qh_removefacet)
    fn position(&self, f: usize) -> usize {
        self.list
            .iter()
            .position(|&g| g == f)
            .expect("the facet is in the list")
    }

    fn next(&self, f: usize) -> Ptr {
        self.list.get(self.position(f) + 1).map_or(Ptr::Tail, |&g| Ptr::At(g))
    }

    /// `FORALLfacet_(from)`: the facets from `from` to the end of the list.
    fn from(&self, from: Ptr) -> Vec<usize> {
        match from {
            Ptr::At(f) => self.list[self.position(f)..].to_vec(),
            _ => Vec::new(),
        }
    }

    fn append(&mut self, f: usize) {
        if self.newfacet_list == Ptr::Tail {
            self.newfacet_list = Ptr::At(f);
            if self.visible_list == Ptr::Tail {
                self.visible_list = Ptr::At(f);
            }
        }
        if self.facet_next == Ptr::Tail {
            self.facet_next = Ptr::At(f);
        }
        self.list.push(f);
    }

    fn remove(&mut self, f: usize) {
        let next = self.next(f);
        for ptr in [&mut self.newfacet_list, &mut self.facet_next, &mut self.visible_list] {
            if *ptr == Ptr::At(f) {
                *ptr = next;
            }
        }
        let at = self.position(f);
        self.list.remove(at);
    }

    fn new_facet(&mut self, v: [usize; 2], top: bool, nb: [Option<usize>; 2]) -> usize {
        self.facets.push(Facet {
            v,
            top,
            nb,
            normal: (0.0, 0.0),
            offset: 0.0,
            outside: None,
            furthest_dist: 0.0,
            visible: false,
            new: false,
            replace: None,
            visit: 0,
        });
        self.facets.len() - 1
    }

    // ---- geometry (qh_distplane, qh_setfacetplane with qh_sethyperplane_det and qh_normalize2)
    fn dist(&self, p: usize, f: usize) -> f64 {
        let (f, (x, y)) = (&self.facets[f], self.pts[p]);
        f.offset + x * f.normal.0 + y * f.normal.1
    }

    fn set_plane(&mut self, f: usize) -> Result<(), Unusual> {
        let facet = &self.facets[f];
        let (p0, p1) = (self.pts[self.vertices[facet.v[0]]], self.pts[self.vertices[facet.v[1]]]);
        let (mut n0, mut n1) = (p1.1 - p0.1, p0.0 - p1.0);
        let mut norm = (n0 * n0 + n1 * n1).sqrt();
        if norm.is_nan() || norm <= self.min_denom {
            return Err(Unusual::Merge);
        }
        if !facet.top {
            norm = -norm;
        }
        n0 /= norm;
        n1 /= norm;
        let facet = &mut self.facets[f];
        facet.normal = (n0, n1);
        facet.offset = -(p0.0 * n0 + p0.1 * n1);
        Ok(())
    }

    /// `qh_DISToutside`: how far above an edge a point must be to go straight into its outside set.
    fn dist_outside(&self) -> f64 {
        fmax(2.0 * self.min_outside, self.max_outside)
    }

    /// `qh_SEARCHdist`.
    fn search_dist(&self) -> f64 {
        self.max_outside + 2.0 * self.dist_round + fmax(self.min_visible, self.max_coplanar)
    }

    // ---- the searches
    /// `qh_findbesthorizon`: from `start`, any neighbouring edge (new or old) the point is higher above.
    fn find_best_horizon(&mut self, p: usize, start: usize, mut bestdist: f64) -> (usize, f64) {
        let mut best = start;
        self.visit_id += 1;
        let visit = self.visit_id;
        let searchdist = self.search_dist();
        let mut minsearch = bestdist - searchdist;
        let mut coplanar: Vec<usize> = Vec::new();
        self.facets[start].visit = visit;
        let mut facet = Some(start);
        let mut nextfacet: Option<usize> = None;
        while let Some(f) = facet {
            for k in 0..2 {
                let Some(g) = self.facets[f].nb[k] else { break };
                if self.facets[g].visit == visit {
                    continue;
                }
                self.facets[g].visit = visit;
                let d = self.dist(p, g);
                if d > bestdist {
                    minsearch = d - searchdist;
                    if d > bestdist + searchdist {
                        coplanar.clear();
                    }
                    (best, bestdist) = (g, d);
                } else if d < minsearch {
                    continue;
                }
                if let Some(waiting) = nextfacet {
                    coplanar.push(waiting);
                }
                nextfacet = Some(g);
            }
            facet = nextfacet.take().or_else(|| coplanar.pop());
        }
        (best, bestdist)
    }

    /// `qh_findbestnew`: every new edge from `start` on, then round from the first new one; `(edge, distance,
    /// is the point outside it)`.
    fn find_best_new(&mut self, p: usize, start: usize) -> (usize, f64, bool) {
        self.visit_id += 1;
        let visit = self.visit_id;
        let distoutside = self.dist_outside();
        let mut order = self.from(Ptr::At(start));
        order.extend(self.from(self.newfacet_list).into_iter().take_while(|&g| g != start));
        let (mut best, mut bestdist) = (None, -REAL_MAX / 2.0);
        for g in order {
            self.facets[g].visit = visit;
            let d = self.dist(p, g);
            if d > bestdist {
                best = Some(g);
                if d >= distoutside {
                    return (g, d, true);
                }
                bestdist = d;
            }
        }
        let (best, bestdist) = self.find_best_horizon(p, best.unwrap_or(start), bestdist);
        (best, bestdist, bestdist >= self.min_outside)
    }

    /// `qh_sharpnewfacets`: do the new edges' normals point into different quadrants?
    fn sharp_new_facets(&self) -> bool {
        let mut quadrant = None;
        for g in self.from(self.newfacet_list) {
            let now = (self.facets[g].normal.0 > 0.0, self.facets[g].normal.1 > 0.0);
            match quadrant {
                None => quadrant = Some(now),
                Some(first) if first != now => return true,
                Some(_) => {}
            }
        }
        false
    }

    /// `qh_findbest(point, start, !bestoutside, qh_ISnewfacets, ...)`: a walk over the new edges that stops at
    /// the first one the point is outside of; `(edge, distance, is the point outside it)`.
    fn find_best(&mut self, p: usize, start: usize) -> (usize, f64, bool) {
        self.visit_id += 1;
        let visit = self.visit_id;
        let d = self.dist(p, start);
        if d >= self.min_outside {
            return (start, d, true);
        }
        let (mut best, mut bestdist) = (start, d);
        self.facets[start].visit = visit;
        let mut facet = Some(start);
        while let Some(f) = facet {
            facet = None;
            for k in 0..2 {
                let Some(g) = self.facets[f].nb[k] else { break };
                if !self.facets[g].new || self.facets[g].visit == visit {
                    continue;
                }
                self.facets[g].visit = visit;
                let d = self.dist(p, g);
                if d > bestdist {
                    if d >= self.min_outside {
                        return (g, d, true);
                    }
                    (best, bestdist) = (g, d);
                    facet = Some(g);
                    break;
                }
            }
        }
        if !self.findbest_notsharp && bestdist < -self.dist_round {
            if self.sharp_new_facets() {
                self.findbestnew = true;
                let (best, bestdist, outside) = self.find_best_new(p, best);
                return (best, bestdist, outside && bestdist >= self.min_outside);
            }
            self.findbest_notsharp = true;
        }
        let (best, bestdist) = self.find_best_horizon(p, best, bestdist);
        (best, bestdist, bestdist >= self.min_outside)
    }

    /// `qh_partitionpoint`: the point goes into the outside set of the edge found for it, or is dropped as
    /// inside (or coplanar: then it only widens `qh.max_outside`).
    fn partition_point(&mut self, p: usize, start: usize) {
        let (best, bestdist, mut outside) = if self.findbestnew {
            self.find_best_new(p, start)
        } else {
            self.find_best(p, start)
        };
        if self.narrow && bestdist >= -self.max_coplanar {
            outside = true;
        }
        if outside {
            let (narrow, min_outside) = (self.narrow, self.min_outside);
            let facet = &mut self.facets[best];
            let mut newoutside = false;
            match &mut facet.outside {
                Some(set) if !set.is_empty() => {
                    let previous = facet.furthest_dist;
                    if previous < bestdist {
                        set.push(p);
                        facet.furthest_dist = bestdist;
                        newoutside = narrow && previous < min_outside && bestdist >= min_outside;
                    } else {
                        // qh_setappend2ndlast: the furthest point stays last
                        set.insert(set.len() - 1, p);
                    }
                }
                _ => {
                    facet.outside = Some(vec![p]);
                    facet.furthest_dist = bestdist;
                    newoutside = !narrow || bestdist > min_outside;
                }
            }
            if newoutside && self.facet_next != Ptr::At(best) {
                if self.facets[best].new {
                    if matches!(self.facet_next, Ptr::At(f) if self.facets[f].new) {
                        self.facet_next = self.newfacet_list;
                    }
                } else {
                    // An old edge gets points again: it moves behind qh.facet_next.
                    self.remove(best);
                    self.append(best);
                    if self.newfacet_list != Ptr::Null {
                        self.facets[best].new = true;
                    }
                }
            }
        } else if bestdist >= -self.max_coplanar && bestdist > self.max_outside {
            // qh_partitioncoplanar, for a hull that keeps no coplanar points
            self.max_outside = bestdist;
        }
    }

    /// The hull: `qh_initialhull`, `qh_partitionall`, `qh_buildhull`, then scipy's walk over the facets.
    fn build(mut self) -> Result<Vec<usize>, Unusual> {
        let n = self.pts.len();
        // qh_initialvertices: the vertex list in descending id; qh_createsimplex: an edge across each vertex.
        self.vertices = self.simplex.to_vec();
        let by_id_down = [2usize, 1, 0];
        let mut top = true;
        for skip in 0..3 {
            let v: Vec<usize> = (0..3).filter(|&j| j != skip).map(|j| by_id_down[j]).collect();
            let f = self.new_facet([v[0], v[1]], top, [None, None]);
            self.append(f);
            top = !top;
        }
        for f in 0..3 {
            let others: Vec<usize> = (0..3).filter(|&g| g != f).collect();
            self.facets[f].nb = [Some(others[0]), Some(others[1])];
        }
        // qh_initialhull: orient the edges away from the triangle's middle.
        (self.newfacet_list, self.visible_list, self.facet_next) = (Ptr::Null, Ptr::Null, Ptr::At(0));
        let mut interior = [0.0f64; 2];
        for (k, c) in interior.iter_mut().enumerate() {
            for &v in &by_id_down {
                let p = self.pts[self.vertices[v]];
                *c += if k == 0 { p.0 } else { p.1 };
            }
            *c /= 3.0;
        }
        self.pts.push((interior[0], interior[1]));
        self.set_plane(0)?;
        if self.dist(n, 0) > self.dist_round {
            for f in &mut self.facets {
                f.top = !f.top;
            }
            self.set_plane(0)?;
        }
        self.set_plane(1)?;
        self.set_plane(2)?;
        for f in 0..3 {
            // Clearly flipped (Qhull re-orients; it does not happen in 2-D), or flat (Qhull stops).
            if self.dist(n, f) >= -self.dist_round {
                return Err(Unusual::Flat);
            }
        }
        let mut minangle = REAL_MAX;
        for f in 0..3 {
            for g in (0..3).filter(|&g| g != f) {
                let (a, b) = (self.facets[f].normal, self.facets[g].normal);
                minangle = fmin(minangle, 0.0 + a.0 * b.0 + a.1 * b.1);
            }
        }
        self.narrow = minangle < MAX_NARROW;
        // qh_partitionall: each remaining point into the outside set of the first edge it is clearly above.
        let mut pointset: Vec<usize> = (0..n).filter(|p| !self.simplex.contains(p)).collect();
        let distoutside = self.dist_outside();
        for f in 0..3 {
            let (mut set, mut rest) = (Vec::new(), Vec::new());
            let mut bestpoint: Option<(usize, f64)> = None;
            for &p in &pointset {
                let d = self.dist(p, f);
                if d < distoutside {
                    rest.push(p);
                    continue;
                }
                match bestpoint {
                    None => bestpoint = Some((p, d)),
                    Some((old, bestdist)) if d > bestdist => {
                        set.push(old);
                        bestpoint = Some((p, d));
                    }
                    Some(_) => set.push(p),
                }
            }
            if let Some((p, d)) = bestpoint {
                set.push(p);
                self.facets[f].outside = Some(set);
                self.facets[f].furthest_dist = d;
            }
            pointset = rest;
        }
        // The points left are inside or within rounding of an edge: qh_partitionpoint with qh.findbestnew.
        self.findbestnew = true;
        for p in pointset {
            self.partition_point(p, self.list[0]);
        }
        self.findbestnew = false;
        // qh_furthestnext: the edge with the furthest point of all goes first.
        let mut best: Option<(usize, f64)> = None;
        for &f in &self.list {
            let facet = &self.facets[f];
            if facet.outside.is_some() && facet.furthest_dist > best.map_or(-REAL_MAX, |b| b.1) {
                best = Some((f, facet.furthest_dist));
            }
        }
        if let Some((f, _)) = best {
            self.list.retain(|&g| g != f);
            self.list.insert(0, f);
        }
        // qh_buildhull
        self.facet_next = Ptr::At(self.list[0]);
        while let Some((furthest, facet)) = self.next_furthest() {
            self.add_point(furthest, facet)?;
        }
        Ok(self.extremes())
    }

    /// `qh_nextfurthest`: the furthest point of the first edge from `qh.facet_next` that has points left.
    fn next_furthest(&mut self) -> Option<(usize, usize)> {
        while let Ptr::At(f) = self.facet_next {
            let facet = &mut self.facets[f];
            if facet.outside.as_ref().is_none_or(Vec::is_empty) {
                facet.outside = None;
            } else if !(self.narrow && facet.furthest_dist < self.min_outside) {
                return facet.outside.as_mut().and_then(Vec::pop).map(|p| (p, f));
            }
            // Nothing above this edge (or, for a narrow hull, nothing clearly above it): on to the next.
            self.facet_next = self.next(f);
        }
        None
    }

    /// `qh_addpoint`: the edges the point sees are replaced by two new ones to it.
    fn add_point(&mut self, furthest: usize, facet: usize) -> Result<(), Unusual> {
        // qh_findhorizon: the visible edges move to the end of the list, in the order they are found.
        self.remove(facet);
        self.append(facet);
        self.visible_list = Ptr::At(facet);
        (self.facets[facet].visible, self.facets[facet].replace) = (true, None);
        self.visit_id += 1;
        let visit = self.visit_id;
        let mut at = self.position(facet);
        while at < self.list.len() && self.facets[self.list[at]].visible {
            let visible = self.list[at];
            self.facets[visible].visit = visit;
            for k in 0..2 {
                let Some(g) = self.facets[visible].nb[k] else { continue };
                if self.facets[g].visit == visit {
                    continue;
                }
                self.facets[g].visit = visit;
                let d = self.dist(furthest, g);
                if d > self.min_visible {
                    self.remove(g);
                    self.append(g);
                    (self.facets[g].visible, self.facets[g].replace) = (true, None);
                } else if d >= -self.max_coplanar {
                    // A coplanar horizon: Qhull merges the new edge into it.
                    return Err(Unusual::Merge);
                }
            }
            at = self.position(visible) + 1;
        }
        // qh_makenewfacets: an edge from the new vertex to each end of the visible stretch.
        self.newfacet_list = Ptr::Tail;
        self.vertices.push(furthest);
        let apex = self.vertices.len() - 1;
        let mut made: Vec<usize> = Vec::with_capacity(2);
        // (Only visible edges follow qh.visible_list at this point.)
        for visible in self.from(self.visible_list) {
            let mut replace = None;
            for k in 0..2 {
                let Some(g) = self.facets[visible].nb[k] else { continue };
                if self.facets[g].visible {
                    continue;
                }
                // qh_makenew_simplicial: the corner the horizon edge shares, and the orientation it hands on.
                let skip = usize::from(self.facets[g].nb[1] == Some(visible));
                let shared = self.facets[g].v[1 - skip];
                let top = if self.facets[g].top { skip == 1 } else { skip == 0 };
                let f = self.new_facet([apex, shared], top, [Some(g), None]);
                self.append(f);
                self.facets[f].new = true;
                self.facets[g].nb[skip] = Some(f);
                made.push(f);
                replace = Some(f);
            }
            self.facets[visible].replace = replace;
            self.facets[visible].nb = [None, None];
        }
        let &[a, b] = made.as_slice() else {
            return Err(Unusual::Merge);
        };
        // qh_matchnewfacets: the two new edges meet at the new vertex; qh_makenewplanes.
        self.facets[a].nb[1] = Some(b);
        self.facets[b].nb[1] = Some(a);
        self.set_plane(a)?;
        self.set_plane(b)?;
        // qh_premerge: qh_checkzero finds the new edges clearly convex, or Qhull goes on to merge.
        for (f, other) in [(a, b), (b, a)] {
            let facet = &self.facets[f];
            let corner = facet.v[1];
            if self.dist(self.vertices[corner], other) >= -2.0 * self.dist_round {
                return Err(Unusual::Merge);
            }
            let horizon = &self.facets[facet.nb[0].expect("a new edge has its horizon")];
            let across = if horizon.v[0] == corner {
                horizon.v[1]
            } else {
                horizon.v[0]
            };
            if self.dist(self.vertices[across], f) >= -2.0 * self.dist_round {
                return Err(Unusual::Merge);
            }
        }
        // qh_partitionvisible: the points of the visible edges go to the new ones (or are now inside).
        self.findbestnew = false;
        for visible in self.from(self.visible_list) {
            if !self.facets[visible].visible {
                // The new edges follow the visible ones in the list; their points are already where they belong.
                break;
            }
            let Some(points) = self.facets[visible].outside.take() else {
                continue;
            };
            let mut start = Some(visible);
            while let Some(f) = start.filter(|&f| self.facets[f].visible) {
                start = self.facets[f].replace;
            }
            let start = match (start, self.newfacet_list) {
                (Some(f), _) | (None, Ptr::At(f)) => f,
                _ => return Err(Unusual::Merge),
            };
            for p in points {
                self.partition_point(p, start);
            }
        }
        (self.findbestnew, self.findbest_notsharp) = (false, false);
        // qh_deletevisible, qh_resetlists
        let gone: Vec<usize> = self.list.iter().copied().filter(|&f| self.facets[f].visible).collect();
        for f in gone {
            self.remove(f);
        }
        for f in &mut self.facets {
            f.new = false;
        }
        (self.newfacet_list, self.visible_list) = (Ptr::Null, Ptr::Null);
        Ok(())
    }

    /// scipy's `_Qhull.get_extremes_2d`: from the first facet of the list, each facet's two corners in its
    /// orientation, then on to the neighbour that shares the second one.
    fn extremes(&self) -> Vec<usize> {
        let mut out = Vec::with_capacity(self.list.len());
        let mut seen = vec![false; self.vertices.len()];
        let start = self.list[0];
        let mut f = start;
        loop {
            let facet = &self.facets[f];
            let (a, b, next) = if facet.top {
                (facet.v[0], facet.v[1], facet.nb[0])
            } else {
                (facet.v[1], facet.v[0], facet.nb[1])
            };
            for v in [a, b] {
                if !seen[v] {
                    seen[v] = true;
                    out.push(self.vertices[v]);
                }
            }
            f = next.expect("a facet of the finished hull has both neighbours");
            if f == start || out.len() > self.vertices.len() {
                return out;
            }
        }
    }
}

/// Qhull's `fmax_(a, b)`.
fn fmax(a: f64, b: f64) -> f64 {
    if a < b { b } else { a }
}

/// Qhull's `fmin_(a, b)`.
fn fmin(a: f64, b: f64) -> f64 {
    if a > b { b } else { a }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_triangle_comes_back_counter_clockwise() {
        // scipy 1.18.1: ConvexHull([[0, 0], [1, 0], [0, 1]]).vertices is [0, 1, 2], and [0, 2, 1] for the
        // triangle listed the other way round.
        assert_eq!(convex_hull(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]), [0, 1, 2]);
        assert_eq!(convex_hull(&[(0.0, 0.0), (0.0, 1.0), (1.0, 0.0)]), [0, 2, 1]);
    }

    #[test]
    fn inner_points_and_points_along_edges_are_left_out() {
        // scipy: [0, 1, 2, 3] for a rectangle and its centre; [0, 2, 4, 6] for a square with its edge midpoints.
        let rect = [(0.0, 0.0), (4.0, 0.0), (4.0, 3.0), (0.0, 3.0), (2.0, 1.5)];
        assert_eq!(qhull_order(&rect), Ok(vec![0, 1, 2, 3]));
        let square = [
            (0.0, 0.0),
            (2.0, 0.0),
            (4.0, 0.0),
            (4.0, 2.0),
            (4.0, 4.0),
            (2.0, 4.0),
            (0.0, 4.0),
            (0.0, 2.0),
        ];
        assert_eq!(qhull_order(&square), Ok(vec![0, 2, 4, 6]));
    }

    #[test]
    fn the_start_is_qhulls_not_a_rule_on_the_points() {
        // scipy 1.18.1 gives [1, 2, 3, 6, 0, 5] for the first cloud (from its leftmost point) and [2, 3, 0, 5, 1]
        // for the second (from its topmost point, which is neither its leftmost nor its rightmost).
        let a = [
            (12.5, 80.25),
            (-40.0, -7.5),
            (33.0, -60.0),
            (71.5, 9.0),
            (0.5, 3.0),
            (-18.0, 66.0),
            (55.0, 62.5),
        ];
        assert_eq!(qhull_order(&a), Ok(vec![1, 2, 3, 6, 0, 5]));
        let b = [(3.0, 1.0), (9.0, 4.0), (7.5, 9.0), (1.0, 6.0), (5.0, 5.0), (8.0, 1.5)];
        let got = qhull_order(&b).expect("general position");
        assert_eq!(got, [2, 3, 0, 5, 1]);
        // Counter-clockwise: every turn is to the left.
        for k in 0..got.len() {
            let (o, p, q) = (b[got[k]], b[got[(k + 1) % got.len()]], b[got[(k + 2) % got.len()]]);
            assert!((p.0 - o.0) * (q.1 - o.1) - (p.1 - o.1) * (q.0 - o.0) > 0.0);
        }
    }

    #[test]
    fn inputs_qhull_treats_specially_fall_back_to_a_plain_hull() {
        // On a line: scipy raises QhullError. Here: the two ends.
        let line = [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)];
        assert_eq!(qhull_order(&line), Err(Unusual::Flat));
        assert_eq!(convex_hull(&line), [0, 3]);
        // Too few points, or not numbers.
        assert_eq!(qhull_order(&[(0.0, 0.0), (1.0, 0.0)]), Err(Unusual::NotEnoughPoints));
        assert_eq!(convex_hull(&[(1.0, 0.0), (0.0, 0.0)]), [1, 0]);
        assert_eq!(convex_hull(&[]), Vec::<usize>::new());
        let holes = [(0.0, 0.0), (f64::NAN, 1.0), (2.0, 0.0), (1.0, 5.0)];
        assert_eq!(qhull_order(&holes), Err(Unusual::NotEnoughPoints));
        assert_eq!(convex_hull(&holes), [0, 2, 3]);
        // Three points in a row that Qhull meets middle first: it merges. scipy has [3, 0, 2] here; the plain
        // hull has the same corners the same way round, from the leftmost (lower) one.
        let row = [(0.0, 4.0), (1.0, 1.0), (0.0, 1.0), (5.0, 1.0), (0.0, 4.0)];
        assert_eq!(qhull_order(&row), Err(Unusual::Merge));
        assert_eq!(convex_hull(&row), [2, 3, 0]);
    }

    #[test]
    fn a_sliver_of_a_first_triangle_is_followed_too() {
        // scipy 1.18.1: [0, 1, 2] and [2, 0, 4, 1] (Qhull's narrow-hull handling: the points within rounding of
        // the long edge wait in its outside set).
        let sliver = [(0.0, 0.0), (100.0, 50.0), (50.0, 25.000_000_001), (20.0, 10.0)];
        assert_eq!(qhull_order(&sliver), Ok(vec![0, 1, 2]));
        let more = [
            (0.0, 0.0),
            (100.0, 50.0),
            (50.0, 25.000_000_001),
            (20.0, 10.0),
            (70.0, 34.999_999_999_5),
            (10.0, 5.0),
        ];
        assert_eq!(qhull_order(&more), Ok(vec![2, 0, 4, 1]));
        // scipy: [0, 2, 8, 6] for the 3 x 3 grid of whole numbers.
        let grid: Vec<(f64, f64)> = (0..9).map(|i| ((i % 3) as f64, (i / 3) as f64)).collect();
        assert_eq!(qhull_order(&grid), Ok(vec![0, 2, 8, 6]));
    }

    #[test]
    fn the_plain_hull_is_counter_clockwise_from_the_left() {
        let square = [
            (1.0, 1.0),
            (0.0, 0.0),
            (1.0, 0.0),
            (0.0, 1.0),
            (0.5, 0.5),
            (0.5, 0.0),
            (1.0, 1.0),
        ];
        assert_eq!(scan_hull(&square), [1, 2, 0, 3]);
    }

    #[test]
    fn a_ring_of_points_starts_where_scipy_starts() {
        // scipy: ConvexHull of 16 points of the ellipse (180 + 90 cos a, 320 + 40 sin a), a = k pi / 8, starts
        // at k = 8, the leftmost point, and goes on 9, 10, ... 15, 0, ... 7.
        let pts: Vec<(f64, f64)> = (0..16)
            .map(|k| {
                let a = 2.0 * bf_core::dmath::PI * k as f64 / 16.0;
                (
                    180.0 + 90.0 * bf_core::dmath::cos(a),
                    320.0 + 40.0 * bf_core::dmath::sin(a),
                )
            })
            .collect();
        let want: Vec<usize> = (8..16).chain(0..8).collect();
        assert_eq!(qhull_order(&pts), Ok(want));
    }
}
