"""TRUMPET (yellow), as built in #29 v3: VALVE COMBO (every 2 beats three brass pistons punch out of the ball 1-2-3 on
the 8ths, aimed at the enemy, out / hold / back, the third harder; a landed punch lengthens the reach), FANFARE
(shockwave: a ring of golden banners), DOUBLE TONGUE (haste: combos twice as fast), HIGH C (ability kind `beam`: the
bell charges for a beat on a dashed sight, then a giant golden beam out of the bell, heat shimmer along it).

Look: the side-on trumpet turns inside its ball so its valves face the punches (the pistons come out of them) and, for
HIGH C, swings its bell down the line; pistons with brass collars, a pearl button rimmed in its colour and a motion
streak, steam from the valve on every punch; scorch marks, brass sparks and a burst where a punch lands (bigger on the
third); FANFARE's light ring + banners + shimmer; the HIGH C lane while it charges, the blast with a white-hot core,
sound-wave ripples, a muzzle star, a streak of light and three scorch marks on the canvas; the bell mouth glows as it
charges / fires / solos; solo notes rise from the bell. It shatters into tubing, valve caps and brass chips.
Sound: every combo's three punches play three notes of ONE valve fingering (the open Bb trumpet's harmonic series
lowered by the valves, chosen to fit the chord: partials 4-6-8 or 3-4-6), a valve hiss on each; landed punches ring out,
misses are ghosts (dropping out when hurt); the leader adds the octave below; DOUBLE TONGUE adds ta-ka graces; FANFARE
is a rising call on the key's minor triad; HIGH C swells through its charge and screams with a shake when it fires.
With a config line (LINES['TRUMPET'], 8 notes a bar) the punches play that line instead: the trumpet takes the melody
(added for #46; without a line the part is #29's, unchanged).
Draw code from projects/ballfight_trumpet_vs_sax/styles.py + render.py (#29 v3), copied unchanged; sound from #29's
audio.py (the Pixel Pitch trumpet). #29's chord table (Cm Ab7 Fm G7) is what the general fingering rule gives, so any
episode's chords pick their fingering the same way. Checked against #29: examples/trumpet_vs_sax (README)."""
import math
import os

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, FPS, BAYER_FULL, ss
from reelkit import fx
import scene as S

from . import base

R = S.R_BALL
BRASS = np.array([232, 184, 72])
BRASS_HI = np.array([255, 236, 160])
BRASS_DK = np.array([150, 104, 34])
PEARL = np.array([238, 236, 228])


# ---------------------------------------------------------------- #29 styles.py helpers (unchanged)
def mask_of(polys, width=None):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if width:
            d.line([tuple(q) for q in pts], fill=255, width=width)
        elif len(pts) >= 3:
            d.polygon([tuple(q) for q in pts], fill=255)
    return np.asarray(m, np.float32) / 255.0


def blend(arr, m, col, a):
    arr += (np.asarray(col, float)[None, None] - arr) * (m * a)[..., None]


# ---------------------------------------------------------------- HIGH C: the charge sight + the blast (#29 styles.py)
def draw_sight(arr, cam, p, a, length, k, t, col):
    """the locked aim while the bell charges: a dashed line marching outward, brighter as the charge fills"""
    d = np.array([math.cos(a), math.sin(a)])
    o = p + d * (R + 0.08)
    segs = []
    n = max(2, int(length / 0.22))
    off = (t * 3.0) % 1.0
    for j in range(n):
        u0 = (j + off) / n
        u1 = (j + off + 0.45) / n
        if u1 > 1:
            continue
        a0 = o + d * length * u0
        a1 = o + d * length * u1
        segs.append(cam.wp(np.array([S.to_world(a0[0], a0[1], R), S.to_world(a1[0], a1[1], R)])))
    m = mask_of(segs, width=1)
    blend(arr, m, np.clip(np.asarray(col) * 0.7 + 90, 0, 255), 0.25 + 0.6 * k)


BELL_IN = 0.76                   # the trumpet sprite's bell mouth sits this far (x R) from its ball's centre, along the aim
FADE = 0.22


def beam_state(t, t_fire, hold=0.12):
    """(extension 0..1, alpha, tail 0..1, age) of a blast fired at t_fire: it shoots out of the bell in 50 ms, holds,
    then fades as its back end lets go of the bell and races after the tip"""
    u = t - t_fire
    if u < 0 or u > hold + FADE:
        return None
    ext = 1 - (1 - min(1.0, u / 0.05)) ** 2
    if u <= hold:
        return ext, 1.0, 0.0, u
    k = (u - hold) / FADE
    return ext, 1 - k, k ** 1.4, u


def _beam_outline(p, a, s0, s1, width, y, n=18, flare_len=0.32):
    """a blast's outline in the ring frame: narrow at the bell mouth (s = 0), opening to full width over `flare_len`,
    a rounded tip; s0..s1 along the aim from the ball's centre + BELL_IN R"""
    d = np.array([math.cos(a), math.sin(a)])
    nrm = np.array([-d[1], d[0]])
    o = p + d * BELL_IN * R
    if s1 - s0 < 1e-3:
        return None
    ss_ = np.linspace(s0, s1, n)
    k = np.clip(ss_ / flare_len, 0, 1)
    w = width / 2 * (0.32 + 0.68 * (k * k * (3 - 2 * k)))
    left = [o + d * sv + nrm * wv for sv, wv in zip(ss_, w)]
    right = [o + d * sv - nrm * wv for sv, wv in zip(ss_, w)]
    tip = [o + d * s1 + w[-1] * (math.cos(q) * d + math.sin(q) * nrm) for q in np.linspace(math.pi / 2, -math.pi / 2, 7)]
    pts = left + tip[1:-1] + right[::-1]
    return np.array([S.to_world(q[0], q[1], y) for q in pts])


def draw_beam(arr, cam, p, a, length, width, st, col, t, y=R):
    """the blast as light out of the trumpet's bell: a soft wide halo, the body, a white-hot core (all with blurred,
    not hard, edges), sound-wave ripples racing along it while it holds"""
    from scipy.ndimage import gaussian_filter
    ext, alpha, tail, u = st
    lv = length + R + 0.04 - BELL_IN * R          # the engine's beam starts just outside the glass; the light, at the bell
    s1 = lv * ext
    s0 = (s1 - 0.05) * tail
    col = np.asarray(col, float)
    thin = 1 - 0.55 * tail
    for wk, c, ak, sig in ((3.2, col * 0.7 + 60, 0.42, 2.4), (1.25, col * 0.85 + 45, 0.9, 0.9),
                           (0.42, np.array([255, 252, 236]), 1.0, 0.5)):
        ww = width * wk if wk < 3 else width + min(width * (wk - 1), 0.5)     # a wide blast keeps a narrow halo
        if wk > 3 and width > 0.6:
            ak *= 0.6
        P = _beam_outline(p, a, s0, s1, max(0.03, ww * thin), y)
        if P is None:
            return
        m = gaussian_filter(mask_of([cam.wp(P)]), sig)
        blend(arr, np.clip(m * 1.15, 0, 1), np.clip(c, 0, 255), ak * alpha)
    if tail == 0 and s1 > 0.4:                    # ripples: little wavefronts bowed forward, moving out at 8 units/s
        d = np.array([math.cos(a), math.sin(a)])
        nrm = np.array([-d[1], d[0]])
        o = p + d * BELL_IN * R
        lines = []
        for j in range(6):
            sj = (0.35 + j * 0.62 + 8.0 * u) % max(0.5, s1 - 0.25)
            if sj < 0.3:
                continue
            hw = width * 1.05
            arc = [o + d * (sj - 0.06 * (1 - q * q)) + nrm * hw * q for q in np.linspace(-1, 1, 7)]
            lines.append(cam.wp(np.array([S.to_world(v[0], v[1], y) for v in arc])))
        if lines:
            blend(arr, mask_of(lines, width=1), np.clip(col * 0.4 + 165, 0, 255), 0.55 * alpha)


