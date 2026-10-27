"""VIOLA (teal), as built in #42 Viola vs Violin: COUNTING RESTS (weapon kind 'rests', the only charger: it rests for 3
beats under a giant quarter rest with the count "1 2 3" above it, half damage taken, then on beat 4 a giant whole note
appears and rolls at the enemy like a heavy wheel for a whole bar, crushing once and flattening projectiles; every
landed note makes the next bigger), DOUBLE STOP (two notes at once: at the enemy and where it is heading), MELODY AT
LAST (for 2 bars the rests shrink to one beat and the music hands the viola the melody), VIOLA JOKE (a rimshot, the
whole crowd bursts out laughing, the enemy freezes grey for a beat while the biggest whole note drops on it).

Look (#42): the whole note as the engraved hollow oval standing up as a wheel (ink with a teal-lit rim, its tilted hole
spinning as it rolls, bobbing on its long axis), its shadow, a soft teal glow, ink flecks, dust and canvas cracks when it
crushes; the rest pool pulsing under the resting ball on each count; the rest glyph + the count (crisp, over the
scene); rising eighth notes off the ball during MELODY AT LAST (and on the champion's solo notes); VIOLA JOKE's target
on the canvas, the falling note, the landing shimmer, the crowd's laugh and the frozen grey enemy. Its sprite uses a
wider crop (the bridge entry sizes the viola x1.12). It shatters into strings, wood chips (its darker varnish), the
bridge and the scroll.
Sound (#42, the Pixel Pitch viola: a darker bowed voice a fifth below the violin): the inner harmony a fifth (or so)
under the line it accompanies (config LINES: its own if set, else the enemy's; chord tones C3-G4); the counting rests
are soft muted plucks on 1-2-3; every whole note is one long viola note with a heavy bow attack (DOUBLE STOP: a second
string a sixth up); a crush is a low string crunch + thud (+ a crash and an 808 drop for the joke's note); MELODY AT
LAST: its own singing lead for those 2 bars; VIOLA JOKE: ba-dum-tss + the crowd laughing, then the open strings ff.
Hurt it thins and dirties, keeping its low strings; the leader doubles an octave down.
Draw code from projects/ballfight_viola_vs_violin/styles.py, layers from its render.py, sound from its audio.py
(#42), copied unchanged; motifs written in C minor are moved to the episode's key (no change in C)."""
import math

import numpy as np
from reelkit.config import W, H, FPS, BAYER_FULL, SAFE, ss, ease
from reelkit.lighting import shade
from reelkit.ui import label
from reelkit.font import ADV
from reelkit import fx
import scene as S

from . import base
from .violin import pizz, scratch, key_shift, draw_shard

R = S.R_BALL

# ---------------------------------------------------------------- the whole note (#42 styles.py, unchanged)
INK = np.array([44, 44, 66], float)            # the notehead's ink (a deep blue-black, so the teal rim reads)
NOTE_A, NOTE_B = 1.0, 0.68                     # outer oval (x the note's radius): wider than tall, as engraved
HOLE_A, HOLE_B = 0.36, 0.56                    # the hole: a narrow oval ...
HOLE_TILT = math.radians(-32)                  # ... leaning like "/" (thick at 10 and 4 o'clock)
THICK = 0.22                                   # slab thickness (x radius)
DRAW = 1.3                                     # drawn half-width x the hit radius
K_PTS = 20


def _oval(a, b, tilt, phi):
    th = np.linspace(0, 2 * math.pi, K_PTS + 1)[:-1]
    x, y = a * np.cos(th), b * np.sin(th)
    c, s = math.cos(tilt - phi), math.sin(tilt - phi)
    return np.stack([x * c - y * s, x * s + y * c], 1)


def support(phi):
    """half-height of the outer oval turned by phi (the wheel's centre height above the canvas, x radius)"""
    return math.sqrt((NOTE_A * math.sin(phi)) ** 2 + (NOTE_B * math.cos(phi)) ** 2)


def note_axes(cam, lean=0.25):
    """the notehead's plane: screen-right (a) and an up axis (b) between the camera's up and the world's up, so the
    oval faces the viewer like the engraved glyph with a little lean that shows the top of its rim; n = its normal"""
    a = cam.M[0].copy()
    a[1] = 0.0
    a /= np.linalg.norm(a)
    b = (1 - lean) * cam.M[1] + lean * np.array([0.0, 1.0, 0.0])
    b -= a * (b @ a)
    b /= np.linalg.norm(b)
    n = np.cross(a, b)
    if n @ (cam.pos - cam.target) < 0:
        n = -n
    return a, b, n


def note_centre(c, r, phi, lift, b):
    """world centre of a note at ring point c: high enough that its lowest point touches the canvas"""
    hc = b[1] * r * support(phi) + lift
    return S.to_world(c[0], c[1], 0.0) + np.array([0.0, hc, 0.0]) - np.array([b[0], 0.0, b[2]]) * r * support(phi) * 0.0


def note_quads(cam, c, r, phi, lift=0.0, scale=1.0):
    """world quads [(Q (4, 3), normal (3,), kind)] of a whole note standing on the canvas at ring point c"""
    r = r * scale * DRAW
    a, b, nrm = note_axes(cam)
    C = note_centre(c, r, phi, lift, b)
    O = _oval(NOTE_A, NOTE_B, 0.0, phi) * r
    I = _oval(HOLE_A, HOLE_B, HOLE_TILT, phi) * r
    th = THICK * r / 2

    def Wp(pq, w):
        return C + a * pq[0] + b * pq[1] + nrm * w
    out = []
    for j in range(K_PTS):
        j1 = (j + 1) % K_PTS
        for sgn in (1, -1):
            out.append((np.array([Wp(O[j], sgn * th), Wp(O[j1], sgn * th), Wp(I[j1], sgn * th), Wp(I[j], sgn * th)]),
                        nrm * sgn, 'face'))
        mo = (O[j] + O[j1]) / 2
        rad = a * mo[0] + b * mo[1]
        rad = rad / (np.linalg.norm(rad) + 1e-9)
        out.append((np.array([Wp(O[j], th), Wp(O[j1], th), Wp(O[j1], -th), Wp(O[j], -th)]), rad, 'rim'))
        mi = (I[j] + I[j1]) / 2
        radi = -(a * mi[0] + b * mi[1])
        radi = radi / (np.linalg.norm(radi) + 1e-9)
        out.append((np.array([Wp(I[j], th), Wp(I[j1], th), Wp(I[j1], -th), Wp(I[j], -th)]), radi, 'hole'))
    return out


