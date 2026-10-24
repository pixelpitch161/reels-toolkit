"""Falling Sand Wars engine: four sands with different physics in a square glass box with one hole.

The grid lives in the BOX's own frame (row 0 = the box's top / lid side when upright, col 0 = its left wall),
with a 1-cell border of wall cells. The box turns in the world; gravity in the box frame is the world's
down rotated by -theta and snapped to one of 8 grid directions (45 deg steps). One substep:

    1. density swaps along gravity  (heavier grains sink through lighter ones: iron down, pumice up)
    2. primary fall along gravity   (pumice falls a little slower: it drifts)
    3. diagonal slides at +-45 deg  (clay only slides off a 2-cell overhang, so it piles steeply)
    4. sideways flow at +-90 deg    (glass runs flat, almost like water; light pumice rolls over the top)
    clay that touches a wall (or clay stuck to a wall) holds on with probability CLAY_HOLD per pass.

Grains that step into an open wall cell (the lid while it is open, the hole once the plug pops) leave the
box. Every exit is recorded (frame, material, cell, tone), so the picture can draw it falling to the dune and
the soundtrack can play each draining stream. Nothing here knows about the video: the schedule (pour, lid,
hole, turns, roll) comes in from the caller, and the seed decides the hopper order, the pour noise, every
turn's direction and every random choice in the grain rules. Runs are never edited."""
import math
import numpy as np

EMPTY, IRON, PUMICE, CLAY, GLASS = 0, 1, 2, 3, 4
WALL, EXIT = 8, 9
NAMES = ['IRON', 'PUMICE', 'CLAY', 'GLASS']           # team q = material q + 1
RING = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)]   # (dr, dc), 45 deg apart


def gravity_index(theta_deg):
    """box angle (deg, CCW on screen) -> index into RING of the box-frame gravity direction"""
    th = math.radians(theta_deg)
    dr, dc = math.cos(th), -math.sin(th)
    a = math.degrees(math.atan2(dc, dr))             # RING[k] has angle atan2(dc, dr) = 45 k
    return int(round(a / 45.0)) % 8


class Params:
    """balance knobs (one set for every run of the episode)"""
    def __init__(self, **kw):
        self.N = 128                     # interior cells per side
        self.K = 3                       # substeps per video frame
        self.dens = {IRON: 4.0, PUMICE: 1.0, CLAY: 2.6, GLASS: 2.0}
        self.swap = 0.6                  # swap probability per unit density difference (capped at 0.95)
        self.clay_swap = 0.8             # clay resists being sunk through / floated out of (multiplier)
        self.fall = {IRON: 1.0, PUMICE: 0.62, CLAY: 1.0, GLASS: 1.0}
        self.slide = {IRON: 0.9, PUMICE: 0.95, CLAY: 0.5, GLASS: 1.0}
        self.flow = 0.3                  # glass sideways flow probability
        self.pumice_roll = 0.2           # light pumice rolls sideways over the surface (spreads into a top layer)
        self.exit = {IRON: 1.0, PUMICE: 1.0, CLAY: 1.0, GLASS: 1.0}   # chance a grain at the hole gets through
        self.clay_hold = 0.5             # stuck clay stays put with this probability per pass
        self.clay_reach = 1              # how many cells deep the wall adhesion reaches through clay
        self.clay_steep = 0.6            # chance per pass that clay needs a 2-cell overhang to slide (steep piles)
        self.hole_w = 0.12               # notch height up the side wall (fraction); the corner comes from the seed
        for k, v in kw.items():
            setattr(self, k, v)


