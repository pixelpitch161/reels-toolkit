"""ACOUSTIC (wood tan), as built in #33 v3: SIX-STRING RAKE (a giant strum: six glowing string lines sweep across the
ring in front of the ball on every beat, alternating down / up, one hit per stroke flicks the enemy along it; +length
per landed stroke), CAPO (strums twice as fast, a capo clamped across the ball), FINGERPICK (single strings flick out
like whips), BIG STRUM (all six strings sweep the whole ring).

Look: the ball flashes on every stroke; six strings (wound bronze E A D, plain G B E) under one warm glow, with a motion
fan between frames and a lit band on the canvas under them; six-line claw marks where every stroke ends; FINGERPICK's
whips with a travelling wave and a bright tip; BIG STRUM's six thick strings across the whole ring over a bright band,
a heat shimmer on its front; the dark capo bar while CAPO lasts; the sprite sits big in its ball (0.86 of the radius:
the uke's big brother). It shatters into strings, spruce top chips, dark side chips and frets.
Sound: every stroke is a strum in the stroke's own direction (down = the full chord low to high, up = the top strings
high to low); CAPO's extra strokes bright and short; a landed stroke adds a body slap + a bass-string pop; the leader
adds the boom-chick alternating bass; it loses strings (6 -> 3) as it gets hurt and the up-strokes drop out; FINGERPICK
whips are single picked notes up the chord; BIG STRUM one giant slow strum over the low E.
Draw code from projects/ballfight_ukulele_vs_acoustic/styles.py + render.py (#33 v3), copied unchanged; sound from #33's
audio.py (the Pixel Pitch steel-string's voice). #33's voicings (Am G F E) are standard open shapes, so any episode's
chords are voiced the same way (open shapes first, then barre shapes)."""
import math

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, FPS, BAYER_FULL, ss
from reelkit import fx
import scene as S

from . import base

R = S.R_BALL
CLAW_COL = (70, 44, 30)
STRINGS6 = [np.array(c, float) for c in ((198, 150, 86), (206, 160, 96), (214, 172, 110), (226, 218, 204),
                                          (234, 228, 218), (242, 238, 230))]   # wound bronze E A D, plain G B E
CAPO_METAL = np.array([44, 40, 52])
STRING_COL = (212, 176, 118)                                                     # bronze steel strings (shards)
WOOD_COL = {'top': np.array([226, 190, 122]), 'side': np.array([112, 52, 30])}


# ---------------------------------------------------------------- #33 styles.py helpers (unchanged)
def mask_of(polys, width=None):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if width:
            d.line([tuple(q) for q in pts], fill=255, width=width)
        elif len(pts) >= 3:
            d.polygon([tuple(q) for q in pts], fill=255)
    return np.asarray(m, np.float32) / 255.0


def blend(arr, m, col, k):
    arr += (np.clip(np.asarray(col, float), 0, 255)[None, None] - arr) * (m * k)[..., None]


def _perp(u):
    return np.array([-u[1], u[0]])


def overlay_polys(arr, polys, alpha_max=1.0):
    """draw [(screen pts, rgb, alpha)] onto one RGBA overlay in the given order, then composite it once"""
    if not polys:
        return arr
    ov = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(ov)
    for pts, rgb, a in polys:
        if len(pts) >= 3:
            d.polygon([tuple(q) for q in pts], fill=tuple(int(v) for v in np.clip(rgb, 0, 255)) + (int(255 * a),))
    o = np.asarray(ov, np.float32)
    k = o[..., 3:4] / 255.0 * alpha_max
    return arr * (1 - k) + o[..., :3] * k


# ---------------------------------------------------------------- CAPO (#33 styles.py)
def draw_capo(arr, s, rp, k, col, t):
    """a capo clamped across the glass: a dark bar with a tension screw and a highlight in the fighter's colour"""
    if k <= 0.02:
        return
    x, y = s
    yb = y - rp * 0.30
    w = rp * 0.92
    m = mask_of([[(x - w, yb - 2), (x + w, yb - 2), (x + w, yb + 1.5), (x - w, yb + 1.5)]])
    blend(arr, m, CAPO_METAL, 0.95 * k)
    hl = mask_of([[(x - w, yb - 2), (x + w, yb - 2)]], width=1)
    blend(arr, hl, np.clip(np.asarray(col, float) * 0.9 + 60, 0, 255), 0.9 * k)
    sc = mask_of([[(x + w - 1, yb - 4), (x + w + 2, yb - 4), (x + w + 2, yb + 3), (x + w - 1, yb + 3)]])
    blend(arr, sc, np.array([210, 210, 222]), 0.9 * k)