def draw_note(fr, cam, L, shade, c, r, phi, col, lift=0.0, scale=1.0, glow=0.0):
    """paint a whole note (far quads first); col = the owner's colour (the rim light); glow 0..1 brightens the rim"""
    quads = note_quads(cam, c, r, phi, lift, scale)
    ctr = np.array([Q.mean(0) for Q, n, k in quads])
    d = cam.depth(ctr)
    order = np.argsort(-d)
    nrm = np.array([n for Q, n, k in quads])
    base_ = np.array([INK if k == 'face' else (INK * 0.6 + col * 0.45 if k == 'rim' else INK * 0.7)
                      for Q, n, k in quads])
    cols = shade(L, nrm, ctr, base_, rimk=0.7)
    for j in order:
        Q, n, kind = quads[j]
        cc = cols[j]
        if kind == 'rim':
            cc = cc + col * (0.35 + 0.45 * glow) + 20
        elif kind == 'face':
            cc = cc + col * 0.08 * (1 + 2 * glow)
        fr.poly(cam, Q, np.clip(cc, 0, 255))
    a, b, nrm = note_axes(cam)                     # a lit edge along the top of the outer oval (the spot catches it)
    rr = r * scale * DRAW
    C = note_centre(c, rr, phi, lift, b)
    O = _oval(NOTE_A, NOTE_B, 0.0, phi) * rr
    top = [C + a * p + b * q + nrm * THICK * rr / 2 for p, q in O if q > 0.25 * rr]
    if len(top) > 1:
        sp = cam.wp(np.array(top))
        order = np.argsort(sp[:, 0])
        fr.draw.line([tuple(sp[j]) for j in order], fill=tuple(int(v) for v in np.clip(col * 0.6 + 110, 0, 255)), width=1)


def note_shadow(cam, c, r, phi, lift=0.0):
    """the note's outer oval as a world polygon (for the spot's shadow on the canvas)"""
    a, b, nrm = note_axes(cam)
    r = r * DRAW
    C = note_centre(c, r, phi, lift, b)
    O = _oval(NOTE_A, NOTE_B, 0.0, phi) * r
    return np.array([C + a * p + b * q for p, q in O[::2]])


# ---------------------------------------------------------------- pixel glyphs (rest, eighth note) (#42 styles.py)
REST_GLYPH = ["..#....", "..##...", "...##..", "....##.", "...###.", "..###..", ".###...", ".##....", "..##...",
              "...##..", "...###.", "..#.##.", ".#.....", ".#.....", "..##...", "....#.."]      # the quarter rest
EIGHTH_GLYPH = ["..#..", "..##.", "..#.#", "..#..", "..#..", "###..", "###.."]               # a flagged eighth


def glyph(rows, scale=1):
    g = np.array([[ch == '#' for ch in r] for r in rows], bool)
    return g.repeat(scale, 0).repeat(scale, 1)


def stamp_glyph(img, g, cx, cy, col, alpha=1.0, outline=(16, 12, 26), shadow=True):
    """stamp a boolean glyph centred at (cx, cy) on a uint8 / float image with a 1 px dark outline (+ drop shadow)"""
    from reelkit.font import dilate
    H_, W_ = img.shape[:2]
    o = dilate(g, 1)
    gh, gw = o.shape
    x0, y0 = int(round(cx - gw / 2)), int(round(cy - gh / 2))
    layers = ([(o, (0, 0, 0), 1, 2, 0.45)] if shadow else []) + [(o, outline, 0, 0, 1.0)]
    off = (o.shape[0] - g.shape[0]) // 2
    layers.append((np.pad(g, ((off, off), (off, off))), col, 0, 0, 1.0))
    for m, c, ox, oy, a0 in layers:
        xa, ya = max(0, x0 + ox), max(0, y0 + oy)
        xb, yb = min(W_, x0 + ox + gw), min(H_, y0 + oy + gh)
        if xb <= xa or yb <= ya:
            continue
        sub = m[ya - y0 - oy:yb - y0 - oy, xa - x0 - ox:xb - x0 - ox]
        reg = img[ya:yb, xa:xb].astype(np.float32)
        a = alpha * a0
        reg[sub] = reg[sub] * (1 - a) + np.array(c, float) * a
        img[ya:yb, xa:xb] = reg.astype(img.dtype)
    return x0, y0, gw, gh


REST_G = glyph(REST_GLYPH, 2)
EIGHTH_G = glyph(EIGHTH_GLYPH, 1)
INKY = [(40, 40, 60), (70, 76, 96), (24, 22, 34)]         # ink flecks off a crushing note
WOOD = np.array([120, 50, 28])                             # its varnish (darker than the violin's)
STRING_COL = (230, 226, 214)
LAUGH = 1.9                                                # s the crowd laughs after the rimshot


def render_padded(name, radius_px, yaw_deg, roll_deg=0.0, pad=1.2):
    """sprites.render_sprite with a wider crop: the VIOLA's bridge size (x1.12, so it reads bigger than the violin)
    reaches past the standard crop at close-up sizes; same camera, rasteriser and outline (#42)"""
    import sprites as SP
    import piano_lowpoly as PL
    RW, RH = PL.RW, PL.RH
    m = int(math.ceil(radius_px * pad)) + 3
    img = np.zeros((RH, RW, 3), np.uint8)
    zb = np.zeros((RH, RW))
    gb = np.zeros((RH, RW), np.int8)
    cam = SP._Cam(yaw_deg, radius_px, m + 0.5, m + 0.5, roll_deg)
    PL.render_geo(SP._geo(name), cam, img, zb, gb, scale=1.0)
    PL.outline(img, zb, gb)
    n = 2 * m + 1
    return np.dstack([img[:n, :n], (gb[:n, :n] > 0).astype(np.uint8) * 255])


