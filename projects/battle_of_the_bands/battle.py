"""Battle engine: physical-process teams fighting over a tile arena (reusable for the whole series).

Nothing here knows about the video: no rendering, no music, no timeline. An episode is a config
(arena shape, team line-up + rule parameters, seed, when the fight starts) and the engine returns a
recording: the owner grid at every frame, every claim (who, where, when), eliminations, and the
per-frame state of each team's process (sand grains, the ball, ripple rings, vine tips) for drawing.

    from battle import Arena, Battle, SandTeam, BallTeam, RippleTeam, VineTeam
    arena = Arena(72, 'circle')
    teams = [SandTeam(hopper_col=30), BallTeam(start=(56, 16)), RippleTeam(base=(44, 3)), VineTeam(base=(53, 66))]
    rec = Battle(arena, teams, seed=7, t_fight=21.4).run(60.0)

Rules (two acts, same for every episode):
  FILL   teams claim only empty tiles, each with its own physical move.
  FIGHT  from t_fight: any empty gaps go to their neighbours, then each team attacks its borders with
         its own move. A team down to `out_at` tiles or fewer is out: its last tiles go dark and its
         neighbours absorb them. When one team is left it floods the stage (the win).

Teams (each a physical process; every parameter is a balance knob):
  SandTeam    grains pour from a hopper above the arena, fall under gravity, slide off slopes and pile
              up. FIGHT: grains pour through their own sand and smash into enemy tiles below; the pile
              avalanches down and sideways into neighbours (never upward).
  BallTeam    a ball bounces under gravity, elastic off everything. FILL: it paints the tiles it crosses
              and bounces off other teams. FIGHT: it flies inside its own land; every enemy tile it hits
              flips to its colour and bounces it back (a fast ball can smash through a few in a row).
  RippleTeam  one ring per beat spreads from its base through its own land; where it reaches the edge
              it claims (FILL) or pushes the front line out by a tile (FIGHT).
  VineTeam    tips grow through open space, branch, and backtrack along their own path when boxed in.
              FIGHT: tips burrow into enemy tiles, leaving tendrils.

Deterministic: everything random comes from one numpy Generator seeded by `seed`.
Grid coordinates: owner[i, j], row i = 0 at the top (gravity points to +i), column j; -1 = outside
the arena, 0 = empty, 1..k = team k (team index + 1)."""
import heapq
import math
from collections import deque
import numpy as np

OUT, EMPTY = -1, 0
N8 = [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (-1, 1), (1, -1), (1, 1)]
N8_LEN = [1, 1, 1, 1, math.sqrt(2), math.sqrt(2), math.sqrt(2), math.sqrt(2)]


# ====================================================================== arena
class Arena:
    """N x N tile grid with a shape mask. shape: 'circle', 'square', 'hexagon', 'diamond', 'ring', or a
    callable (u, v) -> bool on centred coordinates in [-1, 1] (v up)."""
    def __init__(self, n=72, shape='circle'):
        self.n = n
        c = (np.arange(n) + 0.5) / n * 2 - 1
        v, u = -c[:, None] * np.ones((1, n)), c[None, :] * np.ones((n, 1))
        if callable(shape):
            m = shape(u, v)
        elif shape == 'circle':
            m = u * u + v * v <= 1.0
        elif shape == 'square':
            m = np.ones((n, n), bool)
        elif shape == 'hexagon':
            m = (np.abs(v) <= 0.866) & (np.abs(v) * 0.577 + np.abs(u) <= 1.0)
        elif shape == 'diamond':
            m = np.abs(u) + np.abs(v) <= 1.0
        elif shape == 'ring':
            r2 = u * u + v * v
            m = (r2 <= 1.0) & (r2 >= 0.12)
        else:
            raise ValueError(shape)
        self.mask = np.asarray(m, bool)
        self.shape = shape
        self.ntiles = int(self.mask.sum())

    def rim_cell(self, deg, inset=2):
        """the arena cell `inset` tiles in from the rim at angle deg (0 = right, 90 = top)"""
        n, c = self.n, self.n / 2
        a = math.radians(deg)
        best = None
        for r in np.arange(c, 0, -0.25):
            i, j = int(c - r * math.sin(a)), int(c + r * math.cos(a))
            if 0 <= i < n and 0 <= j < n and self.mask[i, j]:
                best = r
                break
        r = (best or c) - inset
        return int(c - r * math.sin(a)), int(c + r * math.cos(a))

    def top_row(self, j):
        rows = np.nonzero(self.mask[:, j])[0]
        return int(rows[0]) if len(rows) else None


# ====================================================================== teams
class Team:
    kind = 'team'
    name = 'TEAM'
    color = (255, 255, 255)

    def __init__(self, name=None, color=None, **params):
        self.name = name or self.name
        self.color = color or self.color
        p = dict(self.defaults)
        bad = set(params) - set(p)
        if bad:
            raise TypeError(f'{type(self).__name__}: unknown parameters {sorted(bad)}')
        p.update(params)
        self.p = p
        self.k = None            # team id in the grid (1..)

    def setup(self, b):
        pass

    def step(self, b, t, dt):
        pass

    def on_out(self, b):
        pass

    def state(self):
        """per-frame drawing state (plain lists/tuples of numbers)"""
        return {}