# ---------------------------------------------------------------- SIX-STRING RAKE, FINGERPICK, BIG STRUM (#33 styles.py)
def rake_segments(c, u, n, s, L, gap, R0=R):
    """the six strings (pairs of ring-frame points) of a rake whose bundle sits at lateral offset s"""
    out = []
    for j in range(6):
        o = (j - 2.5) * gap
        a = c + u * R0 + n * (s + o)
        b = c + u * (R0 + L) + n * (s + o)
        out.append((a, b))
    return out


def draw_strings(arr, cam, segs, glow_col, k, y=0.5, prev=None, thick=(2, 2, 2, 1, 1, 1)):
    """six strings: a glow in the fighter's colour under a bright core; `prev` (the bundle a frame ago) adds a motion
    fan between the two positions"""
    if k <= 0.02:
        return arr
    glow_col = np.asarray(glow_col, float)
    polys = []
    if prev is not None:
        for (a, b), (pa, pb) in zip(segs, prev):
            q = cam.wp(np.array([S.to_world(*a, y), S.to_world(*b, y), S.to_world(*pb, y), S.to_world(*pa, y)]))
            polys.append((q, glow_col * 0.5 + 120, 0.22 * k))
    arr = overlay_polys(arr, polys)
    gm = mask_of([cam.wp(np.array([S.to_world(*a, y), S.to_world(*b, y)])) for a, b in segs], width=5)
    blend(arr, gm, np.clip(glow_col * 1.05 + 30, 0, 255), 0.55 * k)              # one warm glow under the bundle
    for j, (a, b) in enumerate(segs):
        s = cam.wp(np.array([S.to_world(*a, y), S.to_world(*b, y)]))
        blend(arr, mask_of([s], width=thick[j]), np.clip(STRINGS6[j] * 0.6 + 110, 0, 255), 0.98 * k)
    return arr


def draw_whip(arr, cam, p, ang, ln, tau, t_ext, glow_col, y=0.5, j=0):
    """one FINGERPICK string flicking out like a whip and snapping back: a travelling wave along it, a bright tip"""
    if tau < 0 or tau > 2 * t_ext:
        return arr
    ext = tau / t_ext if tau <= t_ext else 1 - (tau - t_ext) / t_ext
    u = np.array([math.cos(ang), math.sin(ang)])
    n = _perp(u)
    Lc = ln * max(0.0, ext)
    if Lc < 0.05:
        return arr
    pts = []
    for x in np.linspace(0, 1, 12):
        lat = 0.09 * math.sin(math.pi * x * 2.5 - tau * 60 + j) * x * (1 - 0.5 * ext)
        q = p + u * (R + Lc * x) + n * lat
        pts.append(S.to_world(q[0], q[1], y))
    s = cam.wp(np.array(pts))
    glow_col = np.asarray(glow_col, float)
    blend(arr, mask_of([s], width=3), np.clip(glow_col * 0.8 + 50, 0, 255), 0.5)
    blend(arr, mask_of([s], width=1), STRINGS6[j % 6] * 1.05 + 20, 0.95)
    tx, ty = int(round(s[-1][0])), int(round(s[-1][1]))
    if 0 <= tx < W and 0 <= ty < H:
        arr[max(0, ty - 1):ty + 2, max(0, tx - 1):tx + 2] = np.array([255, 248, 230])
    return arr


def clip_segment(a, b, lim):
    """Liang-Barsky: the part of segment a-b inside the square |x|, |z| <= lim (or None)"""
    t0, t1 = 0.0, 1.0
    d = b - a
    for p, q in ((-d[0], a[0] + lim), (d[0], lim - a[0]), (-d[1], a[1] + lim), (d[1], lim - a[1])):
        if abs(p) < 1e-12:
            if q < 0:
                return None
            continue
        r = q / p
        if p < 0:
            t0 = max(t0, r)
        else:
            t1 = min(t1, r)
        if t0 > t1:
            return None
    return a + d * t0, a + d * t1


def sweep_segments(c, u, front, lim, gap=0.14):
    """BIG STRUM: six strings lying across the whole ring (perpendicular to u), the first at `front` from c, the rest
    trailing behind it, clipped to the canvas"""
    n = _perp(u)
    out = []
    for j in range(6):
        m = c + u * (front - j * gap)
        seg = clip_segment(m - n * 9.0, m + n * 9.0, lim)
        if seg is not None:
            out.append(seg)
    return out