# ===================================================================================================== look (#42)
class Look(base.Look):
    shards = ['string'] * 4 + ['wood'] * 7 + ['bridge'] * 2 + ['scroll']
    spark_flash = ('note',)
    spark_kinds = ('crushed',)                       # a projectile flattened by a rolling note: a small grey spray
    rope_kinds = {}

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.STUNS = [(e[0], e[2], e[4]) for e in EV if e[1] == 'stun' and e[3] == k]          # t, victim, dur
        self.JOKES = [(e[0], e[2], e[3], e[4], e[5], e[6]) for e in EV if e[1] == 'joke' and e[2] == k]
        self.MELODIES = [(e[0], e[2], e[3]) for e in EV if e[1] == 'melody' and e[2] == k]
        self.NOTE_EV = [(e[0], e[2]) for e in EV if e[1] == 'note' and e[8] == 0 and e[2] == k]
        self._notes = None
        self.CAP_BAND = SAFE[1] + 34                                                  # the top captions' band

    @property
    def NOTES(self):
        """its whole notes' paths (built on first use: it needs the renderer's camera)"""
        if self._notes is None:
            self._notes = self._notes_build()
        return self._notes

    @property
    def CAP_LIVE(self):
        P = self.P
        return [(a, b) for a, b, _ in P.CAP_ABIL[0] + P.CAP_ABIL[1] + P.CAP_KO + P.CAP_WINS + P.CAP_COMMENT]

    def _notes_build(self):
        """per note id: frames, positions, rolling direction and turn (from the recorded path), radius, times"""
        P, T = self.P, self.T
        D = {}
        if 'NOTES' not in P.RUN:
            return D
        for i, rows in enumerate(P.RUN['NOTES']):
            for owner, x, z, r, t0, t1, tl, nid, kind, hit in rows:
                if int(owner) != self.k:
                    continue
                d = D.setdefault(int(nid), dict(owner=int(owner), r=float(r), t0=float(t0), t1=float(t1), tl=float(tl),
                                                kind=int(kind), f=[], p=[], hit=None))
                d['f'].append(i)
                d['p'].append((x, z))
                if hit and d['hit'] is None:
                    d['hit'] = T.T_FIGHT + i / FPS
        aims = {e[3]: e[6] for e in P.EV if e[1] == 'note' and e[2] == self.k}         # id -> launch aim
        NOTE_MEAN = (NOTE_A + NOTE_B) / 2
        right = P.camera(0.0).M[0]
        for nid, d in D.items():
            Pp = np.array(d['p'], float)
            a0 = aims.get(nid, 0.0)
            U = np.zeros_like(Pp)
            u = np.array([math.cos(a0), math.sin(a0)])
            phi = np.zeros(len(Pp))
            for j in range(len(Pp)):
                if j > 0:
                    step = Pp[j] - Pp[j - 1]
                    ds = float(np.hypot(*step))
                    if ds > 1e-4:
                        u = step / ds
                    sw = S.to_world(step[0], step[1], 0.0) - S.to_world(0.0, 0.0, 0.0)      # rolling right on screen
                    sgn = 1.0 if sw @ right >= 0 else -1.0                                   # turns it clockwise
                    phi[j] = phi[j - 1] + sgn * ds / (d['r'] * NOTE_MEAN)
                U[j] = u
            d['P'], d['U'], d['PHI'] = Pp, U, phi
            d['f'] = np.array(d['f'])
        return D

    def notes_at(self, t):
        """[(id, c, u, phi, r, lift, scale, glow, owner, kind)] of its whole notes on screen at t"""
        P, T = self.P, self.T
        out = []
        if t < T.T_FIGHT or t >= P.T_HS + 0.6:
            return out
        tt = min(t, P.T_KO)
        fi = P.sim_i(tt)
        fade = 1.0 - ss(P.T_HS, P.T_HS + 0.6, t)
        for nid, d in self.NOTES.items():
            if not (d['f'][0] <= fi <= d['f'][-1]):
                continue
            j = int(fi - d['f'][0])
            c, u, phi = d['P'][j], d['U'][j], d['PHI'][j]
            lift, glow = 0.0, 0.0
            if d['tl'] > d['t0'] + 1e-6 and tt < d['tl']:                          # dropped from above (VIOLA JOKE)
                s_ = (tt - d['t0']) / (d['tl'] - d['t0'])
                lift = 4.6 * (1 - s_ * s_)
                phi = 0.4 * math.sin(6 * s_)
            sc = ease(min(1.0, (tt - d['t0']) / 0.10)) if d['tl'] <= d['t0'] + 1e-6 else 1.0
            sc *= 1.0 - ss(d['t1'] - 0.14, d['t1'], tt)                             # a whole note's length: then gone
            if d['hit'] is not None and 0 <= tt - d['hit'] < 0.25:
                glow = 1 - (tt - d['hit']) / 0.25
            if tt - d['t0'] < 0.15:
                glow = max(glow, 1 - (tt - d['t0']) / 0.15)
            sc *= fade
            if sc > 0.03:
                out.append((nid, c, u, phi, d['r'], lift, sc, glow, d['owner'], d['kind']))
        return out

    def now(self, F):
        key = ('viola_notes', self.k)
        if key not in F.__dict__:
            F.__dict__[key] = self.notes_at(F.t)
        return F.__dict__[key]

    def rest_state(self, t):
        """(resting 0/1, count, melody) from the record; nothing before the fight or after the K.O."""
        P = self.P
        if 'REST' not in P.RUN or not (self.T.T_FIGHT <= t < P.T_KO):
            return 0, 0, 0
        r = P.RUN['REST'][P.sim_i(t), self.k]
        return int(r[0]), int(r[1]), int(r[2])

    def stun_k(self, kk, t):
        tt = min(t, self.P.T_KO)
        for t0, v, d in self.STUNS:
            if v == kk and t0 <= tt < t0 + d + 0.15:
                return min(1.0, (tt - t0) / 0.06) * (1 - ss(t0 + d, t0 + d + 0.15, tt))
        return 0.0

    def laugh(self, t):
        """the crowd's laugh after VIOLA JOKE's rimshot (0..1)"""
        best = 0.0
        for t0, k, v, tl, x, z in self.JOKES:
            if t0 <= t < t0 + LAUGH + 0.4:
                best = max(best, min(1.0, (t - t0) / 0.12) * (1 - ss(t0 + LAUGH, t0 + LAUGH + 0.4, t)))
        return best

    def melody_k(self, t):
        tt = min(t, self.P.T_KO)
        for t0, kk, until in self.MELODIES:
            if t0 <= tt < until:
                return min(1.0, (tt - t0) / 0.2) * (1 - ss(until - 0.25, until, tt))
        return 0.0

    # ---------------------------------------------------------------- the balls
    def state(self, kk, st, t, phase, gl=0.0):
        if phase != 'fight':
            return
        if kk == self.k:                                   # MELODY AT LAST: the viola glows on the beat
            st['glow'] = 0.45 * self.melody_k(t) * (0.6 + 0.4 * self.T.beat_pulse(t, 0.2))
        else:
            st['viola_freeze'] = self.stun_k(kk, t)        # VIOLA JOKE: frozen, it goes grey and still

    def ball(self, kk, st, t, glass, roll):
        frz = st.get('viola_freeze', 0.0)
        if frz > 0:
            glass = glass * (1 - 0.3 * frz) + np.array([168, 176, 196]) * 0.3 * frz
        return glass, roll

    def ball_tint(self, kk, st, t, col):
        frz = st.get('viola_freeze', 0.0)
        if frz > 0:
            col = col * (1 - frz) + np.array([150, 150, 168]) * frz
        return col

    def sprite_lit(self, kk, st, t, lit):
        frz = st.get('viola_freeze', 0.0)
        if frz > 0:
            g_ = lit.mean(-1, keepdims=True)
            lit = lit * (1 - 0.7 * frz) + (g_ * 0.85 + np.array([20, 24, 40])) * 0.7 * frz
        return lit

    def sprite(self, r, yaw, roll):
        return render_padded(self.cfg['pp'], r, yaw, roll)

    def title_closeup(self, r):
        yc, sw, roll = self.cfg['view']
        return render_padded(self.cfg['pp'], r, float(yc), float(roll))

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit' and e[7] == 'note':
            return 2.5 if e[8] == 2 else 1.3                # VIOLA JOKE's note lands
        return super().shake(e)

    def rope(self, e):
        if e[1] == 'joke':                                  # the big note lands: the ropes shake then
            return (e[4], 0.9)
        return super().rope(e)

    def spark(self, e):
        n, sp, life = (16, 3.2, 0.3) if e[7] == 'note' else (5, 1.8, 0.18)
        return self.P.COL[e[2]] * 0.6 + 100, n, sp, life

    def flashes(self, e):
        return e[7] == 'note' and e[8] == 2

    def fringe(self, e):
        return e[7] == 'note' and e[8] == 2

    def hit_glow(self, e):
        return (14, 0.35) if e[7] == 'note' else None

    def fx_event(self, e):
        """#42: a whole note crushes (the joke's: the big one), a note appears (a teal flash, ink motes), the joke's
        target glows while the note falls"""
        P = self.P
        t_ = e[0]
        up = S.to_world
        if e[1] == 'hit' and e[7] == 'note':
            att, x, z = e[2], e[5], e[6]
            pos = up(x, z, R)
            big = e[8] == 2
            P.BU.add(t_, pos, P.COL[att], 2.0 if big else 1.1)
            P.FL.add(t_, up(x, z, 0.4), P.FL_COL[att], radius=2.6 if big else 1.6, peak=1.0 if big else 0.7,
                     decay=0.45 if big else 0.28)
            P.PA.emit(t_, up(x, z, 0.1), 'dust', n=26 if big else 10, spread=180, speed=(0.6, 1.8), scale=1.3 if big else 1.0)
            P.PA.emit(t_, pos, 'chips', n=8 if big else 4, col=INKY)
            P.MK.add(t_, 'crack', x, z, size=1.4 if big else 0.55, n=8 if big else 4, life=6.0 if big else 3.0,
                     seed=int(t_ * 100))
            if big:
                P.MK.add(t_, 'scorch', x, z, size=1.2, col=(40, 44, 64), life=6.0, seed=int(t_ * 10) + 1, strength=0.5)
        elif e[1] == 'note':
            x, z = e[4], e[5]
            if e[9] <= t_ + 1e-6:                                  # rolling at once (a dropped one shows on landing)
                P.FL.add(t_, up(x, z, 0.5), P.FL_COL[e[2]], radius=1.3, peak=0.45, decay=0.25)
                P.PA.emit(t_, up(x, z, 0.3), 'motes', n=5, col=[tuple(P.COL[e[2]] * 0.5 + 120)])
        elif e[1] == 'joke':
            k, tl, x, z = e[2], e[4], e[5], e[6]
            P.FL.add(t_ + 0.05, up(x, z, 2.0), P.FL_COL[k], radius=2.2, peak=0.6, decay=max(0.3, tl - t_))

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'chips', n=24, spread=180, speed=(0.8, 2.2), col=[tuple(WOOD), tuple(WOOD * 1.3)])

    # ---------------------------------------------------------------- the canvas
    def shadows_free(self, F, spot):
        from reelkit.shadows import proj_point
        return [proj_point(note_shadow(F.cam, c, r * sc, phi, lift), spot)
                for nid, c, u, phi, r, lift, sc, glow, owner, kind in self.now(F)]

    def canvas(self, Ic, C):
        """the resting viola's pool (it pulses on each count; brighter during MELODY AT LAST)"""
        P, T, t = self.P, self.T, C.t
        st = C.fs[self.k]
        resting, count, mel = self.rest_state(t)
        mk_ = self.melody_k(t)
        if C.in_fight and (resting or mk_ > 0):
            rd = np.hypot(C.sxp - st['p'][0], C.szp - st['p'][1])
            ph = ((t - T.T_FIGHT) % T.BEAT) / T.BEAT
            ring_r = R * (1.15 + 0.7 * ph)
            band = np.clip(1 - np.abs(rd - ring_r) / 0.06, 0, 1) * (1 - ph) * (0.6 if resting else 0.4)
            pool = np.clip(1 - rd / (R * 1.5), 0, 1) * 0.10 * (1 + mk_)
            Ic += (band * 0.8 + pool)[:, None] * (P.COL[self.k] / 255.0 + 0.1)[None]
        return Ic

    def canvas_after(self, Ic, C):
        """VIOLA JOKE: the target under the falling note, then the landing ring"""
        P, t = self.P, C.t
        for t0, k, v_, tl, x, z in self.JOKES:
            if t0 <= t < tl and t < P.T_HS:
                u = (t - t0) / (tl - t0)
                rd = np.hypot(C.sxp - x, C.szp - z)
                rad = self.cfg['abilities']['low']['size'] * float(P.RUN['BL'][P.sim_i(t0), k])
                m = rd < rad * (0.4 + 0.6 * u)
                Ic[m] *= 1 - 0.5 * u
                ring = np.abs(rd - rad) < 0.05
                Ic[ring] += (P.COL[k] / 255.0) * (0.4 + 0.5 * (math.sin(t * 40) > 0))
            elif 0 <= t - tl < 0.6 and t < P.T_HS:
                u = (t - tl) / 0.6
                rd = np.hypot(C.sxp - x, C.szp - z)
                band = np.clip(1 - np.abs(rd - (1.2 + 1.8 * u)) / 0.2, 0, 1) * (1 - u)
                Ic += band[:, None] * np.array([0.6, 0.95, 0.9])[None]
        return Ic

    # ---------------------------------------------------------------- the painter pass
    def items_free(self, F):
        return [(F.cam.depth(S.to_world(nt[1][0], nt[1][1], 0.5 * nt[4] * nt[6] + nt[5]))[0], ('note', nt))
                for nt in self.now(F)]

    def draw_item(self, F, payload):
        what, nt = payload
        nid, c, u, phi, r, lift, sc, glow, owner, nkind = nt
        draw_note(F.fr, F.cam, F.L, shade, c, r, phi, self.P.COL[owner], lift=lift, scale=sc,
                  glow=max(glow, 0.5 * self.melody_k(F.t)))

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        return draw_shard(self.P, arr, cam, kind, p, rot, size, curl, WOOD)

    # ---------------------------------------------------------------- lens, glows
    def shimmer_after(self, img, cam, t):
        """the big note's landing ripples the air"""
        for t0, k, v, tl, x, z in self.JOKES:
            if 0 <= t - tl < 0.6 and t < self.P.T_HS:
                u = (t - tl) / 0.6
                img = fx.shimmer_world(img, cam, S.to_world(x, z, 0.05), 1.2 + 1.8 * u, 0.35, 3.0, t, k=1 - u)
        return img

    def glows(self, F, drawn):
        """a soft teal glow round each rolling note"""
        for nt in self.now(F):
            nid, c, u, phi, r, lift, sc, glow, owner, kind = nt
            s_ = F.cam.wp(S.to_world(c[0], c[1], 0.5 * r * sc + lift)[None])[0]
            F.fr.glow(s_[0], s_[1], 10 + 14 * r * sc, tuple(self.P.COL[owner] * 0.7 + 60), 0.10 + 0.25 * glow)

    # ---------------------------------------------------------------- the rests, counts and rising eighths (crisp)
    def caption_up(self, t):
        if not hasattr(self, '_cap_live'):
            self._cap_live = self.CAP_LIVE
        return any(a - 0.05 <= t < b + 0.1 for a, b in self._cap_live)

    @staticmethod
    def _safe_x(x, w, y, h):
        """keep a small label inside the safe box and out of the button column"""
        x1 = 300 if y + h > 290 else SAFE[2]
        return int(min(max(x, SAFE[0] + 1), x1 - w - 1))

    def overlay(self, img, F, drawn):
        P, T, k, t = self.P, self.T, self.k, F.t
        if F.card_closed or k not in drawn:
            return
        resting, count, mel = self.rest_state(t)
        (sx, sy), rp = drawn[k]
        if resting and count > 0:
            # the giant quarter rest over the ball (it settles in on each counted beat, sways a little)
            t_beat = T.T_FIGHT + math.floor((t - T.T_FIGHT) / T.BEAT + 1e-6) * T.BEAT
            u = min(1.0, (t - t_beat) / 0.08)
            gy = sy - rp - REST_G.shape[0] / 2 - 3 - 3 * (1 - u)
            col = np.clip(P.COL[k] * 0.55 + 110, 0, 255)
            stamp_glyph(img, REST_G, sx + 1.5 * math.sin(2 * math.pi * t / T.BAR), gy, col, alpha=0.4 + 0.6 * u,
                        outline=tuple(int(v) for v in P.DARK[k] * 0.45))
            # the small count above it: "1", "1 2", "1 2 3" (the newest bright)
            digits = [str(j + 1) for j in range(count)]
            txt_w = len(digits) * (ADV + 2) - 2
            y_ = int(round(gy - REST_G.shape[0] / 2 - 11))
            x_ = self._safe_x(int(round(sx - txt_w / 2)), txt_w, y_, 7)
            if y_ < self.CAP_BAND and self.caption_up(t):            # under a caption: beside the rest glyph instead
                y_ = int(round(gy - 3))
                x_ = int(round(sx + REST_G.shape[1] / 2 + 4))
                if x_ + txt_w > (300 if y_ + 7 > 290 else SAFE[2]) - 1:
                    x_ = int(round(sx - REST_G.shape[1] / 2 - 4 - txt_w))
            if SAFE[1] <= y_ <= SAFE[3] - 8 and not (y_ < self.CAP_BAND and self.caption_up(t)):
                for j, dg in enumerate(digits):
                    c_ = (255, 250, 236) if j == count - 1 else tuple(int(v) for v in np.clip(P.COL[k] * 0.5 + 90, 0, 255))
                    label(img, dg, x_ + j * (ADV + 2), y_, c_, 1)
        mk = self.melody_k(t)
        rising = [te for te, kk in self.NOTE_EV if 0 <= min(t, P.T_KO) - te < 0.9 and mk > 0]
        if t >= P.T_HS and k == P.WIN:                               # the champion's solo: eighths on each note
            rising = [s_ for s_ in P.SOLO_T if 0 <= t - s_ < 0.9]
        for j, te in enumerate(rising):
            age = (min(t, P.T_KO) if t < P.T_HS else t) - te
            x_ = sx + (8 + 3 * j) * (1 if (int(te * 10) % 2) else -1) + 4 * math.sin(age * 7)
            y_ = sy - rp * 0.4 - 26 * age
            stamp_glyph(img, EIGHTH_G, x_, y_, tuple(int(v) for v in np.clip(P.COL[k] * 0.5 + 125, 0, 255)),
                        alpha=1 - ss(0.5, 0.9, age), shadow=False)

    def focus_points(self, t, st):
        """its whole notes (a dropped one from when it is within 1.5 of its rolling height) (#42's focus check)"""
        out = []
        cam = self.P.camera(t)
        for nid, c, u, phi, r, lift, sc, glow, owner, kind in self.notes_at(t):
            if lift > 1.5:
                continue
            for Q, n, kd in note_quads(cam, c, r, phi, lift, sc)[::6]:
                out.append((Q.mean(0), 'whole note'))
        return out