class SandTeam(Team):
    kind, name, color = 'sand', 'SAND', (255, 186, 60)
    defaults = dict(hopper_col=30, width=2.5, sweep=0.0, sweep_T=8.0, pour=40.0, g=70.0, vmax=45.0, p_impact=0.35,
                    p_slide=0.25, p_down=0.004, p_diag=0.002, p_side=0.0006, side_needs_load=True)

    def setup(self, b):
        self.y, self.x, self.vy = [], [], []
        self.acc = 0.0
        self.landed = []                 # (i, j) of grains that came to rest this step (for drawing puffs)

    def hopper_x(self, t):
        """the hopper swings along its rail: column (float) at time t"""
        return self.p['hopper_col'] + self.p['sweep'] * math.sin(2 * math.pi * t / self.p['sweep_T'])

    def _spawn(self, b, n):
        rng = b.rng
        hx = self.hopper_x(b.t)
        for _ in range(n):
            j = int(np.clip(round(hx + rng.uniform(-self.p['width'], self.p['width'])), 0, b.n - 1))
            i0 = b.arena.top_row(j)
            if i0 is None:
                continue
            c = b.owner[i0, j]
            if b.phase == 'fill' and c != EMPTY:
                continue                                     # hopper blocked: the drummer stalls
            if b.phase != 'fill' and c not in (EMPTY, self.k):
                if rng.random() < self.p['p_impact'] * b.pw(self.k):
                    b.take(i0, j, self.k)
                continue
            self.y.append(i0 + 0.5); self.x.append(j + 0.5); self.vy.append(8.0 + rng.uniform(0, 6))

    def step(self, b, t, dt):
        p, rng, own, k = self.p, b.rng, b.owner, self.k
        self.landed = []
        self.acc += p['pour'] * dt * b.tempo(self.k)
        n = int(self.acc)
        self.acc -= n
        if n:
            self._spawn(b, n)
        fill = b.phase == 'fill'
        ny, nx, nv = [], [], []
        for y, x, vy in zip(self.y, self.x, self.vy):
            vy = min(vy + p['g'] * dt, p['vmax'])
            ty = y + vy * dt
            i, j = int(y), int(x)
            alive = True
            while alive and i + 1 <= int(ty):
                ni = i + 1
                c = own[ni, j] if ni < b.n else OUT
                if c == EMPTY:
                    i = ni
                    continue
                if not fill and c == k:
                    # pouring through its own sand: may slide into an enemy tile diagonally below
                    if rng.random() < p['p_slide']:
                        dj = 1 if rng.random() < 0.5 else -1
                        for jj in (j + dj, j - dj):
                            if 0 <= jj < b.n and own[ni, jj] > 0 and own[ni, jj] != k:
                                if rng.random() < p['p_impact'] * b.pw(k):
                                    b.take(ni, jj, k)
                                alive = False
                                break
                        if not alive:
                            break
                    i = ni
                    continue
                if not fill and c > 0:                       # smash into an enemy tile below
                    b.attempt(ni, j, k)
                    if rng.random() < p['p_impact'] * b.pw(k):
                        b.take(ni, j, k)
                    alive = False
                    break
                # blocked (floor, wall, or anyone's tile in FILL): slide off the slope, or come to rest
                moved = False
                dj = 1 if rng.random() < 0.5 else -1
                for jj in (j + dj, j - dj):
                    if 0 <= jj < b.n and ni < b.n and own[ni, jj] == EMPTY and own[i, jj] in (EMPTY, k):
                        i, j = ni, jj
                        moved = True
                        break
                if moved:
                    continue
                b.attempt(i, j, k)
                if own[i, j] == EMPTY:
                    b.take(i, j, k)
                    self.landed.append((i, j))
                alive = False
            if alive:
                if own[i, j] not in (EMPTY, k):
                    # the tile it is falling through was taken by someone else meanwhile: crushed (FILL)
                    # or smashes into it (FIGHT)
                    if not fill and rng.random() < p['p_impact']:
                        b.take(i, j, k)
                    continue
                ny.append(min(ty, i + 0.999)); nx.append(j + 0.5); nv.append(vy)
        self.y, self.x, self.vy = ny, nx, nv
        if not fill:
            self._avalanche(b)

    def _avalanche(self, b):
        """the pile slides into enemy tiles below / beside it (vectorised, never upward)"""
        p, own, k, rng = self.p, b.owner, self.k, b.rng
        S = own == k
        if not S.any():
            return
        enemy = (own > 0) & (own != k)
        cand = []
        # (di, dj, prob)
        for di, dj, pr in ((1, 0, p['p_down']), (1, -1, p['p_diag']), (1, 1, p['p_diag']), (0, -1, p['p_side']), (0, 1, p['p_side'])):
            if pr <= 0:
                continue
            src = S.copy()
            if di == 0 and p['side_needs_load']:
                src[1:] &= S[:-1]                             # sideways only under load (sand above it)
                src[0] = False
            tgt = np.zeros_like(S)
            ys, xs = np.nonzero(src)
            ti, tj = ys + di, xs + dj
            ok = (ti >= 0) & (ti < b.n) & (tj >= 0) & (tj < b.n)
            ys, xs, ti, tj = ys[ok], xs[ok], ti[ok], tj[ok]
            hit = enemy[ti, tj]
            ti, tj = ti[hit], tj[hit]
            if len(ti):
                r = rng.random(len(ti)) < pr * b.pw(k)
                cand.extend(zip(ti[r].tolist(), tj[r].tolist()))
        for i, j in cand:
            if own[i, j] > 0 and own[i, j] != k:
                b.take(i, j, k)

    def on_out(self, b):
        self.y, self.x, self.vy = [], [], []
        self.p['pour'] = 0.0

    def state(self):
        return {'grains': [(round(y, 2), round(x, 2)) for y, x in zip(self.y, self.x)], 'landed': list(self.landed),
                'hopper': round(self.hopper_x(self._t), 3), 'pouring': self.p['pour'] > 0}


