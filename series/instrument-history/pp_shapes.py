"""Extra low-poly shapes for the Pixel Pitch videos: drums, cymbals, prisms, rods, drumsticks."""
import math
import numpy as np
import trumpet_lowpoly as T
from piano_lowpoly import mix, dim, clamp, lerp

METAL = (186, 186, 198)

def unit(v):
    v = np.asarray(v, float); return v/np.linalg.norm(v)

def ring(p, d, r, n, phase=0.):
    _, u, v = T._basis(np.asarray(d, float)); p = np.asarray(p, float)
    return [p + r*(math.cos(2*math.pi*k/n + phase)*u + math.sin(2*math.pi*k/n + phase)*v) for k in range(n)]

def drum(g, c, ax, r, depth, shell, head, hit=0., hit0=0., n=12, hoop=METAL, spec=.4, back=None):
    c = np.asarray(c, float); ax = unit(ax); p0, p1 = c - ax*depth/2, c + ax*depth/2
    T.tube(g, p0, p1, r, col=shell, n=n, spec=spec)
    g.face(ring(p1 + ax*.002, ax, r*.985, n), mix(head, (255, 250, 220), clamp(hit)*.85), ref=c, emit=hit > .15)
    g.face(ring(p0 - ax*.002, ax, r*.985, n), mix(back or dim(head, .82), (255, 250, 220), clamp(hit0)*.85), ref=c, emit=hit0 > .15)
    for p in (p0, p1): T.tube(g, p - ax*.012, p + ax*.012, r*1.035, col=hoop, n=n, spec=.8)
    return p0, p1

def ropes(g, p0, p1, ax, r, col, n=8):
    a, b = ring(p0, ax, r*1.02, n), ring(p1, ax, r*1.02, n, math.pi/n)
    for k in range(n):
        g.line(a[k], b[k], col); g.line(b[k], a[(k + 1) % n], col)

def cymbal(g, c, nrm, r, col, hit=0., age=9., n=14, wob=.3):
    c = np.asarray(c, float); nrm = unit(nrm); _, u, v = T._basis(nrm)
    w = wob*clamp(hit*1.5)*math.sin(age*26)
    n2 = unit(nrm*math.cos(w) + u*math.sin(w)); _, u2, v2 = T._basis(n2)
    angs = [2*math.pi*k/n for k in range(n)]
    inner = [c + n2*.02 + r*.24*(math.cos(a)*u2 + math.sin(a)*v2) for a in angs]
    outer = [c - n2*.006 + r*(math.cos(a)*u2 + math.sin(a)*v2) for a in angs]
    apex = c + n2*.032; base = mix(col, (255, 250, 220), clamp(hit)*.6)
    for k in range(n):
        k2 = (k + 1) % n
        g.face([inner[k], outer[k], outer[k2], inner[k2]], base if k % 2 else dim(base, .9), ref=c - n2, two=True, spec=.9, emit=hit > .35)
        g.face([apex, inner[k], inner[k2]], dim(base, .95), ref=c - n2, two=True, spec=.9)
    front = np.array([0, 0, 1.]) - n2*n2[2]
    return c + unit(front if np.linalg.norm(front) > 1e-3 else u2)*r*.6

def prism(g, c, nrm, r, h, n, top, side, emit=False, phase=0.):
    c = np.asarray(c, float); nrm = unit(nrm)
    tp, bt = ring(c + nrm*h/2, nrm, r, n, phase), ring(c - nrm*h/2, nrm, r, n, phase)
    g.face(tp, top, ref=c, emit=emit); g.face(bt, dim(side, .7), ref=c)
    for k in range(n):
        k2 = (k + 1) % n; g.face([bt[k], bt[k2], tp[k2], tp[k]], side, ref=c)
    return c + nrm*h/2

def rod(g, a, b, r=.009, col=(48, 46, 54)):
    T.tube(g, a, b, r, col=col, n=5, spec=.5)

def tripod(g, x, z, h, col=(150, 150, 162), spread=.15):
    for k in range(3):
        a = 2*math.pi*k/3 + .5
        for off in (0, .006): g.line((x + off, h, z), (x + spread*math.cos(a) + off, 0, z + spread*math.sin(a)), col)

def stick_pos(hits, t, rest=.13, move=.3):
    """hits: sorted [(time, point)]. returns (point, lift) of a stick tip at time t."""
    if not hits: return None, 0.
    for i, (ht, p) in enumerate(hits):
        if t < ht:
            p = np.asarray(p, float)
            if i == 0: return p, rest
            pt, pp = hits[i - 1]; pp = np.asarray(pp, float); span = ht - pt
            win = min(span, move); pr = clamp((t - (ht - win))/win)
            lift = rest*min(1., (t - pt)/.09, (ht - t)/.09)
            return pp*(1 - pr) + p*pr, lift
    pt, pp = hits[-1]
    return np.asarray(pp, float), min(rest, (t - pt)/.09*rest)

def draw_stick(g, tip, anchor, length=.5, col=(234, 208, 160), mallet=False):
    tip = np.asarray(tip, float); d = unit(np.asarray(anchor, float) - tip); grip = tip + d*length
    for off in ((0, 0, 0), (.007, 0, 0), (0, .007, 0), (.007, .007, 0)):
        g.line(tip + off, grip + off, col)
    s = .032 if mallet else .012
    g.box(tip[0] - s, tip[0] + s, tip[1] - s, tip[1] + s, tip[2] - s, tip[2] + s, (240, 232, 214) if mallet else (246, 226, 186))

def bell_dir(g, start, d, length, r0, r1, col, segs=6, spec=.9):
    """flared bell from start along direction d; returns mouth point"""
    p0 = np.asarray(start, float); d = unit(d); prev = (p0, r0)
    for k in range(1, segs + 1):
        s_ = k/segs; p = p0 + d*length*s_; r = r0 + (r1 - r0)*s_**3
        T.tube(g, prev[0], p, prev[1], r, col=col, two=True, spec=spec); prev = (p, r)
    end = p0 + d*length
    T.tube(g, end - d*.012, end + d*.004, r1*1.07, r1*1.07, col=col, two=True, spec=spec)
    _, u, v = T._basis(d); mid = p0 + d*length*.55
    g.face([mid + r1*.35*(math.cos(a)*u + math.sin(a)*v) for a in np.linspace(0, 2*math.pi, 11)[:-1]], (30, 20, 10), ref=p0, emit=True, two=True)
    return end
