"""Maze Race engine: one maze with four-fold rotational symmetry, four explorers from the four corners to the centre.

The maze (N x N cells, N even): the 2 x 2 centre is the goal room. One quadrant is grown as a random spanning tree
(growing-tree algorithm: mostly newest = long corridors, sometimes random = side branches), joined to the goal room
by one door, and copied three times by 90 deg rotations. A few symmetric links between neighbouring quadrants (and
braids inside a quadrant) add loops, so an explorer can wander into a neighbour's quadrant: a real wrong branch.
Every corner therefore faces exactly the same maze.

Explorers (q = 0..3, starting at the corners rot^q(0, 0)):
    FLOOD   breadth-first: uncovers the next cell of its queue, ring by ring (thorough, spread thin)
    DIVER   depth-first with a seeded random choice at junctions; at a dead end it backtracks (free) to the last
            junction with an unexplored side
    SEEKER  greedy best-first: always uncovers the frontier cell nearest the centre (straight-line guess), so
            pockets near the centre trap it and it darts back to an older frontier
    MARBLE  a real ball: 2D circle-vs-wall physics under a slowly changing board tilt (it is the only thing the
            tilt steers)

Fairness: the three paint explorers uncover exactly the same number of new cells per beat (their step times come
from one schedule, `step_times`, with 1 per beat in the race and 2 per beat in the final stretch). Moving through
cells already uncovered (backtracking, darting) is free. The marble has no budget: its pace is the physics.

Progress = how close to the centre: 1 - (true maze distance to the goal of the best cell reached) / (that distance
from the start corner). An explorer finishes when it uncovers a goal cell.

Nothing here knows about the picture: `Race(seed).run()` returns plain event lists."""
import math
import random
from collections import deque

import numpy as np

N = 20
C0 = N // 2 - 1                              # goal room = rows/cols C0, C0 + 1
GOAL = {(C0, C0), (C0, C0 + 1), (C0 + 1, C0), (C0 + 1, C0 + 1)}
NAMES = ['FLOOD', 'DIVER', 'SEEKER', 'MARBLE']
INSTR = ['STRINGS', 'BASS', 'MARIMBA', 'DRUMS']

# music clock (shared with timeline.py): 122 BPM fitted to 31 whole bars in 61 s
DUR = 61.0
BPM = 31 * 4 * 60.0 / DUR                    # 121.967 = reelkit.config.loop_bpm(122, 61)
BEAT = 60.0 / BPM
BAR = 4 * BEAT
S16 = BEAT / 4
T_STOP = 26 * BAR                            # the race ends here (51.16 s): anyone still out is lost
FINAL_BAR = 20                               # final stretch (39.35 s): 2 cells per beat for everyone
MARBLE_RELEASE = 3 * BEAT                    # the marble's gate opens on beat 4 of bar 1 (its label lands)
TILT_IN = (1.0, 2.2)                         # board tilt fades in ...
TILT_OUT = (T_STOP + 0.2, T_STOP + 4.0)      # ... and back to level after the race


def rot(cell, k=1):
    r, c = cell
    for _ in range(k % 4):
        r, c = c, N - 1 - r
    return (r, c)


def corner(q):
    return rot((0, 0), q)


# ------------------------------------------------------------------ step schedule (the fairness rule)
RACE_PAT = {0: [0], 1: [3], 2: [2]}           # race (bars 2-20): 1 cell per beat each, on different 16ths:
                                              # FLOOD on the beat (strings), SEEKER on the & (marimba), DIVER on the a (bass)
FINAL_PAT = {                                 # final stretch: 2 cells per beat each, interlocking
    0: ([0, 2], [0, 2]),                      # FLOOD: straight 8ths (staccato strings)
    1: ([0, 3], [2, 3]),                      # DIVER: "x..x ..xx" (picked spy bass)
    2: ([1, 2], [1, 3]),                      # SEEKER: ".xx. .x.x" (marimba, between the others)
}
BAR0 = {0: [0.0, 2.0], 1: [1.0, 3.0], 2: [2.0, 3.5]}        # bar 1: a canon, 2 cells each (beats), labels land


def step_times(q, t_end=T_STOP, final_bar=None):
    """the times explorer q (0-2) uncovers a cell; every explorer gets the same count per beat"""
    fb = FINAL_BAR if final_bar is None else final_bar
    out = [b * BEAT for b in BAR0[q]]
    for beat in range(4, int(t_end / BEAT) + 1):
        bar = beat // 4
        t0 = beat * BEAT
        slots = RACE_PAT[q] if bar < fb else FINAL_PAT[q][beat % 2]
        out += [t0 + s * S16 for s in slots]
    return [t for t in out if t < t_end - 1e-9]