# ===================================================================================================== sound (#42)
def addp(*xs):
    n = max(len(x) for x in xs)
    return sum(np.pad(np.asarray(x, float), (0, n - len(x))) for x in xs)


def sub808(A, m0, m1, dur, slide=0.0, vel=1.0):
    """the 808 (#42's drill bed): the joke's big note lands on one"""
    SR = A.SR
    n = int((dur + 0.05) * SR)
    t = A.tt(n)
    f0, f1 = A.mtof(m0), A.mtof(m1)
    u = np.clip((t - (dur - slide)) / max(slide, 1e-3), 0, 1) if slide > 0 else np.zeros(n)
    f = f0 * (f1 / f0) ** (u * u * (3 - 2 * u)) * (1 + 0.5 * np.exp(-t / 0.02))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR)
    x = np.tanh(2.2 * x) / np.tanh(2.2)
    env = np.clip(t / 0.004, 0, 1) * np.clip((dur + 0.05 - t) / 0.05, 0, 1) * (0.75 + 0.25 * np.exp(-t / 0.15))
    return A.lp(x, 900) * env * vel


def tom(A, f0=200.0, dec=0.18, vel=1.0):
    SR = A.SR
    n = int(0.45 * SR)
    t = A.tt(n)
    f = f0 * (1 + 0.6 * np.exp(-t / 0.03))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / dec)
    x += 0.3 * A.bp(np.random.default_rng(int(f0)).normal(0, 1, n), 300, 3000) * np.exp(-t / 0.02)
    return x * vel


