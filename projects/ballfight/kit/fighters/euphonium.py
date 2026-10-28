"""EUPHONIUM (amber), as built in #38 (Bass Clarinet vs Euphonium): BELL MORTAR (every 2 beats the upright bell lobs
brass shells that arc high and land on the enemy's predicted spot; a shadow circle grows where each will land; splash
damage; +half a shell per landed salvo), MARCHING ORDERS (haste: a salvo every beat), IDENTITY CRISIS (copies the
enemy's last ability; for a bar its ball shows the enemy's instrument, tinted amber), NOT A TUBA (a rain of shells
across the whole ring).

Look: the shells on real arcs with smoke trails, the growing shadow circles + splash-radius rings, the amber splashes,
smoke out of the bell on every salvo, scorches only where a shell hits, IDENTITY CRISIS's flare + motes and the amber
stranger in its ball, brass / valve / bell shards. Draw code from projects/ballfight_bassclarinet_vs_euphonium/styles.py
(#38), copied here unchanged; its render.py layers as Look hooks.
Sound: the warm tenor melody (role 'melody'): every salvo plays its line's note on that beat (config LINES, 4 a bar; a
salvo every beat = the whole line), a landed salvo a fat chord tone a sixth under, the mortar's pop and the landings'
booms; MARCHING ORDERS a fanfare triplet; NOT A TUBA a falling run, one note per landing; IDENTITY CRISIS a puzzled
rising bend and a ghost of the enemy, then the copied move in the euphonium's voice. #38's audio.py, unchanged, with the
key-bound calls following the config's KEY (C = #38's notes).

Copies (new with the kit, #48): what IDENTITY CRISIS copies is drawn by the enemy's own look module (instantiated for
the euphonium's side, so it draws in amber) for the copied kinds only - e.g. the trombone's SAD TROMBONE ring and droopy
wobble, PEDAL TONE's bell - and played in the euphonium's voice (SAD TROMBONE = the plunger wah-wah down to the tonic).
#38's copies of the bass clarinet (slap, twin, shadow) sound as #38 played them; their pictures belong to the BASS
CLARINET's module once it is ported. Shells the trombone's lance swats (Rules(swat_shells=True), #48) tumble away in
sparks with a brass ping."""
import math

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, FPS, BAYER_FULL, ss
import scene as S

from . import base

R = S.R_BALL

# ---------------------------------------------------------------- #38 styles.py (BELL MORTAR), unchanged
INK = np.array([18, 10, 26], float)
WHITE = np.array([255, 252, 240], float)
BRASS = np.array([236, 186, 84], float)
BRASS_HI = np.array([255, 240, 190], float)
SMOKE = np.array([196, 188, 176], float)


def _mask(polys, lines=(), width=1):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if len(pts) >= 3:
            d.polygon([tuple(map(float, p)) for p in pts], fill=255)
    for pts in lines:
        if len(pts) >= 2:
            d.line([tuple(map(float, p)) for p in pts], fill=255, width=width)
    return np.asarray(m, np.float32) / 255.0


def blend(img, mk, col, alpha):
    if alpha <= 0:
        return img
    img += (np.asarray(col, float)[None, None] - img) * (mk * alpha)[..., None]
    return img


def wp(cam, x, z, y):
    return cam.wp(S.to_world(np.atleast_1d(x), np.atleast_1d(z), np.atleast_1d(y)))


def shell_pos(sh, t, R):
    """a shell's ring-frame position (x, z, y) at t: a straight line over the canvas, a parabola up (apex grows with the
    range), launched from the top of the euphonium's bell"""
    owner, x0, z0, x1, z1, t0, t1 = sh[:7]
    s = min(1.0, max(0.0, (t - t0) / max(1e-6, t1 - t0)))
    x, z = x0 + (x1 - x0) * s, z0 + (z1 - z0) * s
    rng_ = math.hypot(x1 - x0, z1 - z0)
    apex = 2.2 + 0.35 * rng_
    y = R * 1.7 * (1 - s) + 0.05 * s + 4 * apex * s * (1 - s)
    return x, z, y, s