class BallTeam(Team):
    kind, name, color = 'ball', 'BALL', (176, 96, 255)
    defaults = dict(start=(20.0, 52.0), v0=(-22.0, 6.0), g=45.0, r_paint=1.2, pierce_speed=None, pierce_loss=0.85,
                    max_step=0.35, jitter=4.0, e_relax=0.6, r_smash=0.0)

    def setup(self, b):
        y, x = self.p['start']
        self.y, self.x = float(y), float(x)
        self.vx, self.vy = float(self.p['v0'][0]), float(self.p['v0'][1])
        self.E0 = 0.5 * (self.vx ** 2 + self.vy ** 2) - self.p['g'] * self.y
        r = self.p['r_paint']
        R = int(math.ceil(r))
        self.brush = [(di, dj) for di in range(-R, R + 1) for dj in range(-R, R + 1) if di * di + dj * dj <= r * r + 1e-6]
        r = self.p['r_smash']
        R = int(math.ceil(r))
        self.smash = [(di, dj) for di in range(-R, R + 1) for dj in range(-R, R + 1) if di * di + dj * dj <= r * r + 1e-6]
        self.hits = []                  # (i, j) flipped this step (sparks)
        self.bounces = 0
        self.alive = True
        i, j = int(self.y), int(self.x)
        for di, dj in self.brush:
            if b.arena.mask[i + di, j + dj]:
                b.take(i + di, j + dj, self.k)

    def _paint(self, b):
        i, j = int(self.y), int(self.x)
        for di, dj in self.brush:
            ii, jj = i + di, j + dj
            if 0 <= ii < b.n and 0 <= jj < b.n and b.owner[ii, jj] == EMPTY:
                b.take(ii, jj, self.k)

    def E0_eff(self, b):
        """target energy: a loud ball flies faster (speed ~ tempo)"""
        tp = b.tempo(self.k)
        if tp == 1.0:
            return self.E0
        # scale kinetic energy at mid-height by tempo^2
        ymid = b.n * 0.5
        ke = self.E0 + self.p['g'] * ymid
        return ke * tp * tp - self.p['g'] * ymid

    def _energy(self):
        return 0.5 * (self.vx ** 2 + self.vy ** 2) - self.p['g'] * self.y

    def _renorm(self, target=None):
        """set the total energy (bounces off the stage rim recharge the ball fully)"""
        e = (self.E0 if target is None else target) + self.p['g'] * self.y
        sp = math.hypot(self.vx, self.vy)
        if e > 1 and sp > 1e-6:
            s = math.sqrt(2 * e) / sp
            self.vx *= s; self.vy *= s

    def _jitter(self, rng):
        a = math.radians(rng.normal(0, self.p['jitter']))
        c, s = math.cos(a), math.sin(a)
        self.vx, self.vy = c * self.vx - s * self.vy, s * self.vx + c * self.vy

    def step(self, b, t, dt):
        if not self.alive:
            return
        p, own, k = self.p, b.owner, self.k
        self.hits = []
        fill = b.phase == 'fill'
        ci, cj = int(self.y), int(self.x)
        if not fill and own[ci, cj] > 0 and own[ci, cj] != k:
            b.take(ci, cj, k)                             # defends the tile it is sitting on
            self.hits.append((ci, cj))
        sp = math.hypot(self.vx, self.vy)
        nsub = max(1, int(math.ceil((sp + p['g'] * dt) * dt / p['max_step'])))
        h = dt / nsub
        for _ in range(nsub):
            self.vy += p['g'] * h
            ny, nx = self.y + self.vy * h, self.x + self.vx * h
            oi, oj = int(self.y), int(self.x)
            ni, nj = int(ny), int(nx)
            if (ni, nj) == (oi, oj):
                self.y, self.x = ny, nx
                if fill:
                    self._paint(b)
                continue
            inside = 0 <= ni < b.n and 0 <= nj < b.n
            c = own[ni, nj] if inside else OUT
            blocked = c == OUT or (c > 0 and c != k)
            if not blocked:
                if c == EMPTY:
                    b.take(ni, nj, k)
                self.y, self.x = ny, nx
                self._paint(b)
                continue
            smash = False
            if c > 0:
                b.attempt(ni, nj, k)
            if c > 0 and not fill:
                pw = b.pw(k)
                if b.rng.random() < pw:
                    b.take(ni, nj, k)
                    self.hits.append((ni, nj))
                for di, dj in self.smash:                    # the impact flips a small crater
                    ii, jj = ni + di, nj + dj
                    if 0 <= ii < b.n and 0 <= jj < b.n and own[ii, jj] > 0 and own[ii, jj] != k and b.rng.random() < 0.8 * pw:
                        b.take(ii, jj, k)
                ps = p['pierce_speed']
                if ps and math.hypot(self.vx, self.vy) > ps / b.pw(k):
                    self.vx *= p['pierce_loss']; self.vy *= p['pierce_loss']
                    self.y, self.x = ny, nx
                    smash = True
            if smash:
                continue
            # bounce off the cell face(s) crossed
            def blk(ii, jj):
                if not (0 <= ii < b.n and 0 <= jj < b.n):
                    return True
                cc = own[ii, jj]
                return cc == OUT or (cc > 0 and cc != k)
            if ni != oi and nj == oj:
                self.vy = -self.vy
            elif nj != oj and ni == oi:
                self.vx = -self.vx
            else:
                bx, by = blk(oi, nj), blk(ni, oj)
                if bx:
                    self.vx = -self.vx
                if by:
                    self.vy = -self.vy
                if not bx and not by:
                    self.vx, self.vy = -self.vx, -self.vy
            self._jitter(b.rng)
            if c == OUT:
                self.bounces += 1
                self._renorm(self.E0_eff(b))
        e = self._energy()
        E0 = self.E0_eff(b)
        if e < E0:
            self._renorm(e + (E0 - e) * min(1.0, p['e_relax'] * dt))
        elif e > E0 * 1.02:
            self._renorm(e + (E0 - e) * min(1.0, p['e_relax'] * dt))

    def on_out(self, b):
        self.alive = False

    def state(self):
        return {'ball': (round(self.y, 3), round(self.x, 3), round(self.vy, 2), round(self.vx, 2)) if self.alive else None,
                'hits': list(self.hits)}