def rimshot(A, S16):
    """ba-dum-tss: two toms (high, low + the kick) and a ride / crash 'tss'"""
    SR = A.SR
    n = int(1.6 * SR)
    out = np.zeros(n)

    def put(x, t, g):
        a = int(t * SR)
        b = min(n, a + len(x))
        out[a:b] += x[:b - a] * g
    put(addp(A.snare(0.08) * 0.6, tom(A, 230, 0.14)), 0.0, 0.9)
    put(addp(tom(A, 150, 0.22), A.kick(0.8, 0.25, 120, 50)[:int(0.4 * SR)] * 0.7), 1.5 * S16, 1.0)
    nt = int(1.2 * SR)
    tt_ = A.tt(nt)
    tss = A.hp(np.random.default_rng(77).normal(0, 1, nt), 5500) * np.exp(-tt_ / 0.35)
    tss += 0.12 * sum(np.sin(2 * np.pi * f * tt_) for f in (5100, 6650, 8230)) * np.exp(-tt_ / 0.5)
    put(tss, 3 * S16, 0.55)
    return out


def ha(A, f0, seed, dur=0.13, vowel=((800, 1.0), (1200, 0.6), (2600, 0.25))):
    """one 'ha': a breathy voiced burst (a pulse train through the vowel's formants) with an aspirated onset"""
    SR = A.SR
    n = int(dur * SR)
    t = A.tt(n)
    rng = np.random.default_rng(seed)
    f = f0 * (1 + 0.06 * np.exp(-t / 0.04)) * (1 - 0.08 * t / dur)
    ph = np.cumsum(f) / SR
    pulse = (2 * (ph % 1.0) - 1) ** 3
    noise = rng.normal(0, 1, n)
    src = pulse * np.clip((t - 0.015) / 0.01, 0, 1) + 0.35 * noise
    x = sum(g * A.bp(src, fc * 0.82, fc * 1.18) for fc, g in vowel)
    env = np.clip(t / 0.012, 0, 1) * np.exp(-np.maximum(t - 0.03, 0) / (dur * 0.45))
    return x * env