# ------------------------------------------------------------------ maze
class Maze:
    def __init__(self, seed, p_newest=1.0, links=None, braids=None, grow_from_door=True):
        rng = random.Random(seed * 7919 + 13)
        self.seed = seed
        self.open = set()                     # frozenset({a, b}) for every open passage
        quad = [(r, c) for r in range(N // 2) for c in range(N // 2) if (r, c) not in GOAL]
        qs = set(quad)
        # growing tree over the quadrant, grown from the goal door (so the route from the corner winds a long way)
        door = rng.choice([(C0 - 1, C0), (C0, C0 - 1)])
        start = door if grow_from_door else rng.choice(quad)
        seen, active = {start}, [start]
        while active:
            i = len(active) - 1 if rng.random() < p_newest else rng.randrange(len(active))
            cur = active[i]
            nbs = [n for n in self._grid_nbrs(cur) if n in qs and n not in seen]
            if not nbs:
                active.pop(i)
                continue
            nxt = rng.choice(nbs)
            self._open_sym(cur, nxt)
            seen.add(nxt)
            active.append(nxt)
        # door into the goal room
        self._open_sym(door, (C0, C0))
        for a, b in [((C0, C0), (C0, C0 + 1)), ((C0, C0), (C0 + 1, C0)), ((C0 + 1, C0 + 1), (C0, C0 + 1)), ((C0 + 1, C0 + 1), (C0 + 1, C0))]:
            self.open.add(frozenset((a, b)))
        # links between neighbouring quadrants (loops across), braids inside a quadrant
        nl = rng.choice([1, 1, 2]) if links is None else links
        rows = rng.sample(range(0, C0), nl)
        for r in rows:
            self._open_sym((r, C0), (r, C0 + 1))
        nb = rng.choice([0, 1, 2, 3]) if braids is None else braids
        walls = [(a, b) for a in quad for b in self._grid_nbrs(a) if b in qs and a < b and frozenset((a, b)) not in self.open]
        for a, b in rng.sample(walls, min(nb, len(walls))):
            self._open_sym(a, b)
        self.n_links, self.n_braids = nl, nb
        self.adj = {(r, c): [] for r in range(N) for c in range(N)}
        for e in self.open:
            a, b = tuple(e)
            self.adj[a].append(b)
            self.adj[b].append(a)
        for k in self.adj:                    # fixed neighbour order: N, E, S, W
            self.adj[k].sort(key=lambda n: ((n[0] - k[0], n[1] - k[1]) != (-1, 0), (n[0] - k[0], n[1] - k[1]) != (0, 1),
                                            (n[0] - k[0], n[1] - k[1]) != (1, 0)))
        # true distance to the goal room
        self.dist = {}
        dq = deque()
        for g in GOAL:
            self.dist[g] = 0
            dq.append(g)
        while dq:
            a = dq.popleft()
            for b in self.adj[a]:
                if b not in self.dist:
                    self.dist[b] = self.dist[a] + 1
                    dq.append(b)
        self.D0 = self.dist[(0, 0)]

    @staticmethod
    def _grid_nbrs(c):
        r, cc = c
        return [(r + dr, cc + dc) for dr, dc in ((-1, 0), (0, 1), (1, 0), (0, -1)) if 0 <= r + dr < N and 0 <= cc + dc < N]

    def _open_sym(self, a, b):
        for k in range(4):
            self.open.add(frozenset((rot(a, k), rot(b, k))))

    def is_open(self, a, b):
        return frozenset((a, b)) in self.open

    def walls(self):
        """wall segments in board units: ('h', z, x0, x1) along x at z, ('v', x, z0, z1) along z at x.
        A cell (r, c) spans x c..c+1, z r..r+1. Includes the outer border."""
        out = []
        for r in range(N + 1):
            for c in range(N):
                if r == 0 or r == N or not self.is_open((r - 1, c), (r, c)):
                    out.append(('h', r, c, c + 1))
        for c in range(N + 1):
            for r in range(N):
                if c == 0 or c == N or not self.is_open((r, c - 1), (r, c)):
                    out.append(('v', c, r, r + 1))
        return out


# ------------------------------------------------------------------ the three paint explorers
class Flood:
    def __init__(self, mz, start):
        self.mz, self.start = mz, start
        self.q = deque([start])
        self.depth = {start: 0}
        self.parent = {start: None}
        self.last_depth = -1

    def step(self):
        c = self.q.popleft()
        for n in self.mz.adj[c]:
            if n not in self.depth:
                self.depth[n] = self.depth[c] + 1
                self.parent[n] = c
                self.q.append(n)
        ring = self.depth[c] > self.last_depth
        self.last_depth = self.depth[c]
        return dict(cell=c, parent=self.parent[c], ring=ring, depth=self.depth[c], back=[], dead=False)


class Diver:
    def __init__(self, mz, start, seed):
        self.mz, self.start = mz, start
        self.rng = random.Random(seed * 104729 + 7)
        self.stack, self.seen = [], set()

    def step(self):
        if not self.stack:
            self.stack.append(self.start)
            self.seen.add(self.start)
            return dict(cell=self.start, parent=None, back=[], dead=False, depth=1)
        back = []
        while True:
            head = self.stack[-1]
            opts = [n for n in self.mz.adj[head] if n not in self.seen]
            if opts:
                break
            back.append(self.stack.pop())
        nxt = self.rng.choice(opts)
        self.seen.add(nxt)
        self.stack.append(nxt)
        return dict(cell=nxt, parent=head, back=back, dead=bool(back), depth=len(self.stack))


def centre_h(c):
    return math.hypot(c[0] + 0.5 - N / 2, c[1] + 0.5 - N / 2)


class Seeker:
    def __init__(self, mz, start):
        self.mz, self.start = mz, start
        self.done, self.front = set(), {}
        self.pos = None

    def step(self):
        if self.pos is None:
            f = self.start
            path = [f]
        else:
            f = min(self.front, key=lambda c: (round(centre_h(c), 6), c))
            del self.front[f]
            path = self._path(self.pos, f)
        dead = self.pos is not None and len(path) > 2
        self.done.add(f)
        for n in self.mz.adj[f]:
            if n not in self.done:
                self.front[n] = 1
        prev = self.pos
        self.pos = f
        return dict(cell=f, parent=path[-2] if len(path) > 1 else None, path=path, dead=dead,
                    dead_at=prev if dead else None, back=[])

    def _path(self, a, b):
        """shortest route from a to b through uncovered cells (b is a frontier cell)"""
        prev = {a: None}
        dq = deque([a])
        while dq:
            x = dq.popleft()
            if x == b:
                break
            for n in self.mz.adj[x]:
                if n not in prev and (n in self.done or n == b):
                    prev[n] = x
                    dq.append(n)
        out, x = [], b
        while x is not None:
            out.append(x)
            x = prev[x]
        return out[::-1]


# ------------------------------------------------------------------ the marble
class Marble:
    """circle vs axis-aligned wall boxes, rolling under a tilt vector; board units (1 = one cell)"""
    R = 0.27
    HW = 0.08                                 # wall half thickness
    G = 34.0                                  # cells/s^2 per unit tilt (5/7 g sin(angle), scaled to the board)
    ROLL = 0.6                                # rolling resistance (cells/s^2)
    DAMP = 0.15                               # 1/s
    VMAX = 7.0
    E = 0.45                                  # restitution
    DT = 1.0 / 240

    def __init__(self, mz, start, tilt):
        self.mz, self.tilt = mz, tilt
        self.x, self.z = start[1] + 0.5, start[0] + 0.5
        self.vx = self.vz = 0.0
        rects = []
        for kind, a, b0, b1 in mz.walls():
            if kind == 'h':
                rects.append((b0 - self.HW, b1 + self.HW, a - self.HW, a + self.HW, 'h'))
            else:
                rects.append((a - self.HW, a + self.HW, b0 - self.HW, b1 + self.HW, 'v'))
        self.near = {}
        for r in range(N):
            for c in range(N):
                self.near[(r, c)] = [q for q in rects if q[0] < c + 2.2 and q[1] > c - 1.2 and q[2] < r + 2.2 and q[3] > r - 1.2]

    def cell(self):
        return (min(N - 1, max(0, int(self.z))), min(N - 1, max(0, int(self.x))))

    def advance(self, t0, t1, hits):
        t = t0
        while t < t1 - 1e-12:
            dt = min(self.DT, t1 - t)
            gx, gz = self.tilt(t)
            ax, az = self.G * gx, self.G * gz
            sp = math.hypot(self.vx, self.vz)
            if sp > 1e-6:
                ax -= (self.ROLL + self.DAMP * sp) * self.vx / sp
                az -= (self.ROLL + self.DAMP * sp) * self.vz / sp
            self.vx += ax * dt
            self.vz += az * dt
            sp = math.hypot(self.vx, self.vz)
            if sp > self.VMAX:
                self.vx *= self.VMAX / sp
                self.vz *= self.VMAX / sp
            self.x += self.vx * dt
            self.z += self.vz * dt
            for (x0, x1, z0, z1, kind) in self.near[self.cell()]:
                px = min(max(self.x, x0), x1)
                pz = min(max(self.z, z0), z1)
                dx, dz = self.x - px, self.z - pz
                d2 = dx * dx + dz * dz
                if d2 < self.R * self.R:
                    d = math.sqrt(d2)
                    if d < 1e-9:
                        continue
                    nx, nz = dx / d, dz / d
                    self.x += nx * (self.R - d)
                    self.z += nz * (self.R - d)
                    vn = self.vx * nx + self.vz * nz
                    if vn < 0:
                        self.vx -= (1 + self.E) * vn * nx
                        self.vz -= (1 + self.E) * vn * nz
                        tx, tz = -nz, nx
                        vt = self.vx * tx + self.vz * tz
                        self.vx -= 0.08 * vt * tx
                        self.vz -= 0.08 * vt * tz
                        if -vn > 0.6:
                            hits.append((t + dt, -vn, 'h' if abs(nz) > abs(nx) else 'v', self.x, self.z))
            t += dt


def make_tilt(seed):
    """the board rocks on the beat: on every beat it tips toward a new seeded compass direction (8 choices, seeded
    strength), easing over 0.3 s. +x = right, +z = toward the camera. Faded in after GO, level again after the race.
    (Slow sweeps left the marble parked in pockets for seconds: median progress 0.15-0.3, ~1 wall hit/s.)"""
    rng = np.random.default_rng(seed + 900001)
    per = BEAT
    nseg = int(DUR / per) + 2
    ang = rng.integers(0, 8, nseg) * (math.pi / 4)
    mag = rng.uniform(0.55, 1.0, nseg)
    vx, vz = mag * np.cos(ang), mag * np.sin(ang)
    ease_t = 0.3

    def raw(t):
        k = int(t // per)
        u = min(1.0, (t - k * per) / ease_t)
        u = u * u * (3 - 2 * u)
        a = max(k - 1, 0)
        return vx[a] + (vx[k] - vx[a]) * u, vz[a] + (vz[k] - vz[a]) * u

    def env(t):
        a = min(1.0, max(0.0, (t - TILT_IN[0]) / (TILT_IN[1] - TILT_IN[0])))
        b = 1.0 - min(1.0, max(0.0, (t - TILT_OUT[0]) / (TILT_OUT[1] - TILT_OUT[0])))
        e = min(a, b)
        return e * e * (3 - 2 * e)

    def tilt(t):
        gx, gz = raw(t)
        e = env(t)
        return gx * e, gz * e
    tilt.dirs = (vx, vz)
    return tilt


# ------------------------------------------------------------------ one race
class Race:
    def __init__(self, seed, tilt_seed=None, maze_kw=None):
        self.seed = seed
        self.tilt_seed = seed if tilt_seed is None else tilt_seed
        self.mz = Maze(seed, **(maze_kw or {}))
        self.tilt = make_tilt(self.tilt_seed)

    def run(self, record_marble=True, fps=30):
        self.run_paint()
        if record_marble:
            self.run_marble(fps)
        return self

    def run_paint(self):
        mz = self.mz
        ex = [Flood(mz, corner(0)), Diver(mz, corner(1), self.seed), Seeker(mz, corner(2))]
        steps = [[] for _ in range(4)]
        fin = [None] * 4
        best = [mz.D0] * 4
        prog = [[(0.0, 0.0)] for _ in range(4)]
        events = []
        for q in range(3):
            for t in step_times(q):
                if fin[q] is not None:
                    break
                ev = ex[q].step()
                ev['t'] = t
                steps[q].append(ev)
                d = mz.dist[ev['cell']]
                if d < best[q]:
                    best[q] = d
                    prog[q].append((t, 1 - d / mz.D0))
                if ev['cell'] in GOAL:
                    fin[q] = t
        self.steps, self.fin, self.prog = steps, fin, prog
        self.hits, self.mpos, self.mcells = [], np.zeros((0, 2)), []
        return self

    def run_marble(self, fps=30, t_stop=T_STOP):
        mz, fin, prog = self.mz, self.fin, self.prog
        fin[3] = None
        prog[3] = [(0.0, 0.0)]
        best = mz.D0
        m = Marble(mz, corner(3), self.tilt)
        hits = []
        pos = []
        cells = []
        nf = int(round(DUR * fps))
        last = None
        for f in range(nf):
            t = f / fps
            if t > MARBLE_RELEASE and fin[3] is None and t <= t_stop:
                m.advance(max(MARBLE_RELEASE, (f - 1) / fps), t, hits)
            pos.append((m.x, m.z))
            c = m.cell()
            if fin[3] is None and c != last and t <= t_stop:
                cells.append((t, c))
                last = c
                d = mz.dist[c]
                if d < best:
                    best = d
                    prog[3].append((t, 1 - d / mz.D0))
                if c in GOAL:
                    fin[3] = t
        self.hits = [h for h in hits if fin[3] is None or h[0] <= fin[3]]
        self.mpos = np.array(pos)
        self.mcells = cells
        return self

    # ---- story metrics (used by seed_search and the captions)
    def progress_at(self, q, t):
        p = 0.0
        for tt, v in self.prog[q]:
            if tt <= t:
                p = v
            else:
                break
        return p

    def leaders(self, t0=6.0, t1=None, dt=0.25, hyst=0.04):
        """leader over time with hysteresis: a new leader must pass the old one by `hyst` (progress units).
        Finished explorers rank first, by finish time. Returns [(t, q)] whenever the leader changes."""
        t1 = t1 or T_STOP
        fins = [f for f in self.fin if f is not None]
        first = min(fins) if fins else None
        out, cur = [], None
        t = t0
        while t <= t1 and (first is None or t <= first + 1e-9):
            p = [self.progress_at(q, t) for q in range(4)]
            m = int(np.argmax(p))
            if cur is None:
                cur = m
                out.append((t, m))
            elif m != cur and p[m] >= p[cur] + hyst:
                cur = m
                out.append((t, m))
            t += dt
        if first is not None:
            w = self.fin.index(first)
            if w != cur:
                out.append((first, w))
        return out

    def seeker_traps(self, near=3.6):
        """(t, dead-end cell) where SEEKER had to dart back from a dead end close to the centre"""
        return [(e['t'], e['dead_at']) for e in self.steps[2] if e['dead'] and centre_h(e['dead_at']) <= near]

    def diver_deadends(self):
        return [(e['t'], len(e['back'])) for e in self.steps[1] if e['dead']]

    def marble_runs(self, min_cells=4):
        """times the marble ran >= min_cells cells in a straight line without stopping (a 'long corridor')"""
        out = []
        cs = self.mcells
        i = 0
        while i < len(cs) - 1:
            j = i
            while j + 1 < len(cs) and (cs[j + 1][1][0] == cs[i][1][0] or cs[j + 1][1][1] == cs[i][1][1]) and \
                    abs(cs[j + 1][1][0] - cs[j][1][0]) + abs(cs[j + 1][1][1] - cs[j][1][1]) == 1 and cs[j + 1][0] - cs[j][0] < 0.6:
                j += 1
            if j - i + 1 >= min_cells:
                out.append((cs[i][0], j - i + 1))
            i = max(j, i + 1)
        return out


if __name__ == '__main__':
    import sys
    import time
    s = int(sys.argv[1]) if len(sys.argv) > 1 else 1
    t0 = time.time()
    r = Race(s).run()
    print(f'seed {s}: D0 {r.mz.D0}, links {r.mz.n_links}, braids {r.mz.n_braids}, {time.time() - t0:.2f} s')
    for q in range(4):
        n = len(r.steps[q]) if q < 3 else len(set(c for _, c in r.mcells))
        print(f'  {NAMES[q]:6s} finish {r.fin[q]}  cells {n}  progress@46 {r.progress_at(q, 46):.2f}')
    print('  leaders', [(round(t, 1), NAMES[q]) for t, q in r.leaders()])
    print('  seeker traps', [(round(t, 1), c) for t, c in r.seeker_traps()])
    print('  marble runs', [(round(t, 1), n) for t, n in r.marble_runs()][:12], 'hits', len(r.hits))