class RippleTeam(Team):
    kind, name, color = 'ripple', 'RIPPLE', (40, 222, 206)
    defaults = dict(base=(44, 3), period=0.5, phase=0.0, speed=26.0, p_fill=0.9, p_push=0.22, reach_fill=2,
                    reach_fight=1, reach_power=False, seed_r=2.5)

    def setup(self, b):
        bi, bj = self.p['base']
        r = self.p['seed_r']
        for i in range(b.n):
            for j in range(b.n):
                if b.arena.mask[i, j] and (i - bi) ** 2 + (j - bj) ** 2 <= r * r:
                    b.take(i, j, self.k)
        self.rings = []                  # live rings: dict(id, t0, src, heap of (arrival, i, j, depth))
        self.next_t = self.p['phase']
        self.fields = b.ring_fields      # shared store: id -> distance field (for drawing)
        self.pushed = []

    def _source(self, b):
        bi, bj = self.p['base']
        if b.owner[bi, bj] == self.k:
            return bi, bj
        ys, xs = np.nonzero(b.owner == self.k)
        if not len(ys):
            return None
        q = np.argmin((ys - bi) ** 2 + (xs - bj) ** 2)
        return int(ys[q]), int(xs[q])

    def _emit(self, b, t):
        src = self._source(b)
        if src is None:
            return
        own = b.owner == self.k
        d = geodesic(own, src)
        # frontier: non-own arena cells next to own cells, arrival = nearest own neighbour + step
        arr = np.full(d.shape, np.inf)
        for (di, dj), L in zip(N8, N8_LEN):
            sh = np.full(d.shape, np.inf)
            ys = slice(max(0, -di), b.n - max(0, di)); yd = slice(max(0, di), b.n - max(0, -di))
            xs = slice(max(0, -dj), b.n - max(0, dj)); xd = slice(max(0, dj), b.n - max(0, -dj))
            sh[yd, xd] = d[ys, xs]
            arr = np.minimum(arr, sh + L)
        front = (~own) & b.arena.mask & np.isfinite(arr)
        fy, fx = np.nonzero(front)
        heap = [(float(arr[i, j]), int(i), int(j), 1) for i, j in zip(fy, fx)]
        heapq.heapify(heap)
        rid = len(self.fields)
        self.fields[rid] = d.astype(np.float32)
        self.rings.append(dict(id=rid, t0=t, heap=heap, src=src, seen=set()))

    def step(self, b, t, dt):
        p, own, k, rng = self.p, b.owner, self.k, b.rng
        self.pushed = []
        while t >= self.next_t:
            self._emit(b, self.next_t)
            self.next_t += p['period']
        fill = b.phase == 'fill'
        reach = p['reach_fill'] if fill else p['reach_fight']
        if not fill and p['reach_power']:
            reach += min(3, int(max(0.0, b.pw(k) - 1.0)))         # a loud ripple pushes deeper
        keep = []
        for R in self.rings:
            r = p['speed'] * (t + dt - R['t0'])
            hp_ = R['heap']
            while hp_ and hp_[0][0] <= r:
                a, i, j, dep = heapq.heappop(hp_)
                if (i, j) in R['seen']:
                    continue
                R['seen'].add((i, j))
                c = own[i, j]
                got = False
                if c != k and c != OUT:
                    b.attempt(i, j, k)
                if c == EMPTY:
                    if rng.random() < p['p_fill']:
                        got = b.take(i, j, k)
                elif c > 0 and c != k and not fill:
                    if rng.random() < p['p_push'] * b.pw(k):
                        got = b.take(i, j, k)
                if got:
                    self.pushed.append((i, j))
                    if dep < reach:                            # the wave spills on past the old edge
                        for di, dj in N8[:4]:
                            ii, jj = i + di, j + dj
                            if 0 <= ii < b.n and 0 <= jj < b.n and own[ii, jj] != k and own[ii, jj] != OUT:
                                heapq.heappush(hp_, (a + 1.0, ii, jj, dep + 1))
            if hp_:
                keep.append(R)
            else:
                R.setdefault('r_end', r)
                if r < R['r_end'] + 10:                       # keep drawing the ring a little past its last claim
                    keep.append(R)
        self.rings = keep

    def on_out(self, b):
        self.rings = []
        self.next_t = 1e9

    def state(self):
        return {'rings': [(R['id'], round(self.p['speed'] * (self._t - R['t0']), 2)) for R in self.rings] if hasattr(self, '_t') else [],
                'pushed': list(self.pushed)}