def crowd_laugh(A, dur=2.1, n_voices=34, seed=55):
    """the whole crowd bursts out laughing: many voices, each a run of 'ha's; returns stereo (n, 2)"""
    SR = A.SR
    n = int(dur * SR)
    out = np.zeros((n, 2))
    rng = np.random.default_rng(seed)
    for v in range(n_voices):
        f0 = rng.uniform(140, 330) if v % 3 else rng.uniform(260, 420)      # men, women, a few shrieks
        t = rng.uniform(0.0, 0.28)
        rate = rng.uniform(0.15, 0.21)
        pan = rng.uniform(-0.8, 0.8)
        amp = rng.uniform(0.5, 1.0)
        j = 0
        while t < dur - 0.2:
            x = ha(A, f0 * (1 - 0.03 * j), int(seed * 1000 + v * 37 + j), rng.uniform(0.10, 0.15))
            g = amp * math.exp(-t / (dur * 0.7)) * (1.0 if j < 6 else 0.7)
            a = int(t * SR)
            b = min(n, a + len(x))
            out[a:b, 0] += x[:b - a] * g * (1 - pan) * 0.5
            out[a:b, 1] += x[:b - a] * g * (1 + pan) * 0.5
            t += rate * (1 + 0.04 * j) + rng.uniform(-0.02, 0.02)
            j += 1
    return out / math.sqrt(n_voices) * 2.2


