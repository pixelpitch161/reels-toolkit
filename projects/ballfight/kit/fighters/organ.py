"""ORGAN (wine), as built in #35 (Organ vs Harp, v4): PIPE ROWS (weapon kind 'line': on every second beat a row of
organ pipes erupts out of the canvas from the organ toward the enemy, the front running out at 6 u/s, each pipe up
0.55 s, one hit per row; +length per landed row), CATHEDRAL (shield: an echoing gothic dome), PULL ALL STOPS (haste: a
row every beat), TOCCATA (slam: a forest of pipes erupts inside the target circle).

Look: the tin pipe rows rising and sinking in sequence (hex prisms, a dark mouth, an ink outline, the tops glowing wine
as they speak), their shadows from the spot, the wine glow pools on the canvas, stone dust + a dusty mark under every
third pipe and a wine light along the row; a landed row's burst, wine light, dust and canvas crack; CATHEDRAL's dome
(stained-glass tint, golden ribs, echo rings rolling out on the canvas with a heat shimmer, and its wine light from
inside lighting the scene); TOCCATA's darkening target with a flashing rim, the forest of pipes, the stomp ring, its
shimmer, crack + scorch marks; the ball thumps on every row; the champion's solo as a ring of pipes erupting round it
(taller for lower notes, a puff of dust and a wine light each); shards of tin pipe, dark case wood and gilt. PULL ALL
STOPS has no look of its own beyond the rows coming every beat (as in #35).
Sound: the Pixel Pitch organ (reed voice) plays the chords: every row is a short chord stab on the beat (#35's
balance: the chord of the bar, never held; it loses voices below 50 / 25 HP), a landed row adds the pedal + a high
accent, the leader gets the pedal an octave up; CATHEDRAL is a held full-organ chord, PULL ALL STOPS a swell through the
stops, TOCCATA a falling 16th run over the telegraph and the lowest, fullest chord on the stomp; the title blast is the
full organ pedal to mixture, the count-in a stab per count; it falls apart as clattering pipes down a diminished
arpeggio; the stinger, the toccata solo (chord symbols allowed), the reassembly run. Role 'bass' (config ROLES): every
row is a stab + the pedal note, and the pedal climbs the last chord into the title slam.
#35's voicing tables (VOICING / PEDAL, D minor) are now rules that give exactly those tables for #35's chords: the
chord's tones in G3..F#4, the root in A1..G#2.
Draw code from projects/ballfight_organ_vs_harp/styles.py (#35), copied here unchanged; the layers from #35's
render.py, the sound from #35's audio.py (the organ's part only: its choir, bell, drone, timpani are the gothic bed's)."""
import math

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, BAYER_FULL
from reelkit.shadows import proj_point
from reelkit import fx
import scene as S

from . import base

R = S.R_BALL

# ================================================================ #35 styles.py: organ pipes (line), unchanged
TIN = np.array([222, 228, 242])
TIN_DK = np.array([128, 132, 150])
MOUTH = np.array([30, 22, 34])
PIPE_DX = 0.34                       # spacing along a row
PIPE_R = 0.15
RISE, SINK = 0.07, 0.12              # s a pipe takes to shoot up / sink back


def pipe_height(x, L):
    """a row ramps up toward the enemy (a crescendo of pipes), with a little organ-front rhythm in it"""
    u = x / max(L, 1e-6)
    return 0.95 + 0.95 * u + 0.14 * math.cos(x * 7.3)


def row_pipes(line, t, t_end=None):
    """[(sx, sz, height, glow)] of one row's pipes that are out of the floor at time t (t_end: freeze at the hit-stop)"""
    out = []
    L = line['L']
    if L <= 0.05:
        return out
    n = max(1, int(L / PIPE_DX))
    for j in range(n):
        x = (j + 0.5) * L / n
        te = line['t0'] + x / line['speed']
        u = t - te
        if u < 0:
            continue
        up = min(1.0, u / RISE)
        down = 1.0 - min(1.0, max(0.0, (u - line['linger']) / SINK))
        k = min(up, down)
        if k <= 0:
            continue
        k = 1 - (1 - k) ** 2
        p = line['p0'] + line['u'] * x
        h = pipe_height(x, L) * k
        glow = math.exp(-u / 0.18)
        out.append((float(p[0]), float(p[1]), h, glow))
    return out


def pipe_faces(cx, cz, h, r=PIPE_R, n=6, y0=0.0):
    """side quads + top cap of a hex pipe in world coords: [(Q (k,3), normal, kind)]"""
    a = np.arange(n) * 2 * math.pi / n + math.pi / 6
    ring = np.stack([cx + r * np.cos(a), cz + r * np.sin(a)], 1)
    bot = S.to_world(ring[:, 0], ring[:, 1], y0)
    top = S.to_world(ring[:, 0], ring[:, 1], y0 + h)
    out = []
    for j in range(n):
        j2 = (j + 1) % n
        Q = np.array([bot[j], bot[j2], top[j2], top[j]])
        mid = (ring[j] + ring[j2]) / 2 - np.array([cx, cz])
        nw = S.to_world(mid[0], mid[1], 0.0)
        nw = nw / (np.linalg.norm(nw) + 1e-9)
        out.append((Q, nw, 'side', j))
    out.append((top, np.array([0.0, 1.0, 0.0]), 'top', -1))
    return out


