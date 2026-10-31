"""BASS DRUM (gunmetal), new in #56 Bass Drum vs Tuba: the only roller. STEAMROLLER (every bar it tips onto its side and
rolls across the ring like a wheel at the enemy, a dust trail telegraphing the line; the first contact is a huge BOOM;
roll speed and drum size grow per landed boom), HEARTBEAT (two quick booms, lub-dub: back-to-back shockwaves), DRUM ROLL
(it rolls nonstop for a bar, zig-zagging, with a thundering roll), 1812 OVERTURE (five cannon booms land across the ring,
one a beat).

Look: the new Pixel Pitch bass drum (bassdrum_lowpoly: built from Commons photos; gunmetal shell, plain white heads,
chrome rods, a felt mallet). As it tips it squashes and keels over; rolling, it turns to face the camera head-on and
spins like a wheel (the mallet left behind), drawn bigger as it grows. The line is chalked in dust before it goes (a
dashed trail, fading in while it tips), it leaves a tread of churned canvas and dust puffs behind it, and each BOOM is a
gunmetal-white flash, a burst, a dust ring and a crack in the canvas. HEARTBEAT is two thick rings, one right behind the
other; 1812 OVERTURE marks each cannon's landing spot with a pulsing target, then a smoke-and-scorch blast. It shatters
into pieces of white head, dark hoop and chrome rods. VS card: the drum head's rings and the tension rods, gunmetal-lit.
Sound: it IS the band's bass drum (the bed's own bass drums are off: config DRUMLINE): a kick on every beat on the
chord's root in its own voice (the boom / mallet / roll of bassdrum_lowpoly), off-beat pickups while it leads, thinning
and dirtying as its HP drops; every roll is a 32nd-note roll swelling into the BOOM of its impact; HEARTBEAT two booms;
DRUM ROLL a thundering roll for its whole bar; 1812 OVERTURE fires its five cannons as five tuned booms playing the
opening of "God Save the Tsar!" (A. Lvov, 1833: the anthem Tchaikovsky's 1812 Overture (1880) sets in its finale under
the cannons; both public domain; scale degrees 5 6 6 5 3, from hymnary.org's incipit of the tune RUSSIAN HYMN) in the
episode's relative major, doubled by the pit's timpani and marimba with tubular bells ringing over it (the finale's
chimes), resolving to the tonic on the beat after the fifth."""
import math

import numpy as np
from reelkit.config import BAYER_FULL
import scene as S

from . import base
from ._reeds import _mask, blend, INK, WHITE

R = S.R_BALL
DUST = [(176, 156, 126), (140, 122, 98), (214, 200, 172)]
DUSTC = np.array([196, 178, 146], float)
HEADC = np.array([238, 236, 228], float)
HOOPC = np.array([38, 38, 46], float)
CHROME = np.array([214, 218, 228], float)
SMOKE = [(120, 118, 124), (90, 88, 96), (170, 166, 170)]
HYMN = (7, 9, 9, 7, 4)                 # "God Save the Tsar!" opening, degrees 5 6 6 5 3 as semitones over the tonic
PAD = 1.18                             # its sprite's crop margin (the bridge's x1.1 size, plus growth)
ROLL_Q = 30                            # the rolling sprite's spin in steps of this many degrees


def title_pattern(base_):
    """BASS DRUM: a drum head's rings (a white disc with a dark hoop) and tension rods radiating round it"""
    import title as TT
    XX, YY = TT.XX, TT.YY
    u = (XX - TT.DA[0]) * TT.DD[0] + (YY - TT.DA[1]) * TT.DD[1]
    cu = (u % 240.0) - 120.0
    cv = (np.abs(TT.SIDE) % 240.0) - 120.0
    r = np.hypot(cu, cv)
    a = np.arctan2(cv, cu)
    head = r < 62
    hoop = (r >= 62) & (r < 74)
    rods = (r >= 74) & (r < 96) & (np.abs(((a / (2 * np.pi) * 10) % 1.0) - 0.5) < 0.09)
    out = np.where(head[..., None], base_ * 0.45 + np.array([236, 232, 222]) * 0.55, base_)
    out = np.where(hoop[..., None], base_ * 0.3 + np.array([30, 30, 40]) * 0.7, out)
    return np.where(rods[..., None], base_ * 0.3 + np.array([220, 224, 236]) * 0.7, out)


