"""ELECTRIC (neon green), new in #44 Electric vs Acoustic: LIGHTNING RIFF (a riff every bar, every note an instant
lightning bolt from its headstock, or from the corner post nearest the enemy: the posts are lightning rods, so the enemy
is safe only away from both; +1 note per 2 landed riffs), DISTORTION (the bolts fork and reach further), TAPPING (the
second hand taps a bolt on every 8th for a bar), GUITAR SMASH (it smashes itself on the floor: a shockwave + a burst of
splinters, lies in pieces for a beat, then the pieces snap back together and it fights on).

Look: thin jagged bolts with a white-hot core, a neon-green glow and a soft halo, re-drawn every frame (they flicker),
side branches, a fork round the enemy under DISTORTION, a fizzling crackle when a bolt falls short; a post strike lights
the post top and sparks race along its two ropes into it first; a lit lane on the canvas under every bolt and a small
scorch where it struck; DISTORTION crackles round its glass, TAPPING flickers two hand-sparks on it. GUITAR SMASH: a
neon shockwave ring (lit band on the canvas, inked jagged ring, heat shimmer), splinters flying out, the glass and the
guitar's parts tumbling out and snapping back in a beat, a crack left on the canvas. It shatters into its black body,
the maple neck, pickups and strings. VS card: pickups and neon lightning on its panel.
Sound: the Pixel Pitch electric (the bridge's GUITAR voice, electric60) through a fuzz + cab: every bolt is a riff note
(config LINES, 8 a bar; a landed bolt hits it full with a power-chord stab and a zap, a miss is palm-muted), a post
strike adds the ropes' buzz, DISTORTION doubles the riff an octave up through more gain, TAPPING plays tapped notes high
up the chord, GUITAR SMASH a huge chord crash, feedback while it lies in pieces and a pickup slide as it snaps back."""
import math

import numpy as np
from reelkit.config import W, H, FPS, BAYER_FULL, ss
from reelkit import fx
import scene as S

from . import base
from ._reeds import _mask, blend, add, beam_lane, INK, WHITE

R = S.R_BALL
CORE = np.array([246, 255, 238], float)
SPARK_COLS = [(220, 255, 220), (120, 255, 140), (255, 255, 236)]
POST_NAMES = ('red', 'front', 'blue', 'back')            # the engine's posts 0..3 (fight.Fight.POSTS order)
POST_SIDES = {'red': (0, 3), 'front': (0, 1), 'blue': (1, 2), 'back': (2, 3)}   # the rope sides meeting at each post
BOLT_LIFE = 0.13
BODY = np.array([28, 26, 34], float)                     # black body, maple neck, chrome
MAPLE = np.array([226, 196, 140], float)
CHROME = np.array([214, 218, 228], float)


# ---------------------------------------------------------------- lightning
def bolt_path(p0, p1, seed, jag=0.16, levels=4):
    """a jagged path between two ring points (sx, sz, height): midpoint displacement, seeded per frame (it flickers)"""
    rng = np.random.default_rng(seed)
    P = np.array([p0, p1], float)
    amp = jag * float(np.linalg.norm(P[1] - P[0]))
    for lv in range(levels):
        mids = (P[:-1] + P[1:]) / 2
        d = P[1:] - P[:-1]
        dn = d / (np.linalg.norm(d, axis=1, keepdims=True) + 1e-9)
        r = rng.normal(0, 1, mids.shape)
        r -= (r * dn).sum(1, keepdims=True) * dn
        r *= np.array([1.0, 1.0, 0.6])
        mids = mids + r * amp * (0.5 ** lv)
        out = np.empty((len(P) + len(mids), 3))
        out[0::2], out[1::2] = P, mids
        P = out
    return P


def _screen(cam, P):
    return cam.wp(S.to_world(P[:, 0], P[:, 1], P[:, 2]))


def draw_bolt(img, cam, P, k, col, halo=5, glow=3, core=1):
    """a bolt: soft halo (additive), neon glow, white-hot core"""
    if k <= 0.02 or len(P) < 2:
        return img
    sp = _screen(cam, P)
    col = np.asarray(col, float)
    img = add(img, _mask([], [sp], halo), col * 0.5, 0.36 * k)
    img = blend(img, _mask([], [sp], glow), np.clip(col * 0.75 + 70, 0, 255), 0.85 * k)
    return blend(img, _mask([], [sp], core), CORE, k)