def draw_flash(arr, cam, p, a, k, col, y=R):
    """the muzzle flash at the bell mouth as a blast leaves: a short four-point star"""
    if k <= 0.02:
        return
    d = np.array([math.cos(a), math.sin(a)])
    o = p + d * BELL_IN * R
    s = cam.wp(S.to_world(o[0], o[1], y)[None])[0]
    L = 5 + 7 * k
    lines = [[(s[0] - L, s[1]), (s[0] + L, s[1])], [(s[0], s[1] - L * 0.8), (s[0], s[1] + L * 0.8)]]
    blend(arr, mask_of(lines, width=1), np.array([255, 250, 230]), 0.85 * k)
    blend(arr, mask_of([[(s[0] + 2.2 * math.cos(q), s[1] + 2.2 * math.sin(q)) for q in np.linspace(0, 6.28, 9)[:-1]]]),
          np.clip(np.asarray(col) * 0.3 + 190, 0, 255), k)


def bell_mouth(p, a):
    """the trumpet's bell mouth in the ring frame (inside the glass, along the aim)"""
    return p + np.array([math.cos(a), math.sin(a)]) * BELL_IN * R


def draw_note_rise(arr, cam, x, z, y, col, a):
    """a solo note floating up from the champion's horn (an eighth note, fading)"""
    if a <= 0:
        return
    s = cam.wp(S.to_world(x, z, y)[None])[0]
    k = cam.f / cam.depth(S.to_world(x, z, y)[None])[0]
    hr = 0.07 * k
    head = [(s[0] + hr * 1.25 * math.cos(q) - hr * 0.3 * math.sin(q), s[1] + hr * 0.85 * math.sin(q))
            for q in np.linspace(0, 2 * math.pi, 11)[:-1]]
    c = np.clip(np.asarray(col) * 0.5 + 128, 0, 255)
    blend(arr, mask_of([head]), c, a)
    sx, sy = s[0] + hr * 1.1, s[1]
    top = sy - 0.28 * k
    blend(arr, mask_of([[(sx, sy), (sx, top)], [(sx, top), (sx + 0.09 * k, top + 0.09 * k)]], width=1), c, a)


# ---------------------------------------------------------------- VALVE COMBO: three pistons punching out of the ball
PISTON = np.array([212, 218, 230])          # nickel-silver piston
COLLAR = np.array([214, 168, 62])           # brass valve-casing collar


def piston_ext(u, w, reach):
    """a piston's length beyond the ball's surface at age u (s, already divided by the haste time scale): out
    (ease-out), hold, back (smoothstep); None once it's home"""
    t_out, t_hold, t_back = w['t_out'], w['t_hold'], w['t_back']
    if u < 0:
        return None
    if u < t_out:
        e = u / t_out
        return reach * (1 - (1 - e) ** 2)
    if u < t_out + t_hold:
        return reach
    if u < t_out + t_hold + t_back:
        e = (u - t_out - t_hold) / t_back
        return reach * (1 - e * e * (3 - 2 * e))
    return None


def piston_geom(p, th, off, ext):
    """(base, tip) in the ring frame: the piston leaves the glass at its valve's lateral offset"""
    d = np.array([math.cos(th), math.sin(th)])
    a0 = p + d * R * 0.8 + np.array([-d[1], d[0]]) * off
    return a0, a0 + d * (R * 0.2 + ext), d


def draw_piston(arr, cam, p, th, off, ext, stretch, col, y=R):
    """a brass collar at the glass, the silver piston rod, a pearl finger button on the end (its brass rim in the
    fighter's colour); `stretch` 0..1 adds a motion streak while it shoots out"""
    a0, tip, d = piston_geom(p, th, off, ext)
    n = np.array([-d[1], d[0]])
    W_ = lambda Q: cam.wp(np.array([S.to_world(q[0], q[1], y) for q in Q]))
    if stretch > 0.05:
        streak = [a0 - n * 0.12, tip - n * 0.15, tip + n * 0.15, a0 + n * 0.12]
        blend(arr, mask_of([W_(streak)]), np.clip(np.asarray(col) * 0.5 + 120, 0, 255), 0.35 * stretch)
    rod = [a0 - n * 0.05, tip - n * 0.05, tip + n * 0.05, a0 + n * 0.05]
    blend(arr, mask_of([W_(rod)]), PISTON * 0.82, 1.0)
    blend(arr, mask_of([W_([a0 + n * 0.018, tip + n * 0.018])], width=1), np.array([250, 252, 255]), 0.9)
    for c, rr, colr in ((a0, 0.095, COLLAR), (tip, 0.16, np.clip(np.asarray(col) * 0.8 + 30, 0, 255)),
                        (tip + d * 0.015, 0.125, PEARL)):
        P = np.array([S.to_world(c[0] + rr * math.cos(q), c[1] + rr * math.sin(q), y + 0.01) for q in np.linspace(0, 6.28, 13)[:-1]])
        blend(arr, mask_of([cam.wp(P)]), colr, 1.0)


# ---------------------------------------------------------------- FANFARE: a ring of golden banners snapping outward
BANNER = np.array([255, 214, 72])
BANNER_TRIM = np.array([196, 40, 52])


def draw_banners(arr, cam, c, r, k, t, n=10, y0=0.04, h=0.7):
    """n banners on the expanding ring: a pole, a pennant streaming outward that snaps (flutters) as it goes"""
    if k <= 0.02:
        return
    poles, flags, trims = [], [], []
    for j in range(n):
        a = 2 * math.pi * j / n + 0.3
        d = np.array([math.cos(a), math.sin(a)])
        b = c + d * r
        top = S.to_world(b[0], b[1], y0 + h)
        poles.append(cam.wp(np.array([S.to_world(b[0], b[1], y0), top])))
        fl = 0.06 * math.sin(t * 34 + j * 1.7)
        tipp = b + d * 0.34
        Q = [top, S.to_world(tipp[0], tipp[1], y0 + h - 0.08 + fl), S.to_world(b[0], b[1], y0 + h - 0.2)]
        flags.append(cam.wp(np.array(Q)))
        trims.append(cam.wp(np.array([top, S.to_world(tipp[0], tipp[1], y0 + h - 0.08 + fl)])))
    blend(arr, mask_of(poles, width=1), np.array([240, 230, 200]), k)
    blend(arr, mask_of(flags), BANNER, k)
    blend(arr, mask_of(trims, width=1), BANNER_TRIM, k)