def draw_shell(img, cam, sh, t, R, col):
    """a brass shell in flight: an amber slug with a hot highlight and a short grey smoke trail"""
    x, z, y, s = shell_pos(sh, t, R)
    if s >= 1.0:
        return img
    pts = [shell_pos(sh, t - dt, R) for dt in (0.0, 0.05, 0.10, 0.15)]
    sp = wp(cam, [q[0] for q in pts], [q[1] for q in pts], [q[2] for q in pts])
    trail = _mask([], [sp], width=1)
    img = blend(img, trail, SMOKE, 0.45)
    c = sp[0]
    rp = max(1.4, 0.10 * cam.f / max(1e-6, cam.depth(S.to_world(x, z, y)[None])[0]))
    xi0, xi1 = int(c[0] - rp - 1), int(c[0] + rp + 2)
    yi0, yi1 = int(c[1] - rp - 1), int(c[1] + rp + 2)
    for yi in range(max(0, yi0), min(H, yi1)):
        for xi in range(max(0, xi0), min(W, xi1)):
            dd = ((xi + 0.5 - c[0]) / (rp * 1.25)) ** 2 + ((yi + 0.5 - c[1]) / rp) ** 2
            if dd <= 1.0:
                hi = (xi + 0.5 - c[0]) < -0.2 * rp and (yi + 0.5 - c[1]) < -0.1 * rp
                img[yi, xi] = BRASS_HI if hi else np.clip(BRASS * 0.85 + col * 0.15, 0, 255)
            elif dd <= 1.6:
                img[yi, xi] = img[yi, xi] * 0.4 + INK * 0.6
    return img


def target_light(sxp, szp, sh, t, splash):
    """canvas decals of one incoming shell: (shade, ring) - the shadow circle growing and darkening as it falls, and a
    thin ring in the owner's colour at the splash radius"""
    owner, x0, z0, x1, z1, t0, t1 = sh[:7]
    s = (t - t0) / max(1e-6, t1 - t0)
    if not 0.0 <= s < 1.0:
        return None, None
    d = np.hypot(sxp - x1, szp - z1)
    rr = 0.12 + (splash * 0.85 - 0.12) * s
    shade = np.clip(1 - d / rr, 0, 1) ** 0.6 * (0.25 + 0.6 * s)
    ring = np.clip(1 - np.abs(d - splash) / 0.035, 0, 1) * (0.3 + 0.6 * s)
    return shade, ring


def splash_light(sxp, szp, x, z, age, splash):
    """canvas decal: a landed shell's splash - a ring racing out to the splash radius, fading over 0.4 s"""
    if not 0 <= age < 0.4:
        return None
    d = np.hypot(sxp - x, szp - z)
    r = splash * min(1.0, age / 0.12) ** 0.5
    return np.clip(1 - np.abs(d - r) / 0.10, 0, 1) * (1 - age / 0.4)


# ---------------------------------------------------------------- #48: a swatted shell (Rules(swat_shells=True))
SWAT_T = 0.30                    # s a swatted shell tumbles away before it's gone


def swat_pos(sh, t, R):
    """a swatted shell knocked off its arc: from where the lance met it, it flies on away from the lance, up a little and
    falling, spinning out over SWAT_T s"""
    ts = sh[10]
    x, z, y, s = shell_pos(sh, ts, R)
    owner, x0, z0, x1, z1 = sh[:5]
    d = np.array([x1 - x0, z1 - z0])
    d = d / (np.hypot(*d) + 1e-9)
    u = t - ts
    side = np.array([-d[1], d[0]]) * (1 if sh[8] % 2 else -1)        # knocked sideways off its line
    p = np.array([x, z]) + (side * 2.4 - d * 0.8) * u
    return p[0], p[1], y + 2.2 * u - 9.0 * u * u


def draw_swatted(img, cam, sh, t, R, col):
    """a swatted shell spinning away off its arc: the brass slug tumbling end over end (its long axis turning), a
    bright glint each half turn, a short spark trail back to where the lance met it"""
    u = t - sh[10]
    x, z, y = swat_pos(sh, t, R)
    if y <= 0.02:
        return img
    pts = [swat_pos(sh, max(sh[10], t - dt), R) for dt in (0.0, 0.04, 0.08)]
    sp = wp(cam, [q[0] for q in pts], [q[1] for q in pts], [q[2] for q in pts])
    img = blend(img, _mask([], [sp], width=1), BRASS_HI, 0.6 * (1 - u / SWAT_T))
    c = sp[0]
    rp = max(1.6, 0.11 * cam.f / max(1e-6, cam.depth(S.to_world(x, z, y)[None])[0]))
    a = u * 38.0                                                  # tumbling: ~6 turns a second
    ca, sa = math.cos(a), math.sin(a)
    glint = math.cos(2 * a) > 0.6
    for yi in range(max(0, int(c[1] - rp - 2)), min(H, int(c[1] + rp + 3))):
        for xi in range(max(0, int(c[0] - rp - 2)), min(W, int(c[0] + rp + 3))):
            dx, dy = xi + 0.5 - c[0], yi + 0.5 - c[1]
            ex, ey = dx * ca + dy * sa, -dx * sa + dy * ca
            dd = (ex / (rp * 1.35)) ** 2 + (ey / (rp * 0.8)) ** 2
            if dd <= 1.0:
                img[yi, xi] = WHITE if glint and ex < 0 else (BRASS_HI if ex < -0.3 * rp else
                                                              np.clip(BRASS * 0.85 + col * 0.15, 0, 255))
            elif dd <= 1.7:
                img[yi, xi] = img[yi, xi] * 0.4 + INK * 0.6
    return img