class VineTeam(Team):
    kind, name, color = 'vine', 'VINE', (120, 232, 72)
    defaults = dict(base=(53, 66), tips0=3, max_tips=12, max_tips_fill=None, rate=7.0, rate_fight=None, p_branch=0.05,
                    momentum=3.0, p_burrow=0.3, wander=10, seed_r=1.5)

    def setup(self, b):
        bi, bj = self.p['base']
        r = self.p['seed_r']
        for i in range(b.n):
            for j in range(b.n):
                if b.arena.mask[i, j] and (i - bi) ** 2 + (j - bj) ** 2 <= r * r:
                    b.take(i, j, self.k)
        self.tips = []
        for q in range(self.p['tips0']):
            a = b.rng.uniform(0, 2 * math.pi)
            self.tips.append(dict(i=bi, j=bj, d=(math.sin(a), math.cos(a)), stack=[(bi, bj)], idle=0, acc=b.rng.uniform(0, 1)))
        self.grown = []

    def _choose(self, b, tip, cands):
        """pick a neighbour, preferring the current heading (momentum)"""
        di0, dj0 = tip['d']
        w = []
        for (ii, jj, di, dj) in cands:
            L = math.hypot(di, dj)
            w.append(math.exp(self.p['momentum'] * (di * di0 + dj * dj0) / L))
        w = np.array(w)
        q = b.rng.choice(len(cands), p=w / w.sum())
        return cands[q]

    def _move(self, b, tip, ii, jj, di, dj, claim):
        if claim:
            b.take(ii, jj, self.k)
            b.vine_parent[ii, jj] = N8.index((-di, -dj))
            self.grown.append((ii, jj))
        L = math.hypot(di, dj)
        tip['d'] = (0.6 * tip['d'][0] + 0.4 * di / L, 0.6 * tip['d'][1] + 0.4 * dj / L)
        tip['i'], tip['j'] = ii, jj
        tip['stack'].append((ii, jj))
        if len(tip['stack']) > 400:
            tip['stack'] = tip['stack'][-400:]

    def _respawn(self, b, want):
        """a new tip on a random own cell bordering `want` (EMPTY or 'enemy')"""
        own = b.owner == self.k
        if want == EMPTY:
            tgt = b.owner == EMPTY
        else:
            tgt = (b.owner > 0) & (b.owner != self.k)
        nb = np.zeros_like(own)
        nb[1:] |= tgt[:-1]; nb[:-1] |= tgt[1:]; nb[:, 1:] |= tgt[:, :-1]; nb[:, :-1] |= tgt[:, 1:]
        ys, xs = np.nonzero(own & nb)
        if not len(ys):
            return None
        q = b.rng.integers(len(ys))
        a = b.rng.uniform(0, 2 * math.pi)
        return dict(i=int(ys[q]), j=int(xs[q]), d=(math.sin(a), math.cos(a)), stack=[(int(ys[q]), int(xs[q]))], idle=0, acc=0.0)

    def step(self, b, t, dt):
        p, own, k, rng = self.p, b.owner, self.k, b.rng
        self.grown = []
        fill = b.phase == 'fill'
        new_tips = []
        dead = []
        rate = (p['rate'] if (fill or not p['rate_fight']) else p['rate_fight']) * b.tempo(k)
        for tip in self.tips:
            tip['acc'] += rate * dt
            while tip['acc'] >= 1.0:
                tip['acc'] -= 1.0
                i, j = tip['i'], tip['j']
                if own[i, j] != k:
                    if not fill and own[i, j] > 0 and rng.random() < p['p_burrow']:
                        b.take(i, j, k)                      # re-takes the cell it sits in
                        self.grown.append((i, j))
                    else:
                        dead.append(id(tip))
                        break
                emp, ene, mine = [], [], []
                for di, dj in N8:
                    ii, jj = i + di, j + dj
                    if not (0 <= ii < b.n and 0 <= jj < b.n):
                        continue
                    c = own[ii, jj]
                    if c == EMPTY:
                        emp.append((ii, jj, di, dj))
                    elif c == k:
                        mine.append((ii, jj, di, dj))
                    elif c > 0:
                        ene.append((ii, jj, di, dj))
                if emp:
                    ii, jj, di, dj = self._choose(b, tip, emp)
                    self._move(b, tip, ii, jj, di, dj, True)
                    tip['idle'] = 0
                    cap = p['max_tips_fill'] if (fill and p['max_tips_fill']) else p['max_tips']
                    if len(self.tips) + len(new_tips) < cap and rng.random() < p['p_branch']:
                        a = rng.uniform(0, 2 * math.pi)
                        new_tips.append(dict(i=ii, j=jj, d=(math.sin(a), math.cos(a)), stack=[(ii, jj)], idle=0, acc=0.0))
                    continue
                if ene and not fill:
                    b.attempt(ene[0][0], ene[0][1], k)
                    if rng.random() < p['p_burrow'] * b.pw(k):
                        ii, jj, di, dj = self._choose(b, tip, ene)
                        self._move(b, tip, ii, jj, di, dj, True)
                        tip['idle'] = 0
                        continue
                    tip['idle'] += 0.25
                    continue
                # boxed in: backtrack along its own path (FILL) / walk through its own land (FIGHT)
                if fill:
                    if len(tip['stack']) > 1:
                        tip['stack'].pop()
                        tip['i'], tip['j'] = tip['stack'][-1]
                    else:
                        dead.append(id(tip))
                        break
                else:
                    tip['idle'] += 1
                    if tip['idle'] > p['wander'] or not mine:
                        dead.append(id(tip))
                        break
                    ii, jj, di, dj = self._choose(b, tip, mine)
                    self._move(b, tip, ii, jj, di, dj, False)
        self.tips = [tp for tp in self.tips if id(tp) not in dead] + new_tips
        # keep the vine alive: replace lost tips at its own border
        want = EMPTY if fill else 'enemy'
        tries = 0
        while len(self.tips) < max(1, p['tips0'] if fill else p['max_tips'] // 2) and tries < 4:
            tries += 1
            tp = self._respawn(b, want if (fill or (b.owner == EMPTY).sum() == 0) else EMPTY)
            if tp is None and not fill:
                tp = self._respawn(b, 'enemy')
            if tp is None:
                break
            self.tips.append(tp)

    def on_out(self, b):
        self.tips = []

    def state(self):
        return {'tips': [(tp['i'], tp['j']) for tp in self.tips], 'grown': list(self.grown)}


TEAM_TYPES = {'sand': SandTeam, 'ball': BallTeam, 'ripple': RippleTeam, 'vine': VineTeam}


# ====================================================================== geodesic distance (8-neighbour chamfer)
def geodesic(passable, src):
    d = np.full(passable.shape, np.inf)
    d[src] = 0.0
    n0, n1 = passable.shape
    for _ in range(4 * max(n0, n1)):
        prev = d
        best = d.copy()
        for (di, dj), L in zip(N8, N8_LEN):
            sh = np.full(d.shape, np.inf)
            ys = slice(max(0, -di), n0 - max(0, di)); yd = slice(max(0, di), n0 - max(0, -di))
            xs = slice(max(0, -dj), n1 - max(0, dj)); xd = slice(max(0, dj), n1 - max(0, -dj))
            sh[yd, xd] = prev[ys, xs]
            np.minimum(best, sh + L, out=best)
        best[~passable] = np.inf
        best[src] = 0.0
        if np.array_equal(best, d):
            break
        d = best
    return d


# ====================================================================== the battle
class Battle:
    def __init__(self, arena, teams, seed=0, t_fight=20.0, dt=1 / 60, fps=30, out_at=6, absorb_p=0.2,
                 flood_rate=500.0, heat_T=None, power_alpha=0.0, power_lo=0.35, power_hi=2.0, close_T=0.0,
                 close_p=0.05, capture_n=7, capture_p=0.15, alpha_T=None, ko_share=0.0, tempo_power=0.0,
                 tempo_range=(0.4, 3.0), rounds=False):
        self.arena = arena
        self.n = arena.n
        self.teams = teams
        self.seed = seed
        self.rng = np.random.default_rng(seed)
        self.t_fight = t_fight
        self.dt = dt
        self.fps = fps
        self.out_at = out_at
        self.absorb_p = absorb_p
        self.flood_rate = flood_rate
        self.heat_T = heat_T           # the fight heats up: attack odds x (1 + (t - t_fight) / heat_T)
        self.heat = 1.0
        self.power_alpha = power_alpha   # volume = power: attack odds x (share / fair share) ** alpha
        self.power_lo, self.power_hi = power_lo, power_hi
        self.close_T, self.close_p = close_T, close_p   # last close_T s of FILL: empty tiles next to a team fill in
        self._pw = np.ones(len(teams) + 1)
        self.alpha_T = alpha_T           # the fight gets more decisive: alpha + (t - t_fight) / alpha_T
        self.ko_share = ko_share
        self.tempo_power, self.tempo_range = tempo_power, tempo_range
        self.rounds = rounds             # heat / decisiveness ramp from the last knockout (each round builds anew)         # knocked out below this share of the stage (0 = only at out_at tiles)
        self.capture_n, self.capture_p = capture_n, capture_p   # FIGHT: a tile with >= capture_n of its 8
                                                                # neighbours held by one rival falls to it
        self.owner = np.where(arena.mask, EMPTY, OUT).astype(np.int8)
        self.vine_parent = np.full(self.owner.shape, -1, np.int8)
        self.claim_t = np.full(self.owner.shape, -100.0, np.float32)   # when each tile last changed hands
        self.count = np.zeros(len(teams) + 1, np.int64)
        self.alive = [True] * len(teams)
        self.phase = 'fill'
        self.t = 0.0
        self.events_t, self.events_k, self.events_i, self.events_j, self.events_prev = [], [], [], [], []
        self.att_t, self.att_k, self.att_i, self.att_j = [], [], [], []   # every move tried (success or not)
        self.outs = []                 # (t, team index)
        self.winner = None
        self.t_win = None
        self.ring_fields = {}
        for q, tm in enumerate(teams):
            tm.k = q + 1

    def pw(self, k):
        """attack multiplier of team k right now: the heat of the fight x its volume (stage share)"""
        return self._pw[k]

    def t_round(self):
        if self.rounds and self.outs:
            return max(self.t_fight, self.outs[-1][0])
        return self.t_fight

    def tempo(self, k):
        """how fast team k's process runs in the FIGHT (volume = power: a loud band moves faster)"""
        if self.phase == 'fill' or not self.tempo_power:
            return 1.0
        lo, hi = self.tempo_range
        return min(hi, max(lo, self._pw[k] / self.heat)) ** self.tempo_power

    def _update_power(self):
        alive = [tm.k for q, tm in enumerate(self.teams) if self.alive[q]]
        tot = max(1, sum(self.count[k] for k in alive))
        a = self.power_alpha
        if self.alpha_T:
            a += (self.t - self.t_round()) / self.alpha_T
        for k in alive:
            rel = self.count[k] * len(alive) / tot
            m = rel ** a if a else 1.0
            self._pw[k] = self.heat * min(self.power_hi, max(self.power_lo, m))

    # ------------------------------------------------------------- tile changes (every change goes through here)
    def take(self, i, j, k):
        prev = int(self.owner[i, j])
        if prev == OUT or prev == k:
            return False
        if self.phase == 'fill' and prev != EMPTY:
            return False
        self.owner[i, j] = k
        self.claim_t[i, j] = self.t
        self.count[prev] -= prev > 0
        self.count[k] += 1
        self.events_t.append(self.t); self.events_k.append(k); self.events_i.append(i); self.events_j.append(j)
        self.events_prev.append(prev)
        return True

    def attempt(self, i, j, k):
        """log a move (a claim or an attack, whether or not it lands): the music plays these as ghost notes"""
        self.att_t.append(self.t); self.att_k.append(k); self.att_i.append(i); self.att_j.append(j)

    def _clear(self, i, j):
        prev = int(self.owner[i, j])
        if prev > 0:
            self.owner[i, j] = EMPTY
            self.claim_t[i, j] = self.t
            self.count[prev] -= 1
            self.events_t.append(self.t); self.events_k.append(0); self.events_i.append(i); self.events_j.append(j)
            self.events_prev.append(prev)

    def _neighbour_fill(self, prob=1.0, only=None):
        """empty cells go to a random adjacent team (all at once with prob 1: the gap snap)"""
        E = self.owner == EMPTY
        if not E.any():
            return 0
        ys, xs = np.nonzero(E)
        order = self.rng.permutation(len(ys))
        got = 0
        for q in order:
            i, j = ys[q], xs[q]
            if prob < 1 and self.rng.random() >= prob:
                continue
            nbs = [int(self.owner[i + di, j + dj]) for di, dj in N8 if 0 <= i + di < self.n and 0 <= j + dj < self.n]
            nbs = [c for c in nbs if c > 0 and (only is None or c == only)]
            if nbs:
                self.take(i, j, nbs[self.rng.integers(len(nbs))])
                got += 1
        return got

    def _capture(self):
        if not self.capture_n:
            return
        own = self.owner
        n = self.n
        pad = np.full((n + 2, n + 2), OUT, np.int8)
        pad[1:-1, 1:-1] = own
        best_k = np.zeros((n, n), np.int8)
        best_c = np.zeros((n, n), np.int8)
        for q, tm in enumerate(self.teams):
            if not self.alive[q]:
                continue
            m = (pad == tm.k)
            cnt = sum(m[1 + di:n + 1 + di, 1 + dj:n + 1 + dj].astype(np.int8) for di, dj in N8)
            upd = cnt > best_c
            best_c = np.where(upd, cnt, best_c)
            best_k = np.where(upd, tm.k, best_k)
        cand = (own > 0) & (best_c >= self.capture_n) & (best_k != own)
        ys, xs = np.nonzero(cand)
        if len(ys):
            r = self.rng.random(len(ys)) < self.capture_p
            for i, j in zip(ys[r], xs[r]):
                self.take(i, j, int(best_k[i, j]))

    def _start_fight(self):
        self.phase = 'fight'
        for _ in range(3 * self.n):
            if self._neighbour_fill() == 0:
                break

    def _eliminate(self, q):
        self.alive[q] = False
        self.outs.append((self.t, q))
        tm = self.teams[q]
        tm.on_out(self)
        ys, xs = np.nonzero(self.owner == tm.k)
        for i, j in zip(ys, xs):
            self._clear(i, j)

    # ------------------------------------------------------------- run
    def run(self, t_end, frame_times=None):
        """simulate to t_end; snapshot at frame_times (default every 1/fps). Returns a dict recording."""
        if frame_times is None:
            frame_times = np.arange(0, t_end - 1e-9, 1.0 / self.fps)
        for tm in self.teams:
            tm.setup(self)
        snaps_owner, snaps_parent, snaps_state, snaps_count, snaps_claim = [], [], [], [], []
        fi = 0
        nsteps = int(round(t_end / self.dt))
        for s in range(nsteps + 1):
            self.t = s * self.dt
            while fi < len(frame_times) and frame_times[fi] <= self.t + 1e-9:
                snaps_owner.append(self.owner.copy())
                snaps_parent.append(self.vine_parent.copy())
                snaps_claim.append(self.claim_t.astype(np.float16))
                snaps_count.append(self.count[1:].copy())
                st = []
                for tm in self.teams:
                    tm._t = self.t
                    st.append(tm.state())
                snaps_state.append(st)
                fi += 1
            if s == nsteps:
                break
            if self.phase == 'fill' and self.t >= self.t_fight:
                self._start_fight()
            if self.phase != 'fill' and self.heat_T:
                self.heat = 1.0 + (self.t - self.t_round()) / self.heat_T
            if self.phase != 'fill':
                self._update_power()
            elif self.close_T and self.t >= self.t_fight - self.close_T:
                u = (self.t - (self.t_fight - self.close_T)) / self.close_T
                self._neighbour_fill(self.close_p * u)
            order = self.rng.permutation(len(self.teams))
            for q in order:
                if self.alive[q] and self.phase != 'over':
                    self.teams[q].step(self, self.t, self.dt)
                elif self.phase == 'over' and q == self.winner:
                    self.teams[q].step(self, self.t, self.dt)       # the winner keeps playing (no one left to fight)
            if self.phase == 'fight':
                if self.absorb_p > 0:
                    self._neighbour_fill(self.absorb_p)
                self._capture()
                for q, tm in enumerate(self.teams):
                    if self.alive[q] and (self.count[tm.k] <= self.out_at or self.count[tm.k] < self.ko_share * self.arena.ntiles):
                        self._eliminate(q)
                left = [q for q in range(len(self.teams)) if self.alive[q]]
                if len(left) == 1:
                    self.phase = 'over'
                    self.winner = left[0]
                    self.t_win = self.t
            elif self.phase == 'over':
                # the win: flood every remaining tile from the winner's border outward
                n = int(self.flood_rate * self.dt) + 1
                k = self.teams[self.winner].k
                E = (self.owner != k) & (self.owner != OUT)
                if E.any():
                    own = self.owner == k
                    nb = np.zeros_like(own)
                    nb[1:] |= own[:-1]; nb[:-1] |= own[1:]; nb[:, 1:] |= own[:, :-1]; nb[:, :-1] |= own[:, 1:]
                    ys, xs = np.nonzero(E & nb)
                    sel = self.rng.permutation(len(ys))[:n]
                    for q in sel:
                        prev = int(self.owner[ys[q], xs[q]])
                        if prev == EMPTY:
                            self.take(ys[q], xs[q], k)
                        else:
                            self._clear(ys[q], xs[q]); self.take(ys[q], xs[q], k)
        rec = dict(
            owner=np.array(snaps_owner, np.int8), parent=np.array(snaps_parent, np.int8),
            claim_t=np.array(snaps_claim, np.float16),
            count=np.array(snaps_count, np.int32), state=snaps_state, frame_times=np.asarray(frame_times),
            ev_t=np.array(self.events_t), ev_k=np.array(self.events_k, np.int8), ev_i=np.array(self.events_i, np.int16),
            ev_j=np.array(self.events_j, np.int16), ev_prev=np.array(self.events_prev, np.int8),
            at_t=np.array(self.att_t), at_k=np.array(self.att_k, np.int8), at_i=np.array(self.att_i, np.int16),
            at_j=np.array(self.att_j, np.int16),
            outs=list(self.outs), winner=self.winner, t_win=self.t_win, t_fight=self.t_fight,
            ring_fields=self.ring_fields, names=[tm.name for tm in self.teams], kinds=[tm.kind for tm in self.teams],
            colors=[tm.color for tm in self.teams], ntiles=self.arena.ntiles, seed=self.seed)
        return rec


def summary(rec):
    """one-line story of a run: shares at the bell, eliminations, winner"""
    ft = rec['frame_times']
    names = rec['names']
    nt = rec['ntiles']
    fi = int(np.searchsorted(ft, rec['t_fight']))
    sh = rec['count'][min(fi, len(ft) - 1)] / nt
    s = 'bell ' + ' '.join(f'{n[:2]}{100 * x:3.0f}' for n, x in zip(names, sh))
    s += ' | outs ' + ' '.join(f'{names[q][:2]}@{t:5.1f}' for t, q in rec['outs'])
    if rec['winner'] is not None:
        s += f' | WIN {names[rec["winner"]]} @{rec["t_win"]:.1f}'
    return s