# ---------------------------------------------------------------- the VS card panel (new: #29 predates the card)
def trumpet_valves(base_):
    """the VS card panel: the bell's flare seen mouth-on (rings round the panel's far corner, a bright rim) and three
    piston valves along the divider (caps, pearl buttons, stems): the trumpet's three valves, not the euphonium's four"""
    import title as TT
    out = base_ * 1.0
    b = np.where(TT.SIDE > 0, 0, 1)                                   # the bell sits in the panel's far corner
    cx = np.where(b == 0, -40.0, W + 40.0)
    cy = np.where(b == 0, -40.0, H + 40.0)
    r = np.hypot(TT.XX - cx, TT.YY - cy)
    q = (r / 22.0) % 1.0
    out = out * np.where((q < 0.18)[..., None], 1.10, np.where((q > 0.86)[..., None], 0.88, 1.0))
    rim = np.abs(r - 250.0) < 3.0
    out = np.where(rim[..., None], out * 0.5 + np.array([255, 236, 160]) * 0.5, out)
    along = TT.XX * TT.DD[0] + TT.YY * TT.DD[1]
    u = np.abs(TT.SIDE)
    a_mid = float(np.mean([TT.DA @ TT.DD, TT.DB @ TT.DD]))
    for j in range(3):
        a0 = a_mid - 52.0 + 52.0 * j
        rr = np.hypot(along - a0, u - 58.0)
        cap = rr < 15.0
        ring = np.abs(rr - 15.0) < 1.8
        btn = rr < 7.0
        stem = (np.abs(along - a0) < 3.0) & (u > 73.0) & (u < 118.0)
        out = np.where(stem[..., None], out * 0.55 + np.array([212, 218, 230]) * 0.45, out)
        out = np.where(cap[..., None], out * 0.7 + np.array([255, 214, 120]) * 0.3, out)
        out = np.where(ring[..., None], out * 0.45 + np.array([255, 236, 170]) * 0.55, out)
        out = np.where(btn[..., None], out * 0.4 + np.array([250, 244, 226]) * 0.6, out)
    return out


# ---------------------------------------------------------------- the rolling sprite (#29: the trumpet turns to its aim)
ROLL_STEP = 10
SPR_RADII = list(range(12, 20))                 # #29: the sprites span 13.3-18.9 px (12 for the regrowing loser)
_ATLAS = {}


def _sprite_job(job):
    import sprites as SP
    nm, r, yk, rl = job
    return (r, rl), SP.render_sprite(nm, r, float(yk), float(rl))


def _wdir(d):
    return tuple(S.to_world(d[0], d[1], 0.0))


