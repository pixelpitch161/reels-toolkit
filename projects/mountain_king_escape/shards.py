"""Ring shards: when ring k breaks, each of its blocks splits into two crystal shards that blast outward (away
from the ball's exit point too), tumble, hit the cavern floor, hop once and lie flat. They pile up on the floor
as the video goes on (a carpet of every ring broken so far). In the loop tail every shard flies back up into
its slot in a new ring (innermost ring first), so frame 0 starts with 12 whole rings and a clean floor.
All closed-form (no stepping): pose(t) is evaluated per frame."""
import math
import numpy as np
import timeline as T
import geom as G

GRAV = 7.5
FLOOR_Y = 0.025
REF_DUR = 1.05


def _rotm(axis, ang):
    axis = axis / np.linalg.norm(axis)
    x, y, z = axis
    c, s, C = math.cos(ang), math.sin(ang), 1 - math.cos(ang)
    return np.array([[c + x * x * C, x * y * C - z * s, x * z * C + y * s],
                     [y * x * C + z * s, c + y * y * C, y * z * C - x * s],
                     [z * x * C - y * s, z * y * C + x * s, c + z * z * C]])


class Shard:
    __slots__ = ('k', 'tri', 'ctr', 'loc', 'v', 'axis', 'spin', 't0', 't1', 't2', 'v1', 'p1', 'p2', 'rest',
                 'seg', 'half', 'r0', 'r1', 'flat')


SHARDS = []
_r = np.random.default_rng(1907)
for k in range(1, T.NRINGS + 1):
    mb = T.RING_BREAK[k]
    ph = G.phase(k, mb)
    gpt = G.pol(G.R[k], ph)
    segs = G.ring_segments(k, ph)
    for si, (a0, a1) in enumerate(segs):
        f = G.seg_faces(k, a0, a1)[0][0]                      # front quad (fi0, fo0, fo1, fi1)
        for half, tri in enumerate((f[[0, 1, 2]], f[[0, 2, 3]])):
            s = Shard()
            s.k, s.seg, s.half = k, si, half
            s.ctr = tri.mean(0)
            s.loc = tri - s.ctr
            am = (a0 + a1) / 2
            radial = np.array([math.cos(am), math.sin(am), 0.0])
            away = s.ctr - gpt
            dist = np.linalg.norm(away)
            v = radial * _r.uniform(1.1, 3.0) + np.array([0, _r.uniform(0.0, 1.6), _r.uniform(-1.7, 1.7)])
            v += away / (dist + 1e-9) * 3.2 * math.exp(-dist / 0.9)
            s.v = v
            s.axis = _r.normal(0, 1, 3)
            s.spin = _r.uniform(5, 15) * _r.choice([-1, 1])
            s.t0 = mb
            # first flight -> floor
            y0, vy = s.ctr[1], v[1]
            th = (vy + math.sqrt(vy * vy + 2 * GRAV * (y0 - FLOOR_Y))) / GRAV
            p1 = s.ctr + v * th
            p1[1] = FLOOR_Y
            lim = np.array([6.9, 0, 3.9])
            for ax, lo, hi in ((0, -6.9, 6.9), (2, -5.5, 3.9)):
                if not lo <= p1[ax] <= hi:
                    tgt = min(hi, max(lo, p1[ax]))
                    v[ax] = (tgt - s.ctr[ax]) / th
                    p1[ax] = tgt
            s.t1 = mb + th
            s.p1 = p1
            vy1 = (vy - GRAV * th) * -0.28
            s.v1 = np.array([v[0] * 0.45, vy1, v[2] * 0.45])
            t2 = 2 * vy1 / GRAV
            s.t2 = s.t1 + t2
            s.p2 = p1 + s.v1 * t2
            s.p2[1] = FLOOR_Y
            for ax, lo, hi in ((0, -6.9, 6.9), (2, -5.5, 3.9)):
                s.p2[ax] = min(hi, max(lo, s.p2[ax]))
            # resting pose: the triangle laid flat (keep its in-plane shape, random yaw)
            yaw = _r.uniform(0, 2 * math.pi)
            e1 = s.loc[1] - s.loc[0]
            e2 = s.loc[2] - s.loc[0]
            nrm = np.cross(e1, e2)
            R = _rotm(np.cross(nrm, [0, 1.0, 0]) + 1e-9, math.acos(np.clip(nrm[1] / np.linalg.norm(nrm), -1, 1)))
            flat = s.loc @ R.T
            flat[:, 1] = 0
            Ry = _rotm(np.array([0, 1.0, 0]), yaw)
            s.flat = flat @ Ry.T
            s.rest = s.p2
            # reform window (innermost first)
            s.r0 = T.T_REFORM0 + (k - 1) * 0.06 + _r.uniform(0, 0.22)
            s.r1 = min(s.r0 + REF_DUR, T.T_REFORM1)
            SHARDS.append(s)