# ---------------------------------------------------------------- guitar chord shapes (EADGBE)
OPEN = {'E': (40, 47, 52, 56, 59, 64), 'Em': (40, 47, 52, 55, 59, 64), 'E7': (40, 47, 50, 56, 59, 64),
        'Am': (45, 52, 57, 60, 64), 'A': (45, 52, 57, 61, 64), 'A7': (45, 52, 55, 61, 64), 'Am7': (45, 52, 55, 60, 64),
        'C': (48, 52, 55, 60, 64), 'D': (50, 57, 62, 66), 'Dm': (50, 57, 62, 65), 'D7': (50, 57, 60, 66),
        'G': (43, 47, 50, 55, 59, 67), 'F': (41, 48, 53, 57, 60, 65), 'B7': (47, 51, 57, 59, 66),
        'Bm': (47, 54, 59, 62, 66)}
E_SHAPE = {'': (0, 7, 12, 16, 19, 24), 'm': (0, 7, 12, 15, 19, 24), '7': (0, 7, 10, 16, 19, 24),
           'm7': (0, 7, 10, 15, 19, 24)}
A_SHAPE = {'': (0, 7, 12, 16, 19), 'm': (0, 7, 12, 15, 19), '7': (0, 7, 10, 16, 19), 'm7': (0, 7, 10, 15, 19)}


def guitar_chord(ch):
    """the chord as a strummed guitar shape (midi, low string first): the open shape if it has one (#33's Am G F E are
    these), else the E- or A-shape barre lower on the neck, else its tones stacked from the low strings"""
    if ch in OPEN:
        return OPEN[ch]
    from ballfight.kit.episode import parse_chord
    n = 2 if len(ch) > 1 and ch[1] in '#b' else 1
    q = ch[n:]
    root = parse_chord(ch)[0]
    fe, fa = (root - 4) % 12, (root - 9) % 12
    if q in E_SHAPE and fe <= fa:
        return tuple(40 + fe + i for i in E_SHAPE[q])
    if q in A_SHAPE:
        return tuple(45 + fa + i for i in A_SHAPE[q])
    pcs = parse_chord(ch)[1]
    lo = 40 + fe
    return tuple([lo] + [m for m in range(lo + 1, 68) if m % 12 in pcs][:4])


def h01_33(*xs):
    """#33's deterministic hash in [0, 1) (its up-stroke dropout, as #33 built it)"""
    v = 0
    for x in xs:
        v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
    return ((v * 48271) % 2147483647) / 2147483647.0


def bass_pair(ch):
    """(root on 1, fifth on 3) for the boom-chick (#33: Am 45 52, G 43 50, F 41 48, E 40 47)"""
    from ballfight.kit.episode import parse_chord
    r = 40 + (parse_chord(ch)[0] - 4) % 12
    return r, r + 7