# ===================================================================================================== look (#29 v3)
class Look(base.Look):
    shards = ['tube'] * 4 + ['valve'] * 3 + ['brass'] * 6
    spark_big = ('slam', 'wave', 'shockwave', 'beam')
    spark_flash = ('beam',)
    fringe_hits = ('slam', 'wave', 'shockwave', 'beam')
    title_pattern = staticmethod(trumpet_valves)

    def setup(self):
        P, k = self.P, self.k
        EV, RUN, T = P.EV, P.RUN, P.T
        # beams: charges (t0, owner, aim, t_fire, length, width) and blasts (t_fire, owner, ox, oz, aim, length, width)
        self.CHARGES = [(e[0], e[2], e[3], e[0] + e[4], e[5], e[6]) for e in EV if e[1] == 'charge' and e[2] == k]
        self.BEAMS = [(e[0], e[2], e[3], e[4], e[5], e[6], e[7]) for e in EV if e[1] == 'beam' and e[2] == k]
        self.BEAM_T = np.array([b_[0] for b_ in self.BEAMS]) if self.BEAMS else np.zeros(0)
        self.HOLD = self.cfg['abilities']['low'].get('hold', 0.12)    # HIGH C (an ability beam) holds longer
        # VALVE COMBO: every punch (t0, owner, index, aim, reach, combo no., haste time scale)
        mult = self.cfg['abilities']['unlock2'].get('mult', 1.0)
        HA = RUN['HA']
        self.PUNCHES = [(e[0], e[2], e[3], e[4], e[5], e[6],
                         1.0 / (mult if float(HA[min(len(HA) - 1, max(0, int(round((e[0] - T.T_FIGHT) * FPS)))), e[2]]) > 0
                                else 1.0))
                        for e in EV if e[1] == 'punch' and e[2] == k]
        self.PUNCH_T = np.array([q[0] for q in self.PUNCHES]) if self.PUNCHES else np.zeros(0)
        self.W = self.cfg['weapon']

    # ---------------------------------------------------------------- the run's moves at t (#29 render.py)
    def punches_live(self, t):
        """pistons out at t: [(owner, index, aim, off, ext, stretch)] (they freeze with the hit-stop)"""
        P, T = self.P, self.T
        if not len(self.PUNCH_T) or t < T.T_FIGHT:
            return []
        tt = min(t, P.T_KO)
        j0, j1 = np.searchsorted(self.PUNCH_T, tt - 0.6), np.searchsorted(self.PUNCH_T, tt, 'right')
        out = []
        for j in range(j0, j1):
            t0, k, i, a, reach, cn, ks = self.PUNCHES[j]
            w = self.W
            u = (tt - t0) / ks
            ext = piston_ext(u, w, reach)
            if ext is not None:
                stretch = max(0.0, 1 - u / w['t_out']) if u < w['t_out'] else 0.0
                off = (i - (w['punches'] - 1) / 2) * w['spread']
                out.append((k, i, a, off, ext, stretch))
        if P.T_KO <= t < P.T_HS + 0.2 and t >= P.T_HS:                    # ... then they pull home after the hit-stop
            out = [(k, i, a, off, ext * max(0.0, 1 - (t - P.T_HS) / 0.2), 0.0) for k, i, a, off, ext, stretch in out]
        return out

    def hc_mode(self, t):
        """0..1: how far the trumpet has turned from 'valves at the enemy' to 'bell down the HIGH C line' (eased in as
        the charge starts, out after the blast)"""
        for t0, who, aim, t_fire, ln, wd in self.CHARGES:
            t_end = t_fire + self.HOLD + FADE
            if t0 - 0.02 <= t < t_end + 0.2:
                return ss(t0 - 0.02, t0 + 0.13, t) * (1 - ss(t_end, t_end + 0.2, t))
        return 0.0

    def hc_aim(self, t):
        """the HIGH C line's aim (the latest charge at or before t)"""
        a = None
        for t0, who, aim, t_fire, ln, wd in self.CHARGES:
            if t0 - 0.02 <= t:
                a = aim
        return a

    def charge_at(self, t):
        """the open charge at t: (t0, aim, t_fire, length) or None"""
        for t0, who, aim, t_fire, ln, wd in self.CHARGES:
            if t0 <= t < t_fire and t < self.P.T_KO:
                return t0, aim, t_fire, ln
        return None

    def beams_live(self, t):
        """blasts visible at t (they freeze with the hit-stop, then fade)"""
        P = self.P
        if not len(self.BEAM_T):
            return []
        j0, j1 = np.searchsorted(self.BEAM_T, t - 0.4), np.searchsorted(self.BEAM_T, min(t, P.T_KO), 'right')
        out = []
        for j in range(j0, j1):
            tf, k, ox, oz, a, ln, wd = self.BEAMS[j]
            tt = min(t, P.T_KO) if t < P.T_HS else t - P.HITSTOP
            st = beam_state(tt, tf, self.HOLD)
            if st is not None:
                out.append((k, ox, oz, a, self.clip_len(ox, oz, a, ln), wd, st[0], st[1], st))
        return out

    @staticmethod
    def clip_len(ox, oz, a, ln, lim=3.25):
        """the visible blast ends at the ropes (it stays inside the ring, and inside the lens's sharp band)"""
        d = (math.cos(a), math.sin(a))
        s_max = ln
        for o, dv in ((ox, d[0]), (oz, d[1])):
            if abs(dv) > 1e-9:
                s_max = min(s_max, ((lim if dv > 0 else -lim) - o) / dv)
        return max(0.2, s_max)

    @staticmethod
    def fire_centre(ox, oz, a):
        """the trumpet ball's centre when it fired (the engine's beam origin sits R + 0.04 out along the aim)"""
        return np.array([ox, oz]) - (R + 0.04) * np.array([math.cos(a), math.sin(a)])

    def solo_notes(self, t):
        """the winner's solo: one floating note per solo note, from the bell"""
        P = self.P
        out = []
        for st_, (bt, m, d) in zip(P.SOLO_T, self.T.SOLO.get(self.name, [])):
            u = t - st_
            if 0 <= u < 1.4:
                out.append((u, m, d))
        return out

    def now(self, F):
        """this frame's blasts, sights and pistons (cached on the frame context)"""
        key = ('trumpet_now', self.k)
        if key not in F.__dict__:
            P, T, t, k = self.P, self.T, F.t, self.k
            sights = []
            ch = self.charge_at(min(t, P.T_KO)) if T.T_FIGHT <= t < P.T_KO else None
            if ch is not None:
                sights.append((k, ch[1], ch[3], F.fs[k]['charge']))
            F.__dict__[key] = (self.beams_live(t), sights, self.punches_live(t))
        return F.__dict__[key]

    # ---------------------------------------------------------------- the ball
    def state(self, kk, st, t, phase, gl=0.0):
        if kk != self.k:
            return
        P = self.P
        st['charge'], st['hc'] = 0.0, 0.0
        if phase == 'fight':                                    # HIGH C: the bell glows as it charges, flares as it
            tt = min(t, P.T_KO)                                 # fires; every punch thumps the ball
            st['hc'] = self.hc_mode(tt)
            ch = self.charge_at(tt)
            if ch is not None:
                st['charge'] = (tt - ch[0]) / max(1e-3, ch[2] - ch[0])
            j = np.searchsorted(self.BEAM_T, tt, 'right') - 1
            fire = math.exp(-(tt - self.BEAM_T[j]) / 0.18) if j >= 0 else 0.0
            jp = np.searchsorted(self.PUNCH_T, tt, 'right') - 1
            thump = math.exp(-(tt - self.PUNCH_T[jp]) / 0.08) if jp >= 0 else 0.0
            st['glow'] = max(0.5 * st['charge'] ** 2, 0.8 * fire, 0.3 * thump)
            st['charge'] = max(st['charge'] ** 1.5, fire)
        elif phase == 'tail' and kk == P.WIN:                   # the bell glows on every solo note
            st['charge'] = P.solo_env(t) * (1 - gl)

    def screen_angle(self, cam, p, th):
        s0 = cam.wp(S.to_world(p[0], p[1], R)[None])[0]
        q = p + np.array([math.cos(th), math.sin(th)])
        s1 = cam.wp(S.to_world(q[0], q[1], R)[None])[0]
        return math.degrees(math.atan2(s1[1] - s0[1], s1[0] - s0[0]))

    def trumpet_alpha(self, cam, st, t):
        """the side-on trumpet's screen rotation (deg, y down): valves up (-90 deg) turned to the punch aim, blended
        toward 'bell (0 deg) down the HIGH C line' by the HIGH C mode"""
        a_valves = self.screen_angle(cam, st['p'], st['th']) + 90.0
        hc = st.get('hc', 0.0)
        if hc <= 0:
            return a_valves
        a_bell = self.screen_angle(cam, st['p'], self.hc_aim(t))
        return a_valves + ((a_bell - a_valves + 180) % 360 - 180) * hc

    def ball(self, kk, st, t, glass, roll):
        """v3: the trumpet turns its valves toward the punches (the pistons come out of them); for HIGH C it swings its
        bell down the line (any roll an enemy's debuff adds rides on top)"""
        if kk != self.k:
            return glass, roll
        extra = roll - self.cfg['view'][2]
        rl = int(round((-self.trumpet_alpha(self.P.camera(t), st, t) + extra) / ROLL_STEP)) * ROLL_STEP
        return glass, (rl + 180) % 360 - 180

    def atlas(self):
        """every sprite the trumpet can use: side-on at its view's yaw, radii 12-19, every roll (#29's cache); built
        once into the episode's check/ and shared by the render workers"""
        if not _ATLAS:
            path = os.path.join(self.P.EP.check, 'sprites_trumpet_aim.npz')
            if os.path.exists(path):
                z = np.load(path)
                _ATLAS.update({tuple(int(v) for v in key.split('_')): z[key] for key in z.files})
            else:
                yk = int(round(self.cfg['view'][0] / 6.0)) * 6 % 360
                jobs = [(self.cfg['pp'], r, yk, rl) for r in SPR_RADII for rl in range(-180, 180, ROLL_STEP)]
                import multiprocessing as mp
                if mp.current_process().daemon:                      # inside a render worker: no pool of its own
                    _ATLAS.update(dict(map(_sprite_job, jobs)))
                else:
                    with mp.get_context('fork').Pool(2) as pool:
                        _ATLAS.update(dict(pool.map(_sprite_job, jobs, chunksize=8)))
                np.savez_compressed(path, **{f'{r}_{rl}': v for (r, rl), v in _ATLAS.items()})
                print(len(_ATLAS), 'trumpet sprites ->', path)
        return _ATLAS

    def sprite(self, r, yaw, roll):
        """its own in-ball sprite: always side-on at its view's yaw (#29: no sway), at the roll ball() set"""
        r = int(np.clip(r, SPR_RADII[0], SPR_RADII[-1]))
        key = (r, int(roll))
        A = self.atlas()
        if key not in A:
            A[key] = _sprite_job((self.cfg['pp'], r, int(round(self.cfg['view'][0] / 6.0)) * 6 % 360, int(roll)))[1]
        return A[key]

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit':
            how = e[7]
            if how == 'piston':
                return 0.9 if e[8] == self.W['punches'] - 1 else None
            if how in ('beam', 'slam'):
                return 2.5
            if how in ('shockwave', 'wave'):
                return 1.5
            return None
        if e[1] == 'beam':
            return 1.2
        return super().shake(e)

    def spark(self, e):
        P = self.P
        how = e[7]
        n, sp, life = (16, 3.2, 0.3) if how in self.spark_big else ((9, 2.6, 0.22) if how == 'piston' else (5, 1.8, 0.18))
        return P.COL[e[2]] * 0.6 + 100, n, sp, life

    def hit_glow(self, e):
        if e[7] == 'beam':
            return 8, 0.22
        if e[7] in ('slam', 'wave', 'shockwave'):
            return 14, 0.35
        return None

    # ---------------------------------------------------------------- effects (#29 _fx_build)
    def fx_setup(self):
        """VALVE COMBO: steam bursts from the valve as each piston fires"""
        P, w = self.P, self.W
        for t0, k, i, a, reach, cn, ks_ in self.PUNCHES:
            p = P.RUN['P'][P.sim_i(t0), k]
            a0, tip, d = piston_geom(p, a, (i - (w['punches'] - 1) / 2) * w['spread'], 0.0)
            P.PA.emit(t0, S.to_world(a0[0], a0[1], R * 1.15), 'steam', n=3, dir=(0.0, 1.0, 0.0), vel=_wdir(d * 0.6))

    def fx_event(self, e):
        P, w = self.P, self.W
        if e[1] == 'hit':
            att, x, z, how, sub = e[2], e[5], e[6], e[7], e[8]
            pos = S.to_world(x, z, R)
            if how == 'piston':
                big = sub == w['punches'] - 1
                P.BU.add(e[0], pos, P.COL[att], 1.2 if big else 0.7, bright=big)
                P.FL.add(e[0], pos, P.FL_COL[att], radius=1.4, peak=0.85 if big else 0.55, decay=0.2)
                P.PA.emit(e[0], pos, 'brass', n=8 if big else 4)
                P.PA.emit(e[0], pos, 'steam', n=2)
                P.MK.add(e[0], 'scorch', x, z, size=0.22 if big else 0.14, col=(150, 110, 30), life=2.5,
                         seed=int(e[0] * 100), strength=0.6)
            elif how == 'beam':                                          # HIGH C lands
                P.BU.add(e[0], pos, P.COL[att], 2.0)
                P.FL.add(e[0], pos, P.FL_COL[att], radius=2.4, peak=1.2, decay=0.4)
                P.PA.emit(e[0], pos, 'brass', n=18, speed=(2.0, 4.5))
            elif how == 'slam':
                P.BU.add(e[0], pos, P.COL[att], 2.0)
            elif how in ('shockwave', 'wave'):
                P.BU.add(e[0], pos, P.COL[att], 1.4)
        elif e[1] == 'beam':                                            # HIGH C fires: the bell flares, the line scorches
            tf, k, ox, oz, a, ln, wd = e[0], e[2], e[3], e[4], e[5], e[6], e[7]
            d = np.array([math.cos(a), math.sin(a)])
            for u in (0.15, 0.45, 0.75):
                q = np.array([ox, oz]) + d * ln * u
                P.FL.add(tf, S.to_world(q[0], q[1], R), P.FL_COL[k], radius=2.2, peak=0.7, decay=0.35, hold=0.15)
                P.MK.add(tf, 'scorch', q[0], q[1], size=0.5 * wd + 0.2, angle=a, col=(170, 120, 30), life=6.0,
                         seed=int(tf * 10 + u * 7), strength=0.8)
            P.PA.emit(tf, S.to_world(ox, oz, R), 'brass', n=14, dir=_wdir(d), spread=40, speed=(2.0, 4.0))
        elif e[1] == 'ability' and e[4] == 'shockwave':                 # FANFARE: the ball flares gold
            p = P.RUN['P'][P.sim_i(e[0]), e[2]]
            P.FL.add(e[0], S.to_world(p[0], p[1], 1.0), P.FL_COL[e[2]], radius=2.6, peak=0.9, decay=0.45)

    def ko_fx(self, pos):
        P = self.P
        P.PA.emit(P.T_KO, pos, 'brass', n=26, spread=180, speed=(1.0, 3.0))
        P.PA.emit(P.T_KO, pos, 'steam', n=14, col=None, spread=180, speed=(0.4, 1.2))

    def fx_end(self):
        """the champion's solo: a puff of steam from the horn on every note (effects clock)"""
        P = self.P
        if self.k != P.WIN:
            return
        for t0 in P.SOLO_T:
            if t0 < self.T.T_REASSEMBLE:
                c = P.fighter(P.WIN, t0)['p']
                P.PA.emit(t0 - P.HITSTOP, S.to_world(c[0], c[1], R * 1.6), 'steam', n=2, col=None)
                P.FL.add(t0 - P.HITSTOP, S.to_world(c[0], c[1], 1.0), P.FL_COL[P.WIN], radius=1.6, peak=0.5, decay=0.2)

    # ---------------------------------------------------------------- the canvas
    def canvas(self, Ic, C):
        """each blast throws a streak of light on the canvas; the charging bell lights a faint lane"""
        P = self.P
        blasts, sights, _ = self.now(C)
        sxp, szp, ys, xs = C.sxp, C.szp, C.ys, C.xs
        for k, ox, oz, a_, ln, wd, ext, al, bst in blasts:
            d = np.array([math.cos(a_), math.sin(a_)])
            bm = bell_mouth(self.fire_centre(ox, oz, a_), a_)
            lv = (ln + R + 0.04 - BELL_IN * R) * ext
            rx, rz = sxp - bm[0], szp - bm[1]
            along = np.clip(rx * d[0] + rz * d[1], lv * bst[2], lv)
            off = np.hypot(rx - along * d[0], rz - along * d[1])
            band = np.clip(1 - off / (0.35 + wd * 1.5), 0, 1) ** 1.5
            band = np.floor(band * 4 + BAYER_FULL[ys, xs] * 0.9) / 4
            Ic += (band * al * 0.75)[:, None] * (P.COL[k] / 255.0 * 0.9 + 0.3)[None]
        for k, a_, ln, ck in sights:
            st = C.fs[k]
            d = np.array([math.cos(a_), math.sin(a_)])
            rx, rz = sxp - st['p'][0], szp - st['p'][1]
            along = rx * d[0] + rz * d[1]
            off = np.hypot(rx - along * d[0], rz - along * d[1])
            lane = (along > R) & (along < R + ln) & (off < 0.06)
            Ic += (lane * 0.18 * ck)[:, None] * (P.COL[k] / 255.0)[None]
        return Ic

    def canvas_wave(self, Ic, C, w):
        """FANFARE's ring of light on the canvas (#29: a shockwave band; a wave would be jagged and shivering)"""
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind == 'pulse':
            return Ic
        sxp, szp, ys, xs = C.sxp, C.szp, C.ys, C.xs
        rd = np.hypot(sxp - cx, szp - cz)
        wdt = 0.16 if kind == 'shockwave' else 0.32
        k_ = (1 - r / rmax) * (1.0 if kind == 'shockwave' else 1.3)
        if kind == 'wave':
            ang_ = np.arctan2(szp - cz, sxp - cx)
            rd = rd + 0.10 * (2 * ((7 * ang_ / math.pi + 6 * C.t) % 1) - 1) + 0.04 * np.sin(23 * ang_ - 50 * C.t)
        band = np.clip(1 - np.abs(rd - r) / wdt, 0, 1)
        band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
        Ic += (band * k_ * 0.9)[:, None] * (self.P.COL[owner] / 255.0 * 1.4 + (0.35 if kind == 'wave' else 0.15))[None]
        return Ic

    def shadows_free(self, F, spot):
        """the pistons' shadows from the spot"""
        from reelkit.shadows import proj_point
        polys = []
        for pk, pi, pa, poff, pext, pstr in self.now(F)[2]:
            if F.fs[pk]['vis'] and not F.fs[pk]['shattered']:
                a0, tip, d_ = piston_geom(F.fs[pk]['p'], pa, poff, pext)
                n_ = np.array([-d_[1], d_[0]]) * 0.05
                polys.append(proj_point(np.array([S.to_world(q[0], q[1], R) for q in (a0 - n_, tip - n_, tip + n_, a0 + n_)]),
                                        spot))
        return polys

    # ---------------------------------------------------------------- the painter pass, effects over the ring
    def items_free(self, F):
        """VALVE COMBO: the pistons out right now, each at its own depth"""
        out = []
        for pk, pi, pa, poff, pext, pstr in self.now(F)[2]:
            if F.fs[pk]['vis'] and not F.fs[pk]['shattered']:
                a0, tip, d_ = piston_geom(F.fs[pk]['p'], pa, poff, pext)
                mid = (a0 + tip) / 2
                out.append((F.cam.depth(S.to_world(mid[0], mid[1], R))[0], ('piston', (pk, pi, pa, poff, pext, pstr))))
        return out

    def draw_item(self, F, payload):
        what, obj = payload
        if what == 'piston':
            pk, pi, pa, poff, pext, pstr = obj
            arr, depth = F.fr.array(), F.fr.depth()
            draw_piston(arr, F.cam, F.fs[pk]['p'], pa, poff, pext, pstr, self.P.COL[pk])
            F.fr.set_array(arr, depth)

    def over_ring(self, F, arr, depth):
        """FANFARE's banners, the HIGH C sight and blasts, the champion's solo notes rising from the bell"""
        P, cam, t = self.P, F.cam, F.t
        blasts, sights, _ = self.now(F)
        if F.in_fight:
            for owner, cx, cz, r_, rmax, kind_, t0 in P.RUN['WAVES'][F.i]:
                if kind_ == 'shockwave' and owner == self.k:
                    draw_banners(arr, cam, np.array([cx, cz]), r_, min(1.0, (1 - r_ / rmax) * 1.6), t)
        for k, a_, ln, ck in sights:
            draw_sight(arr, cam, F.fs[k]['p'], a_, ln, ck, t, P.COL[k])
        for k, ox, oz, a_, ln, wd, ext, al, bst in blasts:
            pc = self.fire_centre(ox, oz, a_)
            draw_beam(arr, cam, pc, a_, ln, wd, bst, P.COL[k], t)
            draw_flash(arr, cam, pc, a_, max(0.0, 1 - bst[3] / 0.10), P.COL[k])
        if self.k == P.WIN:
            st = F.fs[P.WIN]
            for u, m, dd_ in self.solo_notes(t):
                org = bell_mouth(st['p'], st['th'])
                side = 0.25 * math.sin(m * 1.7)
                draw_note_rise(arr, cam, org[0] + side * u, org[1] - side * u, R + 0.4 + 1.3 * u, P.COL[P.WIN], 1 - u / 1.4)
        return arr, depth

    # ---------------------------------------------------------------- shards (#29: tubing, valve caps, brass chips)
    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'tube':
            # a bent length of brass tubing: a short arc, 2 px, brass with a bright edge
            pts = [np.array([sx_, sz_]) + d * 0.22 * math.cos(j * 0.5 * curl + 1.0) + nn * 0.22 * math.sin(j * 0.5 * curl + 1.0)
                   for j in range(6)]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            dr = ImageDraw.Draw(im)
            dr.line([tuple(q) for q in sp], fill=tuple(int(v) for v in BRASS_DK), width=2)
            dr.line([tuple(q) for q in sp], fill=tuple(int(v) for v in BRASS_HI), width=1)
            return np.asarray(im).astype(np.float32)
        if kind == 'valve':
            # a valve cap: a small pearl-topped disc
            rr = 0.09
            cen = S.to_world(sx_, sz_, p[1])
            ring = np.array([cen + [rr * math.cos(u), 0, rr * math.sin(u)] for u in np.linspace(0, 2 * math.pi, 9)[:-1]])
            P.layer_blend(arr, [P.screen(cam, ring)], BRASS, 1.0)
            ring2 = np.array([cen + [rr * 0.6 * math.cos(u), 0.01, rr * 0.6 * math.sin(u)] for u in np.linspace(0, 2 * math.pi, 7)[:-1]])
            P.layer_blend(arr, [P.screen(cam, ring2)], PEARL, 1.0)
            return arr
        # brass: tumbling flat chips of the bell / body, banded as they turn
        sz = size * 1.1
        tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
               np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        shade_k = 0.7 + 0.45 * max(0.0, math.cos(rot * 2))
        P.layer_blend(arr, [sp], np.clip(BRASS * shade_k, 0, 255), 1.0)
        return arr

    # ---------------------------------------------------------------- lens, glows
    def shimmer(self, img, cam, t, w):
        """heat shimmer on FANFARE's ring"""
        owner, cx, cz, r_, rmax, kind_, t0 = w
        return fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r_, 0.35, 2.6, t, k=1 - r_ / rmax, seed=owner)

    def shimmer_after(self, img, cam, t):
        """HIGH C: the air shivers along the blast"""
        for k, ox, oz, a_, ln, wd, ext, al, bst in self.beams_live(t):
            d = np.array([math.cos(a_), math.sin(a_)])
            for u in (0.25, 0.55, 0.85):
                q = np.array([ox, oz]) + d * ln * ext * u
                img = fx.shimmer_world(img, cam, S.to_world(q[0], q[1], R), 0.35 + 0.6 * wd * (1 + bst[3]), 0.3, 2.4, t,
                                       k=al, seed=int(u * 10))
        return img

    def glows(self, F, drawn):
        """the bell mouth glows as it charges / fires / solos; glow along the blast"""
        P, cam, fr = self.P, F.cam, F.fr
        st = F.fs[self.k]
        if st['vis'] and not st['shattered']:
            ck = st.get('charge', 0.0)
            if ck > 0.05:
                a = self.hc_aim(F.t) if st.get('hc', 0.0) > 0.5 else st['th']
                sm = cam.wp(S.to_world(*bell_mouth(st['p'], a), R)[None])[0]
                fr.glow(sm[0], sm[1], 6 + 8 * ck, tuple(P.COL[self.k] * 0.6 + 100), 0.35 + 0.55 * ck)
        for k, ox, oz, a_, ln, wd, ext, al, bst in self.now(F)[0]:
            for u in (0.15, 0.5, 0.85):
                q = np.array([ox, oz]) + u * ln * ext * np.array([math.cos(a_), math.sin(a_)])
                s_ = cam.wp(S.to_world(q[0], q[1], R)[None])[0]
                fr.glow(s_[0], s_[1], 10 + 8 * wd, tuple(P.COL[k] * 0.7 + 70), (0.35 if wd < 0.6 else 0.22) * al)

    # ---------------------------------------------------------------- checks
    def focus_points(self, t, st):
        """the bell mouth, every beam tip, every piston head"""
        P = self.P
        out = []
        tip = bell_mouth(st['p'], st['th'])
        out.append((S.to_world(tip[0], tip[1], R), 'bell'))
        for k, ox, oz, a_, ln, wd, ext, al, bst in self.beams_live(t):
            out.append((S.to_world(ox + ln * ext * math.cos(a_), oz + ln * ext * math.sin(a_), R), 'beam tip'))
        for pk, pi, pa, poff, pext, pstr in self.punches_live(t):
            a0, tip, d_ = piston_geom(st['p'], pa, poff, pext)
            out.append((S.to_world(tip[0], tip[1], R), 'piston'))
        return out