N = len(SHARDS)
RING_DONE = {k: max(s.r1 for s in SHARDS if s.k == k) for k in range(1, T.NRINGS + 1)}   # ring whole again


def flight_pose(s, t):
    """vertices (3, 3) of shard s at video time t (after its break, before its reform)"""
    u = t - s.t0
    if t < s.t1:
        p = s.ctr + s.v * u
        p[1] = s.ctr[1] + s.v[1] * u - 0.5 * GRAV * u * u
        return p + s.loc @ _rotm(s.axis, s.spin * u).T
    rot_end = s.loc @ _rotm(s.axis, s.spin * (s.t1 - s.t0)).T
    if t < s.t2:
        w = t - s.t1
        p = s.p1 + s.v1 * w
        p[1] = FLOOR_Y + s.v1[1] * w - 0.5 * GRAV * w * w
        ang = s.spin * 0.35 * w
        loc = rot_end @ _rotm(s.axis, ang).T
        f = min(1.0, w / max(1e-3, s.t2 - s.t1))
        loc = loc * (1 - f) + s.flat * f
        return p + loc
    return s.rest + s.flat + np.array([0, 0.004 * (s.seg % 3), 0])


def slot_pose(s, t):
    """vertices of shard s in its slot of the re-formed ring k at video time t"""
    ph = G.phase(s.k, G.ring_m(t))
    a0, a1 = G.ring_segments(s.k, ph)[s.seg]
    f = G.seg_faces(s.k, a0, a1)[0][0]
    return f[[0, 1, 2]] if s.half == 0 else f[[0, 2, 3]]


def pose(s, t):
    """None if the shard does not exist (ring still whole, or whole again)"""
    m = T.mtime(t)
    if t >= T.T_WRAP or t < s.t0:
        return None
    if t >= s.r1:
        return None
    if t < s.r0:
        return flight_pose(s, t)
    a = flight_pose(s, s.r0)
    b = slot_pose(s, t)
    u = (t - s.r0) / (s.r1 - s.r0)
    e = u * u * (3 - 2 * u)
    lift = np.array([0, 2.4 * math.sin(math.pi * u) * (1 - u * 0.5), 0])
    return a * (1 - e) + b * e + lift


def ring_whole(k, t):
    """is ring k drawn as an intact ring at video time t?"""
    if t >= T.T_WRAP:
        return True
    if t >= RING_DONE[k]:
        return True
    return t < T.RING_BREAK[k]


if __name__ == '__main__':
    print(N, 'shards')
    lands = [s.t2 - s.t0 for s in SHARDS]
    print(f'settle time {min(lands):.2f}..{max(lands):.2f} s')
    print('ring whole again at', {k: round(v, 2) for k, v in RING_DONE.items()})
    P = np.array([s.rest for s in SHARDS])
    print('rest x', P[:, 0].min().round(2), P[:, 0].max().round(2), 'z', P[:, 2].min().round(2), P[:, 2].max().round(2))