def draw_pipe(fr, cam, L, shade, cx, cz, h, glow, col, r=PIPE_R):
    """one tin pipe; `col` = the owner's colour (the top glows in it as the pipe speaks)"""
    if h < 0.03:
        return
    for Q, nw, kind, j in pipe_faces(cx, cz, h, r):
        if kind == 'side':
            if np.dot(nw, cam.pos - Q.mean(0)) <= 0:
                continue
            base_ = TIN if j % 2 == 0 else TIN * 0.92
            c = shade(L, nw[None], Q.mean(0)[None], base_, rimk=0.55)[0]
            fr.poly(cam, Q, c)
        else:
            c = TIN_DK * (0.75 - 0.35 * glow) + np.array(col) * 0.75 * glow + 30 * glow
            fr.poly(cam, Q, np.clip(c, 0, 255))
    # the mouth: a dark slot a third of the way up, facing the camera
    v = cam.pos - S.to_world(cx, cz, 0.0)
    ang = math.atan2(v[2], v[0])
    d = np.array([math.cos(ang), math.sin(ang)])
    # back to the ring frame: world X = (sx - sz)/sqrt2, Z = (sx + sz)/sqrt2
    sx_d, sz_d = (d[0] + d[1]) / math.sqrt(2), (d[1] - d[0]) / math.sqrt(2)
    m = np.array([cx + sx_d * r * 1.01, cz + sz_d * r * 1.01])
    t_ = np.array([-sz_d, sx_d]) * r * 0.45
    ym, hm = min(0.32, h * 0.3), min(0.1, h * 0.12)
    Q = np.array([S.to_world(*(m - t_), ym), S.to_world(*(m + t_), ym), S.to_world(*(m + t_ * 0.6), ym + hm),
                  S.to_world(*(m - t_ * 0.6), ym + hm)])
    fr.poly(cam, Q, MOUTH)
    # an ink outline round the pipe's silhouette, so a row reads as separate pipes
    a = np.arange(6) * math.pi / 3 + math.pi / 6
    ring = np.stack([cx + r * np.cos(a), cz + r * np.sin(a)], 1)
    P = np.vstack([S.to_world(ring[:, 0], ring[:, 1], 0.0), S.to_world(ring[:, 0], ring[:, 1], h)])
    sp = cam.wp(P)
    from scipy.spatial import ConvexHull
    try:
        hull = sp[ConvexHull(sp).vertices]
        fr.draw.polygon([tuple(q) for q in hull], outline=(40, 30, 48))
    except Exception:
        pass


def pipe_shadow_quad(cx, cz, h, r=PIPE_R):
    """a vertical card through the pipe (for the spot's shadow on the canvas)"""
    return np.array([S.to_world(cx - r, cz + r, 0.0), S.to_world(cx + r, cz - r, 0.0),
                     S.to_world(cx + r, cz - r, h), S.to_world(cx - r, cz + r, h)])


# ================================================================ #35 styles.py: masks, CATHEDRAL's dome, TOCCATA's forest
def mask_of(polys):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if len(pts) >= 3:
            d.polygon([tuple(q) for q in pts], fill=255)
    return np.asarray(m, np.float32) / 255.0


def line_mask(segs, width=1):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for a, b in segs:
        d.line([tuple(a), tuple(b)], fill=255, width=width)
    return np.asarray(m, np.float32) / 255.0


GLASS_A = np.array([120, 40, 120])
GLASS_B = np.array([60, 90, 200])
GOLD = np.array([255, 206, 110])


def dome_points(cx, cz, rb, hgt, n_rib=8, m=9):
    """ribs of a pointed (gothic) dome: n_rib arcs from the base ring to the apex"""
    ribs = []
    for j in range(n_rib):
        a = 2 * math.pi * j / n_rib + 0.2
        pts = []
        for i in range(m):
            v = i / (m - 1)
            r = rb * (1 - v) ** 0.7                          # a pointed arch: steep near the top
            y = hgt * math.sin(v * math.pi / 2) ** 0.8
            pts.append(S.to_world(cx + r * math.cos(a), cz + r * math.sin(a), y))
        ribs.append(np.array(pts))
    return ribs