def _nomallet_geo():
    """the drum without its mallet (rolling), in the sprite renderer's frame: centred and scaled exactly like the full
    model (pp_bridge.model_faces), so the two sprites match in size"""
    import pp_bridge as B
    import piano_lowpoly as PL
    full = B.build_geo('BASS DRUM')
    _, modname, key, _, yaw, size = B.entrant('BASS DRUM')
    V = np.concatenate([f[0] for f in full.F])
    lo, hi = V.min(0), V.max(0)
    c = (lo + hi) / 2
    r = np.linalg.norm(V - c, axis=1).max()
    mod = B.module(modname)
    g0 = PL.Geo()
    mod.drum(g0, chunk=B.scene('BASS DRUM').get('chunk', 1.0), mallet=False)
    a = math.radians(yaw)
    Rm = np.array([[math.cos(a), 0, math.sin(a)], [0, 1, 0], [-math.sin(a), 0, math.cos(a)]])
    g = PL.Geo()
    for P_, n, col, two, emit, gid, spec in g0.F:
        Q = ((P_ - c) / r * size) @ Rm.T
        g.F.append((Q, Rm @ n, tuple(int(x) for x in col), two, emit, 2 if gid != 1 else 1, spec))
    return g


def _render(geo, r, yaw, roll, pad=PAD):
    import sprites as SP
    import piano_lowpoly as PL
    R0 = int(round(r * pad))
    cx = cy = R0 + 3
    img = np.zeros((PL.RH, PL.RW, 3), np.uint8)
    zb = np.zeros((PL.RH, PL.RW))
    gb = np.zeros((PL.RH, PL.RW), np.int8)
    cam = SP._Cam(yaw, r, cx + 0.5, cy + 0.5, roll)
    PL.render_geo(geo, cam, img, zb, gb, scale=1.0)
    PL.outline(img, zb, gb)
    n = 2 * R0 + 7
    return np.dstack([img[:n, :n], (gb[:n, :n] > 0).astype(np.uint8) * 255])