class Sound(base.Sound):
    bus_gain, send = 1.7, 0.4
    band = (120, 1500)                                       # #42's band check: the inner voice C3-G4 + harmonics
    colour_args = dict(cut0=70, cut_span=300, drive=4.5)    # it keeps its low strings as it thins
    buses = {'laugh': (1.0, 0.3)}                            # VIOLA JOKE's crowd
    TITLE = (48, 55, 63)                                     # #42: a dark chord on the C string (C3 G3 Eb4)
    LEAD = [(0, 67, 1.0), (1, 70, 0.5), (1.5, 68, 0.5), (2, 67, 1.5), (3.5, 65, 0.5), (4, 63, 1.0), (5, 65, 0.75),
            (5.75, 67, 0.25), (6, 62, 1.0), (7, 60, 1.0)]    # MELODY AT LAST's line (#42, in C minor)
    FALL = (67, 63, 60, 55, 51, 48)                          # a groaning slide down (#42)
    REASSEMBLE = (48, 55, 60, 63, 67, 72)                    # its pieces fly home (#42)

    def __init__(self, M, k):
        super().__init__(M, k)
        self.side = -1.0 if k == 0 else 1.0                  # #42: the left corner (pans -0.2 / -0.15 / -0.1 / -0.25)

    def viola(self, m, dur, vel=0.85, accent=0.0):
        """a bowed viola note with an optional heavy bow attack (rosin crunch)"""
        M = self.M
        x = M.note(M.CFG[self.k]['pp'], m, round(dur, 3), round(dur + 0.3, 2), round(vel, 1))
        if accent > 0:
            s = scratch(M.A, 0.07, int(m * 7))[:len(x)]
            x = x.copy()
            x[:len(s)] += accent * 0.25 * s
        return x

    def voice(self, m, dur, vel=0.85):
        return self.viola(m, dur, vel)

    def cello(self, m, dur, vel=0.85):
        return self.M.note('CELLO', m, round(dur, 3), round(dur + 0.2, 2), round(vel, 1))

    def inner(self, t):
        """the inner voice: a chord tone a fifth (or so) under the line it accompanies, in its warm range (C3-G4)"""
        T = self.T
        if self.name in T.LINES:
            return T.line_at(self.name, t)
        ref = T.line_at(self.M.NAMES[self.enemy], t)
        c = T.chord_at(t)
        tgt = ref - 7
        cands = T.chord_tones(48, 67, c)
        return min(cands, key=lambda m: (abs(m - tgt), m))

    def fit(self, m, chord, lo=48, hi=79):
        """nudge a lead note onto the key's scale; in harmonic minor the raised 7th falls back to the natural one
        over chords that don't hold it (#42: over Cm / Ab / Fm the B becomes Bb)"""
        T = self.T
        pcs = [(T.KEY_PC + s) % 12 for s in T.SCALE]
        if tuple(T.SCALE) == (0, 2, 3, 5, 7, 8, 11):
            lead = (T.KEY_PC + 11) % 12
            if lead not in T.CHORD_PCS[chord]:
                pcs = [p if p != lead else (T.KEY_PC + 10) % 12 for p in pcs]
        while m % 12 not in pcs:
            m -= 1
        return max(lo, min(hi, m))

    def melody_windows(self):
        T, M = self.T, self.M
        out = []
        for e in M.EV:
            if e[1] == 'melody' and e[2] == self.k:
                s = math.ceil((e[0] + 1e-6) / T.BEAT) * T.BEAT              # from the beat after it fires
                out.append((s, min(e[3], M.T_KO)))
        return out

    def title_blast(self, mix, t):
        T = self.T
        sh = key_shift(T)
        ms = T.TITLE_HITS.get(self.name, tuple(m + sh for m in self.TITLE))
        gains = (0.42, 0.36, 0.30, 0.26, 0.22, 0.20)
        for j, m in enumerate(ms):
            mix.place(self.bus, self.viola(m, 0.8, 1.0, accent=0.5), t, gains[min(j, 5)], pan=0.2 * self.side)

    def count_in(self, mix):
        """the viola counts it in: a pizz on each count (the count-in chord)"""
        T = self.T
        ms = T.chord_tones(55, 66, T.CHORDS[T.FIGHT_BAR - 1])[:3]
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place(self.bus, pizz(self.M.A, ms[j % len(ms)], 0.9), tc, 0.22, pan=0.2 * self.side)

    def play(self, mix):
        M, T, A, k = self.M, self.T, self.M.A, self.k
        S16, q16 = T.S16, M.q16
        side = self.side
        mel_win = self.melody_windows()

        def in_melody(t):
            return any(a <= t < b for a, b in mel_win)
        notes = M.P.LOOKS[k].NOTES
        thin = M.Thinner(9)
        r0 = 48 + T.KEY_PC                                               # its open-string fortissimo, on the tonic
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'rest' and e[2] == k:                             # the counting: a soft muted pluck on 1-2-3
                tq = q16(t)
                m = self.inner(tq) - (12 if e[3] == 1 else 0)
                mix.place(self.bus, pizz(A, max(48, m), 0.7, muted=True), tq, 0.22 + 0.03 * e[3], pan=0.2 * side)
            elif kind == 'note' and e[2] == k:
                nkind, t_land, t1 = e[8], e[9], e[10]
                ts = q16(t_land)
                if nkind != 2 and in_melody(ts):
                    continue                                             # the lead line plays these (below)
                slot = round(ts / S16)
                if nkind == 0 and not thin.keep(slot, M.hp_at(k, ts) / 100.0):
                    continue
                dur = max(0.3, min(T.BAR, t1 - t_land) * 0.96)
                m = self.inner(ts)
                if nkind == 2:                                           # VIOLA JOKE: the open strings, fortissimo
                    for mm, g in ((r0, 0.50), (r0 + 7, 0.42), (r0 + 12, 0.26)):
                        mix.place(self.bus, self.viola(mm, dur, 1.0, accent=0.8), ts, g, pan=0.15 * side)
                    continue
                sig, g, lead = self.colour(self.viola(m, dur, 0.9, accent=0.45), ts, 0.8)
                gain = 0.52 if nkind == 0 else 0.38
                mix.place(self.bus, sig, ts, gain * g, pan=0.2 * side)
                if nkind == 1:                                           # DOUBLE STOP: a second string a sixth above
                    m2 = min(T.chord_tones(m + 7, m + 10, T.chord_at(ts)) or [m + 7])
                    mix.place(self.bus, self.colour(self.viola(m2, dur, 0.85, accent=0.3), ts)[0], ts, 0.26 * g,
                              pan=0.25 * side)
                elif lead > 0.04:                                        # the leader: doubled an octave down
                    mix.place(self.bus, self.colour(self.viola(max(48, m - 12), dur, 0.7), ts)[0], ts, 0.16 * g,
                              pan=0.2 * side)
            elif kind == 'crush' and e[2] == k:                          # a crush: a low string crunch + thud
                tq = q16(t)
                big = any(N['kind'] == 2 for nid, N in notes.items() if nid == e[3])
                mix.place('fx', A.thud(0.5 if big else 0.32, 110, 36, seed=int(t * 13)), tq, 0.42 if big else 0.26)
                mix.place('cellos', self.cello(36, 0.12, 1.0), tq, 0.20 if big else 0.12)
                mix.place(self.bus, scratch(A, 0.12, int(t * 17)), tq, 0.10 if big else 0.06, pan=0.2 * side)
                if big:
                    mix.place('drums', A.crash(2.0), tq, 0.12)
                    mix.place('bass808', sub808(A, 24, 24, 0.9), tq, 0.70)
            elif kind == 'ability' and e[2] == k:
                tq = q16(t)
                if e[4] == 'double_stop':                                # two strings dug in at once
                    mix.place(self.bus, scratch(A, 0.1, int(t * 29)), tq, 0.10, pan=0.2 * side)
                elif e[4] == 'joke':                                     # ba-dum-tss + the crowd laughing
                    mix.place('fx', rimshot(A, S16), tq, 0.42)
                    lg = crowd_laugh(A, 2.1)
                    mix.place('laugh', lg[:, 0], tq + 3 * S16 + 0.05, 0.52, pan=-0.6)
                    mix.place('laugh', lg[:, 1], tq + 3 * S16 + 0.05, 0.52, pan=0.6)
                    mix.place('fx', A.whoosh(T.BEAT, 200, 3000, rise=False), tq + 0.1, 0.10)    # the note falling
        sh = key_shift(T)
        for a, b in mel_win:                                             # MELODY AT LAST: it sings the lead
            prev = None
            for bt, m, dl in self.LEAD:
                ts = a + bt * T.BEAT
                if ts >= b - 0.02:
                    break
                mm = self.fit(m + sh, T.chord_at(ts + 0.01), 52, 76)
                x = self.viola(mm, dl * T.BEAT * 0.97, 0.95, accent=0.3 if prev is None or abs(mm - prev) > 2 else 0.1)
                sig, g, _ = self.colour(x, ts, 0.5)
                mix.place(self.bus, sig, ts, 0.50 * g, pan=0.1 * side)
                mix.place(self.bus, self.colour(self.viola(max(48, mm - 12), dl * T.BEAT * 0.97, 0.7), ts)[0], ts, 0.14,
                          pan=0.2 * side)
                prev = mm
            mix.place('fx', A.whoosh(0.5, 300, 4000), a - 0.25, 0.06)

    def fall_apart(self, mix):
        M = self.M
        sh = key_shift(self.T)
        for j, m in enumerate(self.FALL):
            mix.place(self.bus, self.viola(m + sh, 0.12, 0.8), M.P.T_HS + 0.1 * j, 0.24, pan=(j - 2.5) / 6)

    def stinger(self, mix, tw):
        """the tonic chord rung out on the low strings (#42: C3 G3 Eb4 G4)"""
        T = self.T
        r = 48 + T.KEY_PC
        third = T.SCALE[2]                         # the key's third (minor in #42)
        for m, g in ((r, 0.44), (r + 7, 0.36), (r + 12 + third, 0.30), (r + 19, 0.26)):
            mix.place('solo', self.viola(m, 1.1, 1.0, accent=0.6), tw, g)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            mix.place('solo', self.viola(m, d * T.BEAT * 0.95, 0.95, accent=0.3), ts, 0.52)

    def reassemble(self, mix):
        T, M = self.T, self.M
        sh = key_shift(T)
        for j, m in enumerate(self.REASSEMBLE):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / 6)
            mix.place(self.bus, pizz(M.A, m + sh, 0.7), tj, 0.20, pan=0.2 * self.side)

    def last_bar(self, mix):
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        ms = T.chord_tones(55, 67, T.CHORDS[-1])[:4]
        for bt in range(4):
            mix.place(self.bus, self.viola(ms[bt % len(ms)], 0.3, 0.9), t_last + bt * T.BEAT, 0.20, pan=0.2 * self.side)
