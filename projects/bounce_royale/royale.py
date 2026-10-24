"""Bounce Royale engine: 2D billiards in a shrinking ring whose wall is 48 painted tiles.

Knows nothing about the picture or the music. A run is fully decided by its seed (launch angles) and PARAMS.

Rules (the same for every run):
- Four balls, same size, same speed, no gravity. They bounce off the wall like a mirror and off each other with
  real elastic collisions (equal masses swap the velocity component along the line of centres). After a collision
  each ball is set back to the shared speed (the fairness rule: every ball always flies at the same speed).
- The wall is 48 tiles, 12 per colour at the start (one quadrant each).
- Every wall bounce throws a splash of paint centred where the ball touched the wall. The splash is as wide as the
  colour's share of the wall (MORE WALL = BIGGER SPLASH: 12 tiles -> SPLASH_TILES wide). Every enemy tile whose
  centre the splash covers is repainted; your own tiles stay as they are.
  Why a share-sized splash: with one tile per bounce, bounces spread evenly round the ring, so each colour sits at
  12 +- 3 tiles for ever (the chance of reaching 0 is ~1e-6; 0 of 24 test runs popped anyone). With splash width
  proportional to share, every repaint is a fair coin for both colours involved (neutral drift), so a colour can
  really fall to 0 and lead swings and comebacks happen by themselves.
- The wall has a bevel: a grazing bounce leaves at least graze_min from the tangent (without it a ball can end
  up gliding along the wall, which in a simulation means thousands of "bounces" a second).
- A colour with 0 tiles pops: its ball shatters and leaves the ring. The last colour standing wins.
- From t_shrink0 the ring shrinks steadily (the wall moves, so bounces off it carry its speed): the balls cross
  it faster and faster, so the battle accelerates by itself.
Launch: the four balls wait in the centre and fly out one per beat (RUBY, JADE, COBALT, AMBER), each toward its
own quadrant (the seed sets the exact angle); a waiting ball isn't in play yet (nothing hits it).
Video time = run time: frame 0 is the snap-open and the first launch."""
import math
import numpy as np
from dataclasses import dataclass

FPS = 30
DUR = 61.0
NF = int(round(FPS * DUR))
NT = 48                                   # wall tiles
DTH = 2 * math.pi / NT
NAMES = ['RUBY', 'JADE', 'COBALT', 'AMBER']
INSTR = ['DRUMS', 'BASS', 'ARP', 'LEAD']


@dataclass
class Params:
    speed: float = 2.2                    # ring radii per second (ring radius at the start = 1)
    rb: float = 0.06                      # ball radius
    splash_tiles: float = 1.8             # splash width in tiles for a colour holding 12 tiles, full-size ring
    splash_exp: float = 1.0               # width ~ share ** splash_exp (1.0 = neutral drift)
    ring_exp: float = 0.0                 # width ~ (1 / ring radius) ** ring_exp
    t_shrink0: float = 16.0
    t_shrink1: float = 50.0
    r_end: float = 0.42
    t_battle_end: float = 57.0            # no battle after this (the outro)
    sub: int = 16                         # physics substeps per video frame
    launch_r: float = 0.16                # launch circle radius
    graze_min: float = 12.0               # deg: the wall's bevel turns a grazing bounce at least this far inward
    launch_gap: float = 60.0 / 118.03     # one ball launches per beat (RUBY, JADE, COBALT, AMBER): instruments enter in turn


PARAMS = Params()


def ring_radius(t, P=PARAMS):
    if t <= P.t_shrink0:
        return 1.0
    u = min(1.0, (t - P.t_shrink0) / (P.t_shrink1 - P.t_shrink0))
    return 1.0 - (1.0 - P.r_end) * u