class SandBox:
    def __init__(self, P, seed):
        self.P = P
        N = P.N
        self.N = N
        self.rng = np.random.default_rng(seed)
        self.mat = np.full((N + 2, N + 2), WALL, np.int8)
        self.mat[1:-1, 1:-1] = EMPTY
        self.tone = np.zeros((N + 2, N + 2), np.uint8)     # bits 0-5 grain tone, bit 6 moved now, bit 7 was moving
        self.I = (slice(1, N + 1), slice(1, N + 1))
        dens = np.zeros(16)
        for m, v in P.dens.items():
            dens[m] = v
        self.DENS = dens
        self.FALL = self._tab(P.fall)
        self.SLIDE = self._tab(P.slide)
        self.FLOW = self._tab({GLASS: P.flow, PUMICE: P.pumice_roll})
        self.EXITP = self._tab(P.exit)
        self.hold = P.clay_hold             # the caller may lower it (the closing rattle)
        self.rattle = False                  # closing roll: the box rattles, every grain slides freely
        self.SLIDE_RATTLE = self._tab({IRON: 1.0, PUMICE: 1.0, CLAY: 1.0, GLASS: 1.0})
        self.SW = np.zeros((16, 16), np.int16)
        for a in (1, 2, 3, 4):
            for b in (1, 2, 3, 4):
                if dens[a] > dens[b]:
                    p = min(0.95, P.swap * (dens[a] - dens[b]))
                    if CLAY in (a, b):
                        p *= P.clay_swap
                    self.SW[a, b] = int(round(p * 256))
        r = np.arange(N)
        self.PAR_R = [(r[:, None] % 2 == k) & np.ones((1, N), bool) for k in (0, 1)]
        self.PAR_C = [np.ones((N, 1), bool) & (r[None, :] % 2 == k) for k in (0, 1)]
        self.set_hole('R')
        self.lid_open = False
        self.hole_open = False
        self._set_border()
        self.wall_adj = None
        self._update_wall_adj()
        # per-substep stats
        self.landed = np.zeros(5, np.int64)
        self.moves = np.zeros(5, np.int64)
        self.exits = []                                       # (mat, r, c, tone) arrays per substep

    def _tab(self, d):
        t = np.zeros(16, np.int16)
        for m, v in d.items():
            t[int(m)] = int(round(v * 256))
        return t

    # ------------------------------------------------------------------ openings
    def set_hole(self, side, up=None):
        """the hole: a notch cut through one of the bottom corners ('L' or 'R'), up the side wall by hole_w of its
        height and along the bottom wall by half that. A corner is the one place some tilt makes the box's
        lowest point, so every grain can reach it eventually (no sand is trapped for good)."""
        N = self.N
        self.hole_side = side
        self.hole_up = max(4, int(round(self.P.hole_w * N))) if up is None else int(up)
        self.hole_along = max(2, min(N // 2, self.hole_up // 2))
        if hasattr(self, 'lid_open'):
            self._set_border()
            self._update_wall_adj()

    def hole_cells(self):
        """border-array (row, col) cells of the hole"""
        N = self.N
        c = N + 1 if self.hole_side == 'R' else 0
        cells = [(r, c) for r in range(N + 1 - self.hole_up, N + 2)]
        cols = range(N + 1 - self.hole_along, N + 1) if self.hole_side == 'R' else range(1, 1 + self.hole_along)
        cells += [(N + 1, cc) for cc in cols]
        return cells

    def hole_local(self):
        """hole centre in box-local coords (x right, y up, the box spans -1..1)"""
        u = self.hole_up / self.N
        return (1.0 if self.hole_side == 'R' else -1.0, -1.0 + u * 0.5)

    def _set_border(self):
        m = self.mat
        N = self.N
        m[0, :] = EXIT if self.lid_open else WALL
        m[0, 0] = m[0, N + 1] = WALL
        m[:, N + 1] = WALL
        m[N + 1, :] = WALL
        m[:, 0] = WALL
        if self.hole_open:
            for r, c in self.hole_cells():
                m[r, c] = EXIT

    def set_open(self, lid, hole):
        if lid != self.lid_open or hole != self.hole_open:
            self.lid_open, self.hole_open = lid, hole
            self._set_border()
            self._update_wall_adj()

    def _update_wall_adj(self):
        w = (self.mat == WALL)
        N = self.N
        adj = np.zeros((N, N), bool)
        for dr, dc in RING:
            adj |= w[1 + dr:N + 1 + dr, 1 + dc:N + 1 + dc]
        self.wall_adj = adj

    # ------------------------------------------------------------------ pour
    def inject(self, col_lo, col_hi, n, mat):
        """drop up to n grains of mat into the top row between columns col_lo..col_hi (interior coords)"""
        row = self.mat[1, 1 + col_lo:1 + col_hi + 1]
        free = np.nonzero(row == EMPTY)[0]
        if len(free) == 0 or n <= 0:
            return 0
        pick = self.rng.choice(free, size=min(n, len(free)), replace=False)
        row[pick] = mat
        tr = self.tone[1, 1 + col_lo:1 + col_hi + 1]
        tr[pick] = self.rng.integers(0, 64, len(pick)).astype(np.uint8) | 0x80
        return len(pick)

    # ------------------------------------------------------------------ passes
    def _views(self, d):
        dr, dc = d
        N = self.N
        T = (slice(1 + dr, N + 1 + dr), slice(1 + dc, N + 1 + dc))
        return self.mat[self.I], self.mat[T], self.tone[self.I], self.tone[T]

    def _apply(self, d, ok, src, tgt, tsrc, ttgt, count=True):
        s0 = src.copy()
        t0 = tsrc.copy()
        to_exit = ok & (tgt == EXIT)
        to_cell = ok & (tgt == EMPTY)
        src[ok] = EMPTY
        tgt[to_cell] = s0[to_cell]
        ttgt[to_cell] = t0[to_cell] | 0x40
        tsrc[ok] = 0
        if count:
            self.moves += np.bincount(s0[ok], minlength=5)[:5]
        if to_exit.any():
            r, c = np.nonzero(to_exit)
            self.exits.append((s0[to_exit].astype(np.int8), (r + 1 + d[0]).astype(np.int16),
                               (c + 1 + d[1]).astype(np.int16), (t0[to_exit] & 63).astype(np.uint8)))

    def new_pool(self):
        """one block of random bytes per substep; each pass reads it at a random offset"""
        n = self.N
        self.pool = self.rng.integers(0, 256, (n + 48, n + 48), dtype=np.uint8)
        self.offs = self.rng.integers(0, 48, (16, 2))
        self.oi = 0

    def _rand(self):
        a, b = self.offs[self.oi % 16]
        self.oi += 1
        return self.pool[a:a + self.N, b:b + self.N]

    def swap_pass(self, d, par):
        src, tgt, tsrc, ttgt = self._views(d)
        sel = (self.PAR_R if d[0] != 0 else self.PAR_C)[par]
        g_s = (src >= 1) & (src <= 4)
        g_t = (tgt >= 1) & (tgt <= 4)
        p = self.SW[src, tgt]
        ok = sel & g_s & g_t & (self._rand() < p) & ~self.stuck
        if not ok.any():
            return
        s0, t0 = src.copy(), tgt.copy()
        u0, v0 = tsrc.copy(), ttgt.copy()
        src[ok] = t0[ok]
        tgt[ok] = s0[ok]
        tsrc[ok] = v0[ok]
        ttgt[ok] = u0[ok]

    def move_pass(self, d, prob, extra=None, count=True):
        src, tgt, tsrc, ttgt = self._views(d)
        free = (tgt == EMPTY) | (tgt == EXIT)
        ok = (src >= 1) & (src <= 4) & free & (self._rand() < prob[src])
        if extra is not None:
            ok &= extra
        ex = tgt == EXIT
        if ex.any():
            ok &= ~ex | (self._rand() < self.EXITP[src])
        if ok.any():
            self._apply(d, ok, src, tgt, tsrc, ttgt, count)

    def _blocked(self, d):
        """grains whose cell in direction d is occupied (by a grain or a wall)"""
        _, tgt, _, _ = self._views(d)
        return ~((tgt == EMPTY) | (tgt == EXIT))

    def _free(self, d):
        _, tgt, _, _ = self._views(d)
        return (tgt == EMPTY) | (tgt == EXIT)

    def _stuck_mask(self):
        src = self.mat[self.I]
        clay = src == CLAY
        st = clay & self.wall_adj
        N = self.N
        for _ in range(self.P.clay_reach):
            pad = np.zeros((N + 2, N + 2), bool)
            pad[1:-1, 1:-1] = st
            grow = np.zeros((N, N), bool)
            for dr, dc in RING:
                grow |= pad[1 + dr:N + 1 + dr, 1 + dc:N + 1 + dc]
            st = st | (clay & grow)
        return st & (self._rand() < int(round(self.hold * 256)))

    def substep(self, gk):
        P = self.P
        g = RING[gk]
        s1, s2 = RING[(gk + 1) % 8], RING[(gk - 1) % 8]
        t1, t2 = RING[(gk + 2) % 8], RING[(gk - 2) % 8]
        self.new_pool()
        self.stuck = self._stuck_mask()
        free_stuck = ~self.stuck
        par = int(self.rng.integers(0, 2))
        self.swap_pass(g, par)
        self.swap_pass(g, 1 - par)
        self.move_pass(g, self.FALL, free_stuck)
        self.move_pass(g, self.FALL, free_stuck)
        order = [(s1, t1), (s2, t2)] if self.rng.random() < 0.5 else [(s2, t2), (s1, t1)]
        src = self.mat[self.I]
        clay = src == CLAY
        for s, side in order:
            blocked = self._blocked(g)
            if self.rattle:
                self.move_pass(s, self.SLIDE_RATTLE, blocked & free_stuck)
                continue
            # clay only slides off a 2-cell overhang (the side cell must be free too): steep piles
            cl_ok = ~clay | self._free(side) | (self._rand() >= int(round(P.clay_steep * 256)))
            self.move_pass(s, self.SLIDE, blocked & free_stuck & cl_ok)
        src = self.mat[self.I]
        flowing = (src == GLASS) | (src == PUMICE)
        if flowing.any():
            for s in ([t1, t2] if self.rng.random() < 0.5 else [t2, t1]):
                blocked = self._blocked(g)
                self.move_pass(s, self.FLOW, flowing & blocked, count=False)
        # landings: grains that were moving last substep and did not move in this one
        tv = self.tone[self.I]
        src = self.mat[self.I]
        grain = (src >= 1) & (src <= 4)
        landed = grain & ((tv & 0x80) != 0) & ((tv & 0x40) == 0)
        if landed.any():
            self.landed += np.bincount(src[landed], minlength=5)[:5]
        tv[:] = (tv & 63) | ((tv & 0x40) << 1)

    def counts(self):
        return np.bincount(self.mat[self.I].ravel().clip(0, 7), minlength=8)[1:5]

    def snapshot(self):
        """uint8 grid: material * 32 + tone (0-31) for the picture"""
        src = self.mat[self.I].astype(np.uint8)
        return (src << 5) | (self.tone[self.I] & 63) >> 1

    def pop_stats(self):
        out = (self.landed.copy(), self.moves.copy(), self.exits)
        self.landed[:] = 0
        self.moves[:] = 0
        self.exits = []
        return out