# ===================================================================================================== sound (#29 v3)
OPEN_PARTIALS = {3: 65, 4: 70, 6: 77, 8: 82}            # concert pitch of the open Bb trumpet's partials
FINGERING = {0: '0', 1: '2', 2: '1', 3: '12', 4: '23', 5: '13', 6: '123'}   # semitones down -> valves pressed


def valve_shift(chord):
    """(semitones down, valves) of the fingering for a chord: partial 4 (Bb4 open, down to E4 with all three valves)
    lands on the chord's root when it can, else on its fifth (#29's table: Cm -> 3 '12', Ab7 -> 2 '1', Fm -> 5 '13',
    G7 -> 3 '12')"""
    from ballfight.kit.episode import parse_chord
    root, pcs = parse_chord(chord)
    for tgt in (root, (root + 7) % 12):
        for s in range(7):
            if (70 - s) % 12 == tgt:
                return s, FINGERING[s]
    return 0, FINGERING[0]


def h01_29(*xs):
    """#29's deterministic hash in [0, 1) (its ghost-note thinning, as #29 built it)"""
    v = 0
    for x in xs:
        v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
    return ((v * 48271) % 2147483647) / 2147483647.0


class Sound(base.Sound):
    bus_gain, send = 1.8, 0.5                                   # #29's trumpet bus
    band = (300, 5000)                                          # #29's horn band check
    colour_args = dict(cut0=90, cut_span=520, drive=4.0)        # #29's colour (a little less drive than the default)

    def __init__(self, M, k):
        super().__init__(M, k)
        self.pp = M.CFG[k]['pp']
        self.pan = -0.25 if k == 0 else 0.25

    # ---------------------------------------------------------------- voices
    def tpt(self, m, dur=0.16, vel=0.9, cap=None):
        return self.M.note(self.pp, int(np.clip(m, 58, 86)), round(dur, 3), round(cap or dur + 0.22, 2), round(vel, 1))

    def voice(self, m, dur, vel=0.85):
        return self.tpt(m, dur, vel)

    def hiss(self, dur=0.07, seed=0):
        """steam escaping a valve"""
        A, SR = self.M.A, self.M.SR
        n = int(dur * SR)
        x = np.random.default_rng(300 + seed).normal(0, 1, n)
        return A.bp(x, 3500, 11000) * A.expenv(n, dur * 0.4, 0.003)

    def key_clicks(self, n=6, seed=8):
        """valves and caps clacking: short clicks"""
        A, SR = self.M.A, self.M.SR
        out = np.zeros(int(0.5 * SR))
        rng = np.random.default_rng(seed)
        for j in range(n):
            a = int(rng.uniform(0, 0.4) * SR)
            m = int(0.02 * SR)
            c = A.bp(rng.normal(0, 1, m), 900, 4000) * np.exp(-A.tt(m) / 0.004)
            out[a:a + m] += c
        return out

    def swell(self, x, dur):
        """a muted note opening up over `dur`: a crescendo + the harmon mute easing off (lowpass -> open)"""
        A = self.M.A
        n = len(x)
        t = A.tt(n)
        k = np.clip(t / max(1e-3, dur), 0, 1)
        dark = A.lp(x, 1100)
        y = dark * (1 - k) + x * k
        return y * (0.18 + 0.82 * k ** 1.6)

    # ---------------------------------------------------------------- notes
    def combo_notes(self, t, combo_no):
        """(three midi notes, fingering) for the combo that starts at t: partials 4-6-8 (or 3-4-6 on alternate combos)"""
        s, fing = valve_shift(self.T.chord_at(t))
        parts = (4, 6, 8) if combo_no % 2 else (3, 4, 6)
        return [OPEN_PARTIALS[p] - s for p in parts], fing

    def key_root(self, lo=72):
        """the key's root at or above midi lo (C -> 72: #29's calls are on C)"""
        return lo + self.T.KEY_PC % 12

    # ---------------------------------------------------------------- the score
    def title_blast(self, mix, t):
        """a lead trumpet's rip up to a high note, held with a shake (config TITLE_HITS: the rip's notes, last = held)"""
        T, A = self.T, self.M.A
        r = self.key_root()
        hits = tuple(T.TITLE_HITS.get(self.name, (r - 12, r - 5, r, r + 3, r + 7)))
        for j, m in enumerate(hits[:-1]):
            mix.place(self.bus, self.tpt(m, 0.06, 0.85), t + j * 0.028, 0.26, pan=self.pan)
        th = t + (len(hits) - 1) * 0.028
        mix.place(self.bus, self.tpt(hits[-1], 0.75, 1.0, 1.1), th, 0.40, pan=self.pan)
        for j in range(1, 12, 2):                                  # the shake: the note above flicked in on the 32nds
            mix.place(self.bus, self.tpt(hits[-1] + 3, 0.04, 0.7), th + j * T.S16 / 2, 0.08, pan=self.pan)
        mix.place('fx', A.thud(0.5, 120, 40), t, 0.20)

    def count_in(self, mix):
        """a stab on each count: the count-in chord's third nearest B4 (#29: 71 over G7)"""
        T = self.T
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            tc = max(tc, 0.004)
            ch = T.chord_at(tc)
            third = T.CHORD_PCS[ch][1]
            m = min((x for x in range(60, 84) if x % 12 == third), key=lambda x: (abs(x - 71), x))
            mix.place(self.bus, self.tpt(m, 0.14, 0.9), tc, 0.30, pan=self.pan)

    def play(self, mix):
        """VALVE COMBO + its abilities from the run"""
        M, T, k = self.M, self.T, self.k
        A, q16, S16 = M.A, M.q16, T.S16
        RUN, EV = M.RUN, M.EV
        hits = {}
        for e in EV:
            if e[1] == 'hit' and e[2] == k and e[7] == 'piston':
                hits.setdefault(int(e[8]), []).append(e[0])
        hit_t = {i: np.array(v) for i, v in hits.items()}
        combo_start = {}
        for e in EV:
            if e[1] == 'combo' and e[2] == k:
                combo_start[e[3]] = e[0]
        line = self.name in T.LINES
        for e in EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'punch' and e[2] == k:
                i, cn = int(e[3]), int(e[6])
                tq = q16(t)
                if line:                                              # the melody: the config line's 8th
                    m = T.line_at(self.name, tq)
                else:
                    notes, fing = self.combo_notes(combo_start.get(cn, t), cn)
                    m = notes[min(i, 2)]
                ht = hit_t.get(i, np.zeros(0))
                landed = bool(len(ht) and np.any((ht >= t - 1e-6) & (ht <= t + 0.2)))
                hp = M.hp_at(k, t) / 100.0
                if not landed and hp < 0.5 and h01_29(t, 4) < 0.5 * (1 - hp) ** 1.5:   # thinning when hurt: ghosts drop
                    continue
                last = i == 2
                vel = 1.0 if landed else 0.7
                sig, g, lead = self.colour(self.tpt(m, 0.12 if not last else 0.2, vel), t, 0.8)
                mix.place(self.bus, sig, tq, (0.38 if landed else 0.17) * (1.15 if last else 1.0) * g, pan=self.pan)
                mix.place('fx', self.hiss(0.07, int(t * 50) % 97), tq, 0.035, pan=self.pan - 0.05 * (1 if k == 0 else -1))
                if landed:
                    if lead > 0.04:                                   # the leader: the octave below joins
                        mix.place(self.bus, self.colour(self.tpt(m - 12, 0.12, 0.8), t)[0], tq, 0.14 * g, pan=self.pan)
                    mix.place('fx', A.thud(0.22 if not last else 0.32, 150, 60, seed=int(t * 9)), tq,
                              0.09 if not last else 0.16, pan=-0.8 * self.pan)
                if float(RUN['HA'][M.P.sim_i(t), k]) > 0 and not last:   # DOUBLE TONGUE: a ta-ka grace on each punch
                    mix.place(self.bus, self.colour(self.tpt(m, 0.05, 0.7), t)[0], tq + S16 / 2, 0.12, pan=self.pan)
            elif kind == 'ability' and e[2] == k:
                tq = q16(t)
                ab = M.CFG[k]['abilities']
                if e[3] == ab['unlock1']['name']:                     # FANFARE: a rising call on the key's minor triad
                    r = self.key_root()
                    third = 3 if 3 in T.SCALE else 4
                    for j, m in enumerate((r, r + third, r + 7, r + 12)):
                        d = 0.09 if j < 3 else 0.6
                        mix.place(self.bus, self.colour(self.tpt(m, d, 1.0), t)[0], tq + j * S16, 0.36, pan=self.pan)
                    mix.place('drums', A.crash(1.4), tq + 3 * S16, 0.10, pan=0.3)
                    mix.place('fx', A.thud(0.4, 120, 45), tq, 0.25)
                elif e[3] == ab['unlock2']['name']:                   # DOUBLE TONGUE: ta-ka-ta-ka on the fifth
                    for j in range(8):
                        mix.place(self.bus, self.colour(self.tpt(self.key_root() + 7, 0.05, 0.7 + 0.3 * (j % 2 == 0)), t)[0],
                                  tq + j * S16 / 2, 0.22, pan=self.pan)
                elif e[3] == ab['low']['name']:                       # HIGH C: the charge, the high C swelling open
                    tc = T.BEAT
                    x = self.swell(self.tpt(84, tc * 0.95, 0.8, tc + 0.1), tc)
                    mix.place(self.bus, self.colour(x, t, 1.2)[0], tq, 0.26, pan=self.pan)
            elif kind == 'beam' and e[2] == k:                        # HIGH C fires: the scream with a shake + the boom
                tq = q16(t)
                dur = 1.4
                mix.place(self.bus, self.colour(self.tpt(84, dur, 1.0, dur + 0.3), t, 1.5)[0], tq, 0.32, pan=self.pan)
                j = 0
                while j * S16 / 2 < dur:
                    if j % 2:
                        mix.place(self.bus, self.tpt(87, 0.05, 0.7), tq + j * S16 / 2, 0.10 * min(1.0, j / 6), pan=self.pan)
                    j += 1
                mix.place('fx', A.thud(0.7, 120, 34), tq, 0.40)
                mix.place('drums', A.crash(2.0), tq, 0.14, pan=0.3)

    def fall_apart(self, mix):
        """the trumpet cracks: a split note squealing down, the valves clattering"""
        M = self.M
        r = self.key_root()
        for j, m in enumerate((r + 12, r + 8, r + 5, r, r - 4, r - 7, r - 12)):
            mix.place(self.bus, self.tpt(m, 0.06, 0.8), M.P.T_HS + 0.07 * j, 0.20 * (1 - j / 9), pan=0.8 * self.pan)
        mix.place('fx', self.key_clicks(5, 5), M.P.T_HS, 0.14)

    def stinger(self, mix, tw):
        r = self.key_root()
        for m in (r, r + 7, r + 12):
            mix.place('solo', self.tpt(m, 0.5, 1.0, 0.9), tw, 0.20)

    def solo(self, mix, t0):
        """config SOLO, open horn"""
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            dur = d * T.BEAT * 0.9
            mix.place('solo', self.tpt(m, dur, 0.95, dur + 0.3), ts, 0.46, pan=0.0)

    def reassemble(self, mix):
        """its pieces fly home as a rising run of soft staccato notes up the minor pentatonic"""
        T, M = self.T, self.M
        r = self.key_root(60)
        run = [r + i for i in (0, 3, 5, 7, 10, 12, 15, 19)]
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place(self.bus, self.tpt(m, 0.05, 0.6), tj, 0.12, pan=0.8 * self.pan)
            mix.place('fx', self.key_clicks(2, 20 + j), tj, 0.05)

    def last_bar(self, mix):
        """it climbs into the count on the off-beats: the last chord's tones up from C5 (#29: C Eb Gb Ab over Ab7)"""
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        tones = T.chord_tones(72, 86, T.CHORDS[-1])[:4]
        for j, m in enumerate(tones):
            mix.place(self.bus, self.tpt(m, 0.10, 0.6 + 0.1 * j), t_last + j * T.BEAT + T.BEAT / 2, 0.14 + 0.03 * j,
                      pan=self.pan)