def tile_of(a):
    """tile index of a wall angle a (atan2(y, x)); tiles run clockwise from 12 o'clock"""
    return int(((math.pi / 2 - a) % (2 * math.pi)) // DTH) % NT


def tile_centre(k):
    return math.pi / 2 - (k + 0.5) * DTH


def initial_owner():
    return np.repeat(np.arange(4), NT // 4).astype(np.int8)     # RUBY 12-3 o'clock, JADE 3-6, COBALT 6-9, AMBER 9-12


class Run:
    def __init__(self, seed, P=PARAMS, record=False):
        self.seed, self.P, self.record = seed, P, record
        rng = np.random.default_rng(seed)
        self.pos, self.vel = [], []
        for q in range(4):
            mid = tile_centre(q * 12 + 6)                         # middle of the colour's quadrant
            a0 = mid + rng.uniform(-0.5, 0.5)
            r0 = P.launch_r * rng.uniform(0.7, 1.0)
            self.pos.append([r0 * math.cos(a0), r0 * math.sin(a0)])
            d = mid + rng.uniform(-0.9, 0.9)                      # launched outward toward its own quadrant-ish
            self.vel.append([P.speed * math.cos(d), P.speed * math.sin(d)])
        self.owner = initial_owner()
        self.alive = [True] * 4
        self.count = [12] * 4
        self.t_launch = [q * P.launch_gap for q in range(4)]

    def splash_half(self, q, R):
        P = self.P
        w = P.splash_tiles * (self.count[q] / 12.0) ** P.splash_exp * (1.0 / R) ** P.ring_exp
        return 0.5 * w * DTH, w

    def run(self, stop_at_win=False):
        P = self.P
        dt = 1.0 / (FPS * P.sub)
        hits, colls, pops = [], [], []
        own_f = np.zeros((NF, NT), np.int8) if self.record else None
        pos_f = np.zeros((NF, 4, 2), np.float32) if self.record else None
        vel_f = np.zeros((NF, 4, 2), np.float32) if self.record else None
        cnt_f = np.zeros((NF, 4), np.int16)
        alive_f = np.zeros((NF, 4), bool)
        winner, t_win = None, None
        rb, V = P.rb, P.speed
        pos, vel = self.pos, self.vel
        for f in range(NF):
            t_f = f / FPS
            if self.record:
                own_f[f] = self.owner
                pos_f[f] = pos
                vel_f[f] = vel
            cnt_f[f] = self.count
            alive_f[f] = self.alive
            if t_f >= P.t_battle_end:
                if not self.record:
                    break
            for s in range(P.sub):
                t = t_f + s * dt
                R = ring_radius(t + dt, P)
                Rc = R - rb
                for q in range(4):
                    if not self.alive[q] or t < self.t_launch[q]:
                        continue
                    p, v = pos[q], vel[q]
                    nx, ny = p[0] + v[0] * dt, p[1] + v[1] * dt
                    if nx * nx + ny * ny <= Rc * Rc:
                        p[0], p[1] = nx, ny
                        continue
                    # wall contact inside this substep: solve |p + v tau| = Rc
                    pp = p[0] * p[0] + p[1] * p[1]
                    pv = p[0] * v[0] + p[1] * v[1]
                    if pp >= Rc * Rc:
                        tau = 0.0
                        k = Rc / math.sqrt(pp)
                        p[0] *= k
                        p[1] *= k
                    else:
                        disc = pv * pv - V * V * (pp - Rc * Rc)
                        tau = (-pv + math.sqrt(max(disc, 0.0))) / (V * V)
                        tau = min(max(tau, 0.0), dt)
                        p[0] += v[0] * tau
                        p[1] += v[1] * tau
                    cr = math.hypot(p[0], p[1])
                    n0, n1 = p[0] / cr, p[1] / cr
                    u = (ring_radius(t + dt, P) - ring_radius(t, P)) / dt          # wall speed (inward < 0)
                    vn = v[0] * n0 + v[1] * n1 - u
                    if vn > 0:
                        v[0] -= 2 * vn * n0
                        v[1] -= 2 * vn * n1
                        # bevel: keep the outgoing direction at least graze_min from the tangent, then the shared speed
                        m = math.hypot(v[0], v[1])
                        cn = -(v[0] * n0 + v[1] * n1) / m                     # cos of angle to the inward normal
                        smin = math.sin(math.radians(P.graze_min))
                        if cn < smin:
                            tx, ty = -n1, n0
                            if v[0] * tx + v[1] * ty < 0:
                                tx, ty = -tx, -ty
                            ct = math.sqrt(1 - smin * smin)
                            v[0], v[1] = (ct * tx - smin * n0) * m, (ct * ty - smin * n1) * m
                        m = math.hypot(v[0], v[1])
                        v[0] *= V / m
                        v[1] *= V / m
                        th = math.atan2(n1, n0)
                        th_ = t + tau
                        if th_ < P.t_battle_end and winner is None:
                            flipped, w = self._splash(q, th, R)
                        else:
                            flipped, w = [], 0.0
                        hits.append((th_, q, th, w, flipped))
                        if flipped and winner is None:
                            for q2 in range(4):
                                if self.alive[q2] and self.count[q2] == 0:
                                    self.alive[q2] = False
                                    pops.append((th_, q2, tuple(pos[q2])))
                            al = [q2 for q2 in range(4) if self.alive[q2]]
                            if len(al) == 1:
                                winner, t_win = al[0], th_
                                if stop_at_win and not self.record:
                                    return self._rec(hits, colls, pops, winner, t_win, cnt_f[:f + 1], alive_f[:f + 1], own_f, pos_f, vel_f)
                    rem = dt - tau
                    p[0] += v[0] * rem
                    p[1] += v[1] * rem
                    cr = math.hypot(p[0], p[1])
                    if cr > Rc:
                        p[0] *= Rc / cr
                        p[1] *= Rc / cr
                # ball-ball elastic collisions
                for i in range(4):
                    if not self.alive[i] or t < self.t_launch[i]:
                        continue
                    for j in range(i + 1, 4):
                        if not self.alive[j] or t < self.t_launch[j]:
                            continue
                        pi, pj = pos[i], pos[j]
                        dx, dy = pj[0] - pi[0], pj[1] - pi[1]
                        d2 = dx * dx + dy * dy
                        if d2 >= 4 * rb * rb:
                            continue
                        vi, vj = vel[i], vel[j]
                        if (vj[0] - vi[0]) * dx + (vj[1] - vi[1]) * dy >= 0:
                            continue
                        d = math.sqrt(d2) + 1e-12
                        n0, n1 = dx / d, dy / d
                        a = vi[0] * n0 + vi[1] * n1
                        b = vj[0] * n0 + vj[1] * n1
                        vi[0] += (b - a) * n0; vi[1] += (b - a) * n1
                        vj[0] += (a - b) * n0; vj[1] += (a - b) * n1
                        for vv, sg in ((vi, -1), (vj, 1)):
                            m = math.hypot(vv[0], vv[1])
                            if m < 1e-9:
                                vv[0], vv[1] = sg * n0 * V, sg * n1 * V
                            else:
                                vv[0] *= V / m
                                vv[1] *= V / m
                        ov = 2 * rb - d
                        pi[0] -= n0 * ov / 2; pi[1] -= n1 * ov / 2
                        pj[0] += n0 * ov / 2; pj[1] += n1 * ov / 2
                        colls.append((t, i, j, ((pi[0] + pj[0]) / 2, (pi[1] + pj[1]) / 2)))
        return self._rec(hits, colls, pops, winner, t_win, cnt_f, alive_f, own_f, pos_f, vel_f)

    def _splash(self, q, th, R):
        half, w = self.splash_half(q, R)
        flipped = []
        k0 = tile_of(th)
        span = int(half / DTH) + 2
        for dk in range(-span, span + 1):
            k = (k0 + dk) % NT
            c = tile_centre(k)
            dd = abs((th - c + math.pi) % (2 * math.pi) - math.pi)
            if dd <= half and self.owner[k] != q:
                prev = int(self.owner[k])
                self.owner[k] = q
                self.count[prev] -= 1
                self.count[q] += 1
                flipped.append((k, prev))
        return flipped, w

    def _rec(self, hits, colls, pops, winner, t_win, cnt_f, alive_f, own_f, pos_f, vel_f):
        return dict(seed=self.seed, t_launch=self.t_launch, hits=hits, colls=colls, pops=pops, winner=winner, t_win=t_win, count=cnt_f,
                    alive=alive_f, owner=own_f, pos=pos_f, vel=vel_f)