# ===================================================================================================== look (#33 v3)
class Look(base.Look):
    shards = ['string'] * 6 + ['top'] * 5 + ['side'] * 3 + ['fret'] * 3
    spark_big = ()                                   # #33: rake / whip / sweep sparks are the small spray
    flash_hits = ()                                  # #33: none of its hits set off the flashbulbs (abilities do)
    fringe_hits = ('sweep',)
    rope_kinds = {}                                  # no rope-shaking ability
    sprite_k = 0.86                                  # the big brother: the guitar fills its ball

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.RAKES = [dict(t0=e[0], owner=e[2], main=bool(e[3]), L=e[4], c=np.array([e[5], e[6]]), aim=e[7],
                           u=np.array([math.cos(e[7]), math.sin(e[7])]), n=np.array([-math.sin(e[7]), math.cos(e[7])]),
                           dir=e[8], S=e[9], dur=e[10]) for e in EV if e[1] == 'rake' and e[2] == k]
        self.RAKE_T = np.array([r_['t0'] for r_ in self.RAKES]) if self.RAKES else np.zeros(0)
        self.RAKE_HIT = set()
        for e in EV:
            if e[1] == 'hit' and e[7] == 'rake' and e[2] == k:
                j = int(np.searchsorted(self.RAKE_T, e[0], 'right')) - 1
                if j >= 0:
                    self.RAKE_HIT.add(j)
        self.GAP = self.cfg['weapon'].get('gap', 0.085)
        self.WHIPS = [dict(t0=e[0], owner=e[2], j=e[3], ang=e[4], len=e[5]) for e in EV if e[1] == 'whip' and e[2] == k]
        self.T_EXT = self.cfg['abilities']['unlock2'].get('t_ext', 0.07)
        self.SWEEPS = [dict(t0=e[0], owner=e[2], c=np.array([e[3], e[4]]), u=np.array([math.cos(e[5]), math.sin(e[5])]),
                            aim=e[5], reach=e[6], dur=e[7]) for e in EV if e[1] == 'sweep' and e[2] == k]
        capo = [s for s in ('unlock1', 'unlock2', 'low') if self.cfg['abilities'][s]['kind'] == 'haste']
        self.CAPO_DUR = (self.cfg['abilities'][capo[0]]['dur_beats'] * self.T.BEAT) if capo else 0.0

    # ---------------------------------------------------------------- clocks (#33: attacks freeze with the hit-stop)
    def ta(self, t):
        return min(t, self.P.T_KO)

    def show(self, t):
        return self.T.T_FIGHT <= t < self.P.T_HS + 0.12

    def rakes_at(self, t):
        """(j, rake, s_now, s_prev, k) for strokes still showing (the stroke + a 0.2 s fade)"""
        if not len(self.RAKE_T):
            return []
        from fight import Fight
        out = []
        j0, j1 = np.searchsorted(self.RAKE_T, t - 0.4), np.searchsorted(self.RAKE_T, t, 'right')
        for j in range(j0, j1):
            r_ = self.RAKES[j]
            tau = t - r_['t0']
            if tau > r_['dur'] + 0.2:
                continue
            sn = Fight.rake_lateral(tau, r_['dur'], r_['S'], r_['dir'])
            sp = Fight.rake_lateral(max(0.0, tau - 1.0 / FPS), r_['dur'], r_['S'], r_['dir'])
            k = 1.0 if tau <= r_['dur'] else 1 - (tau - r_['dur']) / 0.2
            out.append((j, r_, sn, sp, k))
        return out

    @staticmethod
    def sweep_front_at(sw, t):
        from fight import Fight
        return Fight.sweep_front(t - sw['t0'], sw['dur'], sw['reach'], R)

    def now(self, F):
        """this frame's live strokes, whips and sweeps (cached on the frame context)"""
        key = ('acoustic_now', self.k)
        if key not in F.__dict__:
            t = F.t
            ta = self.ta(t)
            sh = self.show(t)
            F.__dict__[key] = (self.rakes_at(ta) if sh else [],
                               [w_ for w_ in self.WHIPS if 0 <= ta - w_['t0'] <= 2 * self.T_EXT] if sh else [],
                               [sw for sw in self.SWEEPS if 0 <= ta - sw['t0'] <= sw['dur'] + 0.2] if sh else [], ta)
        return F.__dict__[key]

    def state(self, kk, st, t, phase, gl=0.0):
        if phase == 'fight' and kk == self.k and len(self.RAKE_T):        # the acoustic flashes on every stroke
            tc = min(t, self.P.T_KO)
            j = np.searchsorted(self.RAKE_T, tc, 'right') - 1
            if j >= 0:
                st['glow'] = 0.35 * math.exp(-(tc - self.RAKE_T[j]) / 0.09)

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit' and e[7] in ('rake', 'sweep'):
            return {'rake': 0.5, 'sweep': 2.5}[e[7]]
        return super().shake(e)

    def hit_glow(self, e):
        if e[7] == 'sweep':
            return 12, 0.35
        if e[7] == 'rake':
            return 7, 0.22
        return None

    # ---------------------------------------------------------------- effects (#33 _fx_build)
    def fx_setup(self):
        """every stroke lights the strings and leaves six-line claw marks where it ends"""
        P, k = self.P, self.k
        from fight import Fight
        for j, r_ in enumerate(self.RAKES):
            s_end = Fight.rake_lateral(r_['dur'], r_['dur'], r_['S'], r_['dir'])
            mid = r_['c'] + r_['u'] * (R + r_['L'] * 0.5) + r_['n'] * s_end
            P.FL.add(r_['t0'] + r_['dur'] * 0.5, S.to_world(mid[0], mid[1], 0.7), P.FL_COL[k] * 0.9, radius=1.3,
                     peak=0.35 if r_['main'] else 0.22, decay=0.14)
            if r_['t0'] + r_['dur'] < P.T_KO:
                P.MK.add(r_['t0'] + r_['dur'], 'claw', mid[0], mid[1], size=r_['L'] * 0.5, angle=r_['aim'], n=6,
                         col=CLAW_COL, life=2.6, seed=j, strength=0.95 if j in self.RAKE_HIT else 0.5)

    def fx_event(self, e):
        P = self.P
        if e[1] != 'hit':
            return
        att, x, z, how = e[2], e[5], e[6], e[7]
        pos = S.to_world(x, z, R)
        if how == 'rake':
            P.BU.add(e[0], pos, P.COL[att], 0.8)
            P.FL.add(e[0], pos, P.FL_COL[att] * 0.95, radius=1.5, peak=0.65, decay=0.22)
            P.PA.emit(e[0], pos, 'chips', n=8)
        elif how == 'whip':
            P.BU.add(e[0], pos, P.COL[att], 0.4, bright=False)
            P.PA.emit(e[0], pos, 'chips', n=3, speed=(1.0, 2.0))
        elif how == 'sweep':
            P.BU.add(e[0], pos, P.COL[att], 2.0)
            P.FL.add(e[0], pos, P.FL_COL[att], radius=2.4, peak=1.1, decay=0.5)
            P.PA.emit(e[0], pos, 'chips', n=24, speed=(1.5, 3.2), scale=1.2)
            P.MK.add(e[0], 'claw', x, z, size=1.2,
                     angle=math.atan2(*(self.SWEEPS[0]['u'][::-1])) + math.pi / 2 if self.SWEEPS else 0.0,
                     n=6, col=CLAW_COL, life=6.0, seed=77)

    def fx_after(self):
        """BIG STRUM: the sweep lights the ring as it crosses"""
        P = self.P
        for sw in self.SWEEPS:
            for u_ in (0.2, 0.5, 0.8):
                f = self.sweep_front_at(sw, sw['t0'] + sw['dur'] * u_)
                c = sw['c'] + sw['u'] * f
                P.FL.add(sw['t0'] + sw['dur'] * u_, S.to_world(c[0], c[1], 0.9), P.FL_COL[self.k], radius=2.6, peak=0.6,
                         decay=0.25)

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'chips', n=30, spread=180, speed=(0.8, 2.4), scale=1.2)

    # ---------------------------------------------------------------- the canvas
    def canvas(self, Ic, C):
        P = self.P
        rakes_now, _, sweeps_now, ta = self.now(C)
        sxp, szp, ys, xs = C.sxp, C.szp, C.ys, C.xs
        for j, r_, sn, sp_, k_ in rakes_now:                              # light under each stroke's six strings
            ra = (sxp - r_['c'][0]) * r_['u'][0] + (szp - r_['c'][1]) * r_['u'][1]
            rb = (sxp - r_['c'][0]) * r_['n'][0] + (szp - r_['c'][1]) * r_['n'][1]
            inl = np.clip(1 - np.maximum(R * 0.9 - ra, ra - (R + r_['L'])) / 0.15, 0, 1)
            band = np.clip(1 - np.abs(rb - sn) / 0.34, 0, 1) * inl
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * k_ * (0.45 if r_['main'] else 0.3))[:, None] * (P.COL[r_['owner']] / 255.0 * 0.9 + 0.25)[None]
        for sw in sweeps_now:                                             # BIG STRUM: a bright band under the strings
            fpos = self.sweep_front_at(sw, ta)
            ra = (sxp - sw['c'][0]) * sw['u'][0] + (szp - sw['c'][1]) * sw['u'][1] - fpos
            band = np.clip(1 - np.abs(ra + 0.35) / 0.55, 0, 1) * (1 - ss(sw['dur'], sw['dur'] + 0.2, ta - sw['t0']))
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * 0.7)[:, None] * (P.COL[sw['owner']] / 255.0 + 0.3)[None]
        return Ic

    # ---------------------------------------------------------------- the painter pass
    def items_ball(self, F, st):
        """CAPO: clamped across the ball while it lasts (drawn right after the ball: the same depth)"""
        if self.CAPO_DUR > 0 and st['haste'] > 0 and F.in_fight:
            pw = S.to_world(st['p'][0], st['p'][1], R)
            return [(F.cam.depth(pw)[0], ('capo', st))]
        return []

    def items_free(self, F):
        P = self.P
        rakes_now, whips_now, sweeps_now, ta = self.now(F)
        cam = F.cam
        out = []
        for j, r_, sn, sp_, k_ in rakes_now:
            mid = r_['c'] + r_['u'] * (R + r_['L'] * 0.5) + r_['n'] * sn
            out.append((cam.depth(S.to_world(mid[0], mid[1], 0.5))[0], ('rake', (r_, sn, sp_, k_))))
        for w_ in whips_now:
            p_ = F.fs[w_['owner']]['p']
            out.append((cam.depth(S.to_world(p_[0], p_[1], 0.5))[0] - 0.02, ('whip', w_)))
        if sweeps_now:                                                    # #33 drew these after the whole painter pass
            out.append((-1e9, ('sweeps', sweeps_now)))
        return out

    def draw_item(self, F, payload):
        P, cam, fr = self.P, F.cam, F.fr
        what, obj = payload
        ta = self.ta(F.t)
        arr, depth = fr.array(), fr.depth()
        if what == 'capo':
            st = obj
            pw = S.to_world(st['p'][0], st['p'][1], R)
            Pc = cam.to_cam(pw[None])[0]
            s = cam.proj(Pc[None])[0]
            rp = R * cam.f / Pc[2] * st['scale']
            k_ = min(1.0, st['haste'] / 0.15, (self.CAPO_DUR - st['haste']) / 0.12)
            draw_capo(arr, s, rp, max(0.0, k_), P.COL[self.k], F.t)
        elif what == 'rake':
            r_, sn, sp_, k_ = obj
            segs = rake_segments(r_['c'], r_['u'], r_['n'], sn, r_['L'], self.GAP)
            prev = rake_segments(r_['c'], r_['u'], r_['n'], sp_, r_['L'], self.GAP) if abs(sn - sp_) > 0.01 else None
            arr = draw_strings(arr, cam, segs, P.COL[r_['owner']], k_ * (1.0 if r_['main'] else 0.75), 0.5, prev)
        elif what == 'whip':
            w_ = obj
            arr = draw_whip(arr, cam, F.fs[w_['owner']]['p'], w_['ang'], w_['len'], ta - w_['t0'], self.T_EXT,
                            P.COL[w_['owner']], j=w_['j'])
        elif what == 'sweeps':
            for sw in obj:
                tau = ta - sw['t0']
                kk = 1 - ss(sw['dur'], sw['dur'] + 0.2, tau)
                f1 = self.sweep_front_at(sw, ta)
                f0 = self.sweep_front_at(sw, ta - 1.0 / FPS)
                segs = sweep_segments(sw['c'], sw['u'], f1, S.AP)
                prev = sweep_segments(sw['c'], sw['u'], f0, S.AP) if f1 - f0 > 0.02 and len(segs) else None
                if prev is not None and len(prev) != len(segs):
                    prev = None
                arr = draw_strings(arr, cam, segs, P.COL[sw['owner']], kk, 0.55, prev, thick=(3, 3, 3, 2, 2, 2)[:len(segs)])
        fr.set_array(arr, depth)

    # ---------------------------------------------------------------- shards (#33: strings, top, sides, frets)
    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'string':
            pts = [np.array([sx_, sz_]) + d * (j - 3) * 0.16 + nn * curl * 0.10 * math.sin(j) for j in range(7)]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=STRING_COL, width=1)
            return np.asarray(im).astype(np.float32)
        if kind == 'fret':
            Lk, Wk = 0.30, 0.035
            c = np.array([222, 224, 236])
            q4 = [np.array([sx_, sz_]) + d * Lk / 2 * a1 + nn * Wk / 2 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], c, 1.0)
            return arr
        col = WOOD_COL[kind]                                               # top (spruce), side (dark): wooden chips
        sz = size * (1.3 if kind == 'top' else 1.0)
        tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
               np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        shade_k = 0.75 + 0.25 * math.cos(rot * 2)
        P.layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
        return arr

    # ---------------------------------------------------------------- lens
    def shimmer_after(self, img, cam, t):
        """BIG STRUM's sweep front ripples the air"""
        if not self.show(t):
            return img
        ta = self.ta(t)
        for sw in self.SWEEPS:
            tau = ta - sw['t0']
            if 0 <= tau <= sw['dur']:
                f = self.sweep_front_at(sw, ta)
                c = sw['c'] + sw['u'] * f
                img = fx.shimmer_world(img, cam, S.to_world(c[0], c[1], 0.05), 1.6, 0.5, 2.6, t, k=0.8)
        return img

    def focus_points(self, t, st):
        """the strings' far ends while a stroke shows"""
        if not self.show(t):
            return []
        out = []
        for j, r_, sn, sp_, k_ in self.rakes_at(self.ta(t)):
            b = r_['c'] + r_['u'] * (R + r_['L']) + r_['n'] * sn
            out.append((S.to_world(b[0], b[1], 0.5), 'rake'))
        return out