def bolt_branches(P, seed, n=2, frac=0.28):
    """short side branches off the main path (each a smaller jagged path)"""
    rng = np.random.default_rng(seed + 7)
    out = []
    L = float(np.linalg.norm(P[-1] - P[0]))
    for _ in range(n):
        i = int(rng.integers(len(P) // 5, max(len(P) // 5 + 1, 4 * len(P) // 5)))
        d = rng.normal(0, 1, 3) * np.array([1.0, 1.0, 0.5])
        d /= np.linalg.norm(d) + 1e-9
        out.append(bolt_path(P[i], P[i] + d * L * frac * rng.uniform(0.5, 1.0), seed + 31 + i, 0.2, 3))
    return out


def jag_ring(cx, cz, r, t, y=0.08, n=64, amp=0.10):
    a = np.linspace(0, 2 * math.pi, n + 1)
    jag = r + amp * np.sign(np.sin(a * 23 + t * 40)) * np.abs(np.sin(a * 7 + t * 13))
    return np.column_stack([cx + jag * np.cos(a), cz + jag * np.sin(a), np.full(n + 1, y)])


# ---------------------------------------------------------------- the VS card panel
def title_pattern(base):
    """ELECTRIC: humbucker pickups in rows along the divider (black covers, chrome pole pieces) under neon lightning"""
    import title as TT
    XX, YY, SIDE = TT.XX, TT.YY, TT.SIDE
    u = (XX - TT.DA[0]) * TT.DD[0] + (YY - TT.DA[1]) * TT.DD[1]       # along the divider
    v = SIDE                                                         # across it
    cu, cv = (u % 96.0) - 48.0, (np.abs(v) % 110.0) - 55.0
    pick = (np.abs(cu) < 30) & (np.abs(cv) < 13)
    pole = pick & (np.abs(((cu + 30) % 10.0) - 5.0) < 1.6) & (np.abs(np.abs(cv) - 6.5) < 1.6)
    base = np.where(pick[..., None], base * 0.35 + np.array([20, 22, 26]) * 0.65, base)
    base = np.where(pole[..., None], np.array([214, 220, 230], float), base)
    z = (np.abs(v) % 110.0) - 55.0 + 9.0 * np.abs(((u / 26.0) % 2.0) - 1.0) * 2 - 9.0     # zig-zag bolts between rows
    bolt = (np.abs(np.abs(z) - 44.0) < 1.4) & ((np.floor(u / 52.0) % 3) != 1)
    glow = (np.abs(np.abs(z) - 44.0) < 4.0) & ((np.floor(u / 52.0) % 3) != 1)
    base = np.where(glow[..., None], base * 0.6 + np.array([100, 255, 120]) * 0.4, base)
    return np.where(bolt[..., None], np.array([236, 255, 230], float), base)


# ===================================================================================================== look
class Look(base.Look):
    shards = ['body'] * 5 + ['neck'] * 3 + ['pickup'] * 3 + ['string'] * 4
    spark_big = ('smash',)
    spark_flash = ('bolt',)
    flash_hits = ('smash',)
    fringe_hits = ('smash',)
    rope_kinds = {}
    title_pattern = staticmethod(title_pattern)

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.BOLTS = []
        for i, e in enumerate(EV):
            if e[1] == 'bolt' and e[2] == k:
                self.BOLTS.append(dict(i=i, t0=e[0], note=e[3], src=e[4], x0=e[5], y0=e[6], x1=e[7], y1=e[8],
                                       hit=bool(e[9]), riff=e[10], fork=bool(e[11]), tap=bool(e[12])))
        self.BOLT_T = np.array([b['t0'] for b in self.BOLTS]) if self.BOLTS else np.zeros(0)
        self.FORK = [(e[0], e[3]) for e in EV if e[1] == 'fork' and e[2] == k]
        self.TAP = [(e[0], e[3]) for e in EV if e[1] == 'tapping' and e[2] == k]
        self.SMASH = []
        for e in EV:
            if e[1] == 'smash' and e[2] == k:
                sp = []
                for f in EV:
                    if f[1] == 'splinter' and f[2] == k and abs(f[0] - e[0]) < 1e-6:
                        end = next((g for g in EV if g[1] == 'splinter_end' and g[2] == k and g[3] == f[3]
                                    and g[0] >= f[0]), None)
                        sp.append(dict(j=f[3], ang=f[4], speed=f[5], t1=end[0] if end else P.T_KO,
                                       hit=bool(end[6]) if end else False))
                self.SMASH.append(dict(t0=e[0], c=np.array([e[3], e[4]]), radius=e[5], dur=e[6], n=e[7], sp=sp))
        self.post_w = {j: S.to_world(*S.CORNERS[nm], S.POST_H + 0.06) for j, nm in enumerate(POST_NAMES)}

    # ---------------------------------------------------------------- clocks (attacks freeze with the hit-stop)
    def show(self, t):
        return self.T.T_FIGHT <= t < self.P.T_HS + 0.12

    def ta(self, t):
        return min(t, self.P.T_KO)

    def active(self, spans, t):
        return any(t0 <= t < t1 for t0, t1 in spans)

    def live_bolts(self, F):
        key = ('bolts', self.k)
        if key not in F.__dict__:
            out = []
            if self.show(F.t) and len(self.BOLT_T):
                ta = self.ta(F.t)
                j0, j1 = np.searchsorted(self.BOLT_T, ta - BOLT_LIFE), np.searchsorted(self.BOLT_T, ta, 'right')
                for b in self.BOLTS[j0:j1]:
                    age = ta - b['t0']
                    if 0 <= age < BOLT_LIFE:
                        out.append((b, age, (1 - age / BOLT_LIFE) ** 0.7))
            F.__dict__[key] = out
        return F.__dict__[key]

    def bolt_ends(self, F, b):
        """the bolt's two ends now (ring frame + height): from the headstock (following the ball) or a post top, to the
        enemy's glass (following it) or, for a miss, the spot it fizzled at"""
        me = F.fs[self.k]['p']
        en = F.fs[self.enemy]['p']
        if b['src'] >= 0:
            pw = self.post_w[b['src']]
            sx, sz = S.to_ring(pw[0], pw[2])
            p0 = np.array([float(sx), float(sz), S.POST_H + 0.06])
        else:
            tgt = en if b['hit'] else np.array([b['x1'], b['y1']])
            d = tgt - me
            u = d / (np.hypot(*d) + 1e-9)
            p0 = np.array([me[0] + u[0] * R * 0.85, me[1] + u[1] * R * 0.85, R * 1.25])
        if b['hit']:
            d = en - p0[:2]
            u = d / (np.hypot(*d) + 1e-9)
            p1 = np.array([en[0] - u[0] * R * 0.9, en[1] - u[1] * R * 0.9, R])
        else:
            p1 = np.array([b['x1'], b['y1'], R * 0.9])
        return p0, p1

    def smash_at(self, t):
        """(smash, u 0..1) while it lies in pieces (on the attack clock: held through the hit-stop), else None"""
        tc = self.P.fx_clock(t)
        for sm in self.SMASH:
            if sm['t0'] <= tc < sm['t0'] + sm['dur']:
                return sm, (tc - sm['t0']) / sm['dur']
        return None

    # ---------------------------------------------------------------- state
    def state(self, kk, st, t, phase, gl=0.0):
        if kk != self.k or phase == 'rev':
            return
        sm = self.smash_at(t)
        if sm is not None and (phase == 'fight' or t < self.P.T_HS + 0.6):
            st['vis'] = False                                       # in pieces (drawn by items_free)
            return
        if phase != 'fight':
            return
        tc = self.ta(t)
        for sm_ in self.SMASH:                                       # snapped back together: a neon flash
            te = sm_['t0'] + sm_['dur']
            if te <= tc < te + 0.3:
                st['glow'] = max(st['glow'], 1.0 - (tc - te) / 0.3)
        if len(self.BOLT_T):                                          # every bolt lights the glass
            j = int(np.searchsorted(self.BOLT_T, tc, 'right')) - 1
            if j >= 0:
                st['glow'] = max(st['glow'], 0.4 * math.exp(-(tc - self.BOLT_T[j]) / 0.08))

    def hides_hp(self, kk, t):
        return kk == self.k and self.smash_at(t) is not None

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'smash':
            return 2.5
        if e[1] == 'hit' and e[7] == 'bolt':
            return 0.4 if e[8] > 0 else 0.25
        return super().shake(e)

    def spark(self, e):
        if e[7] == 'smash':
            return np.array([140, 255, 160], float), 18, 3.4, 0.32
        if e[7] == 'splinter':
            return np.array([226, 196, 140], float), 6, 2.0, 0.2
        return np.array([170, 255, 180], float), 6, 2.2, 0.16

    def hit_glow(self, e):
        if e[7] == 'smash':
            return 16, 0.4
        if e[7] == 'bolt':
            return 11, 0.35
        return None

    def rope(self, e):
        if e[1] == 'smash':
            return 1.0
        return None

    # ---------------------------------------------------------------- effects
    def fx_setup(self):
        P, k = self.P, self.k
        for b in self.BOLTS:
            if b['t0'] > P.T_KO:
                break
            if b['src'] >= 0:                                         # the post lights up in neon
                P.FL.add(b['t0'], self.post_w[b['src']], P.FL_COL[k], radius=1.4, peak=0.55, decay=0.14)
            end = P.up(b['x1'], b['y1'], R)
            if b['hit']:
                P.FL.add(b['t0'], end, P.FL_COL[k], radius=1.3, peak=0.5 if not b['tap'] else 0.3, decay=0.14)
                P.MK.add(b['t0'], 'scorch', b['x1'], b['y1'], size=0.16, life=2.4, seed=b['i'] % 997, strength=0.4)
            elif not b['tap']:
                P.PA.emit(b['t0'], end, 'sparks', n=3, col=SPARK_COLS, speed=(0.6, 1.4))
        for t0, t1 in self.FORK:                                      # DISTORTION crackles off the glass
            tt_ = t0
            while tt_ < min(t1, P.T_KO):
                p = P.RUN['P'][P.sim_i(tt_), k]
                P.PA.emit(tt_, P.up(p[0], p[1], R * 1.2), 'sparks', n=2, col=SPARK_COLS, speed=(0.8, 1.8))
                tt_ += 0.24
        for sm in self.SMASH:                                         # GUITAR SMASH
            pos = P.up(sm['c'][0], sm['c'][1], R * 0.6)
            P.BU.add(sm['t0'], pos, P.COL[k], 2.0)
            P.FL.add(sm['t0'], pos, P.FL_COL[k], radius=2.8, peak=1.1, decay=0.45)
            P.PA.emit(sm['t0'], pos, 'sparks', n=30, col=SPARK_COLS, spread=180, speed=(1.5, 4.0))
            P.PA.emit(sm['t0'], pos, 'shards', n=14, col=[(28, 26, 34), (100, 255, 120), (226, 196, 140)])
            P.MK.add(sm['t0'], 'crack', sm['c'][0], sm['c'][1], size=0.9, life=7.0, seed=31, strength=0.9)
            P.MK.add(sm['t0'], 'scorch', sm['c'][0], sm['c'][1], size=0.7, life=6.0, seed=32, strength=0.7)
            te = sm['t0'] + sm['dur']
            P.FL.add(te, pos, P.FL_COL[k], radius=1.8, peak=0.6, decay=0.25)       # snapping back together
            P.PA.emit(te, pos, 'motes', n=8, col=SPARK_COLS)

    def fx_event(self, e):
        P = self.P
        if e[1] != 'hit':
            return
        att, x, z, how = e[2], e[5], e[6], e[7]
        pos = S.to_world(x, z, R)
        if how == 'bolt':
            tap = any(abs(b['t0'] - e[0]) < 1e-6 and b['tap'] for b in self.BOLTS[max(0, int(np.searchsorted(
                self.BOLT_T, e[0])) - 2):int(np.searchsorted(self.BOLT_T, e[0], 'right'))])
            P.BU.add(e[0], pos, P.COL[att], 0.5 if tap else 0.8, bright=not tap)
            P.PA.emit(e[0], pos, 'sparks', n=4 if tap else 7, col=SPARK_COLS)
        elif how == 'smash':
            P.BU.add(e[0], pos, P.COL[att], 1.6)
            P.FL.add(e[0], pos, P.FL_COL[att], radius=2.0, peak=0.9, decay=0.3)
        elif how == 'splinter':
            P.BU.add(e[0], pos, (226, 196, 140), 0.5, bright=False)
            P.PA.emit(e[0], pos, 'chips', n=5)

    def ko_fx(self, pos):
        P = self.P
        P.PA.emit(P.T_KO, pos, 'sparks', n=30, spread=180, speed=(0.8, 2.6), col=SPARK_COLS)
        P.PA.emit(P.T_KO, pos, 'shards', n=10, col=[(28, 26, 34), (100, 255, 120), (214, 218, 228)])

    # ---------------------------------------------------------------- the canvas
    def canvas(self, Ic, C):
        P = self.P
        col = P.COL[self.k] / 255.0 * 0.9 + 0.15
        for b, age, k_ in self.live_bolts(C):                         # a lit lane under every bolt
            p0, p1 = self.bolt_ends(C, b)
            d = p1[:2] - p0[:2]
            Ln = float(np.hypot(*d))
            if Ln > 1e-6:
                lane = beam_lane(C.sxp, C.szp, p0[:2], math.atan2(d[1], d[0]), Ln, 0.06, C.ys, C.xs)
                Ic += (lane * (0.55 if b['hit'] else 0.3) * k_)[:, None] * col[None]
        return Ic

    def canvas_wave(self, Ic, C, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind == 'smash':                                          # GUITAR SMASH's shockwave: a jagged neon band
            rd = np.hypot(C.sxp - cx, C.szp - cz)
            ang = np.arctan2(C.szp - cz, C.sxp - cx)
            jag = 0.09 * np.abs(((ang * 14 / math.pi + C.t * 25) % 2) - 1)
            band = np.clip(1 - np.abs(rd - r - jag) / 0.22, 0, 1) * (1 - r / rmax) ** 0.7
            band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
            Ic += (band * 1.15)[:, None] * (self.P.COL[owner] / 255.0 * 0.8 + 0.2)[None]
        return Ic

    # ---------------------------------------------------------------- the painter pass: its pieces while smashed
    def items_free(self, F):
        sm = self.smash_at(F.t)
        if sm is None:
            return []
        p = sm[0]['c']
        return [(F.cam.depth(S.to_world(p[0], p[1], R))[0], ('pieces', sm))]

    def draw_item(self, F, payload):
        what, (sm, u) = payload
        if what != 'pieces':
            return
        fr, cam, P = F.fr, F.cam, self.P
        arr, depth = fr.array(), fr.depth()
        c = sm['c']
        out = math.sin(math.pi * min(1.0, u / 0.7)) if u < 0.7 else 0.0       # fly out, hang, snap back in
        back = ss(0.55, 1.0, u)
        rr = (0.95 * min(1.0, u / 0.3) ** 0.5) * (1 - back)
        rng = np.random.default_rng(int(sm['t0'] * 1000) % 99991)
        parts = [('glass', 0.14)] * 9 + [('body', 0.26), ('neck', 0.3), ('pickup', 0.12), ('pickup', 0.12)]
        for j, (kind, size) in enumerate(parts):
            a = 2 * math.pi * j / len(parts) + rng.uniform(-0.3, 0.3)
            rad = rr * rng.uniform(0.6, 1.1)
            h = R * (0.3 + 0.9 * out * rng.uniform(0.4, 1.0)) * (1 - back) + R * back
            q = np.array([c[0] + rad * math.cos(a), c[1] + rad * math.sin(a), h])
            rot = a + 8 * u * rng.uniform(-1, 1)
            if kind == 'glass':
                d = np.array([math.cos(rot), math.sin(rot)])
                nn = np.array([-d[1], d[0]])
                tri = [q[:2] + d * size, q[:2] - d * size * 0.6 + nn * size * 0.7, q[:2] - d * size * 0.5 - nn * size * 0.6]
                sp = P.screen(cam, np.array([S.to_world(x, z, q[2]) for x, z in tri]))
                P.layer_blend(arr, [sp], P.COL[self.k] * 0.35 + 170, 0.85)
            else:
                arr = self.draw_shard(arr, cam, kind, (q[0], q[2], q[1]), rot, size, 0.5)
        if back > 0.6:                                                   # the snap: a neon ring closing on the ball
            cs = cam.wp(S.to_world(c[0], c[1], R)[None])[0]
            fr.glow(cs[0], cs[1], 18, tuple(P.COL[self.k] * 0.8 + 50), 0.6 * (back - 0.6) / 0.4)
        fr.set_array(arr, depth)

    # ---------------------------------------------------------------- effects over the ring
    def rope_sparks(self, img, F, b, age):
        """a post strike: sparks race along the post's two ropes into it (from 0.10 s before the bolt)"""
        P = self.P
        nm = POST_NAMES[b['src']]
        lead = 0.10
        u = (age + lead) / (lead + 0.04)
        if not 0 <= u <= 1.4:
            return img
        k = 1 - max(0.0, u - 1.0) / 0.4
        col = np.clip(P.COL[self.k] * 0.8 + 60, 0, 255)
        for s in POST_SIDES[nm]:
            a, bnd = P.side_ends(s)
            at_post = 0 if np.allclose(a, S.CORNERS[nm]) else 1
            for y in (S.ROPE_Y[2], S.ROPE_Y[0]):
                pts = P.rope_points(s, y, F.t, n=15)
                if at_post == 0:
                    pts = pts[::-1]                                      # run toward the post end
                for j in range(3):
                    f = min(1.0, u) * (0.55 + 0.45 * ((j * 0.37 + b['i'] * 0.11) % 1.0))
                    for tr in range(3):
                        ff = max(0.0, f - tr * 0.04)
                        i = ff * (len(pts) - 1)
                        i0 = int(i)
                        i1 = min(len(pts) - 1, i0 + 1)
                        w = pts[i0] * (1 - (i - i0)) + pts[i1] * (i - i0)
                        s_ = F.cam.wp(w[None])[0]
                        xi, yi = int(round(s_[0])), int(round(s_[1]))
                        if 1 <= xi < W - 1 and 1 <= yi < H - 1:
                            kk = k * (1.0 - 0.3 * tr)
                            img[yi, xi] = img[yi, xi] * (1 - kk) + (CORE if tr == 0 else col) * kk
                            if tr == 0:
                                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                                    img[yi + dy, xi + dx] = img[yi + dy, xi + dx] * (1 - 0.6 * kk) + col * 0.6 * kk
        return img

    def over_ring(self, F, arr, depth):
        P, cam, k = self.P, F.cam, self.k
        col = P.COL[k]
        t = F.t
        f = F.f
        # posts about to strike: the ropes buzz first
        if self.show(t) and len(self.BOLT_T):
            ta = self.ta(t)
            j0, j1 = np.searchsorted(self.BOLT_T, ta - 0.05), np.searchsorted(self.BOLT_T, ta + 0.11, 'right')
            for b in self.BOLTS[j0:j1]:
                if b['src'] >= 0:
                    arr = self.rope_sparks(arr, F, b, ta - b['t0'])
        for b, age, k_ in self.live_bolts(F):
            p0, p1 = self.bolt_ends(F, b)
            seed = b['i'] * 131 + f * 7
            Pp = bolt_path(p0, p1, seed)
            if b['fork']:                                                # DISTORTION: the bolt forks round the enemy
                mid = Pp[len(Pp) * 3 // 5]
                d = p1[:2] - p0[:2]
                n = np.array([-d[1], d[0]]) / (np.hypot(*d) + 1e-9)
                for sgn in (-1, 1):
                    q = p1 + np.array([n[0] * sgn * 0.32, n[1] * sgn * 0.32, 0.12 * sgn])
                    arr = draw_bolt(arr, cam, bolt_path(mid, q, seed + 3 + sgn, 0.22, 3), k_ * 0.8, col, 4, 2, 1)
            for br in bolt_branches(Pp, seed, n=1 if b['tap'] else 2):
                arr = draw_bolt(arr, cam, br, k_ * 0.7, col, 4, 2, 1)
            arr = draw_bolt(arr, cam, Pp, k_ * (0.8 if b['tap'] else 1.0), col, 6 if b['tap'] else 8,
                            3 if b['tap'] else 4, 1 if b['tap'] else 2)
            if not b['hit']:                                             # a miss fizzles out
                for br in bolt_branches(np.vstack([Pp[-6:]]), seed + 5, n=3, frac=0.9):
                    arr = draw_bolt(arr, cam, br, k_ * 0.5, col, 3, 1, 1)
        if F.in_fight or self.smash_at(t) is not None:
            for owner, cx, cz, r, rmax, kind, t0 in (P.RUN['WAVES'][F.i] if F.in_fight else []):
                if kind == 'smash' and owner == k:                       # the shockwave: an inked jagged neon ring
                    kk = (1 - r / rmax) ** 0.7
                    ring = jag_ring(cx, cz, r, t)
                    sp = _screen(cam, ring)
                    arr = blend(arr, _mask([], [sp], 3), INK, 0.55 * kk)
                    arr = blend(arr, _mask([], [sp], 1), np.clip(col * 0.6 + 100, 0, 255), kk)
            sm = self.smash_at(t)
            if sm is not None:                                           # the splinters flying out
                sm_, u = sm
                tc = P.fx_clock(t)
                for s_ in sm_['sp']:
                    if not (sm_['t0'] <= tc < s_['t1'] + 0.05):
                        continue
                    tau = min(tc, s_['t1']) - sm_['t0']
                    dvec = np.array([math.cos(s_['ang']), math.sin(s_['ang'])])
                    q = sm_['c'] + dvec * s_['speed'] * tau
                    a = q - dvec * 0.22
                    seg = np.array([[a[0], a[1], R * 0.7], [q[0], q[1], R * 0.75]])
                    sp = _screen(cam, seg)
                    arr = blend(arr, _mask([], [sp], 3), np.clip(col * 0.5 + 90, 0, 255), 0.45)
                    arr = blend(arr, _mask([], [sp], 2), MAPLE * 0.8, 1.0)
        # DISTORTION crackles round its glass; TAPPING flickers two hand-sparks on it
        st = F.fs[k]
        if self.show(t) and st['vis'] and t < P.T_KO:
            ta = self.ta(t)
            fork = any(t0 <= ta < t1 for t0, t1 in self.FORK)
            tap = any(t0 <= ta < t1 for t0, t1 in self.TAP)
            p = st['p']
            if fork:
                rng = np.random.default_rng(f * 13 + 5)
                for j in range(2):
                    a0 = rng.uniform(0, 2 * math.pi)
                    arc = np.array([[p[0] + R * 1.08 * math.cos(a), p[1] + R * 1.08 * math.sin(a), R * (1.0 + 0.5 * rng.uniform(-1, 1))]
                                    for a in np.linspace(a0, a0 + 0.9, 4)])
                    arr = draw_bolt(arr, cam, bolt_path(arc[0], arc[-1], f * 17 + j, 0.35, 2), 0.7, col, 3, 2, 1)
            if tap:
                s_ = cam.wp(S.to_world(p[0], p[1], R * 1.3)[None])[0]
                for j, dx in enumerate((-4, 5)):
                    if (f + j) % 2 == 0:
                        x_, y_ = int(s_[0] + dx), int(s_[1] - 3 + 2 * j)
                        if 1 <= x_ < W - 1 and 1 <= y_ < H - 1:
                            arr[y_ - 1:y_ + 2, x_] = CORE
                            arr[y_, x_ - 1:x_ + 2] = CORE
        return arr, depth

    def glows(self, F, drawn):
        """the post tops glow neon as they strike"""
        P = self.P
        for b, age, k_ in self.live_bolts(F):
            p0, p1 = self.bolt_ends(F, b)
            s0 = F.cam.wp(S.to_world(p0[0], p0[1], p0[2])[None])[0]          # where it leaves: the headstock / the post
            F.fr.glow(s0[0], s0[1], 11 if b['src'] >= 0 else 8, tuple(P.COL[self.k] * 0.8 + 50),
                      (0.6 if b['src'] >= 0 else 0.4) * k_)

    def shimmer(self, img, cam, t, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind == 'smash':
            return fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.45, 3.0, t, k=1 - r / rmax, seed=owner)
        return img

    # ---------------------------------------------------------------- its parts (K.O. shatter + the smash)
    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        c = np.array([sx_, sz_])
        if kind == 'string':
            pts = [c + d * (j - 2) * 0.12 + nn * curl * 0.06 * math.sin(j * 1.7) for j in range(5)]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            return blend(arr, _mask([], [sp], 1), CHROME, 0.9)
        if kind == 'neck':
            ln_, wd_ = 0.30, 0.05
            q4 = [c + d * ln_ * a1 + nn * wd_ * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], MAPLE * (0.8 + 0.2 * math.cos(rot * 2)), 1.0)
            return arr
        if kind == 'pickup':
            ln_, wd_ = 0.12, 0.05
            q4 = [c + d * ln_ * a1 + nn * wd_ * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], BODY, 1.0)
            for j in (-1, 0, 1):
                s_ = cam.wp(S.to_world(*(c + d * 0.06 * j), p[1])[None])[0]
                xi, yi = int(round(s_[0])), int(round(s_[1]))
                if 0 <= xi < W and 0 <= yi < H:
                    arr[yi, xi] = CHROME
            return arr
        sz = size * 1.5                                                   # body: black chips with a neon edge
        tri = [c + d * sz, c - d * sz * 0.7 + nn * sz * 0.8, c - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        P.layer_blend(arr, [sp], BODY * (0.8 + 0.4 * abs(math.cos(rot))), 1.0)
        return blend(arr, _mask([], [np.vstack([sp, sp[:1]])], 1), P.COL[self.k] * 0.8 + 40, 0.7)


# ===================================================================================================== sound
class Sound(base.Sound):
    bus_gain, send = 1.5, 0.3
    band = (1200, 6000)                                     # its riff's bite (the fuzz above the bed's chug)
    colour_args = dict(cut0=110, cut_span=500, drive=3.0)

    def __init__(self, M, k):
        super().__init__(M, k)
        self.pp = M.CFG[k]['pp']
        self.pan = -0.22 if k == 0 else 0.22
        self._fz = {}
        lo, hi = M.CFG[k]['register']
        self.lo, self.hi = lo, hi

    # ---------------------------------------------------------------- the voice: the Pixel Pitch electric through a fuzz + cab
    def fuzz(self, x, gain=6.0, cab=4800):
        A = self.M.A
        y = np.tanh(gain * A.hp(x, 120) / (np.abs(x).max() + 1e-9))
        return A.lp(A.lp(y, cab), cab * 1.3) * 0.32

    def note(self, m, dur=0.22, cap=0.6, vel=0.85, gain=6.0, cab=4800):
        key = (int(m), round(dur, 3), round(cap, 2), round(vel, 2), round(gain, 1), int(cab))
        if key not in self._fz:
            self._fz[key] = self.fuzz(self.M.note(self.pp, int(m), dur, cap, vel), gain, cab)
        return self._fz[key]

    def voice(self, m, dur, vel=0.85):
        return self.note(m, dur, max(0.4, dur + 0.25), vel)

    def power(self, root, dur=0.3, cap=0.8, vel=0.95, gain=7.0):
        """a power chord (root + fifth + octave) through the fuzz"""
        x = sum(self.M.note(self.pp, int(m), dur, cap, vel) for m in (root, root + 7, root + 12))
        return self.fuzz(x, gain, 4200)

    def zap(self, dur=0.09, seed=0, f0=3800, f1=500):
        """a lightning zap: a crackle burst over a falling chirp"""
        A, SR = self.M.A, self.M.SR
        n = int(dur * SR)
        t = A.tt(n)
        f = f1 + (f0 - f1) * np.exp(-t / (dur * 0.3))
        chirp = np.sign(np.sin(2 * np.pi * np.cumsum(f) / SR)) * np.exp(-t / (dur * 0.4))
        crk = A.bp(np.random.default_rng(seed).normal(0, 1, n), 2500, 11000) * np.exp(-t / 0.012)
        return (0.35 * A.lp(chirp, 7000) + crk) * 0.6

    def buzz(self, dur=0.18, seed=0):
        """the ropes buzzing: a mains-hum saw with a crackle"""
        A, SR = self.M.A, self.M.SR
        n = int(dur * SR)
        t = A.tt(n)
        saw = 2 * ((t * 120) % 1.0) - 1
        crk = A.bp(np.random.default_rng(seed + 9).normal(0, 1, n), 1500, 7000) * (np.random.default_rng(seed).random(n) < 0.02)
        return (A.lp(saw, 2500) * 0.5 + crk) * np.clip(t / 0.01, 0, 1) * np.exp(-t / (dur * 0.5))

    def root_of(self, t):
        ch = self.T.chord_at(t)
        return self.T.root_above(ch, 40)

    def riff_note(self, t):
        m = self.T.line_at(self.name, t)
        while m > self.hi:
            m -= 12
        while m < self.lo:
            m += 12
        return m

    def tap_note(self, t, j):
        tones = self.T.chord_tones(self.hi - 7, self.hi + 7, self.T.chord_at(t)) or [self.hi]
        return tones[j % len(tones)]

    # ---------------------------------------------------------------- the score
    def title_blast(self, mix, t):
        """a distorted power-chord blast with a crackle of lightning (config TITLE_HITS: its notes, else the tonic)"""
        A = self.M.A
        hits = self.T.TITLE_HITS.get(self.name)
        r = hits[0] if hits else self.root_of(0.0)
        mix.place(self.bus, self.power(r, 0.9, 1.6, 1.0, 8.0), t, 0.55, pan=self.pan)
        for j in range(3):
            mix.place('fx', self.zap(0.12, 3 + j, 6000, 400), t + j * 0.045, 0.22, pan=self.pan * (1 - j))
        if hits:
            for j, m in enumerate(hits[1:]):
                mix.place(self.bus, self.note(m, 0.12, 0.4, 0.9, 7.0), t + 0.09 + j * 0.06, 0.30, pan=self.pan)

    def count_in(self, mix):
        T = self.T
        r = self.root_of(T.BAR)
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place(self.bus, self.power(r, 0.18, 0.5, 0.95), tc, 0.36, pan=self.pan)
        for st in range(12, 16):                                   # palm-muted chugs into FIGHT!
            mix.place(self.bus, self.M.A.lp(self.power(r, 0.07, 0.25, 0.8), 1500), T.BAR + st * T.S16, 0.24, pan=self.pan)

    def play(self, mix):
        M, T, k, A = self.M, self.T, self.k, self.M.A
        q16 = M.q16
        th = M.Thinner(41 + k)
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'bolt' and e[2] == k:
                tq = q16(t)
                hit, src, fork, tap = bool(e[9]), e[4], bool(e[11]), bool(e[12])
                hp01 = M.hp_at(k, t) / 100.0
                if tap:                                            # TAPPING: tapped notes high up the chord
                    m = self.tap_note(tq, int(round(tq / (T.BEAT / 2))))
                    sig, g, _ = self.colour(self.note(m, 0.16, 0.4, 0.8, 5.0, 6000), t)
                    mix.place(self.bus, sig, tq, (0.30 if hit else 0.22) * g, pan=self.pan * 0.6)
                    if hit:
                        mix.place('fx', self.zap(0.05, int(t * 31), 5000, 1500), tq, 0.08, pan=-self.pan)
                    continue
                if not th.keep(round(tq / T.S16), hp01):           # a hurt fighter's riff loses notes
                    continue
                m = self.riff_note(tq)
                if hit:
                    sig, g, _ = self.colour(self.note(m, 0.22, 0.6, 0.95, 7.0 if not fork else 9.5), t)
                    mix.place(self.bus, sig, tq, 0.40 * g, pan=self.pan)
                    if e[3] == 0 or fork:                          # the riff's first note / a forked bolt: a stab
                        mix.place(self.bus, self.colour(self.power(self.root_of(tq), 0.18, 0.5), t)[0], tq, 0.22 * g,
                                  pan=self.pan)
                    mix.place('fx', self.zap(0.09, int(t * 37)), tq, 0.10, pan=-self.pan * 0.5)
                else:                                              # a miss: palm-muted, the bolt fizzles
                    sig, g, _ = self.colour(A.lp(self.note(m, 0.10, 0.3, 0.75, 6.0), 2200), t)
                    mix.place(self.bus, sig, tq, 0.30 * g, pan=self.pan)
                    mix.place('fx', self.zap(0.06, int(t * 41), 2400, 900), tq, 0.05, pan=self.pan)
                if fork:                                           # DISTORTION: the riff doubled an octave up
                    mix.place(self.bus, self.colour(self.note(m + 12, 0.18, 0.5, 0.8, 10.0, 6500), t)[0], tq + 0.004,
                              0.18 * g, pan=-self.pan * 0.4)
                if src >= 0:                                       # a post strike: the ropes buzz into it
                    mix.place('fx', self.buzz(0.16, int(t * 13)), tq - 0.10, 0.07, pan=float(np.clip(e[5] / 4.0, -0.6, 0.6)))
            elif kind == 'ability' and e[2] == k:
                tq = q16(t)
                if e[4] == 'fork':                                 # DISTORTION: the gain kicks in with a squeal
                    x = self.note(self.hi + 7, 0.6, 1.0, 1.0, 12.0, 7000)
                    mix.place(self.bus, M.bend(x, 2.0, 0.1, 0.5), tq, 0.22, pan=self.pan)
                    mix.place('fx', self.zap(0.2, 77, 7000, 300), tq, 0.14)
                elif e[4] == 'tapping':                            # TAPPING: a quick hammer-on flourish to start it
                    for j in range(4):
                        mix.place(self.bus, self.note(self.tap_note(tq, j), 0.08, 0.25, 0.9, 5.0, 6000), tq + j * T.S16 / 2,
                                  0.20, pan=self.pan)
            elif kind == 'smash' and e[2] == k:                    # GUITAR SMASH: the crash, feedback, the snap back
                tq = q16(t)
                r = self.root_of(tq)
                mix.place(self.bus, self.power(r - 12 if r - 12 >= 40 else r, 1.2, 1.8, 1.0, 12.0), tq, 0.55, pan=self.pan)
                mix.place('fx', A.thud(0.9, 110, 30, seed=int(t * 7)), tq, 0.7)
                mix.place('fx', A.crash(1.6), tq, 0.16)
                mix.place('fx', self.zap(0.35, 5, 9000, 120), tq, 0.25)
                n = int(e[6] * M.SR)                               # feedback while it lies in pieces
                tt_ = A.tt(n)
                fb = np.sin(2 * np.pi * A.mtof(self.hi + 12) * tt_ + 3 * np.sin(2 * np.pi * 6 * tt_)) * np.clip(tt_ / 0.15, 0, 1)
                mix.place(self.bus, self.fuzz(fb, 4.0, 6000) * np.linspace(0.6, 1.0, n), tq + 0.05, 0.16, pan=self.pan)
                te = q16(t + e[6])
                for j, m in enumerate((r, r + 7, r + 12, r + 19)):  # the pickup slide up as it snaps back together
                    mix.place(self.bus, self.note(m, 0.06, 0.2, 0.9, 8.0), te - (4 - j) * 0.035, 0.22, pan=self.pan)
                mix.place(self.bus, self.power(r, 0.25, 0.6, 1.0, 9.0), te, 0.36, pan=self.pan)

    def fall_apart(self, mix):
        """the amp dies: a dive-bombed chord, crackle and the hum fading"""
        M, A = self.M, self.M.A
        t0 = M.P.T_HS
        x = self.power(self.root_of(t0), 1.4, 1.6, 1.0, 10.0)
        mix.place(self.bus, M.bend(x, -12.0, 0.05, 1.2), t0, 0.40, pan=self.pan)
        for j in range(5):
            mix.place('fx', self.zap(0.08, 60 + j, 5000 - 700 * j, 400), t0 + 0.07 * j, 0.10, pan=(j - 2) / 5)
        mix.place('fx', self.buzz(1.2, 61), t0 + 0.2, 0.10)

    def stinger(self, mix, tw):
        M = self.M
        r = self.root_of(0.0)
        x = self.power(r, 1.0, 1.8, 1.0, 9.0)
        mix.place('solo', x, tw, 0.42)
        mix.place('solo', M.bend(self.note(r + 24, 0.8, 1.2, 1.0, 10.0, 6500), -0.5, 0.4, 1.0), tw + 0.02, 0.18)
        mix.place('fx', self.zap(0.18, 88, 8000, 300), tw, 0.14)

    def solo(self, mix, t0):
        """config SOLO: a shredded line through the fuzz (bends into the long notes)"""
        T, M = self.T, self.M
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            x = self.note(m, max(0.08, d * T.BEAT * 0.95), max(0.3, d * T.BEAT + 0.2), 0.95, 8.0, 5500)
            if d >= 1.0:
                x = M.vibrato(M.gliss(x, 1.0, 0.06), 0.4, 6.0, 0.12)
            mix.place('solo', x, ts, 0.42)

    def reassemble(self, mix):
        """its pieces fly home as a rising slide, the amp humming back on"""
        T, M = self.T, self.M
        r = self.root_of(0.0)
        run = [r, r + 3, r + 5, r + 7, r + 10, r + 12, r + 15, r + 19]
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place('solo', self.note(m, 0.14, 0.4, 0.7, 6.0), tj, 0.22)
        mix.place('fx', self.buzz(T.T_GLIDE - T.T_REASSEMBLE, 62), T.T_REASSEMBLE, 0.05)