def draw_dome(arr, cam, cx, cz, t, k, col, echo_t0):
    """k: 0..1 strength. Stained-glass tint inside the silhouette, golden ribs, and echo rings rolling out on the canvas"""
    rb, hgt = R * 1.55, R * 3.0
    ribs = dome_points(cx, cz, rb, hgt)
    pts = np.vstack([cam.wp(r) for r in ribs])
    from scipy.spatial import ConvexHull
    try:
        hull = pts[ConvexHull(pts).vertices]
    except Exception:
        return
    mk = mask_of([hull])
    yy = np.arange(H)[:, None] * np.ones((1, W))
    y0, y1 = hull[:, 1].min(), hull[:, 1].max()
    v = np.clip((yy - y0) / max(1.0, y1 - y0), 0, 1)
    tint = GLASS_A[None, None] * (1 - v[..., None]) + GLASS_B[None, None] * v[..., None]
    tint = tint * 0.5 + np.array(col)[None, None] * 0.5
    dz = np.floor(BAYER_FULL * 0.9 + 0.5) if k > 0.6 else np.floor(BAYER_FULL * 0.9 + k)
    arr += (tint - arr) * (mk * 0.30 * k * dz)[..., None]
    segs = []
    for r in ribs:
        s = cam.wp(r)
        segs += list(zip(s[:-1], s[1:]))
    a = np.linspace(0, 2 * math.pi, 33)
    base_ = cam.wp(S.to_world(cx + rb * np.cos(a), cz + rb * np.sin(a), 0.03))
    segs += list(zip(base_[:-1], base_[1:]))
    mr = line_mask(segs)
    arr += (np.clip(GOLD * 0.8 + np.array(col) * 0.3, 0, 255)[None, None] - arr) * (mr * 0.85 * k)[..., None]