class Look(base.Look):
    shards = ['brass'] * 8 + ['valve'] * 4 + ['bell'] * 2
    spark_big = ()
    flash_hits = ()
    fringe_hits = ()
    rope_kinds = {'barrage': 0.5}

    def setup(self):
        P, k = self.P, self.k
        EV, CFG = P.EV, P.CFG
        w = CFG[k]['weapon']
        low = CFG[k]['abilities']['low']
        self.SPLASH = {'mortar': w['splash'], 'barrage': low['splash'] if low['kind'] == 'barrage' else w['splash']}
        self.SALVOS = [(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'mortar' and e[2] == k]      # t, k, n, salvo
        self.LANDS = [(e[0], e[2], e[3], e[4], e[5], e[6], e[7], e[8]) for e in EV if e[1] == 'shell' and e[2] == k]
        self.LAND_T = np.array([q[0] for q in self.LANDS])
        self.BARRAGE_SALVOS = {e[4] for e in EV if e[1] == 'barrage' and e[2] == k}
        self.COPIES = [(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'copy' and e[2] == k]      # t, k, name, kind
        self.SWATS = [e for e in EV if e[1] == 'swat_shell' and e[3] == k]
        # IDENTITY CRISIS: the copied kinds the enemy's own look draws (for this side, in amber)
        own = {'mortar', 'barrage', 'haste', 'copy'}
        self.copied = {kind for t, kk, name, kind in self.COPIES if t < P.T_KO and kind not in own}
        self.dup = None
        if self.copied:
            from . import module
            m = module(P.NAMES[self.enemy])
            if m is not None:
                self.dup = m.Look(P, k)
                self.dup.cfg = CFG[self.enemy]                   # the copied abilities' numbers are the enemy's
                self.dup.setup()
                self.flash_hits = tuple(self.dup.flash_hits)
                self.fringe_hits = tuple(self.dup.fringe_hits)

    def _dup_kind(self, how):
        return self.dup is not None and how not in ('shell',)

    # ---------------------------------------------------------------- IDENTITY CRISIS in the ball
    def crisis_at(self, t):
        """for a bar after a copy the euphonium's ball shows the enemy's instrument, tinted (0..1)"""
        T = self.T
        for t0, kk, name, kind in self.COPIES:
            if t0 <= t < t0 + T.BAR and t0 < self.P.T_KO:
                return min(1.0, (t - t0) / 0.08) * (1 - ss(t0 + T.BAR - 0.3, t0 + T.BAR, t))
        return 0.0

    def state(self, kk, st, t, phase, gl=0.0):
        if kk != self.k:
            return
        st['crisis'] = 0.0
        if phase == 'fight':
            st['crisis'] = self.crisis_at(t)
            if st['crisis'] > 0:
                st['glow'] = max(st['glow'], 0.5 * st['crisis'])

    def ball_sprite(self, kk, st, t, rp, yaw):
        if st.get('crisis', 0.0) <= 0.5:
            return None
        P, e = self.P, self.enemy
        col = P.COL[self.k]
        spr = P.sprite(e, rp * 0.80, yaw, P.CFG[e]['view'][2])
        return spr, (lambda lit: lit * 0.55 + col[None, None] * 0.55)    # an amber stranger: who am I?

    def extra_sprites(self):
        if not self.COPIES:
            return []
        yc, sw, _ = self.cfg['view']
        return [(self.enemy, yc, sw, self.P.CFG[self.enemy]['view'][2])]

    def hides_hp(self, kk, t):
        return self.dup.hides_hp(kk, t) if self.dup is not None else False

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit' and e[7] == 'shell':
            return 0.5
        if self.dup is not None and (e[1] != 'hit' or self._dup_kind(e[7])):
            return self.dup.shake(e)
        return super().shake(e)

    def rope(self, e):
        if e[1] == 'copy' and self.dup is not None:
            return self.dup.rope_kinds.get(e[4])
        return super().rope(e)

    def spark(self, e):
        if e[7] == 'shell':
            return self.P.COL[e[2]] * 0.6 + 100, 7, 2.2, 0.22
        if self._dup_kind(e[7]):
            return self.dup.spark(e)
        return super().spark(e)

    def hit_glow(self, e):
        if e[7] == 'shell':
            return 8, 0.2
        if self._dup_kind(e[7]):
            return self.dup.hit_glow(e)
        return None

    def fringe(self, e):
        return e[7] in self.fringe_hits

    def fringe_size(self, e):
        return self.dup.fringe_size(e) if self.dup is not None else super().fringe_size(e)

    # ---------------------------------------------------------------- effects (#38's _fx_build, in its order)
    def _top(self, t_):
        P = self.P
        p = P.RUN['P'][P.sim_i(t_), self.k]
        return S.to_world(p[0], p[1], R * 1.7)

    def fx_salvos(self):
        P = self.P
        for t_, k, n, salvo in self.SALVOS:                       # BELL MORTAR: smoke out of the bell
            if t_ > P.T_KO:
                break
            P.PA.emit(t_, self._top(t_), 'smoke', n=3 + min(4, n), col=SMOKE_C, scale=0.8)
            P.FL.add(t_, self._top(t_), P.COL[k], radius=1.2, peak=0.35, decay=0.2)

    def fx_lands(self):
        P, up = self.P, self.P.up
        for t_, k, x, z, hit, salvo, j, kind in self.LANDS:       # the shells land
            if t_ > P.T_KO:
                break
            pos = up(x, z, 0.08)
            P.PA.emit(t_, pos, 'dust', n=2 if kind == 'barrage' else 3, col=[(196, 178, 150), (150, 132, 110)])
            if hit:                                               # only a hit scorches the canvas (misses just kick dust)
                P.MK.add(t_, 'scorch', x, z, size=0.30, col=(44, 34, 30), life=2.5, n=1, seed=int(t_ * 100) + j,
                         strength=0.5)
                P.FL.add(t_, up(x, z, 0.4), P.COL[k], radius=1.6, peak=0.7, decay=0.25)
                P.PA.emit(t_, up(x, z, 0.3), 'sparks', n=8, col=BRASS_SPARK)
            elif j == 0:
                P.FL.add(t_, up(x, z, 0.3), P.COL[k], radius=1.2, peak=0.35, decay=0.2)

    def fx_copies(self):
        P = self.P
        for t_, k, name, kind in self.COPIES:                     # IDENTITY CRISIS: an amber flare, question-mark motes
            if t_ > P.T_KO:
                break
            P.FL.add(t_, self._top(t_), P.COL[k], radius=1.8, peak=0.7, decay=0.35)
            P.PA.emit(t_, self._top(t_), 'motes', n=10, col=BRASS_SPARK + SLATE_SPARK)

    def fx_swats(self):
        P, up = self.P, self.P.up
        for e in self.SWATS:                                      # #48: a lance swats a shell away: sparks, a flare
            if e[0] > P.T_KO:
                break
            pos = up(e[6], e[7], R)
            P.BU.add(e[0], pos, P.COL[e[2]] * 0.5 + 120, 1.0)
            P.FL.add(e[0], pos, P.FL_COL[e[2]], radius=1.4, peak=0.6, decay=0.2)
            P.PA.emit(e[0], pos, 'sparks', n=12, col=BRASS_SPARK)

    def fx_setup(self):
        self.fx_salvos()
        self.fx_lands()
        self.fx_copies()
        self.fx_swats()
        if self.dup is not None:
            self.dup.fx_setup()

    def fx_event(self, e):
        P = self.P
        if e[1] == 'hit' and e[7] == 'shell':
            P.BU.add(e[0], P.up(e[5], e[6], R * 0.6), P.COL[e[2]], 0.8)
        elif self.dup is not None and (e[1] != 'hit' or self._dup_kind(e[7])):
            self.dup.fx_event(e)                                  # a copied move's effects (the enemy's, in amber)

    def fx_after(self):
        if self.dup is not None:
            self.dup.fx_after()

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'sparks', n=26, spread=180, speed=(0.8, 2.2),
                       col=[(255, 214, 120), (240, 172, 32), (214, 216, 226)])

    # ---------------------------------------------------------------- the canvas
    def live_shells(self, t):
        """its shells showing at t (in flight or landed < 0.6 s ago), from the recorded SHELLS"""
        P = self.P
        if not (P.T.T_FIGHT <= t < P.T_HS) or 'SHELLS' not in P.RUN:
            return []
        return [sh for sh in P.RUN['SHELLS'][P.sim_i(min(t, P.T_KO))] if sh[0] == self.k]

    def recent_lands(self, t, span=0.4):
        i0, i1 = np.searchsorted(self.LAND_T, t - span), np.searchsorted(self.LAND_T, t, 'right')
        return [self.LANDS[i] for i in range(i0, i1) if self.LANDS[i][0] <= self.P.T_KO]

    def canvas(self, Ic, C):
        return self.dup.canvas(Ic, C) if self.dup is not None else Ic

    def canvas_wave(self, Ic, C, w):
        return self.dup.canvas_wave(Ic, C, w) if self.dup is not None else Ic

    def canvas_after(self, Ic, C):
        """the shells' growing shadows + splash-radius rings, the splashes, then the shadows darken it all (#38 did the
        darkening last, after every canvas light)"""
        P, k = self.P, self.k
        if self.dup is not None:
            Ic = self.dup.canvas_after(Ic, C)
        if not C.in_fight:
            return Ic
        t = C.t
        shade_c = np.zeros(len(C.xs))
        for sh in self.live_shells(t):
            if len(sh) > 10 and t >= sh[10]:                      # #48: swatted: its target circle is gone
                continue
            kind = 'barrage' if sh[7] in self.BARRAGE_SALVOS else 'mortar'
            sd, ring = target_light(C.sxp, C.szp, sh, t, self.SPLASH[kind])
            if sd is not None:
                shade_c = np.maximum(shade_c, sd * 0.7)
                ring = np.floor(ring * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
                Ic += (ring * 0.9)[:, None] * (P.COL[sh[0]] / 255.0 * 1.2 + 0.2)[None]
        for t_, kk, x, z, hit, salvo, j, kind in self.recent_lands(t):
            sp_ = splash_light(C.sxp, C.szp, x, z, t - t_, self.SPLASH[kind])
            if sp_ is not None:
                Ic += (sp_ * (1.1 if hit else 0.6))[:, None] * (P.COL[kk] / 255.0 * 1.2 + 0.25)[None]
        Ic *= (1 - shade_c)[:, None]
        return Ic

    # ---------------------------------------------------------------- the shells in the air, shards
    def over_ring(self, F, arr, depth):
        P = self.P
        if self.dup is not None:
            arr, depth = self.dup.over_ring(F, arr, depth)
        tt = min(F.t, P.T_KO)
        for sh in (self.live_shells(F.t) if F.in_fight else []):
            if len(sh) > 10 and tt >= sh[10]:                     # #48: swatted: it tumbles away and is gone
                u = tt - sh[10]
                if u < SWAT_T:
                    arr = draw_swatted(arr, F.cam, sh, tt, R, P.COL[sh[0]])
                continue
            arr = draw_shell(arr, F.cam, sh, tt, R, P.COL[sh[0]])
        return arr, depth

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'brass':                                       # a length of tubing
            ln_, wd_ = 0.22, 0.045
            q4 = [np.array([sx_, sz_]) + d * ln_ * a1 + nn * wd_ * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], BRASS, 1.0)
            return arr
        col = {'valve': BRASS_HI, 'bell': BRASS}[kind]            # valve (a chip), bell (a big curved chip)
        sz = size * (0.7 if kind == 'valve' else 1.6)
        tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
               np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        shade_k = 0.75 + 0.25 * math.cos(rot * 2)
        P.layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
        return arr

    # ---------------------------------------------------------------- lens
    def shimmer(self, img, cam, t, w):
        return self.dup.shimmer(img, cam, t, w) if self.dup is not None else img

    def shimmer_after(self, img, cam, t):
        return self.dup.shimmer_after(img, cam, t) if self.dup is not None else img

    def focus_points(self, t, st):
        if self.dup is None:
            return []
        return [(p, lab) for p, lab in self.dup.focus_points(t, st) if lab != 'lance tip']


SMOKE_C = [(150, 144, 136), (190, 182, 170), (120, 116, 112)]
BRASS_SPARK = [(255, 214, 120), (240, 172, 32), (255, 246, 210)]
SLATE_SPARK = [(196, 210, 236), (110, 132, 170), (240, 244, 255)]


# ===================================================================================================== sound (#38)
_VC = {}
_MODS = {}


def _pp(name):
    if name not in _MODS:
        import pp_bridge as B
        _MODS[name] = B.module(name)
    return _MODS[name]


class Sound(base.Sound):
    bus_gain, send = 1.3, 0.35
    band = (150, 1200)                                            # the tenor line G3-C5 + its brass harmonics
    colour_args = dict(cut0=40, cut_span=200, drive=4.0)          # #38's colour()

    # ---------------------------------------------------------------- its voice (#38's eup(): level-matched)
    def eup(self, m, dur, vel=0.85):
        key = ('e', round(m, 3), round(dur, 3), round(vel, 2))
        if key not in _VC:
            import pp_bridge as B
            from reelkit import audio as A
            EUM = _pp('euphonium_lowpoly')
            x = EUM.euph(EUM._midi_hz(m), dur, vel)
            x = np.asarray(B._resample(np.asarray(x, float)), float)
            n = int((dur + 0.16) * A.SR)                          # #38's _fit (cap = dur + 0.16, 30 ms fade)
            x = x[:n].copy()
            if len(x) < n:
                x = np.pad(x, (0, n - len(x)))
            fd = min(len(x) // 3, int(0.03 * A.SR))
            x[-fd:] *= np.linspace(1, 0, fd)
            r = np.sqrt((x[:int(0.25 * A.SR)] ** 2).mean()) + 1e-9   # #38's _norm (ref 0.16)
            _VC[key] = x * (0.16 / r)
        return _VC[key]

    def voice(self, m, dur, vel=0.85):
        return self.eup(m, dur, vel)

    def boom(self, seed=0, dur=0.5):
        """a shell landing: a short thud + a crunchy noise burst (#38)"""
        from reelkit import audio as A
        n = int(dur * A.SR)
        t = A.tt(n)
        th = A.lp(A.thud(dur, 110, 40, seed=seed)[:n], 140)
        th = np.pad(th, (0, n - len(th)))
        crunch = A.bp(np.random.default_rng(seed + 9).normal(0, 1, n), 900, 5000) * np.exp(-t / 0.05)
        return th + 0.5 * crunch

    def slap(self, m, vel=1.0):
        """#38's bass-clarinet slap (a copied SLAP TONGUE's crack)"""
        key = ('s', round(m, 2), round(vel, 2))
        if key not in _VC:
            import pp_bridge as B
            BCM = _pp('bassclarinet_lowpoly')
            _VC[key] = B._resample(np.asarray(BCM.slap(BCM._midi_hz(m), vel), float)) * 0.9
        return _VC[key]

    # ---------------------------------------------------------------- the key (C minor = #38's notes)
    def top(self):
        """the key's tonic at the top of its register (C5 = 72 in #38)"""
        hi = self.M.CFG[self.k]['register'][1]
        return hi - (hi - self.T.KEY_PC) % 12

    def melody_at(self, t):
        return self.T.line_at(self.name, t)

    def windows(self, kind):
        """[(t0, t1)] its own and copied abilities of a kind (MARCHING ORDERS: the haste)"""
        M, k = self.M, self.k
        out = []
        for e in M.EV:
            if e[0] > M.T_KO + 0.01:
                break
            if e[1] == 'ability' and e[2] == k and e[4] == kind:
                ab = [a for a in M.CFG[k]['abilities'].values() if a['name'] == e[3]][0]
                out.append((e[0], e[0] + (ab['dur'] if 'dur' in ab else ab['dur_beats'] * self.T.BEAT)))
            elif e[1] == 'copy' and e[2] == k and e[4] == kind:
                abs_ = [a for a in M.CFG[self.enemy]['abilities'].values() if a['name'] == e[3]]
                if abs_:
                    ab = abs_[0]
                    out.append((e[0], e[0] + (ab['dur'] if 'dur' in ab else ab['dur_beats'] * self.T.BEAT)))
        return out

    def twin_on(self, t):
        return any(e[1] == 'twin' and e[2] == self.k and e[0] <= t < e[3] for e in self.M.EV)

    # ---------------------------------------------------------------- the score
    def title_blast(self, mix, t):
        top = self.top()
        hits = self.T.TITLE_HITS.get(self.name, (top - 12, top - 5, top - 24))     # a blast, a fat fifth (#38)
        for m, g_ in zip(hits, (0.50, 0.36, 0.30, 0.26, 0.22)):
            mix.place(self.bus, self.eup(m, 1.5 * self.T.BEAT, 1.0), t, g_, pan=0.2)

    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        A, EV, q16 = M.A, M.EV, M.q16
        P = M.P
        top = self.top()
        look = P.LOOKS[k]
        haste_w = self.windows('haste')
        dropped = False
        lands = {}
        for t, kk, x, z, hit, salvo, j, kind in look.LANDS:
            if t > M.T_KO + 0.01:
                break
            q = lands.setdefault((round(q16(t) / T.S16), kk, kind), dict(t=t, n=0, hits=0, kind=kind, salvo=salvo))
            q['n'] += 1
            q['hits'] += int(hit)
        for t0, kk, n, salvo in look.SALVOS:                      # a salvo: the line's note on its beat
            if t0 > M.T_KO + 0.01:
                break
            tq = q16(t0)
            slot = round(tq / T.S16)
            hp = M.hp_at(k, t0) / 100.0
            haste = any(a <= t0 < b for a, b in haste_w)
            dropped = M.thin_37(slot, 11, hp, dropped)
            m = self.melody_at(tq)
            if dropped and haste:                                 # thinned: a note of the every-beat run drops out
                continue
            dur = (0.85 if haste else 1.7) * T.BEAT
            if dropped:                                           # (on the 2-beat grid it is clipped, not dropped:
                dur = 0.5 * T.BEAT                                # no hole while it is still firing)
            sig, g, lead = self.colour(self.eup(m, dur, 0.75 + 0.04 * min(n, 5)), t0)
            mix.place(self.bus, sig, tq, (0.26 if dropped else 0.40) * g, pan=0.2)
            if self.twin_on(t0):                                  # a copied MULTIPHONICS: two notes at once
                mix.place(self.bus, self.colour(self.eup(m + 7, dur, 0.6), t0)[0], tq, 0.20 * g, pan=0.25)
            if lead > 0.04:                                       # the leader: a third under (fuller)
                under = T.chord_tones(m - 5, m - 3, T.chord_at(tq))
                if under:
                    mix.place(self.bus, self.colour(self.eup(under[-1], dur, 0.55), t0)[0], tq, 0.15 * g, pan=0.25)
            mix.place('fx', A.thud(0.18, 220, 90, seed=int(t0 * 11)), tq, 0.06 + 0.01 * min(n, 6))   # the mortar's pop
        for (slot, kk, kind), q in sorted(lands.items()):
            t = q['t']
            tq = slot * T.S16
            g_n = min(1.0, 0.5 + 0.12 * q['n'])
            mix.place('fx', self.boom(int(t * 13) % 1000, 0.45), tq, (0.16 if q['hits'] else 0.09) * g_n, pan=0.1)
            if q['hits'] and kind == 'mortar':                    # landed: a fat chord tone a sixth under
                m = self.melody_at(tq)
                tones = T.chord_tones(m - 10, m - 7, T.chord_at(tq))
                if tones:
                    sig, g, _ = self.colour(self.eup(tones[-1], 0.35, 0.95), t)
                    mix.place(self.bus, sig, tq, 0.30 * g, pan=0.25)
        ab = M.CFG[k]['abilities']
        es = M.SOUNDS[self.enemy]
        for e in EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'barrage' and e[2] == k:                   # NOT A TUBA: a falling run, one note per landing
                tq = q16(t)
                mix.place(self.bus, self.eup(top, 1.0 * T.BEAT, 1.0), tq, 0.46, pan=0.2)
                mix.place(self.bus, self.eup(top - 12, 1.0 * T.BEAT, 0.9), tq, 0.30, pan=0.2)
                run = [l for l in look.LANDS if l[5] == e[4]]
                seq = tuple(top - d for d in (0, 2, 4, 5, 7, 9, 10, 12))    # down the natural minor (#38: C5 .. C4)
                seen = []
                for l in run:
                    tl = q16(l[0])
                    if tl in seen:
                        continue
                    seen.append(tl)
                    m = seq[min(len(seen) - 1, len(seq) - 1)]
                    mix.place(self.bus, self.colour(self.eup(m, 0.4 * T.BEAT, 0.95), l[0])[0], tl, 0.34, pan=0.2)
                mix.place('fx', M.thunder(1.4, 44, crack=False), tq, 0.30)
            elif kind == 'copy' and e[2] == k:                    # IDENTITY CRISIS: "wait... who am I?"
                tq = q16(t)
                if e[4] == 'shockwave':                           # a copied SAD TROMBONE: the wah-wah in its own voice
                    self.wah_wah(mix, tq, t)
                    continue
                x = self.eup(top - 10, 0.8 * T.BEAT, 0.9)
                mix.place(self.bus, M.bend(x, 5.0, 0.08, 0.8 * T.BEAT), tq, 0.40, pan=0.2)
                lo = M.CFG[self.enemy]['register'][0]             # a ghost of the enemy (#38: a bass-clarinet growl)
                mix.place(es.bus, es.voice(lo + 3, 0.3, 0.6), tq + T.BEAT * 0.5, 0.14, pan=0.25)
                if e[4] == 'haste':                               # a copied rip (POWER SLIDE): up an octave into it
                    mix.place(self.bus, self.colour(M.gliss(self.eup(top - 5, 0.45, 1.0), 12.0, 0.3), t)[0],
                              tq + T.BEAT, 0.30, pan=0.2)
                elif e[4] == 'slam':                              # a copied PEDAL TONE: its low pedal under the bell
                    tb = T.BEAT * self.M.CFG[self.enemy]['abilities']['low'].get('tele', 2)
                    mix.place(self.bus, self.eup(top - 24, tb, 1.0), tq, 0.40, pan=0.2)
                    mix.place('fx', A.whoosh(tb, 150, 1800), tq, 0.10)
            elif kind == 'slam' and e[2] == k:                    # ... and its bell stomps
                ts = e[0]
                M.bed.boom(mix, ts, 0.8)
                mix.place('fx', A.thud(0.8, 90, 30), ts, 0.5)
                mix.place('fx', M.clank(64, 9), ts, 0.10)
                mix.place(self.bus, self.eup(top - 12, 0.5, 1.0), ts, 0.36, pan=0.2)
            elif kind == 'snip' and e[2] == k:                    # a copied SLAP TONGUE: a brass slap
                tq = q16(t)
                mix.place(self.bus, self.eup(self.melody_at(tq) - 12, 0.12, 1.0), tq, 0.46, pan=0.2)
                mix.place('fx', self.slap(72, 0.8) * 0.6, tq, 0.25)
                if e[3]:
                    mix.place('fx', A.thud(0.35, 180, 60, seed=int(t * 7)), tq, 0.26)
            elif kind == 'ability' and e[2] == k and e[3] == ab['unlock1']['name'] and ab['unlock1']['kind'] == 'haste':
                tq = q16(t)                                       # MARCHING ORDERS: a fanfare triplet + a roll
                for j, m in enumerate((top - 5, top - 5, top)):
                    mix.place(self.bus, self.eup(m, 0.13 if j < 2 else 0.5, 1.0), tq + j * T.BEAT / 3, 0.40, pan=0.2)
                if hasattr(M.bed, 'press_roll'):                  # (a bed with a snare: #38's drumline roll)
                    M.bed.press_roll(mix, tq, tq + T.BEAT, 0.10, 0.26)
            elif kind == 'lights_out' and e[2] == k:              # a copied LIGHTS OUT
                mix.place(self.bus, self.eup(top - 26, e[3] - t, 0.9), q16(t), 0.34, pan=0.2)
            elif kind == 'erupt' and e[2] == k:
                mix.place(self.bus, self.eup(top - 14, 0.4, 1.0), q16(t), 0.5, pan=0.2)
                mix.place('fx', M.thunder(1.4, int(t * 10)), q16(t), 0.4)
            elif kind == 'swat_shell' and e[3] == k:              # #48: a shell swatted out of the air: a brass ping
                mix.place('fx', M.clank(88 + (e[5] % 3) * 2, int(t * 17)), t, 0.07, pan=float(np.clip(e[6] / 3.5, -0.6, 0.6)))
                mix.place('fx', A.thud(0.12, 260, 120, seed=int(t * 5)), t, 0.04)

    def wah_wah(self, mix, tq, t):
        """IDENTITY CRISIS copies SAD TROMBONE: the euphonium plays the plunger wah-wah down to the tonic, in amber
        (#48: for a moment the euphonium IS a trombone)"""
        M, T = self.M, self.T
        m0 = self.top() - 12
        while m0 - 1 < self.M.CFG[self.k]['register'][0]:
            m0 += 12
        for j, m in enumerate((m0 + 3, m0 + 2, m0 + 1, m0)):
            d = 0.5 * T.BEAT if j < 3 else 1.6 * T.BEAT
            x = M.gliss(self.eup(m, d, 0.9), 1.0 if j else 0.0, 0.04) if j else self.eup(m, d, 0.9)
            if j == 3:
                x = M.vibrato(x, 0.5, 5.0, 0.2)
            x = M.wah(x, [(0.0, min(0.18, d * 0.6))])
            mix.place(self.bus, self.colour(x, t)[0], tq + j * 0.5 * T.BEAT, 0.42, pan=0.2)
        mix.place('fx', M.A.whoosh(0.6, 200, 1500, rise=False), tq, 0.08)

    def fall_apart(self, mix):                                    # a falling blast, valves (#38)
        M = self.M
        x = self.eup(self.top() - 14, 1.0, 1.0)
        mix.place(self.bus, M.bend(x, -9.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), M.P.T_HS, 0.42)

    def stinger(self, mix, tw):
        top = self.top()
        mix.place('solo', self.eup(top - 7, 0.9, 1.0), tw, 0.45)
        mix.place('solo', self.eup(top, 0.9, 0.9), tw, 0.30)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            mix.place('solo', self.eup(m, d * T.BEAT * 0.95, 0.95), ts, 0.52)

    def reassemble(self, mix):
        T, M = self.T, self.M
        run = [self.top() + d for d in (-17, -14, -12, -9, -5, 0)]      # #38: G3 Bb3 C4 Eb4 G4 C5
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place(self.bus, self.eup(m, 0.14, 0.7), tj, 0.22)

    def last_bar(self, mix):
        """revving in its corner: the melody builds into the frame-0 slam (#38: from the second-last bar's beat 2)"""
        T = self.T
        t30 = (T.N_BARS - 2) * T.BAR
        for j in range(1, 8):
            tb = t30 + j * T.BEAT
            mix.place(self.bus, self.eup(self.melody_at(tb), 0.9 * T.BEAT, 0.85), tb, 0.36, pan=0.2)