# ===================================================================================================== look
class Look(base.Look):
    shards = ['head'] * 6 + ['hoop'] * 4 + ['rod'] * 4
    spark_big = ('roll', 'drumroll', 'cannon', 'shockwave')
    spark_flash = ('roll',)
    flash_hits = ('roll', 'cannon')
    fringe_hits = ('roll', 'cannon')
    rope_kinds = {'heartbeat': 0.7, 'drum_roll': 0.4, 'cannons': 0.8}
    sprite_k = 0.80
    title_pattern = staticmethod(title_pattern)

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.SEGS = []                                          # every roll: tip, start, planned end, actual end, line
        for i, e in enumerate(EV):
            if e[1] == 'roll' and e[2] == k:
                t_end = e[9]
                for f in EV[i + 1:]:
                    if f[2] == k and f[1] in ('roll_end', 'roll_off'):
                        t_end = f[0]
                        break
                self.SEGS.append(dict(t_tip=e[0], t0=e[8], t1=e[9], t_end=min(t_end, e[9]), kind=e[10],
                                      p0=np.array([e[3], e[4]]), p1=np.array([e[5], e[6]])))
        self.BOOMS = [(e[0], e[3], e[4], e[5]) for e in EV if e[1] == 'roll_boom' and e[2] == k]
        self.BEATS = [(e[0], e[3], e[4], e[5]) for e in EV if e[1] == 'heartbeat' and e[2] == k]
        self.CANNONS = [(e[0], e[3], e[4], e[5], e[6]) for e in EV if e[1] == 'cannon' and e[2] == k]
        self.DROLLS = []
        for i, e in enumerate(EV):
            if e[1] == 'drum_roll' and e[2] == k:
                t_end = e[3]
                for f in EV[i + 1:]:
                    if f[1] == 'drum_roll_end' and f[2] == k:
                        t_end = f[0]
                        break
                self.DROLLS.append((e[0], t_end))
        self._spr = {}
        self._geo = None

    # ---------------------------------------------------------------- clocks + the record
    def ta(self, t):
        return min(t, self.P.T_KO)

    def row(self, t):
        P = self.P
        if 'RCHG' not in P.RUN or not (self.T.T_FIGHT <= t < P.T_KO):
            return None
        return P.RUN['RCHG'][P.sim_i(t), self.k]

    def seg_at(self, t, fade=0.6):
        for S_ in self.SEGS:
            if S_['t_tip'] <= t < S_['t_end'] + fade:
                return S_
        return None

    def size_k(self, t):
        rw = self.row(t)
        if rw is None or np.isnan(rw[4]):
            return 1.0
        smax = max(1e-6, self.cfg['weapon'].get('size_max', 0.25))
        return 1.0 + 0.16 * float(rw[4]) / smax

    # ---------------------------------------------------------------- sprites
    def sprite(self, r, yaw, roll):
        if roll >= 1000:                                        # rolling: no mallet, tumbling by the wheel angle
            if self._geo is None:
                self._geo = _nomallet_geo()
            key = ('roll', int(r), int(roll))
            if key not in self._spr:
                self._spr[key] = _render(self._geo, r, 10.0, float(roll - 1000))
            return self._spr[key]
        import pp_bridge  # noqa: F401  (puts the series on the path)
        key = ('std', int(r), int(round(yaw)), int(roll))
        if key not in self._spr:
            import sprites as SP
            self._spr[key] = _render(SP._geo(self.cfg['pp']), r, yaw, roll)
        return self._spr[key]

    def title_closeup(self, r):
        yc, sw, roll = self.cfg['view']
        return self.sprite(r, float(yc), float(roll))

    def ball_sprite(self, kk, st, t, rp, yaw):
        if kk != self.k or st.get('shattered'):
            return None
        sk = self.size_k(t)
        r = int(np.clip(round(rp * self.sprite_k * sk), 8, 40))
        yk = int(round(yaw / 6.0)) * 6
        rc = st.get('drum_roll')
        if rc is not None:                                      # rolling
            return self.sprite(r, 0.0, 1000 + rc), None
        tip = st.get('drum_tip', 0.0)
        if tip > 0:                                             # keeling over onto its side
            return self.sprite(r, float(yk), int(round(tip * 90 / 15.0)) * 15), None
        return self.sprite(r, float(yk), 0), None

    # ---------------------------------------------------------------- state
    def state(self, kk, st, t, phase, gl=0.0):
        if kk != self.k or phase != 'fight':
            return
        rw = self.row(t)
        if rw is None:
            return
        if rw[0] == 1:                                          # tipping: squash and keel over
            u = float(np.clip(rw[1], 0, 1))
            st['drum_tip'] = u
            st['sx'] = 1.0 + 0.10 * math.sin(math.pi * u)
            st['sy'] = 1.0 - 0.10 * math.sin(math.pi * u)
        elif rw[0] == 2 or rw[6] == 1:                          # rolling (or between DRUM ROLL legs)
            ang = math.degrees(float(rw[3]))
            # spin direction: the way the wheel turns as seen head-on (rolling screen-right turns it clockwise)
            ca = math.cos(float(rw[2])) if not np.isnan(rw[2]) else 1.0
            spin = (-ang if ca >= 0 else ang) % 360.0
            st['drum_roll'] = int(round(spin / ROLL_Q)) * ROLL_Q % 360
            st['lift'] = st.get('lift', 0.0) + 0.012 * abs(math.sin(math.radians(ang * 2)))
        for t0, x, y, kind in self.BOOMS:                       # a BOOM: the ball flashes
            if 0 <= t - t0 < 0.3:
                st['glow'] = max(st['glow'], 0.7 * (1 - (t - t0) / 0.3))
        for t0, i, x, y in self.BEATS:
            if 0 <= t - t0 < 0.22:
                st['glow'] = max(st['glow'], 0.5 * (1 - (t - t0) / 0.22))
                st['scale'] = st['scale'] * (1.0 + 0.06 * (1 - (t - t0) / 0.22))

    # ---------------------------------------------------------------- events -> camera, ropes, sparks
    def shake(self, e):
        if e[1] == 'roll_boom':
            return 1.0 if e[5] == 0 else 0.5
        if e[1] == 'heartbeat':
            return 0.55
        if e[1] == 'slam':                                      # a cannon landing
            return 1.1
        if e[1] == 'roll':
            return 0.15
        return None

    def rope(self, e):
        if e[1] == 'ability':
            return self.rope_kinds.get(e[4])
        if e[1] == 'slam':
            return 0.6
        return None

    def spark(self, e):
        how = e[7]
        if how in ('roll', 'cannon'):
            return np.array([240, 236, 230], float), 18, 3.4, 0.32
        if how == 'drumroll':
            return np.array([210, 200, 180], float), 9, 2.4, 0.24
        if how == 'shockwave':
            return np.array([200, 200, 214], float), 10, 2.6, 0.26
        return np.array([200, 196, 200], float), 6, 1.8, 0.2

    def hit_glow(self, e):
        if e[7] in ('roll', 'cannon'):
            return 16, 0.45
        if e[7] in ('drumroll', 'shockwave'):
            return 11, 0.3
        return None

    # ---------------------------------------------------------------- effects
    def fx_setup(self):
        P, k = self.P, self.k
        col = P.FL_COL[k]
        for S_ in self.SEGS:                                    # dust puffs along every roll
            if S_['t0'] > P.T_KO:
                break
            n = int((S_['t_end'] - S_['t0']) / 0.05)
            for j in range(n):
                tj = S_['t0'] + j * 0.05
                if tj > P.T_KO:
                    break
                u = (tj - S_['t0']) / max(1e-9, S_['t1'] - S_['t0'])
                q = S_['p0'] + (S_['p1'] - S_['p0']) * u
                P.PA.emit(tj, P.up(q[0], q[1], 0.05), 'dust', n=2, col=DUST)
                if j % 3 == 0:
                    P.MK.add(tj, 'dust', q[0], q[1], size=0.26, life=2.0, seed=int(tj * 97) % 991, strength=0.3,
                             col=(186, 168, 138))
        for t0, x, y, kind in self.BOOMS:
            if t0 > P.T_KO:
                break
            pos = P.up(x, y, R * 0.8)
            big = kind == 0
            P.FL.add(t0, pos, col * 0.6 + 100, radius=2.8 if big else 2.0, peak=1.1 if big else 0.6, decay=0.3)
            P.BU.add(t0, pos, (246, 244, 236), 1.3 if big else 0.8)
            P.PA.emit(t0, P.up(x, y, 0.08), 'dust', n=14 if big else 6, col=DUST, speed=(0.6, 1.8))
            if big:
                P.MK.add(t0, 'crack', x, y, size=0.4, life=4.0, seed=int(t0 * 31) % 997, strength=0.45)
        for t0, i, x, y in self.BEATS:
            if t0 > P.T_KO:
                break
            P.FL.add(t0, P.up(x, y, R), col * 0.6 + 100, radius=2.4, peak=0.8, decay=0.22)
            P.PA.emit(t0, P.up(x, y, 0.06), 'dust', n=8, col=DUST, speed=(0.8, 1.6))
        for t0, i, x, y, t_land in self.CANNONS:                # the cannon landings (they come as 'slam' events too)
            if t_land > P.T_KO:
                break
            pos = P.up(x, y, 0.2)
            P.FL.add(t_land, pos, (255, 196, 120), radius=2.6, peak=1.1, decay=0.32)
            P.BU.add(t_land, pos, (255, 232, 196), 1.3)
            P.PA.emit(t_land, P.up(x, y, 0.1), 'smoke', n=10, col=SMOKE)
            P.PA.emit(t_land, P.up(x, y, 0.1), 'sparks', n=12, col=[(255, 210, 140), (255, 240, 210)])
            P.MK.add(t_land, 'scorch', x, y, size=0.6, life=5.0, seed=int(t_land * 53) % 991, strength=0.55)

    def ko_fx(self, pos):
        P = self.P
        P.PA.emit(P.T_KO, pos, 'sparks', n=28, spread=180, speed=(0.8, 2.4), col=[(240, 238, 230), (200, 204, 216)])
        P.PA.emit(P.T_KO, pos, 'chips', n=12, col=[tuple(HEADC), tuple(HOOPC), tuple(CHROME)])

    # ---------------------------------------------------------------- the canvas: the dust line, tread, rings, targets
    def canvas(self, Ic, C):
        P, k = self.P, self.k
        t = C.t
        if not (self.T.T_FIGHT <= t < P.T_KO + 0.6):
            return Ic
        tc = self.ta(t)
        sxp, szp = C.sxp, C.szp
        for S_ in self.SEGS:
            if not (S_['t_tip'] <= tc < S_['t_end'] + 0.6):
                continue
            a, b = S_['p0'], S_['p1']
            d = b - a
            L = float(np.hypot(*d)) + 1e-9
            d = d / L
            rx, rz = sxp - a[0], szp - a[1]
            along = rx * d[0] + rz * d[1]
            off = np.abs(-rx * d[1] + rz * d[0])
            out = max(0.0, (tc - S_['t_end']) / 0.6)
            if tc < S_['t0'] + 0.05 and S_['kind'] == 0:        # the telegraph: a dashed dust line, fading in
                fin = min(1.0, (tc - S_['t_tip']) / max(1e-3, S_['t0'] - S_['t_tip']))
                dash = ((along / 0.24 - tc * 3.0) % 1.0) < 0.55
                lane = (off < 0.06) & (along > R * 0.6) & (along < L + 0.15) & dash
                Ic += (lane * 0.5 * fin)[:, None] * (DUSTC / 255.0)[None]
            u = min(1.0, max(0.0, (min(tc, S_['t_end']) - S_['t0']) / max(1e-9, S_['t1'] - S_['t0'])))
            if u > 0:                                           # the tread behind it: churned canvas
                tread = (off < R * 0.42) & (along > 0) & (along < L * u)
                Ic *= (1 - (tread * 0.18 * (1 - out)))[:, None]
                edge = tread & (off > R * 0.30)
                Ic += (edge * 0.25 * (1 - out))[:, None] * (DUSTC / 255.0)[None]
        return Ic

    def canvas_wave(self, Ic, C, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind != 'shockwave':
            return Ic
        rd = np.hypot(C.sxp - cx, C.szp - cz)                   # HEARTBEAT's thick rings
        k_ = (1 - r / rmax)
        band = np.clip(1 - np.abs(rd - r) / 0.22, 0, 1)
        band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
        Ic += (band * k_ * 1.0)[:, None] * (self.P.COL[owner] / 255.0 * 1.2 + 0.3)[None]
        return Ic

    def canvas_after(self, Ic, C):
        P = self.P
        sl = P.RUN['SLAMS'][C.i] if C.in_fight else []
        for owner, cx, cz, t0, t_slam, rad in sl:                # 1812 OVERTURE: the target, then the blast
            if owner != self.k:
                continue
            rd = np.hypot(C.sxp - cx, C.szp - cz)
            if C.t < t_slam:
                u = (C.t - t0) / max(1e-9, t_slam - t0)
                Ic[rd < rad] *= 1 - 0.4 * u
                ring = np.abs(rd - rad * (1.3 - 0.3 * u)) < 0.05
                cross = ((np.abs(C.sxp - cx) < 0.03) | (np.abs(C.szp - cz) < 0.03)) & (rd < rad * 0.8)
                on = 0.5 + 0.5 * (math.sin(C.t * 36) > 0)
                Ic[ring | cross] += np.array([1.0, 0.72, 0.4]) * (0.35 + 0.45 * on)
            else:
                u = (C.t - t_slam) / 0.5
                if u < 1:
                    band = np.clip(1 - np.abs(rd - rad * (0.6 + 0.6 * u)) / 0.2, 0, 1) * (1 - u)
                    Ic += (band * 1.1)[:, None] * np.array([1.0, 0.8, 0.55])[None]
        return Ic

    # ---------------------------------------------------------------- over the ring: the booms' sound rings
    def over_ring(self, F, arr, depth):
        P, cam = self.P, F.cam
        t = F.t
        if not (self.T.T_FIGHT <= t < P.T_HS + 0.5):
            return arr, depth
        for t0, x, y, kind in self.BOOMS:
            age = t - t0
            if not (0 <= age < 0.35) or t0 > P.T_KO:
                continue
            kk = 1 - age / 0.35
            for j in range(2 if kind == 0 else 1):
                rr = R * (1.1 + 3.0 * age / 0.35 + 0.5 * j)
                th = np.linspace(0, 2 * math.pi, 25)
                Z = np.stack([x + rr * np.cos(th), y + rr * np.sin(th), np.full(25, R * 0.9)], 1)
                sp = cam.wp(S.to_world(Z[:, 0], Z[:, 1], Z[:, 2]))
                arr = blend(arr, _mask([], [sp], 3), INK, 0.35 * kk)
                arr = blend(arr, _mask([], [sp], 1), WHITE, 0.85 * kk)
        return arr, depth

    # ---------------------------------------------------------------- its parts (K.O. shatter)
    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        c = np.array([p[0], p[2]])
        if kind == 'rod':                                       # a chrome tension rod
            pts = [c - d * 0.07, c + d * 0.07]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            arr = blend(arr, _mask([], [sp], 3), INK, 0.7)
            return blend(arr, _mask([], [sp], 1), CHROME, 1.0)
        if kind == 'hoop':                                      # an arc of dark hoop
            pts = [c + d * (j - 2.5) * 0.05 + nn * curl * 0.025 * (j - 2.5) ** 2 for j in range(6)]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            arr = blend(arr, _mask([], [sp], 3), HOOPC, 1.0)
            return blend(arr, _mask([], [sp], 1), np.array([140, 140, 156], float), 0.8)
        sz = size * 1.9                                         # a torn piece of white head
        tri = [c + d * sz, c - d * sz * 0.6 + nn * sz * 0.9, c - d * sz * 0.5 - nn * sz * 0.6]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        P.layer_blend(arr, [sp], HEADC * (0.8 + 0.22 * abs(math.cos(rot))), 1.0)
        return blend(arr, _mask([], [np.vstack([sp, sp[:1]])], 1), np.array([110, 110, 124], float), 0.8)

    # ---------------------------------------------------------------- checks
    def focus_points(self, t, st):
        return []


# ===================================================================================================== sound
class Sound(base.Sound):
    bus_gain, send = 1.5, 0.16
    band = (35, 160)                                        # the kick's and booms' fundamentals (the bed's bass drums off)
    colour_args = dict(cut0=60, cut_span=240, drive=2.4)

    def __init__(self, M, k):
        super().__init__(M, k)
        import pp_bridge as B
        self.B = B
        self.mod = B.module('bassdrum_lowpoly')
        self.pan = -0.08 if k == 0 else 0.08
        self.lo, self.hi = M.CFG[k]['register']
        self._c = {}
        self.g0 = M.inst_gain(M.CFG[k]['pp'])

    # ---------------------------------------------------------------- its voice
    def hit(self, m, vel=0.75, boom=False, dur=0.2):
        key = ('h', int(m), round(vel, 2), bool(boom), round(dur, 3))
        if key not in self._c:
            x = self.mod.bd_hit(self.mod.midi_hz(int(m)), vel, boom=boom, dur=None if boom else dur)
            self._c[key] = self.B._resample(np.asarray(x, float)) * self.g0
        return self._c[key]

    def roll(self, m, dur, vel=0.8):
        key = ('r', int(m), round(dur, 3), round(vel, 2))
        if key not in self._c:
            x = self.mod.bd_roll(self.mod.midi_hz(int(m)), dur, vel, bpm=self.T.BPM)
            self._c[key] = self.B._resample(np.asarray(x, float)) * self.g0
        return self._c[key]

    def voice(self, m, dur, vel=0.85):
        return self.hit(self.fit_reg(m), vel, boom=vel >= 0.9, dur=dur)

    def fit_reg(self, m, lo=None, hi=None):
        lo, hi = lo or self.lo, hi or self.hi
        while m > hi:
            m -= 12
        while m < lo:
            m += 12
        return m

    def root_of(self, t):
        return self.T.root_above(self.T.chord_at(t), self.lo + 3)

    def col(self, x, t, **kw):
        return self.colour(x, t, low=True, **kw)

    def tonic_major(self):
        """the hymn's tonic: the key's relative major when the key is minor"""
        T = self.T
        minor = (T.KEY_PC + 3) % 12 in [(T.KEY_PC + s) % 12 for s in T.SCALE]
        return T.KEY_PC + (3 if minor else 0)

    def hymn_tonic(self):
        """the hymn's tonic in the drum's register, low enough that its sixth still fits"""
        t0 = self.fit_reg(self.tonic_major() + 24, self.lo + 2, self.lo + 13)
        return t0 - 12 if t0 + max(HYMN) > self.hi else t0

    def hymn_note(self, i):
        return self.hymn_tonic() + (HYMN[i] if i < len(HYMN) else 0)

    # ---------------------------------------------------------------- the score
    def title_blast(self, mix, t):
        """a huge boom on the tonic, then a short roll into the next word"""
        T = self.T
        hits = T.TITLE_HITS.get(self.name) or (self.root_of(0.0),)
        r = self.fit_reg(hits[0])
        mix.place(self.bus, self.hit(r, 1.0, boom=True), t, 0.72, pan=self.pan)
        mix.place('fx', self.M.A.thud(0.6, 100, 30, seed=5), t, 0.22)
        mix.place(self.bus, self.roll(r, T.BEAT * 0.5, 0.6), t + T.BEAT * 0.5, 0.30, pan=self.pan)

    def count_in(self, mix):
        """a boom on each count, a roll into FIGHT!"""
        T = self.T
        r = self.root_of(T.BAR)
        for j in range(3):
            mix.place(self.bus, self.hit(r, 0.95, boom=True), T.BAR + j * T.BEAT, 0.52, pan=self.pan)
        mix.place(self.bus, self.roll(r, T.BEAT * 0.95, 0.85), T.BAR + 3 * T.BEAT, 0.42, pan=self.pan)

    def play(self, mix):
        M, T, k, A = self.M, self.T, self.k, self.M.A
        q16 = M.q16
        th = M.Thinner(560 + k)
        look = M.P.LOOKS[k]
        busy = [(S_['t0'], S_['t_end']) for S_ in look.SEGS] + list(look.DROLLS)

        def rolling(t):
            return any(a - 1e-6 <= t < b for a, b in busy)
        # the kick: every beat on the chord's root (the band's bass drum), off-beat pickups while it leads
        nb0, nb1 = int(math.ceil(T.T_FIGHT / T.BEAT)), int(M.T_KO / T.BEAT)
        for nb in range(nb0, nb1 + 1):
            tq = nb * T.BEAT
            if rolling(tq):
                continue
            hp = M.hp_at(k, tq) / 100.0
            if not th.keep(nb * 4, hp):
                continue
            sig, g, _ = self.col(self.hit(self.root_of(tq), 0.8, dur=T.BEAT * 0.6), tq)
            mix.place(self.bus, sig, tq, 0.50 * g, pan=self.pan)
            if hp > M.hp_at(1 - k, tq) / 100.0 + 0.08 and nb % 2 == 1 and not rolling(tq + 3 * T.S16):
                sig, g, _ = self.col(self.hit(self.root_of(tq), 0.62, dur=T.S16), tq + 3 * T.S16)
                mix.place(self.bus, sig, tq + 3 * T.S16, 0.34 * g, pan=self.pan)
        for S_ in look.SEGS:                                    # every roll: a 32nd roll swelling into the boom
            if S_['t0'] > M.T_KO:
                break
            t0, t1 = q16(S_['t0']), max(q16(S_['t_end']), q16(S_['t0']) + T.S16)
            if S_['kind'] == 0:
                sig, g, _ = self.col(self.roll(self.root_of(t0), t1 - t0, 0.8), t0)
                mix.place(self.bus, sig, t0, 0.40 * g, pan=self.pan)
        for t0, t1 in look.DROLLS:                              # DRUM ROLL: a thundering roll for the whole bar
            if t0 > M.T_KO:
                break
            a, b = q16(t0), min(q16(t1), M.T_KO)
            if b > a:
                r = self.root_of(a)
                mix.place(self.bus, self.roll(r, b - a, 0.95), a, 0.50, pan=self.pan)
                mix.place(self.bus, self.roll(r - 12 if r - 12 >= self.lo - 12 else r, b - a, 0.8), a + 0.012, 0.28,
                          pan=-self.pan)
                mix.place('fx', M.thunder(b - a + 0.6, seed=int(t0 * 7), crack=False), a, 0.16)
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if e[2] != k:
                continue
            tq = q16(t)
            if kind == 'roll_boom':                             # the BOOM
                big = e[5] == 0
                sig, g, _ = self.col(self.hit(self.root_of(tq), 1.0 if big else 0.9, boom=True), t, drive_k=1.2)
                mix.place(self.bus, sig, tq, (0.70 if big else 0.48) * g, pan=self.pan)
                mix.place('fx', A.thud(0.6, 110, 30, seed=int(t * 11)), tq, 0.26 if big else 0.14)
            elif kind == 'heartbeat':                           # lub-dub
                r = self.root_of(tq)
                sig, g, _ = self.col(self.hit(r if e[3] == 0 else r + 7, 0.98, boom=True), t)
                mix.place(self.bus, sig, tq, (0.62 if e[3] == 0 else 0.52) * g, pan=self.pan)
                mix.place('fx', A.thud(0.4, 90, 28, seed=int(t * 13)), tq, 0.18)
            elif kind == 'slam':                                # 1812 OVERTURE: a cannon = a tuned boom of the hymn
                i = sum(1 for c in M.P.LOOKS[k].CANNONS if c[4] <= t + 1e-6) - 1
                m = self.hymn_note(max(0, i))
                mix.place(self.bus, self.hit(m, 1.0, boom=True), tq, 0.74, pan=self.pan)
                mix.place('fx', M.thunder(1.3, seed=60 + i), tq, 0.30)
                mix.place('fx', A.thud(0.8, 120, 26, seed=70 + i), tq, 0.34)
                self._pit(mix, tq, m)
                if i == len(HYMN) - 1:                          # the resolution: the tonic on the next beat
                    tn = tq + T.BEAT
                    tonic = self.hymn_note(len(HYMN))
                    mix.place(self.bus, self.hit(tonic, 1.0, boom=True), tn, 0.6, pan=self.pan)
                    self._pit(mix, tn, tonic)

    def _pit(self, mix, t, m):
        """the hymn note doubled by the pit's timpani + marimba (when the bed has them) and a tubular bell"""
        hm = m
        while hm < 72:
            hm += 12
        b = getattr(self.M, 'bed', None)
        if b is not None and hasattr(b, 'timp') and hasattr(b, 'marimba'):
            tm = m
            while tm < 36:
                tm += 12
            while tm > 50:
                tm -= 12
            mix.place('pit', b.timp(tm), t, 0.42)
            for j, dm in enumerate((0, 12)):
                for r_ in range(6):                             # a marimba roll on the note
                    mix.place('pit', b.marimba(hm + dm), t + r_ * self.T.S16 / 2, 0.05 + 0.01 * (r_ == 0))
        mix.place('fx', self.bell(hm + 12), t, 0.10)

    def bell(self, m):
        key = ('bell', int(m))
        if key not in self._c:
            A, SR = self.M.A, self.M.SR
            n = int(2.4 * SR)
            t = A.tt(n)
            f = A.mtof(m)
            x = np.zeros(n)
            for ratio, amp, dk in ((0.5, .5, 1.6), (1.0, 1.0, 1.2), (1.19, .45, .9), (1.5, .4, .8), (2.0, .55, .7),
                                   (2.74, .3, .5), (3.0, .25, .4), (4.07, .18, .3)):
                x += amp * np.sin(2 * np.pi * f * ratio * t) * np.exp(-t / dk)
            self._c[key] = x * np.clip(t / 0.002, 0, 1) * 0.35
        return self._c[key]

    def fall_apart(self, mix):
        """the drum caves in: a dying roll, a last flat thud, the rods rattling"""
        M, A = self.M, self.M.A
        t0 = M.P.T_HS
        r = self.root_of(t0)
        mix.place(self.bus, self.roll(r, 0.8, 0.6), t0, 0.32, pan=self.pan)
        mix.place('fx', A.thud(0.5, 80, 30, seed=81), t0 + 0.8, 0.22)
        for j in range(6):
            mix.place('fx', A.thud(0.05, 1400, 900, seed=90 + j), t0 + 0.1 + 0.08 * j, 0.05)

    def stinger(self, mix, tw):
        r = self.root_of(0.0)
        S16 = self.T.S16
        mix.place('solo', self.hit(r, 0.95, boom=True), tw, 0.5)
        mix.place('solo', self.hit(r, 0.9, boom=True), tw + 2 * S16, 0.46)
        mix.place('solo', self.hit(r, 1.0, boom=True), tw + 4 * S16, 0.6)

    def solo(self, mix, t0):
        """config SOLO: a bass-drum cadence ((beat, midi, beats); midi < 24 = a roll on midi + 24; vel >= .9 booms)"""
        T = self.T
        for ev_ in T.SOLO.get(self.name, []):
            bt, m, d = ev_[:3]
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            if m < 24:
                mix.place('solo', self.roll(self.fit_reg(m + 24), max(0.1, d * T.BEAT * 0.95), 0.85), ts, 0.46)
            else:
                mix.place('solo', self.hit(self.fit_reg(m), 0.95 if d >= 1 else 0.75, boom=d >= 1,
                                           dur=d * T.BEAT * 0.8), ts, 0.6 if d >= 1 else 0.48)

    def reassemble(self, mix):
        """its pieces fly home: a roll climbing in pitch"""
        T, M = self.T, self.M
        r = self.root_of(0.0)
        for j, m in enumerate((r, r + 3, r + 5, r + 7, r + 10, r + 12)):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / 6)
            mix.place('solo', self.hit(self.fit_reg(m, self.lo, self.lo + 26), 0.7, dur=0.1), tj, 0.32)

    def last_bar(self, mix):
        """the band's bass drum in the build: a boom on every beat of the last bar, a roll into the title slam"""
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        r = self.root_of(t_last)
        for j in range(3):
            mix.place(self.bus, self.hit(r, 0.95, boom=True), t_last + j * T.BEAT, 0.5, pan=self.pan)
        mix.place(self.bus, self.roll(r, T.BEAT - 0.02, 0.9), t_last + 3 * T.BEAT, 0.42, pan=self.pan)