def echo_rings(t, t0, period):
    """radii (beyond the dome) + alpha of the echo rings: one rolls out every `period` s while the dome stands"""
    out = []
    if t < t0:
        return out
    j = int((t - t0) // period)
    for q in range(max(0, j - 2), j + 1):
        u = t - (t0 + q * period)
        if 0 <= u < 1.0:
            out.append((R * 1.55 + 2.2 * u, (1 - u) * 0.6))
    return out


def forest(cx, cz, rad, seed, n=13):
    """pipe positions + heights + stagger inside the slam circle (seeded per slam)"""
    rng = np.random.default_rng(seed)
    out = []
    for j in range(n):
        rr = rad * math.sqrt(rng.uniform(0.02, 1.0))
        a = rng.uniform(0, 2 * math.pi)
        out.append((cx + rr * math.cos(a), cz + rr * math.sin(a), rng.uniform(1.1, 2.5), rng.uniform(0, 0.12),
                    rng.uniform(0.14, 0.20)))
    return out


def forest_pipes(t, t_slam, cx, cz, rad, seed):
    """[(sx, sz, h, glow, r)] while the forest is up (from the slam to 0.65 s after)"""
    out = []
    for x, z, hmax, lag, r in forest(cx, cz, rad, seed):
        u = t - t_slam - lag
        if u < 0:
            continue
        up = min(1.0, u / 0.08)
        down = 1 - min(1.0, max(0.0, (u - 0.45) / 0.18))
        kk = min(up, down)
        if kk <= 0:
            continue
        out.append((x, z, hmax * (1 - (1 - kk) ** 2), math.exp(-u / 0.25), r))
    return out


# ================================================================ #35 render.py: the organ's layers
DUST_COL = [(178, 160, 150), (150, 128, 126), (208, 190, 176), (176, 120, 136)]     # stone dust with a hint of wine
CASE = np.array([92, 52, 40])                                                       # the dark case wood (shards)


class Look(base.Look):
    shards = ['pipe'] * 7 + ['case'] * 6 + ['gold'] * 2           # #35's organ shatter (after the 24 glass shards)
    spark_big = ('pipe', 'slam', 'wave', 'shockwave')              # #35's BIG_HITS
    spark_flash = ('pipe', 'slam')
    flash_hits = ('pipe', 'slam', 'wave', 'shockwave')
    fringe_hits = ('pipe', 'slam', 'wave')
    block_burst = (220, 230, 255)                                  # a hit on CATHEDRAL: #35's block burst + spray
    block_spark = ((255, 226, 170), 12, 2.6, 0.3)

    def setup(self):
        P, k = self.P, self.k
        self.lines, by_id = [], {}                                 # its pipe rows, rebuilt from the run (#35 sim.hazards)
        for e in P.EV:
            if e[1] == 'line' and e[2] == k:
                d = dict(id=e[3], owner=e[2], t0=e[0], p0=np.array(e[4:6]), u=np.array(e[6:8]), L=e[8], speed=e[9],
                         linger=e[10], width=e[11], t_hit=None)
                self.lines.append(d)
                by_id[e[3]] = d
            elif e[1] == 'hit' and e[2] == k and e[7] == 'pipe' and e[8] in by_id:
                by_id[e[8]]['t_hit'] = e[0]
        self.line_t = np.array([l['t0'] for l in self.lines]) if self.lines else np.zeros(0)
        self.slam_ev = [(e[0], e[2], e[3], e[4]) for e in P.EV if e[1] == 'slam' and e[2] == k]   # (t_slam, owner, cx, cz)
        abl = self.cfg['abilities']
        self.forest_rad = abl['low'].get('radius', 1.4)
        sh = [a for a in abl.values() if a['kind'] == 'shield']
        self.shield = (sh[0]['name'], sh[0]['dur_beats'] * P.T.BEAT) if sh else None   # CATHEDRAL
        self.shield_on = [e[0] for e in P.EV if e[1] == 'ability' and e[2] == k and self.shield and e[3] == self.shield[0]]
        self.solo_n = P.T.SOLO.get(self.name, [])
        self.reg = self.cfg.get('register', (36, 84))
        self._pipes = (None, None)

    # ---------------------------------------------------------------- state
    def state(self, kk, st, t, phase, gl=0.0):
        P, k = self.P, self.k
        if kk != k or phase != 'fight' or not len(self.line_t):    # the organ ball thumps on every row
            return
        tt = min(t, P.T_KO)
        j = np.searchsorted(self.line_t, tt, 'right') - 1
        if j >= 0:
            st['glow'] = 0.42 * math.exp(-(tt - self.line_t[j]) / 0.10)

    def shield_k(self, st):
        """0..1 strength of CATHEDRAL's look (fades in over 0.12 s and out over the last 0.2 s)"""
        if st['shield'] <= 0 or self.shield is None:
            return 0.0
        dur = self.shield[1]
        return float(min(1.0, st['shield'] / 0.2, (dur - st['shield']) / 0.12 + 0.2))

    def solo_pipes(self, t):
        """the organ's solo: a ring of pipes erupts around the champion on every solo note (taller for lower notes)"""
        P = self.P
        if P.WIN != self.k:
            return []
        out = []
        for j, (bt, m, d) in enumerate(self.solo_n):
            t0 = P.T_SOLO + bt * P.T.BEAT
            if t0 <= t < t0 + 0.7 and t0 < P.T.T_REASSEMBLE - 0.05:     # (the notes the Sound plays)
                out.append((t0, j, m))
        return out

    def pipes_at(self, t):
        """[(sx, sz, h, glow, owner, r)]: its pipes out of the floor (the rows, the TOCCATA forest, the solo ring)"""
        if self._pipes[0] == t:
            return self._pipes[1]
        P, T = self.P, self.P.T
        out = []
        tc = P.fx_clock(t)
        if tc >= T.T_FIGHT and len(self.line_t):
            j1 = int(np.searchsorted(self.line_t, tc, 'right'))
            for l in self.lines[max(0, j1 - 8):j1]:
                for sx, sz, h, g in row_pipes(l, tc):
                    out.append((sx, sz, h, g, l['owner'], PIPE_R))
        for ts, owner, cx, cz in self.slam_ev:
            if ts - 0.05 <= tc < ts + 1.0:
                for sx, sz, h, g, r in forest_pipes(tc, ts, cx, cz, self.forest_rad, int(ts * 1000)):
                    out.append((sx, sz, h, g, owner, r))
        lo, hi = self.reg
        for t0, j, m in self.solo_pipes(t):                        # the champion's solo ring
            c = P.fighter(self.k, t)['p']
            a = 2 * math.pi * j / 8.0 + 0.4
            u = t - t0
            k = min(1.0, u / 0.07) * (1 - min(1.0, max(0.0, (u - 0.45) / 0.2)))
            mm = m[1] if isinstance(m, tuple) else m
            h = (0.7 + 1.6 * (hi - mm) / float(hi - lo)) * (1 - (1 - k) ** 2)
            out.append((c[0] + 1.25 * math.cos(a), c[1] + 1.25 * math.sin(a), h, math.exp(-u / 0.2), self.k, 0.14))
        self._pipes = (t, out)
        return out

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit' and e[7] in ('pipe', 'slam'):
            return {'pipe': 1.1, 'slam': 2.5}[e[7]]
        return super().shake(e)                                    # TOCCATA's stomp (the 'slam' event): 1.6

    def hit_glow(self, e):
        if e[7] in self.spark_big:
            return 12, 0.4
        return None

    def fringe_size(self, e):
        return (18, 1.6) if e[7] == 'pipe' else (26, 2.5)

    def light(self, L, t):
        """CATHEDRAL glows from inside: a wine point light in the dome lights the canvas, ropes and crowd"""
        P = self.P
        if self.shield is None or not (P.T.T_FIGHT <= t < P.T_HS):
            return
        st = P.fighter(self.k, t)
        sk = self.shield_k(st)
        if sk > 0:
            L.point(S.to_world(st['p'][0], st['p'][1], R * 1.6), P.COL[self.k] / 255.0 * 0.9 * sk, 2.0)

    # ---------------------------------------------------------------- effects (reelkit.fx), as #35 built them
    def fx_setup(self):
        """the rows: each pipe kicks up dust as it bursts out; light runs along the row; the floor stays dusty a while"""
        P = self.P
        for l in self.lines:
            n = max(1, int(l['L'] / PIPE_DX))
            for j in range(n):
                x = (j + 0.5) * l['L'] / n
                te = l['t0'] + x / l['speed']
                p = l['p0'] + l['u'] * x
                P.PA.emit(te, S.to_world(p[0], p[1], 0.06), 'dust', n=2, col=DUST_COL, vel=S.to_world(*(l['u'] * 0.3), 0.0))
                if j % 3 == 1:
                    P.FL.add(te, S.to_world(p[0], p[1], 1.0), P.FL_COL[self.k], radius=1.2, peak=0.6, decay=0.18)
                if j % 3 == 0:
                    P.MK.add(te, 'dust', p[0], p[1], size=0.24, col=(196, 180, 172), life=2.4, seed=l['id'] * 31 + j,
                             strength=0.7)

    def fx_event(self, e):
        P = self.P
        if e[1] == 'hit' and e[2] == self.k:
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = S.to_world(x, z, R)
            if how == 'pipe':                                      # a row lands: burst, wine light, dust, a crack
                P.BU.add(e[0], pos, P.COL[att], 1.3)
                P.FL.add(e[0], pos, P.FL_COL[att], radius=1.6, peak=0.9, decay=0.28)
                P.PA.emit(e[0], S.to_world(x, z, 0.1), 'dust', n=10, col=DUST_COL, speed=(0.6, 1.6))
                P.MK.add(e[0], 'crack', x, z, size=0.55, n=5, life=4.0, seed=int(e[0] * 100))
            elif how == 'slam':
                P.BU.add(e[0], pos, P.COL[att], 2.0)
        elif e[1] == 'block' and e[2] == self.k:                   # CATHEDRAL blocks a hit: stone dust
            P.PA.emit(e[0], S.to_world(e[4], e[5], R), 'dust', n=5, col=DUST_COL)

    def fx_after(self):
        """TOCCATA's forest: dust everywhere, a cracked crater, a big wine light (after the event pass, as #35)"""
        P = self.P
        for ts, owner, cx, cz in self.slam_ev:
            c = S.to_world(cx, cz, 0.1)
            P.FL.add(ts, S.to_world(cx, cz, 1.2), P.FL_COL[owner], radius=2.4, peak=1.2, decay=0.5)
            P.PA.emit(ts, c, 'dust', n=30, col=DUST_COL, spread=180, speed=(0.6, 1.8), scale=1.3)
            P.MK.add(ts, 'crack', cx, cz, size=1.6, n=8, life=7.0, seed=int(ts * 10))
            P.MK.add(ts, 'scorch', cx, cz, size=1.2, col=(120, 40, 70), life=7.0, seed=int(ts * 10) + 1, strength=0.6)

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'dust', n=30, spread=180, speed=(0.8, 2.2), col=DUST_COL)

    def fx_end(self):
        """the organ's solo: every solo pipe kicks up a little dust and lights up wine (after the K.O., as #35)"""
        P = self.P
        for t0, j, m in [(P.T_SOLO + bt * P.T.BEAT, j, m) for j, (bt, m, d) in enumerate(self.solo_n)] \
                if P.WIN == self.k else []:
            if t0 < P.T.T_REASSEMBLE - 0.05:
                c = P.fighter(self.k, t0)['p']
                a = 2 * math.pi * j / 8.0 + 0.4
                q = (c[0] + 1.25 * math.cos(a), c[1] + 1.25 * math.sin(a))
                P.PA.emit(t0 - P.HITSTOP, S.to_world(q[0], q[1], 0.06), 'dust', n=3, col=DUST_COL)
                P.FL.add(t0 - P.HITSTOP, S.to_world(q[0], q[1], 1.2), P.FL_COL[self.k], radius=1.4, peak=0.7, decay=0.25)

    # ---------------------------------------------------------------- the canvas
    def shadows_free(self, F, spot):
        return [proj_point(pipe_shadow_quad(sx, sz, h, r), spot) for sx, sz, h, g, owner, r in self.pipes_at(F.t)
                if h > 0.05]

    def canvas(self, Ic, C):
        P, k = self.P, self.k
        for psx, psz, h, g, owner, r in self.pipes_at(C.t):        # the pipes' glow pools while they speak
            if g > 0.15:
                m_ = (np.abs(C.sxp - psx) < 0.5) & (np.abs(C.szp - psz) < 0.5)
                if m_.any():
                    dd_ = np.hypot(C.sxp[m_] - psx, C.szp[m_] - psz)
                    Ic[m_] += (np.clip(1 - dd_ / 0.45, 0, 1) * g * 0.35)[:, None] * (P.COL[owner] / 255.0 + 0.15)[None]
        st = C.fs[k]                                               # CATHEDRAL's echo rings
        if self.shield is not None and st['shield'] > 0 and C.in_fight:
            past = [x for x in self.shield_on if x <= C.t]
            if past:
                for rr_, a_ in echo_rings(C.t, past[-1], 2 * P.T.BEAT):
                    rd = np.hypot(C.sxp - st['p'][0], C.szp - st['p'][1])
                    band = np.clip(1 - np.abs(rd - rr_) / 0.08, 0, 1)
                    Ic += (band * a_ * 0.7)[:, None] * (P.COL[k] / 255.0 + 0.25)[None]
        return Ic

    def canvas_after(self, Ic, C):
        P = self.P
        sl = P.RUN['SLAMS'][C.i] if C.in_fight else []
        for owner, cx, cz, t0, t_slam, rad in sl:                  # TOCCATA's target + the stomp ring
            if owner != self.k:
                continue
            rd = np.hypot(C.sxp - cx, C.szp - cz)
            if C.t < t_slam:
                u = (C.t - t0) / (t_slam - t0)
                m = rd < rad
                Ic[m] *= 1 - 0.55 * u
                ring = np.abs(rd - rad) < 0.05
                Ic[ring] += (P.COL[owner] / 255.0) * (0.4 + 0.5 * (math.sin(C.t * 40) > 0))
            else:
                u = (C.t - t_slam) / 0.6
                band = np.clip(1 - np.abs(rd - (rad + 1.6 * u)) / 0.2, 0, 1) * (1 - u)
                Ic += band[:, None] * np.array([0.9, 0.9, 1.0])[None]
        return Ic

    # ---------------------------------------------------------------- the painter: pipes, the dome
    def items_ball(self, F, st):
        sk = self.shield_k(st)
        if sk <= 0:
            return []
        dz = F.cam.depth(S.to_world(st['p'][0], st['p'][1], R))[0]
        return [(dz - 0.05, ('dome', sk))]

    def items_free(self, F):
        return [(F.cam.depth(S.to_world(pp[0], pp[1], 0.0))[0], ('pipe', pp)) for pp in self.pipes_at(F.t)]

    def draw_item(self, F, payload):
        kind, obj = payload
        if kind == 'pipe':
            psx, psz, h, g, owner, r = obj
            draw_pipe(F.fr, F.cam, F.L, self.P.shade, psx, psz, h, g, self.P.COL[owner], r)
        elif kind == 'dome':
            p = F.fs[self.k]['p']
            arr, depth = F.fr.array(), F.fr.depth()
            draw_dome(arr, F.cam, p[0], p[1], F.t, obj, self.P.COL[self.k], 0.0)
            F.fr.set_array(arr, depth)

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'pipe':                                         # a length of tin pipe
            q4 = [np.array([sx_, sz_]) + d * 0.22 * a1 + nn * 0.06 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], TIN * (0.8 + 0.2 * math.cos(rot * 2)), 1.0)
            return arr
        col = {'gold': GOLD, 'case': CASE}[kind]                   # case wood chips, gilt
        sz = size * (1.2 if kind == 'case' else 0.9)
        tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
               np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        shade_k = 0.75 + 0.25 * math.cos(rot * 2)
        P.layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
        return arr

    # ---------------------------------------------------------------- lens, glows
    def shimmer_after(self, img, cam, t):
        P = self.P
        tc = P.fx_clock(t)
        for ts, owner, cx, cz in self.slam_ev:                     # TOCCATA's shock ring
            u = (tc - ts) / 0.6
            if 0 <= u < 1:
                img = fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), self.forest_rad + 1.6 * u, 0.35, 3.0, t,
                                       k=1 - u)
        if self.shield is not None and P.T.T_FIGHT <= t < P.T_HS:  # CATHEDRAL's echo rings
            st = P.fighter(self.k, t)
            if st['shield'] > 0:
                past = [x for x in self.shield_on if x <= t]
                if past:
                    for rr_, a_ in echo_rings(t, past[-1], 2 * P.T.BEAT):
                        img = fx.shimmer_world(img, cam, S.to_world(st['p'][0], st['p'][1], 0.05), rr_, 0.2, 1.6, t,
                                               k=a_ / 0.6)
        return img

    def glows(self, F, drawn):
        for psx, psz, h, g, owner, r in self.pipes_at(F.t):        # the speaking pipe tops glow
            if g > 0.3:
                s_ = F.cam.wp(S.to_world(psx, psz, h)[None])[0]
                F.fr.glow(s_[0], s_[1], 5, tuple(self.P.COL[owner] * 0.7 + 70), 0.35 * g)

    def focus_points(self, t, st):
        return [(S.to_world(sx, sz, h), 'pipe top') for sx, sz, h, g, owner, r in self.pipes_at(t)]