# ===================================================================================================== sound (#33)
class Sound(base.Sound):
    bus_gain, send = 1.45, 0.25
    band = (300, 5000)                                                     # #33's band check
    pan = 0.25

    def __init__(self, M, k):
        super().__init__(M, k)
        self.pp = M.CFG[k]['pp']
        self.pan = -0.25 if k == 0 else 0.25

    def note(self, m, dur=0.3, cap=0.9, vel=0.85):
        return self.M.note(self.pp, int(m), dur, cap, vel)

    def voice(self, m, dur, vel=0.85):
        return self.note(m, dur, max(0.6, dur + 0.3), vel)

    def strum(self, notes, vel=0.85, spread=0.011, up=False, dur=0.40, cap=1.0):
        """a strum: the chord's strings a few ms apart, low to high (down) or high to low (up)"""
        SR = self.M.SR
        ns = list(notes)[::-1] if up else list(notes)
        d = int(spread * SR)
        out = np.zeros(int(cap * SR) + d * len(ns))
        for j, m in enumerate(ns):
            v = round(vel * (1.0 if j == 0 else 0.9), 2)
            x = self.note(m, dur, cap, v)
            out[j * d:j * d + len(x)] += x
        return out / math.sqrt(len(ns)) * 1.5

    @staticmethod
    def strings_left(hp):
        return 6 if hp > 60 else (5 if hp > 35 else (4 if hp > 15 else 3))

    def title_blast(self, mix, t):
        """the full open chord rung out, the bass root under it (config TITLE_HITS: the chord's notes)"""
        T = self.T
        ch = guitar_chord(T.CHORDS[0])
        hits = tuple(T.TITLE_HITS.get(self.name, tuple(ch) + ((ch[-1] + 5,) if len(ch) < 6 else ())))
        mix.place(self.bus, self.strum(hits, 1.0, 0.016, dur=0.9, cap=1.8), t, 0.55, pan=self.pan)
        mix.place(self.bus, self.note(hits[0], 0.9, 1.6, 1.0), t, 0.30, pan=self.pan)

    def count_in(self, mix):
        T = self.T
        ch = guitar_chord(T.CHORDS[1])
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place(self.bus, self.strum(ch, 0.9, 0.013), tc, 0.30, pan=self.pan)

    def play(self, mix):
        """SIX-STRING RAKE: every stroke is a strum in its own direction; a landed stroke adds a body slap + a bass pop;
        FINGERPICK whips are single picked notes up the chord, a landed whip a harmonic ping; BIG STRUM one giant strum"""
        M, T, k, A = self.M, self.T, self.k, self.M.A
        q16, SR = M.q16, M.SR
        other_pan = -self.pan
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'rake' and e[2] == k:
                tq = q16(t)
                hp = M.hp_at(k, t)
                ch = T.chord_at(tq)
                full = guitar_chord(ch)
                vo = full[:max(3, self.strings_left(hp) - (6 - len(full)))]   # broken strings go from the top
                main_ = bool(e[3])
                down = e[8] > 0
                lead = (hp - M.hp_at(1 - k, t)) / 100.0
                if down:
                    sig = self.strum(vo, 0.92 if main_ else 0.7, 0.013 if main_ else 0.009, up=False,
                                     dur=0.40 if main_ else 0.2, cap=1.0 if main_ else 0.6)
                    gain = 0.36 if main_ else 0.24
                else:
                    if main_ and h01_33(tq, 7) < 0.6 * (1 - hp / 100.0) ** 1.6:   # the up-strokes drop out when hurt
                        continue
                    top = vo[-min(4, len(vo)):]
                    sig = self.strum(top, 0.74 if main_ else 0.6, 0.008, up=True, dur=0.2, cap=0.6)
                    gain = 0.27 if main_ else 0.2
                if not main_:
                    sig = A.hp(sig, 350)                                        # CAPO: bright and short
                y, g, _ = self.colour(sig, t, 0.7)
                mix.place(self.bus, y, tq, gain * g, pan=self.pan)
                if main_ and down and lead > 0.04:                 # the leader: boom-chick alternating bass under it
                    r, f5 = bass_pair(ch)
                    pos = int(round(tq / T.BEAT)) % 4
                    mix.place(self.bus, self.colour(self.note(r if pos in (0, 1) else f5, 0.4, 0.7, 1.0), t)[0],
                              tq, 0.24 * g, pan=self.pan)
            elif kind == 'hit' and e[2] == k and e[7] == 'rake':
                tq = q16(t)
                r = bass_pair(T.chord_at(tq))[0]
                mix.place(self.bus, self.colour(self.note(r - 12 if r - 12 >= 40 else r, 0.3, 0.6, 1.0), t)[0],
                          tq, 0.18, pan=self.pan)
                mix.place('fx', A.thud(0.25, 180, 80, seed=int(t * 9)), tq, 0.12, pan=other_pan)   # the strings' slap
            elif kind == 'whip' and e[2] == k:                    # FINGERPICK: one picked note per whip, up the chord
                tq = q16(t)
                ch = guitar_chord(T.chord_at(tq))
                ms = list(ch[2:]) + [ch[-1] + 12, ch[-2] + 12]
                m = ms[int(e[3]) % len(ms)]
                mix.place(self.bus, self.colour(self.note(m, 0.3, 0.7, 0.9), t)[0], tq, 0.30,
                          pan=self.pan + 0.08 * (int(e[3]) - 2.5) / 2.5)
                n = int(0.03 * SR)
                mix.place('fx', A.hp(np.random.default_rng(int(t * 50)).normal(0, 1, n), 4000) * np.exp(-A.tt(n) / 0.004),
                          tq, 0.05, pan=self.pan)                  # the crack of the string whip
            elif kind == 'hit' and e[2] == k and e[7] == 'whip':  # a whip lands: a harmonic ping
                tq = q16(t)
                m = T.scale_notes(76, 93, T.chord_at(tq))[int(e[8]) % 6]
                mix.place(self.bus, A.chime(m, 0.6), tq, 0.10, pan=other_pan)
            elif kind == 'sweep' and e[2] == k:                   # BIG STRUM: one giant slow strum across the ring
                ts = q16(e[0])
                dur = e[7]
                ch = guitar_chord(T.chord_at(ts))
                mix.place(self.bus, self.strum(ch, 1.0, min(0.09, dur / 7), dur=1.0, cap=2.0), ts, 0.62, pan=self.pan)
                mix.place(self.bus, self.note(40, 1.0, 1.8, 1.0), ts, 0.40, pan=self.pan)
                mix.place('fx', A.whoosh(dur, 200, 3500), ts, 0.16)
            elif kind == 'hit' and e[2] == k and e[7] == 'sweep':
                mix.place('fx', A.thud(0.7, 90, 32), e[0], 0.55)
            elif kind == 'ability' and e[2] == k and e[4] == 'haste':    # CAPO: the clamp, a metal click and a ring
                tq = q16(t)
                n = int(0.05 * SR)
                mix.place('fx', A.hp(np.random.default_rng(8).normal(0, 1, n), 3000) * np.exp(-A.tt(n) / 0.004), tq, 0.18)
                mix.place('fx', A.chime(96, 0.4), tq, 0.06)

    def fall_apart(self, mix):
        """the strings snapping, top to bottom, and the body cracking"""
        M, A, SR = self.M, self.M.A, self.M.SR
        snap = (64, 59, 55, 50, 45, 40)
        for j, m in enumerate(snap):
            mix.place('solo', A.hp(self.note(m, 0.2, 0.5, 1.0), 200), M.P.T_HS + 0.09 * j, 0.30,
                      pan=(j - len(snap) / 2) / 6)
            n = int(0.04 * SR)
            mix.place('fx', A.hp(np.random.default_rng(40 + j).normal(0, 1, n), 3000) * np.exp(-A.tt(n) / 0.006),
                      M.P.T_HS + 0.09 * j, 0.10)
        n = int(0.3 * SR)
        mix.place('fx', A.bp(np.random.default_rng(77).normal(0, 1, n), 150, 1200) * np.exp(-A.tt(n) / 0.05), M.P.T_HS,
                  0.35)

    def stinger(self, mix, tw):
        ch = guitar_chord(self.T.CHORDS[0])
        mix.place('solo', self.strum(tuple(ch) + ((ch[-1] + 5,) if len(ch) < 6 else ()), 1.0, 0.014, dur=0.9, cap=1.8),
                  tw, 0.46)

    def solo(self, mix, t0):
        """config SOLO: a tuple is a strum (up on the off-beats), a number a picked note"""
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            if isinstance(m, (tuple, list)):
                mix.place('solo', self.strum(m, 0.9, 0.010, up=(bt % 1) > 0.1, dur=max(0.12, d * T.BEAT), cap=1.0), ts, 0.40)
            else:
                mix.place('solo', self.note(m, max(0.1, d * T.BEAT * 0.9), 0.8, 0.9), ts, 0.48)

    def reassemble(self, mix):
        """its pieces fly home as a rising run"""
        T, M = self.T, self.M
        run = [52, 57, 60, 64, 69, 72, 76, 81]
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place('solo', self.note(m, 0.2, 0.5, 0.7), tj, 0.22)