# ===================================================================================================== sound (#35)
def h01_35(*xs):
    """#35's deterministic hash in [0, 1) (which voices a hurt organ's stab keeps: part of the score)"""
    v = 0
    for x in xs:
        v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
    return ((v * 48271) % 2147483647) / 2147483647.0


class Sound(base.Sound):
    bus_gain, send = 2.0, 0.55                                     # #35's organ bus (x2: short stabs every 2 beats)
    band = (180, 1200)                                             # the stabs G3-F#4 (+ the reed's 2nd / 3rd harmonics)

    # ---------------------------------------------------------------- voicings (#35's tables, as rules)
    def voicing(self, ch):
        """the chord's tones in G3..F#4: #35's VOICING (Dm 57 62 65, Bb 58 62 65, Gm 55 58 62, A 57 61 64)"""
        return tuple(self.T.chord_tones(55, 66, ch))

    def pedal(self, ch):
        """the chord's root in A1..G#2: #35's PEDAL (Dm 38, Bb 34, Gm 43, A 33)"""
        return self.T.root_above(ch, 33)

    def full(self, ch):
        """the full organ, pedal to mixture: the root in three octaves under the voicing + its lower two an octave up
        (#35's title blast on Dm: 26 38 50 57 62 65 69 74)"""
        r = self.T.root_above(ch, 48)
        vo = self.voicing(ch)
        return list(dict.fromkeys([r - 24, r - 12, r] + list(vo) + [vo[0] + 12, vo[1] + 12]))   # (A minor: no doubled A3)

    def toccata_run(self):
        """a falling 16th run over an octave from the key's fifth, the leading tone raised (#35 in D minor: A5 G F E D
        C# Bb A)"""
        T = self.T
        pcs = [(T.KEY_PC + s) % 12 for s in T.SCALE]
        if (T.KEY_PC + 10) % 12 in pcs:
            pcs[pcs.index((T.KEY_PC + 10) % 12)] = (T.KEY_PC + 11) % 12
        top = 72 + ((T.KEY_PC + 7) - 72) % 12                     # the fifth in C5..B5 (D minor: A5 = 81)
        return [m for m in range(top - 12, top + 1) if m % 12 in pcs][::-1]

    def chord(self, ms, dur=0.17, vel=0.85, cap=None):
        """#35's organ_chord: the Pixel Pitch organ's notes summed, balanced by 1 / n^0.6"""
        M = self.M
        cap = cap or round(dur + 0.22, 2)
        return sum(M.note(M.CFG[self.k]['pp'], m, dur, cap, vel) for m in ms) / max(1.0, len(ms) ** 0.6)

    @staticmethod
    def clank(seed, vel=1.0):
        """a tin pipe clattering on the canvas (#35)"""
        from reelkit import audio as A
        n = int(0.4 * A.SR)
        t = A.tt(n)
        rng = np.random.default_rng(seed)
        x = np.zeros(n)
        for r in (1.0, 2.76, 5.4, 8.9):
            x += np.sin(2 * np.pi * 620 * r * (1 + rng.uniform(-0.05, 0.05)) * t) * np.exp(-t * (8 + 4 * r))
        return x * vel * 0.5

    # ---------------------------------------------------------------- the score
    def title_blast(self, mix, t):
        notes = self.T.TITLE_HITS.get(self.name) or self.full(self.T.CHORDS[0])
        mix.place(self.bus, self.chord(list(notes), 0.5, 1.0, cap=1.2), t, 0.40)

    def count_in(self, mix):
        T = self.T
        for tc, txt in T.COUNT[:3]:                                # a stab on each count (on the count-in's chord)
            mix.place(self.bus, self.chord(self.voicing(T.CHORDS[1]), 0.18, 0.9), tc, 0.30)

    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        A, EV, q16, S16 = M.A, M.EV, M.q16, T.S16
        side = -1.0 if k == 0 else 1.0                             # #35's organ sat left: pan toward its own side
        bass = self.role == 'bass'
        ab = M.CFG[k]['abilities']
        for e in EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if len(e) < 3 or e[2] != k or kind not in ('line', 'hit', 'ability', 'slam'):
                continue
            tq = q16(t)
            ch = T.chord_at(tq)
            if kind == 'line':                                     # a pipe row = a chord stab on the beat
                hp = M.hp_at(k, t) / 100.0
                vo = list(self.voicing(ch))
                if hp < 0.5:
                    vo = vo[:2] if h01_35(t, 1) < 0.6 else vo[1:]
                if hp < 0.25:
                    vo = vo[:1]
                sig, g, lead = self.colour(self.chord(vo, 0.17, 0.85), t, 0.8)
                mix.place(self.bus, sig, tq, 0.34 * g, pan=0.15 * side)
                if bass:                                           # the bass role: the pedal under every stab
                    mix.place(self.bus, self.colour(self.chord([self.pedal(ch)], 0.3, 0.9), t, 0.6, low=True)[0], tq,
                              0.30 * g, pan=0.1 * side)
                if lead > 0.04:                                    # the leader: the pedal an octave up under it
                    mix.place(self.bus, self.colour(self.chord([self.pedal(ch) + 12], 0.2, 0.7), t)[0], tq, 0.16 * g,
                              pan=0.1 * side)
            elif kind == 'hit' and e[7] == 'pipe':                 # a row lands: the pedal + a high accent
                pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
                tl = tq + S16 if q16(t) < t else tq
                vo = self.voicing(ch)
                if bass:                                           # (the pedal is already under every row)
                    sig, g, _ = self.colour(self.chord([vo[-1] + 12], 0.22, 1.0), t, 0.6)
                    mix.place(self.bus, sig, tl, 0.30 * g / 2 ** 0.6, pan=pan)
                else:
                    sig, g, _ = self.colour(self.chord([self.pedal(ch), vo[-1] + 12], 0.22, 1.0), t, 0.6)
                    mix.place(self.bus, sig, tl, 0.30 * g, pan=pan)
                mix.place('fx', A.thud(0.3, 130, 55, seed=int(t * 7)), tq, 0.14)
            elif kind == 'ability':
                name = e[3]
                vo = self.voicing(ch)
                if name == ab['unlock1']['name']:                  # CATHEDRAL: a held full-organ chord under the dome
                    mix.place(self.bus, self.chord([self.pedal(ch), vo[0] - 12] + list(vo) + [vo[-1] + 12], 2 * T.BEAT,
                                                   0.8, cap=2 * T.BEAT + 0.6), tq, 0.20)
                elif name == ab['unlock2']['name']:                # PULL ALL STOPS: a swell, the chord climbing the stops
                    for j, m in enumerate([self.pedal(ch)] + list(vo) + [vo[-1] + 12]):
                        mix.place(self.bus, self.chord([m], 2 * T.BEAT - j * S16, 0.75, cap=2 * T.BEAT + 0.3),
                                  tq + j * S16, 0.12 + 0.03 * j)
                elif name == ab['low']['name']:                    # TOCCATA: a falling 16th run over the telegraph
                    for j, m in enumerate(self.toccata_run()):
                        mix.place(self.bus, self.chord([m, m - 12], 0.11, 0.9), tq + j * S16, 0.22)
            elif kind == 'slam':                                   # TOCCATA's stomp: the lowest, fullest chord
                ts = e[0]
                vo = self.voicing(T.chord_at(ts))
                mix.place(self.bus, self.chord([T.KEY_ROOT - 12, T.KEY_ROOT, vo[0] - 12] + list(vo) + [vo[-1] + 12],
                                               0.6, 1.0, cap=1.4), ts, 0.34)
                mix.place('fx', A.thud(0.7, 90, 30), ts, 0.55)
                M.bed.boom(mix, ts, 0.6)                           # (#35: its bed's timpani + bell)

    def fall_apart(self, mix):                                     # the pipes clattering down a diminished arpeggio
        M, T = self.M, self.T
        for j in range(7):
            mix.place('fx', self.clank(j, 1.0 - 0.08 * j), M.P.T_HS + 0.09 * j, 0.20, pan=(j - 3) / 6)
            mix.place(self.bus, self.chord([T.KEY_ROOT + 24 - 3 * j], 0.08, 0.6), M.P.T_HS + 0.09 * j, 0.10)

    def stinger(self, mix, tw):
        mix.place('solo', self.chord(self.full(self.T.CHORDS[0])[1:], 0.9, 1.0, cap=1.6), tw, 0.30)

    def solo(self, mix, t0):
        """#35's toccata solo: single notes doubled an octave down; ('dim', r) a diminished seventh on r, (chord, r) that
        chord on r with r an octave down under it; stops at the reassembly"""
        from ..episode import QUALITY
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts >= T.T_REASSEMBLE - 0.05:                        # a late K.O. leaves less room: stop at the reassembly
                break
            dur = min(d * T.BEAT * 0.92, T.T_REASSEMBLE - ts)
            if isinstance(m, tuple):
                kind_, r = m
                if kind_ == 'dim':
                    ms = [r, r + 3, r + 6, r + 9]
                else:
                    q = kind_[2:] if len(kind_) > 1 and kind_[1] in '#b' else kind_[1:]
                    ms = [r - 12] + [r + i for i in QUALITY.get(q, (0, 3, 7))]
            else:
                ms = [m, m - 12]
            mix.place('solo', self.chord(ms, dur, 0.9, cap=round(dur + 0.3, 2)), ts, 0.34)

    def reassemble(self, mix):                                     # its pipes fly home: a rising run up the tonic chord
        T, M = self.T, self.M
        r = T.root_above(T.CHORDS[0], 48)
        run_ = T.chord_tones(r, r + 36, T.CHORDS[0])[:8]
        for j, m in enumerate(run_):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run_))
            mix.place(self.bus, self.chord([m], 0.1, 0.7), tj, 0.16)

    def last_bar(self, mix):
        """the bass role: the pedal climbs the last chord (the dominant) into the title slam, root to octave"""
        if self.role != 'bass':
            return
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        r = self.pedal(ch)
        climb = T.chord_tones(r, r + 12, ch)[:4]
        side = -1.0 if self.k == 0 else 1.0
        for bt in range(4):
            m = climb[bt % len(climb)]
            mix.place(self.bus, self.chord([m, m + 12], 0.35, 0.9), t_last + bt * T.BEAT, 0.34, pan=0.1 * side)
